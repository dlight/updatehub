// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: MPL-2.0
//

use chrono::{DateTime, Duration, Utc};
use failure::Error;
use rand::{self, Rng};
use states::{probe::Probe, InnerState, State};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

#[derive(Debug, PartialEq)]
pub(crate) struct Poll {
    pub(crate) inner: InnerState,
    pub(crate) applied_package_uid: Option<String>,
}

impl Poll {
    pub fn new(inner: InnerState, applied_package_uid: Option<String>) -> Box<State> {
        Box::new(Self {
            inner,
            applied_package_uid,
        })
    }
}

/// Implements the state change for `Poll`. This state is used to
/// control when to go to the `Probe`.
impl State for Poll {
    fn handle(self: Box<Self>) -> Result<Box<State>, Error> {
        let s = *self; // Drop when NLL is stable
        let settings = s.inner.settings;
        let runtime_settings = s.inner.runtime_settings;
        let firmware = s.inner.firmware;
        let applied_package_uid = s.applied_package_uid;

        let current_time: DateTime<Utc> = Utc::now();

        let probe_now = runtime_settings.polling.now;
        if probe_now {
            debug!("Moving to Probe state as soon as possible.");
            return Ok(Probe::new(
                InnerState {
                    settings,
                    runtime_settings,
                    firmware,
                },
                applied_package_uid,
            ));
        }

        let last_poll = runtime_settings.polling.last.unwrap_or_else(|| {
            // When no polling has been done before, we choose an
            // offset between current time and the intended polling
            // interval and use it as last_poll
            let mut rnd = rand::thread_rng();
            let interval = settings.polling.interval.num_seconds();
            let offset = Duration::seconds(rnd.gen_range(0, interval));

            current_time + offset
        });

        if last_poll > current_time {
            info!("Forcing to Probe state as last polling seems to happened in future.");
            return Ok(Probe::new(
                InnerState {
                    settings,
                    runtime_settings,
                    firmware,
                },
                applied_package_uid,
            ));
        }

        let extra_interval = runtime_settings.polling.extra_interval;
        if last_poll + extra_interval.unwrap_or_else(|| Duration::seconds(0)) < current_time {
            debug!("Moving to Probe state as the polling's due extra interval.");
            return Ok(Probe::new(
                InnerState {
                    settings,
                    runtime_settings,
                    firmware,
                },
                applied_package_uid,
            ));
        }

        let probe = Arc::new((Mutex::new(()), Condvar::new()));
        let probe2 = probe.clone();
        let interval = settings.polling.interval;
        thread::spawn(move || {
            let (_, ref cvar) = *probe2;
            thread::sleep(interval.to_std().unwrap());
            cvar.notify_one();
        });

        let (ref lock, ref cvar) = *probe;
        let _ = cvar.wait(lock.lock().unwrap());

        debug!("Moving to Probe state.");
        return Ok(Probe::new(
            InnerState {
                settings,
                runtime_settings,
                firmware,
            },
            applied_package_uid,
        ));
    }
}

#[test]
fn extra_poll_in_past() {
    use super::*;
    use firmware::tests::{create_fake_metadata, FakeDevice};

    let mut settings = Settings::default();
    settings.polling.enabled = true;

    let mut runtime_settings = RuntimeSettings::default();
    runtime_settings.polling.last = Some(Utc::now() - Duration::seconds(10));
    runtime_settings.polling.extra_interval = Some(Duration::seconds(10));

    let machine = Poll::new(
        InnerState {
            settings: settings,
            runtime_settings: runtime_settings,
            firmware: Metadata::new(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap(),
        },
        None,
    ).handle();

    assert_state!(machine, Probe);
}

#[test]
fn probe_now() {
    use super::*;
    use firmware::tests::{create_fake_metadata, FakeDevice};

    let mut settings = Settings::default();
    settings.polling.enabled = true;

    let mut runtime_settings = RuntimeSettings::default();
    runtime_settings.polling.last = Some(Utc::now());
    runtime_settings.polling.now = true;

    let machine = Poll::new(
        InnerState {
            settings: settings,
            runtime_settings: runtime_settings,
            firmware: Metadata::new(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap(),
        },
        None,
    ).handle();

    assert_state!(machine, Probe);
}

#[test]
fn last_poll_in_future() {
    use super::*;
    use firmware::tests::{create_fake_metadata, FakeDevice};

    let mut settings = Settings::default();
    settings.polling.enabled = true;

    let mut runtime_settings = RuntimeSettings::default();
    runtime_settings.polling.last = Some(Utc::now() + Duration::days(1));

    let machine = Poll::new(
        InnerState {
            settings: settings,
            runtime_settings: runtime_settings,
            firmware: Metadata::new(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap(),
        },
        None,
    ).handle();

    assert_state!(machine, Probe);
}

#[test]
fn interval_1_second() {
    use super::*;
    use firmware::tests::{create_fake_metadata, FakeDevice};

    let mut settings = Settings::default();
    settings.polling.enabled = true;
    settings.polling.interval = Duration::seconds(1);

    let mut runtime_settings = RuntimeSettings::default();
    runtime_settings.polling.last = Some(Utc::now());

    let machine = Poll::new(
        InnerState {
            settings: settings,
            runtime_settings: runtime_settings,
            firmware: Metadata::new(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap(),
        },
        None,
    ).handle();

    assert_state!(machine, Probe);
}

#[test]
fn never_polled() {
    use super::*;
    use firmware::tests::{create_fake_metadata, FakeDevice};

    let mut settings = Settings::default();
    settings.polling.enabled = true;
    settings.polling.interval = Duration::seconds(1);

    let machine = Poll::new(
        InnerState {
            settings: settings,
            runtime_settings: RuntimeSettings::default(),
            firmware: Metadata::new(&create_fake_metadata(FakeDevice::NoUpdate)).unwrap(),
        },
        None,
    ).handle();

    assert_state!(machine, Probe);
}
