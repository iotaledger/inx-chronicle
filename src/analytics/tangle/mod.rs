// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod payloads;
mod transactions;

pub use self::{
    payloads::{BlockPayloadAnalytics, BlockPayloadStatistic},
    transactions::{TransactionAnalytics, TransactionStatistic},
};
// TODO: Remove `inx` from interface
use crate::inx::BlockWithMetadataMessage;

pub trait TangleAnalytics {
    type Measurement;
    fn begin(&mut self);
    fn handle_block(&mut self, block: &BlockWithMetadataMessage);
    fn flush(&mut self) -> Option<Self::Measurement>;
}

