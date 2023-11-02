// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing [`Block`] types.

use derive_more::From;
use iota_sdk::types::{
    block as iota,
    block::{
        signature::Signature,
        slot::{SlotCommitmentId, SlotIndex},
        IssuerId,
    },
};
use serde::{Deserialize, Serialize};

use self::{basic::BasicBlockDto, validation::ValidationBlockDto};
use super::TryFromDto;

pub mod basic;
pub mod payload;
pub mod validation;

/// The Block type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedBlockDto {
    pub protocol_version: u8,
    pub network_id: u64,
    pub issuing_time: u64,
    pub slot_commitment_id: SlotCommitmentId,
    pub latest_finalized_slot: SlotIndex,
    pub issuer_id: IssuerId,
    pub block: BlockDto,
    pub signature: Signature,
}

#[derive(Clone, Debug, Eq, PartialEq, From, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum BlockDto {
    Basic(Box<BasicBlockDto>),
    Validation(Box<ValidationBlockDto>),
}

impl From<iota::SignedBlock> for SignedBlockDto {
    fn from(value: iota::SignedBlock) -> Self {
        todo!()
    }
}

impl TryFromDto<SignedBlockDto> for iota::SignedBlock {
    type Error = iota::Error;

    fn try_from_dto_with_params_inner(
        dto: SignedBlockDto,
        params: iota_sdk::types::ValidationParams<'_>,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl From<iota::Block> for BlockDto {
    fn from(value: iota::Block) -> Self {
        match value {
            iota::Block::Basic(_) => todo!(),
            iota::Block::Validation(_) => todo!(),
        }
    }
}

impl TryFromDto<BlockDto> for iota::Block {
    type Error = iota::Error;

    fn try_from_dto_with_params_inner(
        dto: BlockDto,
        params: iota_sdk::types::ValidationParams<'_>,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

// #[cfg(test)]
// mod test {
//     use mongodb::bson::{doc, from_bson, to_bson, to_document, Bson};
//     use pretty_assertions::assert_eq;

//     use super::*;
//     use crate::model::payload::TransactionEssence;

//     #[test]
//     fn test_block_id_bson() {
//         let block_id = BlockId::rand();
//         let bson = to_bson(&block_id).unwrap();
//         assert_eq!(Bson::from(block_id), bson);
//         from_bson::<BlockId>(bson).unwrap();
//     }

//     #[test]
//     fn test_transaction_block_bson() {
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         let block = Block::rand_transaction(&ctx);
//         let mut bson = to_bson(&block).unwrap();
//         // Need to re-add outputs as they are not serialized
//         let outputs_doc = if let Some(Payload::Transaction(payload)) = &block.payload {
//             let TransactionEssence::Regular { outputs, .. } = &payload.essence;
//             doc! { "outputs": outputs.iter().map(to_document).collect::<Result<Vec<_>, _>>().unwrap() }
//         } else {
//             unreachable!();
//         };
//         let doc = bson
//             .as_document_mut()
//             .unwrap()
//             .get_document_mut("payload")
//             .unwrap()
//             .get_document_mut("essence")
//             .unwrap();
//         doc.extend(outputs_doc);
//         assert_eq!(block, from_bson::<Block>(bson).unwrap());
//     }

//     #[test]
//     fn test_milestone_block_bson() {
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         let block = Block::rand_milestone(&ctx);
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         iota::Block::try_from_with_context(&ctx, block.clone()).unwrap();
//         let bson = to_bson(&block).unwrap();
//         assert_eq!(block, from_bson::<Block>(bson).unwrap());
//     }

//     #[test]
//     fn test_tagged_data_block_bson() {
//         let block = Block::rand_tagged_data();
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         iota::Block::try_from_with_context(&ctx, block.clone()).unwrap();
//         let bson = to_bson(&block).unwrap();
//         assert_eq!(block, from_bson::<Block>(bson).unwrap());
//     }

//     #[test]
//     fn test_treasury_transaction_block_bson() {
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         let block = Block::rand_treasury_transaction(&ctx);
//         let bson = to_bson(&block).unwrap();
//         assert_eq!(block, from_bson::<Block>(bson).unwrap());
//     }

//     #[test]
//     fn test_no_payload_block_bson() {
//         let block = Block::rand_no_payload();
//         let ctx = iota_sdk::types::block::protocol::protocol_parameters();
//         iota::Block::try_from_with_context(&ctx, block.clone()).unwrap();
//         let bson = to_bson(&block).unwrap();
//         assert_eq!(block, from_bson::<Block>(bson).unwrap());
//     }
// }
