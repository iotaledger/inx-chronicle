// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::types::stardust::block::BlockId;
use crypto::hashes::blake2b::Blake2b256;
use iota_types::block::{payload::dto::MilestonePayloadDto, BlockDto};
use serde::{Deserialize, Serialize};

use crate::api::responses::impl_success_response;

use super::merkle_hasher::MerkleHasher;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofDto {}

impl ProofDto {
    pub(crate) fn contains_block_id(&self, block_id: &BlockId) -> Result<bool, ()> {
        Ok(true)
    }

    pub(crate) fn hash(&self, hasher: &mut MerkleHasher<Blake2b256>) -> &[u8] {
        todo!()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProofResponse {
    pub milestone: MilestonePayloadDto,
    pub block: BlockDto,
    pub proof: ProofDto,
}

impl_success_response!(CreateProofResponse);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateProofResponse {
    pub valid: bool,
}

impl_success_response!(ValidateProofResponse);
