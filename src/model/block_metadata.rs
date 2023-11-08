// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing block metadata types.

use iota_sdk::types::{
    api::core::{BlockFailureReason, BlockState, TransactionState},
    block::{semantic::TransactionFailureReason, BlockId, SignedBlock},
};
use serde::{Deserialize, Serialize};

use super::raw::Raw;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BlockMetadata {
    pub block_id: BlockId,
    pub block_state: BlockState,
    pub transaction_state: Option<TransactionState>,
    pub block_failure_reason: Option<BlockFailureReason>,
    pub transaction_failure_reason: Option<TransactionFailureReason>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BlockWithMetadata {
    pub metadata: BlockMetadata,
    pub block: Raw<SignedBlock>,
}
