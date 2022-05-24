// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::types::{
    ledger::OutputMetadata,
    stardust::block::{Output, OutputId},
};

/// Contains all informations related to an output.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputDocument {
    #[serde(rename = "_id")]
    output_id: OutputId,
    #[serde(flatten)]
    inner: Output,
    metadata: OutputMetadata,
}

impl OutputDocument {
    /// The stardust outputs collection name.
    pub const COLLECTION: &'static str = "stardust_outputs";
}
