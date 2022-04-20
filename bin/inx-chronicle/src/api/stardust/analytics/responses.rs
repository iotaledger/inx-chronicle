// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::api::{impl_success_response, responses::SuccessBody};

/// Response of `GET /api/v2/analytics/addresses[?start_timestamp=<u32>&end_timestamp=<u32>]`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddressAnalyticsResponse {
    #[serde(rename = "totalAddresses")]
    pub total_addresses: u64,
    #[serde(rename = "receivingAddresses")]
    pub recv_addresses: u64,
    #[serde(rename = "sendingAddresses")]
    pub send_addresses: u64,
}

impl_success_response!(AddressAnalyticsResponse);
