// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing milestone-related types.

mod milestone_id;
mod milestone_index;
mod milestone_timestamp;

use std::borrow::Borrow;

use iota_types::block::payload::milestone as iota;
use serde::{Deserialize, Serialize};

pub use self::{milestone_id::MilestoneId, milestone_index::MilestoneIndex, milestone_timestamp::MilestoneTimestamp};
use crate::model::{
    block::BlockId, bytify, payload::TreasuryTransactionPayload, signature::Signature, stringify, utxo::Address,
    TryFromWithContext, TryIntoWithContext,
};

/// [`MilestoneIndex`] and [`MilestoneTimestamp`] pair.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash, Ord, PartialOrd)]
#[allow(missing_docs)]
pub struct MilestoneIndexTimestamp {
    pub milestone_index: MilestoneIndex,
    pub milestone_timestamp: MilestoneTimestamp,
}

impl From<MilestoneIndexTimestamp> for mongodb::bson::Bson {
    fn from(value: MilestoneIndexTimestamp) -> Self {
        // Unwrap: Cannot fail as type is well defined
        mongodb::bson::to_bson(&value).unwrap()
    }
}

/// Represents a milestone payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MilestonePayload {
    /// The essence of the milestone.
    pub essence: MilestoneEssence,
    /// A list of [`Signature`]s.
    pub signatures: Box<[Signature]>,
}

impl MilestonePayload {
    /// A `&str` representation of the type.
    pub const KIND: &'static str = "milestone";
}

impl<T: Borrow<iota::MilestonePayload>> From<T> for MilestonePayload {
    fn from(value: T) -> Self {
        Self {
            essence: MilestoneEssence::from(value.borrow().essence()),
            signatures: value.borrow().signatures().iter().map(Into::into).collect(),
        }
    }
}

impl TryFromWithContext<MilestonePayload> for iota::MilestonePayload {
    type Error = iota_types::block::Error;

    fn try_from_with_context(
        ctx: &iota_types::block::protocol::ProtocolParameters,
        value: MilestonePayload,
    ) -> Result<Self, Self::Error> {
        iota::MilestonePayload::new(
            value.essence.try_into_with_context(ctx)?,
            value
                .signatures
                .into_vec()
                .into_iter()
                .map(Into::into)
                .collect::<Vec<_>>(),
        )
    }
}

impl From<MilestonePayload> for iota::dto::MilestonePayloadDto {
    fn from(value: MilestonePayload) -> Self {
        Self {
            kind: iota::MilestonePayload::KIND,
            index: value.essence.index.0,
            timestamp: value.essence.timestamp.0,
            protocol_version: value.essence.protocol_version,
            previous_milestone_id: value.essence.previous_milestone_id.to_hex(),
            parents: value
                .essence
                .parents
                .into_vec()
                .into_iter()
                .map(|id| id.to_hex())
                .collect(),
            inclusion_merkle_root: prefix_hex::encode(value.essence.inclusion_merkle_root),
            applied_merkle_root: prefix_hex::encode(value.essence.applied_merkle_root),
            options: value.essence.options.into_vec().into_iter().map(Into::into).collect(),
            metadata: prefix_hex::encode(value.essence.metadata),
            signatures: value.signatures.into_vec().into_iter().map(Into::into).collect(),
        }
    }
}

/// The milestone essence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MilestoneEssence {
    /// The index of the milestone.
    pub index: MilestoneIndex,
    /// The UNIX timestamp of the issued milestone.
    pub timestamp: MilestoneTimestamp,
    /// The protocol version of the issued milestone.
    pub protocol_version: u8,
    /// The id of the previous milestone, as they form a chain.
    pub previous_milestone_id: MilestoneId,
    /// The parents of the milestone.
    pub parents: Box<[BlockId]>,
    #[serde(with = "bytify")]
    /// The Merkle root of all blocks included in this milestone.
    pub inclusion_merkle_root: [u8; Self::MERKLE_PROOF_LENGTH],
    #[serde(with = "bytify")]
    /// The Merkle root of all blocks that contain state-mutating transactions.
    pub applied_merkle_root: [u8; Self::MERKLE_PROOF_LENGTH],
    /// The metadata of the milestone.
    #[serde(with = "serde_bytes")]
    pub metadata: Vec<u8>,
    /// Additional information that can get transmitted with an milestone.
    pub options: Box<[MilestoneOption]>,
}

