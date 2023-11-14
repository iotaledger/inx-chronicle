// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

use inx::proto;
use iota_sdk::types::block::{slot::SlotCommitmentId, BlockId, SignedBlock};
use packable::PackableExt;

use super::{
    convert::{ConvertTo, TryConvertFrom, TryConvertTo},
    InxError,
};
use crate::{
    maybe_missing,
    model::{
        block_metadata::{BlockMetadata, BlockWithMetadata},
        ledger::{LedgerOutput, LedgerSpent},
        node::{BaseToken, NodeConfiguration, NodeStatus},
        protocol::ProtocolParameters,
        raw::{InvalidRawBytesError, Raw},
        slot::Commitment,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub block_id: BlockId,
    pub block: Raw<SignedBlock>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootBlocks {
    pub root_blocks: Vec<RootBlock>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootBlock {
    pub block_id: BlockId,
    pub commitment_id: SlotCommitmentId,
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
                .map_err(|e| InvalidRawBytesError(format!("error unpacking protocol parameters: {e:?}")))?,
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
            last_accepted_block_slot: proto.last_accepted_block_slot.into(),
            last_confirmed_block_slot: proto.last_confirmed_block_slot.into(),
            latest_commitment: maybe_missing!(proto.latest_commitment).try_convert()?,
            latest_finalized_commitment_id: maybe_missing!(proto.latest_finalized_commitment_id).try_convert()?,
            pruning_epoch: proto.pruning_epoch.into(),
            is_bootstrapped: proto.is_bootstrapped,
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
            commitment: maybe_missing!(proto.commitment).try_into()?,
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
            block: maybe_missing!(proto.block).try_into()?,
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

impl TryConvertFrom<proto::BlockWithMetadata> for BlockWithMetadata {
    type Error = InxError;

    fn try_convert_from(proto: proto::BlockWithMetadata) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self {
            metadata: maybe_missing!(proto.metadata).try_convert()?,
            block: maybe_missing!(proto.block).try_into()?,
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
