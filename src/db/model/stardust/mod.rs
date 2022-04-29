// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Module containing the message model.
pub mod message;
/// Module containing the milestone model.
pub mod milestone;

/// These are the names of the collections that we create in the database. At some point we could use a macro to make
/// this list nicer.
mod collection {
    pub const MESSAGE_RECORDS: &str = "stardust_message_records";
    pub const MILESTONE_RECORDS: &str = "stardust_milestone_records";
}
