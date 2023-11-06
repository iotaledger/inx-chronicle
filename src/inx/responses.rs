// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use inx::proto;
use iota_sdk::types::{
    api::core::{BlockFailureReason, BlockState, TransactionState},
    block::{
        semantic::TransactionFailureReason,
        slot::{EpochIndex, SlotCommitment, SlotCommitmentId, SlotIndex},
        BlockId, SignedBlock,
    },
};
use packable::PackableExt;
use serde::{Deserialize, Serialize};

use super::{
    convert::{ConvertTo, TryConvertFrom, TryConvertTo},
    ledger::{LedgerOutput, LedgerSpent},
    raw::Raw,
    InxError,
};
use crate::maybe_missing;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub block_id: BlockId,
    pub block: Raw<SignedBlock>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockMetadata {
    pub block_id: BlockId,
    pub block_state: BlockState,
    pub transaction_state: Option<TransactionState>,
    pub block_failure_reason: Option<BlockFailureReason>,
    pub transaction_failure_reason: Option<TransactionFailureReason>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    pub latest_commitment_id: SlotCommitmentId,
    pub payload: OutputPayload,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutputPayload {
    Spent(LedgerSpent),
    Output(LedgerOutput),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolParameters {
    pub start_epoch: EpochIndex,
    pub parameters: iota_sdk::types::block::protocol::ProtocolParameters,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseToken {
    pub name: String,
    pub ticker_symbol: String,
    pub unit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subunit: Option<String>,
    pub decimals: u32,
    pub use_metric_prefix: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeConfiguration {
    pub base_token: BaseToken,
    pub protocol_parameters: Vec<ProtocolParameters>,
}

pub struct NodeStatus {
    pub is_healthy: bool,
    pub accepted_tangle_time: Option<u64>,
    pub relative_accepted_tangle_time: Option<u64>,
    pub confirmed_tangle_time: Option<u64>,
    pub relative_confirmed_tangle_time: Option<u64>,
    pub latest_commitment_id: SlotCommitmentId,
    pub latest_finalized_slot: SlotIndex,
    pub latest_accepted_block_slot: Option<SlotIndex>,
    pub latest_confirmed_block_slot: Option<SlotIndex>,
    pub pruning_epoch: EpochIndex,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootBlocks {
    pub root_blocks: Vec<RootBlock>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootBlock {
    pub block_id: BlockId,
    pub commitment_id: SlotCommitmentId,
}

#[derive(Clone, Debug, PartialEq, Eq)]

pub struct Commitment {
    pub commitment_id: SlotCommitmentId,
    pub commitment: Raw<SlotCommitment>,
}

impl TryConvertFrom<proto::RawProtocolParameters> for ProtocolParameters {
    type Error = InxError;

    fn try_convert_from(proto: proto::RawProtocolParameters) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self {
            start_epoch: proto.start_epoch.into(),
            parameters: PackableExt::unpack_unverified(proto.params)
                .map_err(|e| InxError::InvalidRawBytes(format!("{e:?}")))?,
        })
    }
}

impl TryConvertFrom<proto::NodeStatus> for NodeStatus {
    type Error = InxError;

    fn try_convert_from(proto: proto::NodeStatus) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self {
            is_healthy: proto.is_healthy,
            accepted_tangle_time: todo!(),
            relative_accepted_tangle_time: todo!(),
            confirmed_tangle_time: todo!(),
            relative_confirmed_tangle_time: todo!(),
            latest_commitment_id: todo!(),
            latest_finalized_slot: todo!(),
            latest_accepted_block_slot: todo!(),
            latest_confirmed_block_slot: todo!(),
            pruning_epoch: todo!(),
        })
    }
}

impl TryConvertFrom<proto::BaseToken> for BaseToken {
    type Error = InxError;

    fn try_convert_from(proto: proto::BaseToken) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self {
            name: proto.name,
            ticker_symbol: proto.ticker_symbol,
            unit: proto.unit,
            subunit: Some(proto.subunit),
            decimals: proto.decimals,
            use_metric_prefix: proto.use_metric_prefix,
        })
    }
}

impl TryConvertFrom<proto::NodeConfiguration> for NodeConfiguration {
    type Error = InxError;

    fn try_convert_from(proto: proto::NodeConfiguration) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self {
            base_token: maybe_missing!(proto.base_token).try_convert()?,
            protocol_parameters: proto
                .protocol_parameters
                .into_iter()
                .map(TryConvertTo::try_convert)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl TryConvertFrom<proto::RootBlock> for RootBlock {
    type Error = InxError;

    fn try_convert_from(proto: proto::RootBlock) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self {
            block_id: maybe_missing!(proto.block_id).try_convert()?,
            commitment_id: maybe_missing!(proto.commitment_id).try_convert()?,
        })
    }
}

impl TryConvertFrom<proto::RootBlocksResponse> for RootBlocks {
    type Error = InxError;

    fn try_convert_from(proto: proto::RootBlocksResponse) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self {
            root_blocks: proto
                .root_blocks
                .into_iter()
                .map(TryConvertTo::try_convert)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl TryConvertFrom<proto::Commitment> for Commitment {
    type Error = InxError;

    fn try_convert_from(proto: proto::Commitment) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self {
            commitment_id: maybe_missing!(proto.commitment_id).try_convert()?,
            commitment: maybe_missing!(proto.commitment).into(),
        })
    }
}

impl TryConvertFrom<proto::Block> for Block {
    type Error = InxError;

    fn try_convert_from(proto: proto::Block) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self {
            block_id: maybe_missing!(proto.block_id).try_convert()?,
            block: maybe_missing!(proto.block).into(),
        })
    }
}

impl TryConvertFrom<proto::BlockMetadata> for BlockMetadata {
    type Error = InxError;

    fn try_convert_from(proto: proto::BlockMetadata) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self {
            block_state: proto.block_state().convert(),
            transaction_state: proto.transaction_state().convert(),
            block_failure_reason: proto.block_failure_reason().convert(),
            transaction_failure_reason: proto.transaction_failure_reason().convert(),
            block_id: maybe_missing!(proto.block_id).try_convert()?,
        })
    }
}

impl TryConvertFrom<proto::OutputResponse> for Output {
    type Error = InxError;

    fn try_convert_from(proto: proto::OutputResponse) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self {
            latest_commitment_id: maybe_missing!(proto.latest_commitment_id).try_convert()?,
            payload: maybe_missing!(proto.payload).try_convert()?,
        })
    }
}

impl TryConvertFrom<proto::output_response::Payload> for OutputPayload {
    type Error = InxError;

    fn try_convert_from(proto: proto::output_response::Payload) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(match proto {
            proto::output_response::Payload::Output(o) => Self::Output(o.try_convert()?),
            proto::output_response::Payload::Spent(o) => Self::Spent(o.try_convert()?),
        })
    }
}
