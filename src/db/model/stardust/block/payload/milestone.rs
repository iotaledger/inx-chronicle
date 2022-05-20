// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_block_stardust::payload::milestone as bee;
use serde::{Deserialize, Serialize};

use super::super::{Address, BlockId, Signature, TreasuryTransactionPayload};
use crate::db;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MilestoneId(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl From<bee::MilestoneId> for MilestoneId {
    fn from(value: bee::MilestoneId) -> Self {
        Self(value.to_vec().into_boxed_slice())
    }
}

impl TryFrom<MilestoneId> for bee::MilestoneId {
    type Error = db::error::Error;

    fn try_from(value: MilestoneId) -> Result<Self, Self::Error> {
        Ok(bee::MilestoneId::new(value.0.as_ref().try_into()?))
    }
}

impl FromStr for MilestoneId {
    type Err = db::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::MilestoneId::from_str(s)?.into())
    }
}

pub type MilestoneIndex = u32;

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
    type Error = db::error::Error;

    fn try_from(value: MilestonePayload) -> Result<Self, Self::Error> {
        Ok(bee::MilestonePayload::new(
            value.essence.try_into()?,
            Vec::from(value.signatures)
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, _>>()?,
        )?)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MilestoneEssence {
    pub index: MilestoneIndex,
    pub timestamp: u32,
    pub previous_milestone_id: MilestoneId,
    pub parents: Box<[BlockId]>,
    #[serde(with = "serde_bytes")]
    pub confirmed_merkle_proof: Box<[u8]>,
    #[serde(with = "serde_bytes")]
    pub applied_merkle_proof: Box<[u8]>,
    #[serde(with = "serde_bytes")]
    pub metadata: Vec<u8>,
    pub options: Box<[MilestoneOption]>,
}

impl From<&bee::MilestoneEssence> for MilestoneEssence {
    fn from(value: &bee::MilestoneEssence) -> Self {
        Self {
            index: value.index().0,
            timestamp: value.timestamp(),
            previous_milestone_id: (*value.previous_milestone_id()).into(),
            parents: value.parents().iter().map(|id| BlockId::from(*id)).collect(),
            confirmed_merkle_proof: value.confirmed_merkle_root().to_vec().into_boxed_slice(),
            applied_merkle_proof: value.applied_merkle_root().to_vec().into_boxed_slice(),
            metadata: value.metadata().to_vec(),
            options: value.options().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<MilestoneEssence> for bee::MilestoneEssence {
    type Error = db::error::Error;

    fn try_from(value: MilestoneEssence) -> Result<Self, Self::Error> {
        Ok(bee::MilestoneEssence::new(
            value.index.into(),
            value.timestamp,
            value.previous_milestone_id.try_into()?,
            bee_block_stardust::parent::Parents::new(
                Vec::from(value.parents)
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
            value.confirmed_merkle_proof.as_ref().try_into()?,
            value.applied_merkle_proof.as_ref().try_into()?,
            value.metadata,
            bee_block_stardust::payload::MilestoneOptions::new(
                Vec::from(value.options)
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
        )?)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum MilestoneOption {
    #[serde(rename = "receipt")]
    Receipt {
        migrated_at: MilestoneIndex,
        last: bool,
        funds: Box<[MigratedFundsEntry]>,
        transaction: TreasuryTransactionPayload,
    },
    #[serde(rename = "parameters")]
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
                migrated_at: r.migrated_at().0,
                last: r.last(),
                funds: r.funds().iter().map(Into::into).collect(),
                transaction: r.transaction().into(),
            },
            bee::MilestoneOption::Parameters(p) => Self::Parameters {
                target_milestone_index: p.target_milestone_index().0,
                protocol_version: p.protocol_version(),
                binary_parameters: p.binary_parameters().to_owned().into_boxed_slice(),
            },
        }
    }
}

impl TryFrom<MilestoneOption> for bee::MilestoneOption {
    type Error = db::error::Error;

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
                bee::MilestoneIndex(target_milestone_index),
                protocol_version,
                binary_parameters.into_vec(),
            )?),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigratedFundsEntry {
    #[serde(with = "serde_bytes")]
    tail_transaction_hash: Box<[u8]>,
    address: Address,
    #[serde(with = "crate::db::model::util::stringify")]
    amount: u64,
}

impl From<&bee::option::MigratedFundsEntry> for MigratedFundsEntry {
    fn from(value: &bee::option::MigratedFundsEntry) -> Self {
        Self {
            tail_transaction_hash: value.tail_transaction_hash().as_ref().to_vec().into_boxed_slice(),
            address: (*value.address()).into(),
            amount: value.amount(),
        }
    }
}

impl TryFrom<MigratedFundsEntry> for bee::option::MigratedFundsEntry {
    type Error = db::error::Error;

    fn try_from(value: MigratedFundsEntry) -> Result<Self, Self::Error> {
        Ok(Self::new(
            bee::option::TailTransactionHash::new(value.tail_transaction_hash.as_ref().try_into()?)?,
            value.address.try_into()?,
            value.amount,
        )?)
    }
}

#[cfg(test)]
pub(crate) mod test {
    const TAIL_TRANSACTION_HASH1: [u8; 49] = [
        222, 235, 107, 67, 2, 173, 253, 93, 165, 90, 166, 45, 102, 91, 19, 137, 71, 146, 156, 180, 248, 31, 56, 25, 68,
        154, 98, 100, 64, 108, 203, 48, 76, 75, 114, 150, 34, 153, 203, 35, 225, 120, 194, 175, 169, 207, 80, 229, 10,
    ];
    const TAIL_TRANSACTION_HASH2: [u8; 49] = [
        222, 235, 107, 67, 2, 173, 253, 93, 165, 90, 166, 45, 102, 91, 19, 137, 71, 146, 156, 180, 248, 31, 56, 25, 68,
        154, 98, 100, 64, 108, 203, 48, 76, 75, 114, 150, 34, 153, 203, 35, 225, 120, 194, 175, 169, 207, 80, 229, 11,
    ];
    const TAIL_TRANSACTION_HASH3: [u8; 49] = [
        222, 235, 107, 67, 2, 173, 253, 93, 165, 90, 166, 45, 102, 91, 19, 137, 71, 146, 156, 180, 248, 31, 56, 25, 68,
        154, 98, 100, 64, 108, 203, 48, 76, 75, 114, 150, 34, 153, 203, 35, 225, 120, 194, 175, 169, 207, 80, 229, 12,
    ];

    use bee_block_stardust::payload::TreasuryTransactionPayload;
    use mongodb::bson::{from_bson, to_bson};

    use super::*;
    use crate::db::model::stardust::block::signature::test::get_test_signature;

    #[test]
    fn test_milestone_id_bson() {
        let milestone_id = MilestoneId::from(bee_test::rand::milestone::rand_milestone_id());
        let bson = to_bson(&milestone_id).unwrap();
        assert_eq!(milestone_id, from_bson::<MilestoneId>(bson).unwrap());
    }

    #[test]
    fn test_milestone_payload_bson() {
        let payload = get_test_milestone_payload();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(payload, from_bson::<MilestonePayload>(bson).unwrap());
    }

    pub(crate) fn get_test_ed25519_migrated_funds_entry() -> MigratedFundsEntry {
        MigratedFundsEntry::from(
            &bee::option::MigratedFundsEntry::new(
                bee::option::TailTransactionHash::new(TAIL_TRANSACTION_HASH1).unwrap(),
                bee_block_stardust::address::Address::Ed25519(bee_test::rand::address::rand_ed25519_address()),
                2000000,
            )
            .unwrap(),
        )
    }

    pub(crate) fn get_test_alias_migrated_funds_entry() -> MigratedFundsEntry {
        MigratedFundsEntry::from(
            &bee::option::MigratedFundsEntry::new(
                bee::option::TailTransactionHash::new(TAIL_TRANSACTION_HASH2).unwrap(),
                bee_block_stardust::address::Address::Alias(bee_test::rand::address::rand_alias_address()),
                2000000,
            )
            .unwrap(),
        )
    }

    pub(crate) fn get_test_nft_migrated_funds_entry() -> MigratedFundsEntry {
        MigratedFundsEntry::from(
            &bee::option::MigratedFundsEntry::new(
                bee::option::TailTransactionHash::new(TAIL_TRANSACTION_HASH3).unwrap(),
                bee_block_stardust::address::Address::Nft(bee_test::rand::address::rand_nft_address()),
                2000000,
            )
            .unwrap(),
        )
    }

    pub(crate) fn get_test_milestone_essence() -> MilestoneEssence {
        MilestoneEssence::from(
            &bee::MilestoneEssence::new(
                1.into(),
                12345,
                bee_test::rand::milestone::rand_milestone_id(),
                bee_test::rand::parents::rand_parents(),
                bee_test::rand::bytes::rand_bytes_array(),
                bee_test::rand::bytes::rand_bytes_array(),
                "Foo".as_bytes().to_vec(),
                bee::MilestoneOptions::new(vec![bee::option::MilestoneOption::Receipt(
                    bee::option::ReceiptMilestoneOption::new(
                        1.into(),
                        false,
                        vec![
                            get_test_ed25519_migrated_funds_entry().try_into().unwrap(),
                            get_test_alias_migrated_funds_entry().try_into().unwrap(),
                            get_test_nft_migrated_funds_entry().try_into().unwrap(),
                        ],
                        TreasuryTransactionPayload::new(
                            bee_test::rand::input::rand_treasury_input(),
                            bee_test::rand::output::rand_treasury_output(),
                        )
                        .unwrap(),
                    )
                    .unwrap(),
                )])
                .unwrap(),
            )
            .unwrap(),
        )
    }

    pub(crate) fn get_test_milestone_payload() -> MilestonePayload {
        MilestonePayload::from(
            &bee::MilestonePayload::new(
                get_test_milestone_essence().try_into().unwrap(),
                vec![get_test_signature().try_into().unwrap()],
            )
            .unwrap(),
        )
    }
}
