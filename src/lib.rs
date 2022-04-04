// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![warn(missing_docs)]

//! TODO

/// Module containing the API.
#[cfg(feature = "api")]
pub mod api;
/// Module containing the configuration.
pub mod config;

mod types;

/// Re-exporting the Chrysalis types.
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
/// Re-exporting the Stardust types.
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
