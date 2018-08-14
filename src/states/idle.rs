// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: MPL-2.0
//

use failure::Error;
use states::{park::Park, poll::Poll, InnerState, State};

#[derive(Debug, PartialEq)]
pub(crate) struct Idle {
    pub(crate) inner: InnerState,
    pub(crate) applied_package_uid: Option<String>,
}

impl Idle {
    pub fn new(inner: InnerState, applied_package_uid: Option<String>) -> Box<State> {
        Box::new(Self {
            inner,
            applied_package_uid,
        })
    }
}

/// Implements the state change for `Idle`. If polling is disabled it
/// stays in `Idle`, otherwise, it moves to `Poll` state.
impl State for Idle {
    // FIXME: when supporting the HTTP API we need allow going to
    // Probe.
    fn handle(self: Box<Self>) -> Result<Box<State>, Error> {
        let s = *self; // Drop when NLL is stable
        let settings = s.inner.settings;
        let runtime_settings = s.inner.runtime_settings;
        let firmware = s.inner.firmware;
        let applied_package_uid = s.applied_package_uid;

        if !settings.polling.enabled {
            debug!("Polling is disabled, staying on Idle state.");
            return Ok(Park::new(
                InnerState {
                    settings,
                    runtime_settings,
                    firmware,
                },
                applied_package_uid,
            ));
        }

        debug!("Polling is enabled, moving to Poll state.");
        Ok(Poll::new(
            InnerState {
                settings,
                runtime_settings,
                firmware,
            },
            applied_package_uid,
        ))
    }
}

#[test]
fn polling_disable() {
    use super::*;
    use firmware::tests::{create_fake_metadata, FakeDevice};

    let mut settings = Settings::default();
    settings.polling.enabled = false;

    let machine = Idle::new(
        InnerState {
            settings: settings,
            runtime_settings: RuntimeSettings::default(),
            firmware: Metadata::new(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap(),
        },
        None,
    ).handle();

    assert_state!(machine, Park);
}

#[test]
fn polling_enabled() {
    use super::*;
    use firmware::tests::{create_fake_metadata, FakeDevice};

    let mut settings = Settings::default();
    settings.polling.enabled = true;

    let machine = Idle::new(
        InnerState {
            settings: settings,
            runtime_settings: RuntimeSettings::default(),
            firmware: Metadata::new(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap(),
        },
        None,
    ).handle();

    assert_state!(machine, Poll);
}
