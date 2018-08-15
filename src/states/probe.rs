// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: MPL-2.0
//

use client::Api;
use failure::{Error, ResultExt};
use firmware::Metadata;
use runtime_settings::RuntimeSettings;
use settings::Settings;
use states::{download::Download, idle::Idle, poll::Poll, State};

#[derive(Debug, PartialEq)]
pub(crate) struct Probe {
    pub(crate) settings: Settings,
    pub(crate) runtime_settings: RuntimeSettings,
    pub(crate) firmware: Metadata,
    pub(crate) applied_package_uid: Option<String>,
}

/// Implements the state change for State<Probe>.
impl State for Probe {
    fn handle(self: Box<Self>) -> Result<Box<State>, Error> {
        let s = *self; // Drop when NLL is stable
        let settings = s.settings;
        let mut runtime_settings = s.runtime_settings;
        let firmware = s.firmware;
        let applied_package_uid = s.applied_package_uid;

        use chrono::Duration;
        use client::ProbeResponse;
        use std::thread;

        let r = loop {
            let probe = Api::new(&settings, &runtime_settings, &firmware).probe();
            if let Err(e) = probe {
                error!("{}", e);
                runtime_settings.polling.retries += 1;
                thread::sleep(Duration::seconds(1).to_std().unwrap());
            } else {
                runtime_settings.polling.retries = 0;
                break probe?;
            }
        };

        runtime_settings.polling.extra_interval = match r {
            ProbeResponse::ExtraPoll(s) => {
                info!("Delaying the probing as requested by the server.");
                Some(Duration::seconds(s))
            }
            _ => None,
        };

        // Save any changes we due the probing
        if !settings.storage.read_only {
            debug!("Saving runtime settings.");
            runtime_settings
                .save()
                .context("Saving runtime due probe changes")?;
        } else {
            debug!("Skipping runtime settings save, read-only mode enabled.");
        }

        match r {
            ProbeResponse::NoUpdate => {
                debug!("Moving to Idle state as no update is available.");
                Ok(Box::new(Idle {
                    settings,
                    runtime_settings,
                    firmware,
                    applied_package_uid,
                }))
            }

            ProbeResponse::ExtraPoll(_) => {
                debug!("Moving to Poll state due the extra polling interval.");
                Ok(Box::new(Poll {
                    settings,
                    runtime_settings,
                    firmware,
                    applied_package_uid,
                }))
            }

            ProbeResponse::Update(u) => {
                // Ensure the package is compatible
                u.compatible_with(&firmware)?;

                if Some(u.package_uid()) == applied_package_uid {
                    info!(
                        "Not applying the update package. Same package has already been installed."
                    );
                    debug!("Moving to Idle state as this update package is already installed.");
                    Ok(Box::new(Idle {
                        settings,
                        runtime_settings,
                        firmware,
                        applied_package_uid,
                    }))
                } else {
                    debug!("Moving to Download state to process the update package.");
                    Ok(Box::new(Download {
                        settings,
                        runtime_settings,
                        firmware,
                        update_package: u,
                    }))
                }
            }
        }
    }
}

