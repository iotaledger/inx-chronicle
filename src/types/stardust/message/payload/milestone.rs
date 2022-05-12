// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::payload::milestone as stardust;
use serde::{Deserialize, Serialize};

use crate::types::stardust::message::{Address, MessageId, Signature, TreasuryTransactionPayload};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

pub type MilestoneIndex = u32;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
            address: value.address().into(),
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
