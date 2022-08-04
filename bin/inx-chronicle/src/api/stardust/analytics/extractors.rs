// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use axum::extract::{FromRequest, Query};
use chronicle::types::tangle::MilestoneIndex;
use serde::Deserialize;

use crate::api::ApiError;

const MAX_TOP_RICHLIST: usize = 1000;
const DEFAULT_TOP_RICHLIST: usize = 100;

#[derive(Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RichlistQuery {
    pub top: usize,
    pub ledger_index: Option<MilestoneIndex>,
}

impl Default for RichlistQuery {
    fn default() -> Self {
        Self {
            top: DEFAULT_TOP_RICHLIST,
            ledger_index: None,
        }
    }
}

#[async_trait]
impl<B: Send> FromRequest<B> for RichlistQuery {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(mut query) = Query::<RichlistQuery>::from_request(req)
            .await
            .map_err(ApiError::QueryError)?;
        query.top = query.top.min(MAX_TOP_RICHLIST);
        Ok(query)
    }
}

#[derive(Copy, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct LedgerIndex {
    pub ledger_index: Option<MilestoneIndex>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for LedgerIndex {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<LedgerIndex>::from_request(req)
            .await
            .map_err(ApiError::QueryError)?;
        Ok(query)
    }
}

#[derive(Copy, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct MilestoneRange {
    pub start_index: Option<MilestoneIndex>,
    pub end_index: Option<MilestoneIndex>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for MilestoneRange {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(MilestoneRange { start_index, end_index }) = Query::<MilestoneRange>::from_request(req)
            .await
            .map_err(ApiError::QueryError)?;
        if matches!((start_index, end_index), (Some(start), Some(end)) if end < start) {
            return Err(ApiError::BadTimeRange);
        }
        Ok(MilestoneRange { start_index, end_index })
    }
}
