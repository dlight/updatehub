// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: MPL-2.0
//

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

use failure::Error;
use firmware::Metadata;
use runtime_settings::RuntimeSettings;
use settings::Settings;
use states::{idle::Idle, park::Park};
use std::any::TypeId;

pub fn run(settings: Settings, runtime_settings: RuntimeSettings, firmware: Metadata) {
    fn inner_runner(state: Box<State>) {
        match state.handle() {
            Ok(ref s) if s.downcast_ref::<Park>().is_some() => {
                debug!("Parking state machine.");
                return;
            }
            Ok(s) => inner_runner(s),
            Err(e) => panic!("{}", e),
        }
    }

    inner_runner(Idle::new(
        InnerState {
            settings,
            runtime_settings,
            firmware,
        },
        None,
    ));
}

#[derive(Debug, PartialEq)]
pub(crate) struct InnerState {
    pub(crate) settings: Settings,
    pub(crate) runtime_settings: RuntimeSettings,
    pub(crate) firmware: Metadata,
}

pub trait State {
    fn handle(self: Box<Self>) -> Result<Box<State>, Error>;

    fn __private_get_type_id__(&self) -> TypeId
    where
        Self: 'static,
    {
        TypeId::of::<Self>()
    }
}

impl State {
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

#[cfg(test)]
mod tests {
    use super::*;

    struct A {}
    impl State for A {
        fn handle(self: Box<Self>) -> Result<Box<State>, Error> {
            Ok(Box::new(B {}))
        }
    }

    struct B {}
    impl State for B {
        fn handle(self: Box<Self>) -> Result<Box<State>, Error> {
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
