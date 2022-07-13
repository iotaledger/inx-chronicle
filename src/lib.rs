// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![warn(missing_docs)]
#![warn(unreachable_pub)]
// TODO: This is currently broken, but may be fixed, so we should check it every once in a while
#![allow(clippy::needless_borrow)]

//! TODO

/// Module that contains the database and associated models.
pub mod db;
/// Module that contains the actor runtime.
pub mod runtime;
/// Module that contains the types.
pub mod types;
