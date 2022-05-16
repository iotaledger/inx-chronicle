// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use bee_message_stardust::payload::milestone as stardust;
use serde::{Deserialize, Serialize};

use crate::types::stardust::message::{Address, MessageId, Signature, TreasuryTransactionPayload};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MilestoneId(#[serde(with = "serde_bytes")] pub Box<[u8]>);

impl From<stardust::MilestoneId> for MilestoneId {
    fn from(value: stardust::MilestoneId) -> Self {
        Self(value.to_vec().into_boxed_slice())
    }
}

impl TryFrom<MilestoneId> for stardust::MilestoneId {
    type Error = crate::types::error::Error;

    fn try_from(value: MilestoneId) -> Result<Self, Self::Error> {
        Ok(stardust::MilestoneId::new(value.0.as_ref().try_into()?))
    }
}

impl FromStr for MilestoneId {
    type Err = crate::types::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(stardust::MilestoneId::from_str(s)?.into())
    }
}

pub type MilestoneIndex = u32;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MilestonePayload {
    pub essence: MilestoneEssence,
    pub signatures: Box<[Signature]>,
}

impl From<&stardust::MilestonePayload> for MilestonePayload {
    fn from(value: &stardust::MilestonePayload) -> Self {
        Self {
            essence: MilestoneEssence::from(value.essence()),
            signatures: value.signatures().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<MilestonePayload> for stardust::MilestonePayload {
    type Error = crate::types::error::Error;

    fn try_from(value: MilestonePayload) -> Result<Self, Self::Error> {
        Ok(stardust::MilestonePayload::new(
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
    pub parents: Box<[MessageId]>,
    #[serde(with = "serde_bytes")]
    pub confirmed_merkle_proof: Box<[u8]>,
    #[serde(with = "serde_bytes")]
    pub applied_merkle_proof: Box<[u8]>,
    #[serde(with = "serde_bytes")]
    pub metadata: Vec<u8>,
    pub options: Box<[MilestoneOption]>,
}

impl From<&stardust::MilestoneEssence> for MilestoneEssence {
    fn from(value: &stardust::MilestoneEssence) -> Self {
        Self {
            index: value.index().0,
            timestamp: value.timestamp(),
            previous_milestone_id: (*value.previous_milestone_id()).into(),
            parents: value.parents().iter().map(|id| MessageId::from(*id)).collect(),
            confirmed_merkle_proof: value.confirmed_merkle_root().to_vec().into_boxed_slice(),
            applied_merkle_proof: value.applied_merkle_root().to_vec().into_boxed_slice(),
            metadata: value.metadata().to_vec(),
            options: value.options().iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<MilestoneEssence> for stardust::MilestoneEssence {
    type Error = crate::types::error::Error;

    fn try_from(value: MilestoneEssence) -> Result<Self, Self::Error> {
        Ok(stardust::MilestoneEssence::new(
            value.index.into(),
            value.timestamp,
            value.previous_milestone_id.try_into()?,
            bee_message_stardust::parent::Parents::new(
                Vec::from(value.parents)
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
            value.confirmed_merkle_proof.as_ref().try_into()?,
            value.applied_merkle_proof.as_ref().try_into()?,
            value.metadata,
            bee_message_stardust::payload::MilestoneOptions::new(
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
    #[serde(rename = "pow")]
    Pow {
        next_pow_score: u32,
        next_pow_score_milestone_index: u32,
    },
}

impl From<&stardust::MilestoneOption> for MilestoneOption {
    fn from(value: &stardust::MilestoneOption) -> Self {
        match value {
            stardust::MilestoneOption::Receipt(r) => Self::Receipt {
                migrated_at: r.migrated_at().0,
                last: r.last(),
                funds: r.funds().iter().map(Into::into).collect(),
                transaction: r.transaction().into(),
            },
            stardust::MilestoneOption::Pow(p) => Self::Pow {
                next_pow_score: p.next_pow_score(),
                next_pow_score_milestone_index: p.next_pow_score_milestone_index(),
            },
        }
    }
}

impl TryFrom<MilestoneOption> for stardust::MilestoneOption {
    type Error = crate::types::error::Error;

    fn try_from(value: MilestoneOption) -> Result<Self, Self::Error> {
        Ok(match value {
            MilestoneOption::Receipt {
                migrated_at,
                last,
                funds,
                transaction,
            } => Self::Receipt(stardust::ReceiptMilestoneOption::new(
                migrated_at.into(),
                last,
                Vec::from(funds)
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
                transaction.try_into()?,
            )?),
            MilestoneOption::Pow {
                next_pow_score,
                next_pow_score_milestone_index,
            } => Self::Pow(stardust::PowMilestoneOption::new(
                next_pow_score,
                next_pow_score_milestone_index,
            )?),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigratedFundsEntry {
    #[serde(with = "serde_bytes")]
    tail_transaction_hash: Box<[u8]>,
    address: Address,
    #[serde(with = "crate::types::stringify")]
    amount: u64,
}

impl From<&stardust::option::MigratedFundsEntry> for MigratedFundsEntry {
    fn from(value: &stardust::option::MigratedFundsEntry) -> Self {
        Self {
            tail_transaction_hash: value.tail_transaction_hash().as_ref().to_vec().into_boxed_slice(),
            address: (*value.address()).into(),
            amount: value.amount(),
        }
    }
}

impl TryFrom<MigratedFundsEntry> for stardust::option::MigratedFundsEntry {
    type Error = crate::types::error::Error;

    fn try_from(value: MigratedFundsEntry) -> Result<Self, Self::Error> {
        Ok(Self::new(
            stardust::option::TailTransactionHash::new(value.tail_transaction_hash.as_ref().try_into()?)?,
            value.address.try_into()?,
            value.amount,
        )?)
    }
}

#[cfg(test)]
pub(crate) mod test {
    pub(crate) const MILESTONE_ID: &str = "0x52fdfc072182654f163f5f0f9a621d729566c74d10037c4d7bbb0407d1e2c649";
    pub(crate) const MERKLE_PROOF: [u8; stardust::MilestoneEssence::MERKLE_ROOT_LENGTH] =
        [0; stardust::MilestoneEssence::MERKLE_ROOT_LENGTH];
    pub(crate) const APPLIED_MERKLE_PROOF: [u8; stardust::MilestoneEssence::MERKLE_ROOT_LENGTH] =
        [0; stardust::MilestoneEssence::MERKLE_ROOT_LENGTH];
    pub(crate) const METADATA: &str = "Foo";
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

    use bee_message_stardust::{
        input::TreasuryInput, output::TreasuryOutput, parent::Parents, payload::TreasuryTransactionPayload,
    };
    use mongodb::bson::{from_bson, to_bson};

    use super::*;
    use crate::types::stardust::message::{
        address::test::{get_test_alias_address, get_test_ed25519_address, get_test_nft_address},
        signature::test::get_test_signature,
        tests::get_test_message_id,
    };

    #[test]
    fn test_milestone_id_bson() {
        let milestone_id = get_test_milestone_id();
        let bson = to_bson(&milestone_id).unwrap();
        assert_eq!(milestone_id, from_bson::<MilestoneId>(bson).unwrap());
    }

    #[test]
    fn test_milestone_payload_bson() {
        let payload = get_test_milestone_payload();
        let bson = to_bson(&payload).unwrap();
        assert_eq!(payload, from_bson::<MilestonePayload>(bson).unwrap());
    }

    pub(crate) fn get_test_milestone_id() -> MilestoneId {
        MilestoneId::from_str(MILESTONE_ID).unwrap()
    }

    pub(crate) fn get_test_ed25519_migrated_funds_entry() -> MigratedFundsEntry {
        MigratedFundsEntry::from(
            &stardust::option::MigratedFundsEntry::new(
                stardust::option::TailTransactionHash::new(TAIL_TRANSACTION_HASH1).unwrap(),
                get_test_ed25519_address().try_into().unwrap(),
                2000000,
            )
            .unwrap(),
        )
    }

    pub(crate) fn get_test_alias_migrated_funds_entry() -> MigratedFundsEntry {
        MigratedFundsEntry::from(
            &stardust::option::MigratedFundsEntry::new(
                stardust::option::TailTransactionHash::new(TAIL_TRANSACTION_HASH2).unwrap(),
                get_test_alias_address().try_into().unwrap(),
                2000000,
            )
            .unwrap(),
        )
    }

    pub(crate) fn get_test_nft_migrated_funds_entry() -> MigratedFundsEntry {
        MigratedFundsEntry::from(
            &stardust::option::MigratedFundsEntry::new(
                stardust::option::TailTransactionHash::new(TAIL_TRANSACTION_HASH3).unwrap(),
                get_test_nft_address().try_into().unwrap(),
                2000000,
            )
            .unwrap(),
        )
    }

    pub(crate) fn get_test_milestone_essence() -> MilestoneEssence {
        MilestoneEssence::from(
            &stardust::MilestoneEssence::new(
                1.into(),
                12345,
                get_test_milestone_id().try_into().unwrap(),
                Parents::new(vec![get_test_message_id().try_into().unwrap()]).unwrap(),
                MERKLE_PROOF,
                APPLIED_MERKLE_PROOF,
                METADATA.as_bytes().to_vec(),
                stardust::MilestoneOptions::new(vec![stardust::option::MilestoneOption::Receipt(
                    stardust::option::ReceiptMilestoneOption::new(
                        1.into(),
                        false,
                        vec![
                            get_test_ed25519_migrated_funds_entry().try_into().unwrap(),
                            get_test_alias_migrated_funds_entry().try_into().unwrap(),
                            get_test_nft_migrated_funds_entry().try_into().unwrap(),
                        ],
                        TreasuryTransactionPayload::new(
                            TreasuryInput::new(get_test_milestone_id().try_into().unwrap()),
                            TreasuryOutput::new(100).unwrap(),
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
            &stardust::MilestonePayload::new(
                get_test_milestone_essence().try_into().unwrap(),
                vec![get_test_signature().try_into().unwrap()],
            )
            .unwrap(),
        )
    }
}