impl MilestoneEssence {
    const MERKLE_PROOF_LENGTH: usize = iota::MerkleRoot::LENGTH;
}

impl<T: Borrow<iota::MilestoneEssence>> From<T> for MilestoneEssence {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            index: value.index().0.into(),
            timestamp: value.timestamp().into(),
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

impl TryFromWithContext<MilestoneEssence> for iota::MilestoneEssence {
    type Error = iota_types::block::Error;

    fn try_from_with_context(
        ctx: &iota_types::block::protocol::ProtocolParameters,
        value: MilestoneEssence,
    ) -> Result<Self, Self::Error> {
        iota::MilestoneEssence::new(
            value.index.into(),
            value.timestamp.0,
            value.protocol_version,
            value.previous_milestone_id.into(),
            iota_types::block::parent::Parents::new(
                value.parents.into_vec().into_iter().map(Into::into).collect::<Vec<_>>(),
            )?,
            iota_types::block::payload::milestone::MerkleRoot::from(value.inclusion_merkle_root),
            iota_types::block::payload::milestone::MerkleRoot::from(value.applied_merkle_root),
            value.metadata,
            iota_types::block::payload::MilestoneOptions::new(
                value
                    .options
                    .into_vec()
                    .into_iter()
                    .map(|x| x.try_into_with_context(ctx))
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
        )
    }
}

/// Additional information that belongs to a milestone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum MilestoneOption {
    /// The receipt of a Chrysalis migration process.
    Receipt {
        /// The index of the legacy milestone in which the listed funds were migrated at.
        migrated_at: MilestoneIndex,
        /// Indicates that this receipt is the last receipt for the given `migrated_at` index.
        last: bool,
        /// The funds that have been migrated.
        funds: Box<[MigratedFundsEntry]>,
        /// The payload that updates the treasury accordingly.
        transaction: TreasuryTransactionPayload,
    },
    /// An update of the [`ProtocolParameters`](crate::model::protocol::ProtocolParameters).
    Parameters {
        /// The target milestone for when the update will become active.
        target_milestone_index: MilestoneIndex,
        /// The new protocol version.
        protocol_version: u8,
        /// The [`ProtocolParameters`](crate::model::protocol::ProtocolParameters) in binary representation.
        binary_parameters: Box<[u8]>,
    },
}

impl<T: Borrow<iota::MilestoneOption>> From<T> for MilestoneOption {
    fn from(value: T) -> Self {
        match value.borrow() {
            iota::MilestoneOption::Receipt(r) => Self::Receipt {
                migrated_at: r.migrated_at().into(),
                last: r.last(),
                funds: r.funds().iter().map(Into::into).collect(),
                transaction: r.transaction().into(),
            },
            iota::MilestoneOption::Parameters(p) => Self::Parameters {
                target_milestone_index: p.target_milestone_index().into(),
                protocol_version: p.protocol_version(),
                binary_parameters: p.binary_parameters().to_owned().into_boxed_slice(),
            },
        }
    }
}

impl TryFromWithContext<MilestoneOption> for iota::MilestoneOption {
    type Error = iota_types::block::Error;

