// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that contains the types.

#![allow(missing_docs)] // TODO Remove this once everything has settled.

/// Module containing the ledger data models.
#[cfg(feature = "stardust")]
pub mod ledger;
/// Module containing Stardust data models.
#[cfg(feature = "stardust")]
pub mod stardust;
/// Module containing the tangle models.
#[cfg(feature = "stardust")]
pub mod tangle;
/// Module contain utility functions.
pub mod util;
