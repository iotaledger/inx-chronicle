// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use axum::extract::{FromRequest, Query};
use chronicle::types::stardust::block::OutputId;
use serde::Deserialize;

use crate::api::{error::ParseError, ApiError};

const DEFAULT_PAGE_SIZE: usize = 100;

#[derive(Clone)]
pub struct HistoryByAddressPagination {
    pub page_size: usize,
    pub start_milestone_index: Option<u32>,
    pub start_output_id: Option<OutputId>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default)]
pub struct HistoryByAddressPaginationQuery {
    #[serde(rename = "pageSize")]
    pub page_size: Option<usize>,
    #[serde(rename = "startMilestoneIndex")]
    pub start_milestone_index: Option<u32>,
    pub cursor: Option<String>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for HistoryByAddressPagination {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(HistoryByAddressPaginationQuery {
            mut page_size,
            mut start_milestone_index,
            cursor,
        }) = Query::<HistoryByAddressPaginationQuery>::from_request(req)
            .await
            .map_err(ApiError::QueryError)?;
        let mut start_output_id = None;
        if let Some(cursor) = cursor {
            let parts = cursor.split('.').collect::<Vec<_>>();
            if parts.len() != 3 {
                return Err(ApiError::bad_parse(ParseError::BadPagingState));
            } else {
                start_milestone_index.replace(parts[0].parse().map_err(ApiError::bad_parse)?);
                start_output_id.replace(parts[1].parse().map_err(ApiError::bad_parse)?);
                page_size.replace(parts[2].parse().map_err(ApiError::bad_parse)?);
            }
        }
        Ok(HistoryByAddressPagination {
            page_size: page_size.unwrap_or(DEFAULT_PAGE_SIZE),
            start_milestone_index,
            start_output_id,
        })
    }
}

#[derive(Clone)]
pub struct HistoryByMilestonePagination {
    pub page_size: usize,
    pub start_output_id: Option<OutputId>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default)]
pub struct HistoryByMilestonePaginationQuery {
    #[serde(rename = "pageSize")]
    pub page_size: Option<usize>,
    pub cursor: Option<String>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for HistoryByMilestonePagination {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(HistoryByMilestonePaginationQuery { mut page_size, cursor }) =
            Query::<HistoryByMilestonePaginationQuery>::from_request(req)
                .await
                .map_err(ApiError::QueryError)?;
        let mut start_output_id = None;
        if let Some(cursor) = cursor {
            let parts = cursor.split('.').collect::<Vec<_>>();
            if parts.len() != 2 {
                return Err(ApiError::bad_parse(ParseError::BadPagingState));
            } else {
                start_output_id.replace(parts[0].parse().map_err(ApiError::bad_parse)?);
                page_size.replace(parts[1].parse().map_err(ApiError::bad_parse)?);
            }
        }
        Ok(HistoryByMilestonePagination {
            page_size: page_size.unwrap_or(DEFAULT_PAGE_SIZE),
            start_output_id,
        })
    }
}
