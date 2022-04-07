// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! This module defines the names of the collections in the MongoDB database.

pub mod stardust {
    pub const MILESTONES: &str = "stardust_milestones";

    pub mod raw {
        pub const MESSAGES: &str = "stardust_raw_messages";
    }
}
