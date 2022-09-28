// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod milestone_id;

use std::borrow::Borrow;

use bee_block_stardust::payload::milestone::{self as bee};
use serde::{Deserialize, Serialize};

pub use self::milestone_id::MilestoneId;
use crate::types::{
    context::{TryFromWithContext, TryIntoWithContext},
    stardust::block::{payload::treasury_transaction::TreasuryTransactionPayload, Address, BlockId, Signature},
    tangle::MilestoneIndex,
    util::bytify,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MilestonePayload {
    pub essence: MilestoneEssence,
    pub signatures: Box<[Signature]>,
}

impl<T: Borrow<bee::MilestonePayload>> From<T> for MilestonePayload {
    fn from(value: T) -> Self {
        Self {
            essence: MilestoneEssence::from(value.borrow().essence()),
            signatures: value.borrow().signatures().iter().map(Into::into).collect(),
        }
    }
}

impl TryFromWithContext<MilestonePayload> for bee::MilestonePayload {
    type Error = bee_block_stardust::Error;

    fn try_from_with_context(
        ctx: &bee_block_stardust::protocol::ProtocolParameters,
        value: MilestonePayload,
    ) -> Result<Self, Self::Error> {
        bee::MilestonePayload::new(
            value.essence.try_into_with_context(ctx)?,
            Vec::from(value.signatures)
                .into_iter()
                .map(Into::into)
                .collect::<Vec<_>>(),
        )
    }
}

impl From<MilestonePayload> for bee::dto::MilestonePayloadDto {
    fn from(value: MilestonePayload) -> Self {
        Self {
            kind: bee::MilestonePayload::KIND,
            index: value.essence.index.0,
            timestamp: value.essence.timestamp,
            protocol_version: value.essence.protocol_version,
            previous_milestone_id: value.essence.previous_milestone_id.to_hex(),
            parents: value
                .essence
                .parents
                .to_vec()
                .into_iter()
                .map(|id| id.to_hex())
                .collect(),
            inclusion_merkle_root: prefix_hex::encode(value.essence.inclusion_merkle_root),
            applied_merkle_root: prefix_hex::encode(value.essence.applied_merkle_root),
            options: value.essence.options.to_vec().into_iter().map(Into::into).collect(),
            metadata: prefix_hex::encode(value.essence.metadata),
            signatures: value.signatures.to_vec().into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MilestoneEssence {
    pub index: MilestoneIndex,
    pub timestamp: u32,
    pub protocol_version: u8,
    pub previous_milestone_id: MilestoneId,
    pub parents: Box<[BlockId]>,
    #[serde(with = "bytify")]
    pub inclusion_merkle_root: [u8; Self::MERKLE_PROOF_LENGTH],
    #[serde(with = "bytify")]
    pub applied_merkle_root: [u8; Self::MERKLE_PROOF_LENGTH],
    #[serde(with = "serde_bytes")]
    pub metadata: Vec<u8>,
    pub options: Box<[MilestoneOption]>,
}

impl MilestoneEssence {
    const MERKLE_PROOF_LENGTH: usize = bee::MerkleRoot::LENGTH;
}

impl<T: Borrow<bee::MilestoneEssence>> From<T> for MilestoneEssence {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            index: value.index().0.into(),
            timestamp: value.timestamp(),
            protocol_version: value.protocol_version(),
            previous_milestone_id: (*value.previous_milestone_id()).into(),
            parents: value.parents().iter().map(|&id| BlockId::from(id)).collect(),
            inclusion_merkle_root: **value.inclusion_merkle_root(),
            applied_merkle_root: **value.applied_merkle_root(),
            metadata: value.metadata().to_vec(),
            options: value.options().iter().map(Into::into).collect(),
        }
    }
}

impl TryFromWithContext<MilestoneEssence> for bee::MilestoneEssence {
    type Error = bee_block_stardust::Error;

    fn try_from_with_context(
        ctx: &bee_block_stardust::protocol::ProtocolParameters,
        value: MilestoneEssence,
    ) -> Result<Self, Self::Error> {
        bee::MilestoneEssence::new(
            value.index.into(),
            value.timestamp,
            value.protocol_version,
            value.previous_milestone_id.into(),
            bee_block_stardust::parent::Parents::new(
                Vec::from(value.parents).into_iter().map(Into::into).collect::<Vec<_>>(),
            )?,
            bee_block_stardust::payload::milestone::MerkleRoot::from(value.inclusion_merkle_root),
            bee_block_stardust::payload::milestone::MerkleRoot::from(value.applied_merkle_root),
            value.metadata,
            bee_block_stardust::payload::MilestoneOptions::new(
                Vec::from(value.options)
                    .into_iter()
                    .map(|x| x.try_into_with_context(ctx))
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum MilestoneOption {
    Receipt {
        migrated_at: MilestoneIndex,
        last: bool,
        funds: Box<[MigratedFundsEntry]>,
        transaction: TreasuryTransactionPayload,
    },
    Parameters {
        target_milestone_index: MilestoneIndex,
        protocol_version: u8,
        binary_parameters: Box<[u8]>,
    },
}

impl<T: Borrow<bee::MilestoneOption>> From<T> for MilestoneOption {
    fn from(value: T) -> Self {
        match value.borrow() {
            bee::MilestoneOption::Receipt(r) => Self::Receipt {
                migrated_at: r.migrated_at().into(),
                last: r.last(),
                funds: r.funds().iter().map(Into::into).collect(),
                transaction: r.transaction().into(),
            },
            bee::MilestoneOption::Parameters(p) => Self::Parameters {
                target_milestone_index: p.target_milestone_index().into(),
                protocol_version: p.protocol_version(),
                binary_parameters: p.binary_parameters().to_owned().into_boxed_slice(),
            },
        }
    }
}

impl TryFromWithContext<MilestoneOption> for bee::MilestoneOption {
    type Error = bee_block_stardust::Error;

    fn try_from_with_context(
        ctx: &bee_block_stardust::protocol::ProtocolParameters,
        value: MilestoneOption,
    ) -> Result<Self, Self::Error> {
        Ok(match value {
            MilestoneOption::Receipt {
                migrated_at,
                last,
                funds,
                transaction,
            } => Self::Receipt(bee::ReceiptMilestoneOption::new(
                migrated_at.into(),
                last,
                Vec::from(funds)
                    .into_iter()
                    .map(|x| x.try_into_with_context(ctx))
                    .collect::<Result<Vec<_>, _>>()?,
                transaction.try_into_with_context(ctx)?,
                ctx.token_supply(),
            )?),
            MilestoneOption::Parameters {
                target_milestone_index,
                protocol_version,
                binary_parameters,
            } => Self::Parameters(bee::ParametersMilestoneOption::new(
                target_milestone_index.into(),
                protocol_version,
                binary_parameters.into_vec(),
            )?),
        })
    }
}

impl From<MilestoneOption> for bee::option::dto::MilestoneOptionDto {
    fn from(value: MilestoneOption) -> Self {
        todo!()
        // match value {
        //     MilestoneOption::Receipt {
        //         migrated_at,
        //         last,
        //         funds,
        //         transaction,
        //     } => Self::Receipt(bee::option::dto::ReceiptMilestoneOptionDto {
        //         kind: bee::option::ReceiptMilestoneOption::KIND,
        //         migrated_at: migrated_at.0,
        //         funds: funds.to_vec().into_iter().map(Into::into).collect(),
        //         transaction: bee_block_stardust::payload::dto::PayloadDto::TreasuryTransaction(Box::new(
        //             transaction.into(),
        //         )),
        //         last,
        //     }),
        //     MilestoneOption::Parameters {
        //         target_milestone_index,
        //         protocol_version,
        //         binary_parameters,
        //     } => Self::Parameters(bee::option::dto::ParametersMilestoneOptionDto {
        //         kind: bee::option::ParametersMilestoneOption::KIND,
        //         target_milestone_index: target_milestone_index.0,
        //         protocol_version,
        //         binary_parameters: prefix_hex::encode(binary_parameters),
        //     }),
        // }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigratedFundsEntry {
    #[serde(with = "bytify")]
    tail_transaction_hash: [u8; Self::TAIL_TRANSACTION_HASH_LENGTH],
    address: Address,
    #[serde(with = "crate::types::util::stringify")]
    amount: u64,
}

impl MigratedFundsEntry {
    const TAIL_TRANSACTION_HASH_LENGTH: usize = bee::option::TailTransactionHash::LENGTH;
}

impl<T: Borrow<bee::option::MigratedFundsEntry>> From<T> for MigratedFundsEntry {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            // Unwrap: Should not fail as the length is defined by the struct
            tail_transaction_hash: value.tail_transaction_hash().as_ref().try_into().unwrap(),
            address: (*value.address()).into(),
            amount: value.amount(),
        }
    }
}

impl TryFromWithContext<MigratedFundsEntry> for bee::option::MigratedFundsEntry {
    type Error = bee_block_stardust::Error;

    fn try_from_with_context(
        ctx: &bee_block_stardust::protocol::ProtocolParameters,
        value: MigratedFundsEntry,
    ) -> Result<Self, Self::Error> {
        Self::new(
            bee::option::TailTransactionHash::new(value.tail_transaction_hash)?,
            value.address.into(),
            value.amount,
            ctx.token_supply(),
        )
    }
}

// impl From<MigratedFundsEntry> for bee::option::receipt::dto::MigratedFundsEntryDto {
//     fn from(value: MigratedFundsEntry) -> Self {
//         Self {
//             tail_transaction_hash: prefix_hex::encode(value.tail_transaction_hash),
//             address: value.address.into(),
//             deposit: value.amount,
//         }
//     }
// }

#[cfg(feature = "rand")]
mod rand {
    use bee_block_stardust::rand::{
        bytes::rand_bytes, milestone::rand_merkle_root, milestone_option::rand_receipt_milestone_option,
        number::rand_number, payload::rand_milestone_payload, receipt::rand_migrated_funds_entry,
    };

    use super::*;

    impl MilestonePayload {
        /// Generates a random [`MilestonePayload`].
        pub fn rand(ctx: &bee_block_stardust::protocol::ProtocolParameters) -> Self {
            rand_milestone_payload(ctx.protocol_version()).into()
        }
    }

    impl MilestoneEssence {
        /// Generates a random [`MilestoneEssence`].
        pub fn rand(ctx: &bee_block_stardust::protocol::ProtocolParameters) -> Self {
            Self {
                index: rand_number::<u32>().into(),
                timestamp: rand_number::<u32>(),
                protocol_version: 0,
                previous_milestone_id: MilestoneId::rand(),
                parents: BlockId::rand_parents(),
                inclusion_merkle_root: *rand_merkle_root(),
                applied_merkle_root: *rand_merkle_root(),
                metadata: rand_bytes(32),
                options: Box::new([MilestoneOption::rand_receipt(ctx)]),
            }
        }
    }

    impl MilestoneOption {
        /// Generates a random receipt [`MilestoneOption`].
        pub fn rand_receipt(ctx: &bee_block_stardust::protocol::ProtocolParameters) -> Self {
            bee::MilestoneOption::from(rand_receipt_milestone_option(ctx.token_supply())).into()
        }

        /// Generates a random parameters [`MilestoneOption`].
        pub fn rand_parameters() -> Self {
            Self::Parameters {
                target_milestone_index: rand_number::<u32>().into(),
                protocol_version: rand_number(),
                binary_parameters: rand_bytes(100).into_boxed_slice(),
            }
        }
    }

    impl MigratedFundsEntry {
        /// Generates a random [`MigratedFundsEntry`].
        pub fn rand(ctx: &bee_block_stardust::protocol::ProtocolParameters) -> Self {
            rand_migrated_funds_entry(ctx.token_supply()).into()
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{from_bson, to_bson, Bson};

    use super::*;

    #[test]
    fn test_milestone_id_bson() {
        let milestone_id = MilestoneId::rand();
        let bson = to_bson(&milestone_id).unwrap();
        assert_eq!(Bson::from(milestone_id), bson);
        assert_eq!(milestone_id, from_bson::<MilestoneId>(bson).unwrap());
    }

    #[test]
    fn test_milestone_payload_bson() {
        let ctx = bee_block_stardust::protocol::protocol_parameters();
        let payload = MilestonePayload::rand(&ctx);
        bee::MilestonePayload::try_from_with_context(&ctx, payload.clone()).unwrap();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(payload, from_bson::<MilestonePayload>(bson).unwrap());
    }
}
