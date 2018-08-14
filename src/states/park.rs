// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: MPL-2.0
//

use Result;

use states::{InnerState, StateTransitioner, State};

#[derive(Debug, PartialEq)]
pub(crate) struct Park {
    pub(crate) inner: InnerState,
    pub(crate) applied_package_uid: Option<String>,
}

impl Park {
    pub fn transition(inner: InnerState, applied_package_uid: Option<String>) -> StateTransitioner {
        StateTransitioner {
            inner,
            applied_package_uid,
            transition: Box::new(|inner, applied_package_uid| Box::new(Self {
                inner,
                applied_package_uid,
            }))
        }
    }
}

/// Implements the state change for `Park`. It stays in `Park` state.
impl State for Park {
    // FIXME: turn this into #[derive(inner)]
    fn inner(&self) -> &InnerState {
        &self.inner
    }

    fn handle(self: Box<Self>) -> Result<StateTransitioner> {
        let s = *self; // Drop when NLL is stable
        let inner = s.inner;
        let applied_package_uid = s.applied_package_uid;

        debug!("Staying on Park state.");

        // FIXME: we shouldn't need to realloc Park every time it transitions to itself
        Ok(Park::transition(inner, applied_package_uid))
    }
}
