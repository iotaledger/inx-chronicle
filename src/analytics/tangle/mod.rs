// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub trait BlockAnalytics {
    type Measurement;
    fn begin(&mut self);
    fn handle_block(&mut self, block: &Block);
    fn flush(&mut self) -> Option<Self::Measurement>;
}
