// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Name of the MongoDB database.
pub const DB_NAME: &str = "chronicle-test";

#[allow(missing_docs)]
/// Names of the collections in the MongoDB database.
pub mod collections {
    pub mod stardust {
        pub const MILESTONES: &str = "stardust_milestones";

        pub mod raw {
            pub const MESSAGES: &str = "stardust_raw_messages";
        }
    }
}