#[test]
fn update_not_available() {
    use super::*;
    use client::tests::{create_mock_server, FakeServer};
    use firmware::tests::{create_fake_metadata, FakeDevice};
    use std::fs;
    use tempfile::NamedTempFile;

    let tmpfile = NamedTempFile::new().unwrap();
    let tmpfile = tmpfile.path();
    fs::remove_file(&tmpfile).unwrap();

    let mock = create_mock_server(FakeServer::NoUpdate);

    let machine = Box::new(Probe {
        settings: Settings::default(),
        runtime_settings: RuntimeSettings::new()
            .load(tmpfile.to_str().unwrap())
            .unwrap(),
        firmware: Metadata::new(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap(),
        applied_package_uid: None,
    }).handle();

    mock.assert();

    assert_state!(machine, Idle);
}

#[test]
fn update_available() {
    use super::*;
    use client::tests::{create_mock_server, FakeServer};
    use firmware::tests::{create_fake_metadata, FakeDevice};
    use std::fs;
    use tempfile::NamedTempFile;

    let tmpfile = NamedTempFile::new().unwrap();
    let tmpfile = tmpfile.path();
    fs::remove_file(&tmpfile).unwrap();

    let mock = create_mock_server(FakeServer::HasUpdate);

    let machine = Box::new(Probe {
        settings: Settings::default(),
        runtime_settings: RuntimeSettings::new()
            .load(tmpfile.to_str().unwrap())
            .unwrap(),
        firmware: Metadata::new(&create_fake_metadata(FakeDevice::HasUpdate)).unwrap(),
        applied_package_uid: None,
    }).handle();

    mock.assert();

    assert_state!(machine, Download);
}

#[test]
fn invalid_hardware() {
    use super::*;
    use client::tests::{create_mock_server, FakeServer};
    use firmware::tests::{create_fake_metadata, FakeDevice};
    use std::fs;
    use tempfile::NamedTempFile;

    let tmpfile = NamedTempFile::new().unwrap();
    let tmpfile = tmpfile.path();
    fs::remove_file(&tmpfile).unwrap();

    let mock = create_mock_server(FakeServer::InvalidHardware);

    let machine = Box::new(Probe {
        settings: Settings::default(),
        runtime_settings: RuntimeSettings::new()
            .load(tmpfile.to_str().unwrap())
            .unwrap(),
        firmware: Metadata::new(&create_fake_metadata(FakeDevice::InvalidHardware)).unwrap(),
        applied_package_uid: None,
    }).handle();

    mock.assert();

    assert!(machine.is_err(), "Did not catch an incompatible hardware");
}

#[test]
fn extra_poll_interval() {
    use super::*;
    use client::tests::{create_mock_server, FakeServer};
    use firmware::tests::{create_fake_metadata, FakeDevice};
    use std::fs;
    use tempfile::NamedTempFile;

    let tmpfile = NamedTempFile::new().unwrap();
    let tmpfile = tmpfile.path();
    fs::remove_file(&tmpfile).unwrap();

    let mock = create_mock_server(FakeServer::ExtraPoll);

    let machine = Box::new(Probe {
        settings: Settings::default(),
        runtime_settings: RuntimeSettings::new()
            .load(tmpfile.to_str().unwrap())
            .unwrap(),
        firmware: Metadata::new(&create_fake_metadata(FakeDevice::ExtraPoll)).unwrap(),
        applied_package_uid: None,
    }).handle();

    mock.assert();

    assert_state!(machine, Poll);
}

#[test]
fn skip_same_package_uid() {
    use super::*;
    use client::tests::{create_mock_server, FakeServer};
    use client::ProbeResponse;
    use firmware::tests::{create_fake_metadata, FakeDevice};
    use std::fs;
    use tempfile::NamedTempFile;

    let tmpfile = NamedTempFile::new().unwrap();
    let tmpfile = tmpfile.path();
    fs::remove_file(&tmpfile).unwrap();

    let mock = create_mock_server(FakeServer::HasUpdate).expect(2);

    // We first get the package_uid that will be returned so we can
    // use it for the upcoming test.
    //
    // This has been done so we don't need to manually update it every
    // time we change the package payload.
    let package_uid = {
        let probe = Api::new(
            &Settings::default(),
            &RuntimeSettings::default(),
            &Metadata::new(&create_fake_metadata(FakeDevice::HasUpdate)).unwrap(),
        ).probe()
        .unwrap();

        if let ProbeResponse::Update(u) = probe {
            Some(u.package_uid())
        } else {
            None
        }
    };

    let machine = Box::new(Probe {
        settings: Settings::default(),
        runtime_settings: RuntimeSettings::new()
            .load(tmpfile.to_str().unwrap())
            .unwrap(),
        firmware: Metadata::new(&create_fake_metadata(FakeDevice::HasUpdate)).unwrap(),
        applied_package_uid: package_uid,
    }).handle();

    mock.assert();

    assert_state!(machine, Idle);
}

#[test]
fn error() {
    use super::*;
    use client::tests::{create_mock_server, FakeServer};
    use firmware::tests::{create_fake_metadata, FakeDevice};
    use std::fs;
    use tempfile::NamedTempFile;

    let tmpfile = NamedTempFile::new().unwrap();
    let tmpfile = tmpfile.path();
    fs::remove_file(&tmpfile).unwrap();

    // The server here waits for the second request which includes the
    // retries to succeed.
    let mock = create_mock_server(FakeServer::ErrorOnce);

    let machine = Box::new(Probe {
        settings: Settings::default(),
        runtime_settings: RuntimeSettings::new()
            .load(tmpfile.to_str().unwrap())
            .unwrap(),
        firmware: Metadata::new(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap(),
        applied_package_uid: None,
    }).handle();

    mock.assert();

    assert_state!(machine, Idle);
}
