// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: MPL-2.0
//

#![allow(dead_code)]

//! Controls the state machine of the system
//!
//! It supports following states, and transitions, as shown in the
//! below diagram:
//!
//! ```text
//!           .--------------.
//!           |              v
//! Park <- Idle -> Poll -> Probe -> Download -> Install -> Reboot
//!           ^      ^        '          '          '
//!           '      '        '          '          '
//!           '      `--------'          '          '
//!           `---------------'          '          '
//!           `--------------------------'          '
//!           `-------------------------------------'
//! ```

#[macro_use]
mod macros;

mod download;
mod idle;
mod install;
mod park;
mod poll;
mod probe;
mod reboot;
mod transition;

use Result;

use firmware::Metadata;
use runtime_settings::RuntimeSettings;
use settings::Settings;
use states::{idle::Idle, park::Park};
use std::any::TypeId;

pub fn run(settings: Settings, runtime_settings: RuntimeSettings, firmware: Metadata) {
    fn inner_runner(state: Box<State>) {
        match state.handle_callbacks() {
            Ok(ref s) if s.downcast_ref::<Park>().is_some() => {
                debug!("Parking state machine.");
                return;
            }
            Ok(s) => {
                inner_runner(s)
            },
            Err(e) => panic!("{}", e),
        }
    }

    inner_runner(Box::new(Idle {
        inner: InnerState {
            settings,
            runtime_settings,
            firmware,
        },
        applied_package_uid: None,
    }));
}

#[derive(Debug, PartialEq)]
pub struct InnerState {
    pub(crate) settings: Settings,
    pub(crate) runtime_settings: RuntimeSettings,
    pub(crate) firmware: Metadata,
}

pub struct StateTransitioner {
    pub(crate) inner: InnerState,
    pub(crate) applied_package_uid: Option<String>,

    pub(crate) transition: Box<FnOnce(InnerState, Option<String>) -> Box<State>>,
}

impl StateTransitioner {
    fn transition(self) -> Box<State> {
        let Self { inner, applied_package_uid, transition } = self;

        transition(inner, applied_package_uid)
    }

    fn cancel(self) -> Box<State> {
        let Self { inner, applied_package_uid, .. } = self;

        Box::new(Idle { inner, applied_package_uid })
    }
}

pub trait State {
    fn inner(&self) -> &InnerState;

    fn callback_state_name(&self) -> Option<&'static str> {
        None
    }

    fn handle(self: Box<Self>) -> Result<StateTransitioner>;

    fn __private_get_type_id__(&self) -> TypeId
    where
        Self: 'static,
    {
        TypeId::of::<Self>()
    }
}

impl State {
    fn handle_callbacks(self: Box<Self>) -> Result<Box<State>> {
        use states::transition::{ Transition, state_change_callback };

        let callback = self.callback_state_name();

        // FIXME: remove this clone
        let firmware_path = &self.inner().settings.firmware.metadata_path.clone();

        let transitioner = self.handle()?;

        match callback {
            None => Ok(transitioner.transition()),
            Some(callback_name) => match state_change_callback(firmware_path, callback_name) {
                Ok(Transition::Continue) => Ok(transitioner.transition()),
                Ok(Transition::Cancel) => {
                    debug!("State transition cancelled.");

                    Ok(transitioner.cancel())
                },
                Err(e) => panic!("{}", e),
            }
        }
    }

    fn downcast_ref<S: State + 'static>(&self) -> Option<&S> {
        if self.__private_get_type_id__() == TypeId::of::<S>() {
            unsafe { Some(&*(self as *const State as *const S)) }
        } else {
            None
        }
    }

    #[cfg(test)]
    fn is<T: State + 'static>(&self) -> bool {
        self.__private_get_type_id__() == TypeId::of::<T>()
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    struct A {}
    impl State for A {
        fn handle(self: Box<Self>) -> Result<StateTransitioner> {
            Ok(Box::new(B {}))
        }
    }

    struct B {}
    impl State for B {
        fn handle(self: Box<Self>) -> Result<StateTransitioner> {
            Ok(Box::new(A {}))
        }
    }

    #[test]
    fn assert_works() {
        let to_b = Box::new(A {}).handle();
        assert_state!(to_b, B);

        let to_a = Box::new(B {}).handle();
        assert_state!(to_a, A);
    }

    #[test]
    #[should_panic]
    fn must_fail() {
        let to_b = Box::new(A {}).handle();
        assert_state!(to_b, A);
    }
}
*/
