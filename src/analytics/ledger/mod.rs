// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! A module that provides analytics for various aspects of the tangle.

// TODO: Remove
#![allow(missing_docs)]

mod size;

pub use self::size::{LedgerSizeAnalytics, LedgerSizeStatistic};
use crate::{
    inx::LedgerUpdateMarker,
    types::ledger::{LedgerOutput, LedgerSpent},
};

pub trait LedgerUpdateAnalytics {
    type Measurement;
    fn begin(&mut self, _marker: LedgerUpdateMarker);
    fn handle_created(&mut self, output: &LedgerOutput);
    fn handle_consumed(&mut self, spent: &LedgerSpent);
    fn flush(&mut self, _marker: LedgerUpdateMarker) -> Option<Self::Measurement>;
}
