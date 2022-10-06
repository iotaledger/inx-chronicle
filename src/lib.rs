// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

// Ideally, this would be handled completely by CI, but there is a bug in `petgraph` that prevents us from doing that.
#![warn(missing_docs)]

//! The basic types and MongoDb queries for Chronicle.

pub mod db;
/// Module that contains the types.
pub mod types;
