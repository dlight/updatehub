// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: MPL-2.0
//

use failure::Error;
use states::{InnerState, State};

#[derive(Debug, PartialEq)]
pub(crate) struct Park {
    pub(crate) inner: InnerState,
    pub(crate) applied_package_uid: Option<String>,
}

impl Park {
    pub fn new(inner: InnerState, applied_package_uid: Option<String>) -> Box<State> {
        Box::new(Self {
            inner,
            applied_package_uid,
        })
    }
}

/// Implements the state change for `Park`. It stays in `Park` state.
impl State for Park {
    fn handle(self: Box<Self>) -> Result<Box<State>, Error> {
        debug!("Staying on Park state.");
        Ok(self)
    }
}
