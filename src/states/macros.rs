// Copyright (C) 2018 O.S. Systems Sofware LTDA
//
// SPDX-License-Identifier: MPL-2.0
//

#[cfg(test)]
macro_rules! assert_state {
    ($machine:ident, $state:ident) => {
        assert!(
            if let Ok(s) = $machine {
                s.is::<$state>()
            } else {
                false
            },
            "Failed to get to {} state.",
            stringify!($state)
        );
    };
}
