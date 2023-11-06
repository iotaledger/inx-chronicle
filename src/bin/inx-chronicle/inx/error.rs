// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::slot::SlotIndex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InxWorkerError {
    #[error("expected INX address with format `http://<address>:<port>`, but found `{0}`")]
    InvalidAddress(String),
    #[error("invalid unspent output stream: found ledger index {found}, expected {expected}")]
    InvalidUnspentOutputIndex { found: SlotIndex, expected: SlotIndex },
    #[cfg(feature = "analytics")]
    #[error("missing application state")]
    MissingAppState,
    #[error("network changed from previous run. old network name: `{old}`, new network name: `{new}`")]
    NetworkChanged { old: String, new: String },
    #[error("node pruned required slots between `{start}` and `{end}`")]
    SyncGap { start: SlotIndex, end: SlotIndex },
    #[error("node confirmed slot index `{node}` is less than index in database `{db}`")]
    SyncSlotIndexMismatch { node: SlotIndex, db: SlotIndex },
}
