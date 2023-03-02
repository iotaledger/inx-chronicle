// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

// Ideally, this would be handled completely by CI, but there is a bug in `petgraph` that prevents us from doing that.
#![warn(missing_docs)]
#![deny(unreachable_pub, private_in_public)]

//! The basic types and MongoDb queries for Chronicle.

#[cfg(feature = "analytics")]
pub mod analytics;
pub mod db;
#[cfg(feature = "inx")]
pub mod inx;
#[cfg(feature = "metrics")]
pub mod metrics;
pub mod model;
pub mod tangle;
