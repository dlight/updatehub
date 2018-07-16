// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: MPL-2.0
//

use failure::{Error, ResultExt};
use firmware::Metadata;
use runtime_settings::RuntimeSettings;
use settings::Settings;
use states::{reboot::Reboot, State};
use update_package::UpdatePackage;

#[derive(Debug, PartialEq)]
pub(crate) struct Install {
    pub(crate) settings: Settings,
    pub(crate) runtime_settings: RuntimeSettings,
    pub(crate) firmware: Metadata,
    pub(crate) update_package: UpdatePackage,
}

impl State for Install {
    // FIXME: When adding state-chance hooks, we need to go to Idle if
    // cancelled.
    fn handle(self: Box<Self>) -> Result<Box<State>, Error> {
        let s = *self; // Drop when NLL is stable
        let settings = s.settings;
        let mut runtime_settings = s.runtime_settings;
        let firmware = s.firmware;
        let update_package = s.update_package;

        info!("Installing update: {}", update_package.package_uid());

        // FIXME: Check if A/B install
        // FIXME: Check InstallIfDifferent

        // Ensure we do a probe as soon as possible so full update
        // cycle can be finished.
        runtime_settings.polling.now = true;

        // Avoid installing same package twice.
        let applied_package_uid = Some(update_package.package_uid());

        if !settings.storage.read_only {
            debug!("Saving install settings.");
            runtime_settings
                .save()
                .context("Saving runtime due install changes")?;
        } else {
            debug!("Skipping install settings save, read-only mode enabled.");
        }

        info!("Update installed successfully");
        Ok(Box::new(Reboot {
            settings,
            runtime_settings,
            firmware,
            applied_package_uid,
        }))
    }
}

#[test]
fn has_package_uid_if_succeed() {
    use super::*;
    use firmware::tests::{create_fake_metadata, FakeDevice};
    use std::fs;
    use tempfile::NamedTempFile;
    use update_package::tests::get_update_package;

    let tmpfile = NamedTempFile::new().unwrap();
    let tmpfile = tmpfile.path();
    fs::remove_file(&tmpfile).unwrap();

    let machine = Box::new(Install {
        settings: Settings::default(),
        runtime_settings: RuntimeSettings::new()
            .load(tmpfile.to_str().unwrap())
            .unwrap(),
        firmware: Metadata::new(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap(),
        update_package: get_update_package(),
    }).handle();

    assert_state!(machine, Reboot);
}

#[test]
fn polling_now_if_succeed() {
    use super::*;
    use firmware::tests::{create_fake_metadata, FakeDevice};
    use std::fs;
    use tempfile::NamedTempFile;
    use update_package::tests::get_update_package;

    let tmpfile = NamedTempFile::new().unwrap();
    let tmpfile = tmpfile.path();
    fs::remove_file(&tmpfile).unwrap();

    let machine = Box::new(Install {
        settings: Settings::default(),
        runtime_settings: RuntimeSettings::new()
            .load(tmpfile.to_str().unwrap())
            .unwrap(),
        firmware: Metadata::new(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap(),
        update_package: get_update_package(),
    }).handle();

    assert_state!(machine, Reboot);
}
