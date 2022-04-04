// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::unnecessary_lazy_evaluations)]
#![warn(missing_docs)]

//! TODO

#[cfg(feature = "api")]
pub mod api;
pub mod config;
mod types;

pub mod cpt2 {
    //! Chrysalis pt.2 bee types
    pub use bee_message_cpt2::{
        self,
        *,
    };
    pub use bee_rest_api_cpt2::{
        self,
        *,
    };
}
pub mod stardust {
    //! Stardust bee types
    pub use bee_message_stardust::{
        self,
        *,
    };
    pub use bee_rest_api_stardust::{
        self,
        *,
    };
}

#[cfg(test)]
mod test {
    #[test]
    fn test_fn() {
        assert_eq!(true, true);
    }
}
