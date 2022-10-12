// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod client;
mod request;
mod ledger;
mod error;
mod node;
mod milestone;
mod raw;
mod protocol;

use crate::types::stardust::block::output::OutputId;

pub use self::{request::MilestoneRangeRequest, ledger::LedgerUpdateMessage, error::InxError, client::Inx, protocol::RawProtocolParametersMessage, node::NodeStatusMessage};

use super::types::stardust::block::{
    payload::{MilestoneId, TransactionId},
    BlockId,
};

/// Tries to access the field of a protobug messages and returns an appropriate error if the field is not present.
#[macro_export]
macro_rules! maybe_missing {
    ($object:ident.$field:ident) => {
        $object.$field.ok_or(crate::inx::InxError::MissingField(stringify!($field)))?
    };
}

/// Implements `TryFrom` for the different ids that are sent via INX.
#[macro_export]
macro_rules! impl_try_from_id {
    ($inx_id:ty, $own_id:ty) => {
        impl TryFrom<$inx_id> for $own_id {
            type Error = InxError;

            fn try_from(value: $inx_id) -> Result<Self, Self::Error> {
                let data = <[u8; <$own_id>::LENGTH]>::try_from(value.id).map_err(|e| InxError::InvalidByteLength {
                    actual: e.len(),
                    expected: <$own_id>::LENGTH,
                })?;
                Ok(Self(data))
            }
        }
    };
}

impl_try_from_id!(inx::proto::BlockId, BlockId);
impl_try_from_id!(inx::proto::TransactionId, TransactionId);
impl_try_from_id!(inx::proto::MilestoneId, MilestoneId);

impl TryFrom<inx::proto::OutputId> for OutputId {
    type Error = crate::inx::InxError;

    fn try_from(value: inx::proto::OutputId) -> Result<Self, Self::Error> {
        let (transaction_id, index) = value.id.split_at(TransactionId::LENGTH);

        Ok(Self {
            // Unwrap is fine because size is already known and valid.
            transaction_id: TransactionId(<[u8; TransactionId::LENGTH]>::try_from(transaction_id).map_err(|_| {
                InxError::InvalidByteLength {
                    actual: transaction_id.len(),
                    expected: TransactionId::LENGTH,
                }
            })?),
            // Unwrap is fine because size is already known and valid.
            index: u16::from_le_bytes(index.try_into().unwrap()),
        })
    }
}

// TODO: Should we write test cases for all the id types here?
