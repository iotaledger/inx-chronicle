// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use async_trait::async_trait;
use axum::extract::{FromRequest, Query};
use chronicle::types::stardust::block::OutputId;
use regex::Regex;
use serde::Deserialize;

use crate::api::{error::ParseError, ApiError};

const HISTORY_PAGING_REGEX: &str = r"^([0-9]+)\.(0x(?:[0-9a-fA-F]{2})+)\.([0-9]+)$";

#[derive(Clone)]
pub struct HistoryPagination {
    pub page_size: usize,
    pub start_milestone_index: Option<u32>,
    pub start_output_id: Option<OutputId>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default)]
pub struct HistoryPaginationQuery {
    pub page_size: Option<usize>,
    pub start_milestone_index: Option<u32>,
    pub cursor: Option<String>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for HistoryPagination {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(HistoryPaginationQuery {
            mut page_size,
            mut start_milestone_index,
            cursor,
        }) = Query::<HistoryPaginationQuery>::from_request(req)
            .await
            .map_err(ApiError::QueryError)?;
        let mut start_output_id = None;
        if let Some(cursor) = cursor {
            // Unwrap: Infallable as long as the regex is valid
            let regex = Regex::new(HISTORY_PAGING_REGEX).unwrap();
            let captures = regex.captures(&cursor).ok_or(ParseError::BadPagingState)?;
            start_milestone_index.replace(captures.get(1).unwrap().as_str().parse().map_err(ApiError::bad_parse)?);
            start_output_id
                .replace(OutputId::from_str(captures.get(2).unwrap().as_str()).map_err(ApiError::bad_parse)?);
            page_size.replace(captures.get(3).unwrap().as_str().parse().map_err(ApiError::bad_parse)?);
        }
        Ok(HistoryPagination {
            page_size: page_size.unwrap_or(100),
            start_milestone_index,
            start_output_id,
        })
    }
}
