// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the node data models.

mod config;
mod status;

pub use self::{config::NodeConfiguration, status::NodeStatusMessage};