    fn try_from_with_context(
        ctx: &iota_types::block::protocol::ProtocolParameters,
        value: MilestoneOption,
    ) -> Result<Self, Self::Error> {
        Ok(match value {
            MilestoneOption::Receipt {
                migrated_at,
                last,
                funds,
                transaction,
            } => Self::Receipt(iota::ReceiptMilestoneOption::new(
                migrated_at.into(),
                last,
                funds
                    .into_vec()
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
            } => Self::Parameters(iota::ParametersMilestoneOption::new(
                target_milestone_index.into(),
                protocol_version,
                binary_parameters.into_vec(),
            )?),
        })
    }
}

impl From<MilestoneOption> for iota::option::dto::MilestoneOptionDto {
    fn from(value: MilestoneOption) -> Self {
        match value {
            MilestoneOption::Receipt {
                migrated_at,
                last,
                funds,
                transaction,
            } => Self::Receipt(iota::option::dto::ReceiptMilestoneOptionDto {
                kind: iota::option::ReceiptMilestoneOption::KIND,
                migrated_at: migrated_at.0,
                funds: funds.into_vec().into_iter().map(Into::into).collect(),
                transaction: iota_types::block::payload::dto::PayloadDto::TreasuryTransaction(Box::new(
                    transaction.into(),
                )),
                last,
            }),
            MilestoneOption::Parameters {
                target_milestone_index,
                protocol_version,
                binary_parameters,
            } => Self::Parameters(iota::option::dto::ParametersMilestoneOptionDto {
                kind: iota::option::ParametersMilestoneOption::KIND,
                target_milestone_index: target_milestone_index.0,
                protocol_version,
                binary_parameters: prefix_hex::encode(binary_parameters),
            }),
        }
    }
}

/// Represents the migration of a given address.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigratedFundsEntry {
    /// The tail transaction hash of the bundle in which these funds were migrated.
    #[serde(with = "bytify")]
    tail_transaction_hash: [u8; Self::TAIL_TRANSACTION_HASH_LENGTH],
    /// The target address.
    address: Address,
    /// The amount of tokens that have been migrated.
    #[serde(with = "stringify")]
    amount: u64,
}

impl MigratedFundsEntry {
    const TAIL_TRANSACTION_HASH_LENGTH: usize = iota::option::TailTransactionHash::LENGTH;
}

impl<T: Borrow<iota::option::MigratedFundsEntry>> From<T> for MigratedFundsEntry {
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

impl TryFromWithContext<MigratedFundsEntry> for iota::option::MigratedFundsEntry {
    type Error = iota_types::block::Error;

    fn try_from_with_context(
        ctx: &iota_types::block::protocol::ProtocolParameters,
        value: MigratedFundsEntry,
    ) -> Result<Self, Self::Error> {
        Self::new(
            iota::option::TailTransactionHash::new(value.tail_transaction_hash)?,
            value.address.into(),
            value.amount,
            ctx.token_supply(),
        )
    }
}

impl From<MigratedFundsEntry> for iota::option::dto::MigratedFundsEntryDto {
    fn from(value: MigratedFundsEntry) -> Self {
        Self {
            tail_transaction_hash: prefix_hex::encode(value.tail_transaction_hash),
            address: value.address.into(),
            deposit: value.amount,
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use iota_types::block::rand::{
        bytes::rand_bytes, milestone::rand_merkle_root, milestone_option::rand_receipt_milestone_option,
        number::rand_number, payload::rand_milestone_payload, receipt::rand_migrated_funds_entry,
    };

    use super::*;

    impl MilestonePayload {
        /// Generates a random [`MilestonePayload`].
        pub fn rand(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
            rand_milestone_payload(ctx.protocol_version()).into()
        }
    }

    impl MilestoneEssence {
        /// Generates a random [`MilestoneEssence`].
        pub fn rand(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
            Self {
                index: rand_number::<u32>().into(),
                timestamp: rand_number::<u32>().into(),
                protocol_version: rand_number::<u8>(),
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
        pub fn rand_receipt(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
            iota::MilestoneOption::from(rand_receipt_milestone_option(ctx.token_supply())).into()
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
        pub fn rand(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
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
        let ctx = iota_types::block::protocol::protocol_parameters();
        let payload = MilestonePayload::rand(&ctx);
        iota::MilestonePayload::try_from_with_context(&ctx, payload.clone()).unwrap();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(payload, from_bson::<MilestonePayload>(bson).unwrap());
    }
}
