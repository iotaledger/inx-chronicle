// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![warn(missing_docs)]

//! TODO

/// Module that contains utilities to serialize values to BSON.
pub mod bson;
/// Module that contains the database types and constants.
pub mod db;
#[cfg(feature = "stardust")]
/// Module that contains INX bindings and configuration.
pub mod inx;
/// Module that contains the actor runtime.
pub mod runtime;

/// Module re-exporting Chrysalis types.
#[cfg(feature = "chrysalis")]
pub mod chrysalis {
    //! Chrysalis pt.2 bee types
    pub use bee_message_chrysalis::{self, *};
    pub use bee_rest_api_chrysalis::{self, *};
}

/// Module re-exporting Stardust types.
#[cfg(feature = "stardust")]
pub mod stardust {
    //! Stardust bee types
    pub use bee_message_stardust::{self, *};
    pub use bee_rest_api_stardust::{self, *};
}
