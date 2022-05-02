// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::dto;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TreasuryTransactionPayload {
    input: dto::Input,
    output: dto::Output,
}
