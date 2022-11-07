// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::types::stardust::block::BlockId;

use super::responses::ProofDto;

pub(crate) fn create_proof(block_ids: Vec<BlockId>, block_id: BlockId) -> ProofDto {
    todo!()
}
pub(crate) fn validate_proof(proof: ProofDto) -> bool {
    todo!()
}
