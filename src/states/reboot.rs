// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: MPL-2.0
//

use easy_process;
use failure::Error;
use firmware::Metadata;
use runtime_settings::RuntimeSettings;
use settings::Settings;
use states::{idle::Idle, State};

#[derive(Debug, PartialEq)]
pub(crate) struct Reboot {
    pub(crate) settings: Settings,
    pub(crate) runtime_settings: RuntimeSettings,
    pub(crate) firmware: Metadata,
    pub(crate) applied_package_uid: Option<String>,
}

/// Implements the state change for `Reboot`.
impl State for Reboot {
    // FIXME: When adding state-chance hooks, we need to go to Idle if
    // cancelled.
    fn handle(self: Box<Self>) -> Result<Box<State>, Error> {
        let s = *self; // Drop when NLL is stable
        let settings = s.settings;
        let runtime_settings = s.runtime_settings;
        let firmware = s.firmware;
        let applied_package_uid = s.applied_package_uid;

        info!("Triggering reboot");
        let output = easy_process::run("reboot")?;
        if !output.stdout.is_empty() || !output.stderr.is_empty() {
            info!(
                "  reboot output: stdout: {}, stderr: {}",
                output.stdout, output.stderr
            );
        }

        Ok(Box::new(Idle {
            settings,
            runtime_settings,
            firmware,
            applied_package_uid,
        }))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::Path;

    fn create_reboot(path: &Path) {
        use std::fs::create_dir_all;
        use std::fs::metadata;
        use std::fs::File;
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        // ensure path exists
        create_dir_all(path).unwrap();

        let mut file = File::create(&path.join("reboot")).unwrap();
        file.write_all(b"#!/bin/sh\necho reboot").unwrap();

        let mut permissions = metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        file.set_permissions(permissions).unwrap();
    }

    #[test]
    fn runs() {
        use firmware::tests::{create_fake_metadata, FakeDevice};
        use firmware::Metadata;
        use runtime_settings::RuntimeSettings;
        use settings::Settings;
        use std::env;
        use tempfile::tempdir;

        // create the fake reboot command
        let tmpdir = tempdir().unwrap();
        let tmpdir = tmpdir.path();
        create_reboot(&tmpdir);
        env::set_var(
            "PATH",
            format!(
                "{}:{}",
                &tmpdir.to_string_lossy(),
                env::var("PATH").unwrap_or_default()
            ),
        );

        let machine = Box::new(Reboot {
            settings: Settings::default(),
            runtime_settings: RuntimeSettings::default(),
            firmware: Metadata::new(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap(),
            applied_package_uid: None,
        }).handle();

        assert_state!(machine, Idle);
    }
}