// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![warn(missing_docs)]

//! TODO

/// Module that contains the database types and constants.
pub mod db;
/// Module containing data transfer objects of the Bee types.
pub mod dto;
/// Module that contains the actor runtime.
pub mod runtime;
/// Module re-exporting Stardust types.
#[cfg(feature = "stardust")]
pub mod stardust {
    //! Stardust bee types
    pub use bee_message_stardust::{self, *};
    pub use bee_rest_api_stardust::{self, *};
}
