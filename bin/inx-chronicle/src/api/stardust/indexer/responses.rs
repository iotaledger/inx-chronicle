// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use super::extractors::{MessagesQuery, OutputsQuery};
use crate::api::{impl_success_response, responses::Expansion, SuccessBody};

/// Response of `GET /api/v2/messages`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessagesForQueryResponse {
    pub query: MessagesQuery,
    #[serde(rename = "maxResults")]
    pub max_results: usize,
    pub count: usize,
    #[serde(rename = "messageIds")]
    pub message_ids: Vec<Expansion>,
}

impl_success_response!(MessagesForQueryResponse);

/// Response of `GET /api/v2/addresses/<address>/outputs`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputsForQueryResponse {
    pub query: OutputsQuery,
    #[serde(rename = "maxResults")]
    pub max_results: usize,
    pub count: usize,
    #[serde(rename = "outputIds")]
    pub output_ids: Vec<Expansion>,
}

impl_success_response!(OutputsForQueryResponse);
