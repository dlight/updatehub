// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: MPL-2.0
//

use failure::Error;
use firmware::Metadata;
use runtime_settings::RuntimeSettings;
use settings::Settings;
use states::{park::Park, poll::Poll, State};

#[derive(Debug, PartialEq)]
pub(crate) struct Idle {
    pub(crate) settings: Settings,
    pub(crate) runtime_settings: RuntimeSettings,
    pub(crate) firmware: Metadata,
    pub(crate) applied_package_uid: Option<String>,
}

/// Implements the state change for `Idle`. If polling is disabled it
/// stays in `Idle`, otherwise, it moves to `Poll` state.
impl State for Idle {
    // FIXME: when supporting the HTTP API we need allow going to
    // Probe.
    fn handle(self: Box<Self>) -> Result<Box<State>, Error> {
        let s = *self; // Drop when NLL is stable
        let settings = s.settings;
        let runtime_settings = s.runtime_settings;
        let firmware = s.firmware;
        let applied_package_uid = s.applied_package_uid;

        if !settings.polling.enabled {
            debug!("Polling is disabled, staying on Idle state.");
            return Ok(Box::new(Park {
                settings,
                runtime_settings,
                firmware,
                applied_package_uid,
            }));
        }

        debug!("Polling is enabled, moving to Poll state.");
        Ok(Box::new(Poll {
            settings,
            runtime_settings,
            firmware,
            applied_package_uid,
        }))
    }
}

#[test]
fn polling_disable() {
    use super::*;
    use firmware::tests::{create_fake_metadata, FakeDevice};

    let mut settings = Settings::default();
    settings.polling.enabled = false;

    let machine = Box::new(Idle {
        settings: settings,
        runtime_settings: RuntimeSettings::default(),
        firmware: Metadata::new(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap(),
        applied_package_uid: None,
    }).handle();

    assert_state!(machine, Park);
}

#[test]
fn polling_enabled() {
    use super::*;
    use firmware::tests::{create_fake_metadata, FakeDevice};

    let mut settings = Settings::default();
    settings.polling.enabled = true;

    let machine = Box::new(Idle {
        settings: settings,
        runtime_settings: RuntimeSettings::default(),
        firmware: Metadata::new(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap(),
        applied_package_uid: None,
    }).handle();

    assert_state!(machine, Poll);
}
