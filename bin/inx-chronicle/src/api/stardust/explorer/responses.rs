// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::api::{impl_success_response, responses::Transfer, SuccessBody};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionHistoryResponse {
    pub address: String,
    pub transactions: Vec<Transfer>,
}

impl_success_response!(TransactionHistoryResponse);
