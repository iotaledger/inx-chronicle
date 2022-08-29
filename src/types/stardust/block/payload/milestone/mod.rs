// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod milestone_id;

use bee_block_stardust::payload::milestone as bee;
use serde::{Deserialize, Serialize};

pub use self::milestone_id::MilestoneId;
use crate::types::{
    stardust::block::{payload::treasury_transaction::TreasuryTransactionPayload, Address, BlockId, Signature},
    tangle::MilestoneIndex,
    util::bytify,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MilestonePayload {
    pub essence: MilestoneEssence,
    pub signatures: Box<[Signature]>,
}

impl From<&bee::MilestonePayload> for MilestonePayload {
    fn from(value: &bee::MilestonePayload) -> Self {
        Self {
            essence: MilestoneEssence::from(value.essence()),
            signatures: value.signatures().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<MilestonePayload> for bee::MilestonePayload {
    type Error = bee_block_stardust::Error;

    fn try_from(value: MilestonePayload) -> Result<Self, Self::Error> {
        bee::MilestonePayload::new(
            value.essence.try_into()?,
            Vec::from(value.signatures)
                .into_iter()
                .map(Into::into)
                .collect::<Vec<_>>(),
        )
    }
}

impl TryFrom<MilestonePayload> for bee::dto::MilestonePayloadDto {
    type Error = bee_block_stardust::Error;

    fn try_from(value: MilestonePayload) -> Result<Self, Self::Error> {
        let stardust = bee::MilestonePayload::try_from(value)?;
        Ok(Self::from(&stardust))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MilestoneEssence {
    pub index: MilestoneIndex,
    pub timestamp: u32,
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

impl From<&bee::MilestoneEssence> for MilestoneEssence {
    fn from(value: &bee::MilestoneEssence) -> Self {
        Self {
            index: value.index().0.into(),
            timestamp: value.timestamp(),
            previous_milestone_id: (*value.previous_milestone_id()).into(),
            parents: value.parents().iter().map(|&id| BlockId::from(id)).collect(),
            inclusion_merkle_root: **value.inclusion_merkle_root(),
            applied_merkle_root: **value.applied_merkle_root(),
            metadata: value.metadata().to_vec(),
            options: value.options().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<MilestoneEssence> for bee::MilestoneEssence {
    type Error = bee_block_stardust::Error;

    fn try_from(value: MilestoneEssence) -> Result<Self, Self::Error> {
        bee::MilestoneEssence::new(
            value.index.into(),
            value.timestamp,
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
                    .map(TryInto::try_into)
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

impl From<&bee::MilestoneOption> for MilestoneOption {
    fn from(value: &bee::MilestoneOption) -> Self {
        match value {
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

impl TryFrom<MilestoneOption> for bee::MilestoneOption {
    type Error = bee_block_stardust::Error;

    fn try_from(value: MilestoneOption) -> Result<Self, Self::Error> {
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
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
                transaction.try_into()?,
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

impl From<&bee::option::MigratedFundsEntry> for MigratedFundsEntry {
    fn from(value: &bee::option::MigratedFundsEntry) -> Self {
        Self {
            // Unwrap: Should not fail as the length is defined by the struct
            tail_transaction_hash: value.tail_transaction_hash().as_ref().try_into().unwrap(),
            address: (*value.address()).into(),
            amount: value.amount(),
        }
    }
}

impl TryFrom<MigratedFundsEntry> for bee::option::MigratedFundsEntry {
    type Error = bee_block_stardust::Error;

    fn try_from(value: MigratedFundsEntry) -> Result<Self, Self::Error> {
        Self::new(
            bee::option::TailTransactionHash::new(value.tail_transaction_hash)?,
            value.address.into(),
            value.amount,
        )
    }
}

#[cfg(test)]
mod test {
    use mongodb::bson::{from_bson, to_bson, Bson};

    use super::*;
    use crate::types::stardust::util::payload::milestone::get_test_milestone_payload;

    #[test]
    fn test_milestone_id_bson() {
        let milestone_id = MilestoneId::from(bee_block_stardust::rand::milestone::rand_milestone_id());
        let bson = to_bson(&milestone_id).unwrap();
        assert_eq!(Bson::from(milestone_id), bson);
        assert_eq!(milestone_id, from_bson::<MilestoneId>(bson).unwrap());
    }

    #[test]
    fn test_milestone_payload_bson() {
        let payload = get_test_milestone_payload();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(payload, from_bson::<MilestonePayload>(bson).unwrap());
    }
}
