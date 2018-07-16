// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: MPL-2.0
//

use failure::Error;
use firmware::Metadata;
use runtime_settings::RuntimeSettings;
use settings::Settings;
use states::State;

#[derive(Debug, PartialEq)]
pub(crate) struct Park {
    pub(crate) settings: Settings,
    pub(crate) runtime_settings: RuntimeSettings,
    pub(crate) firmware: Metadata,
    pub(crate) applied_package_uid: Option<String>,
}

/// Implements the state change for `Park`. It stays in `Park` state.
impl State for Park {
    fn handle(self: Box<Self>) -> Result<Box<State>, Error> {
        let s = *self; // Drop when NLL is stable
        let settings = s.settings;
        let runtime_settings = s.runtime_settings;
        let firmware = s.firmware;
        let applied_package_uid = s.applied_package_uid;

        debug!("Staying on Park state.");
        Ok(Box::new(Park {
            settings,
            runtime_settings,
            firmware,
            applied_package_uid,
        }))
    }
}
