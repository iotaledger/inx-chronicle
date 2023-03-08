// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Display, str::FromStr};

use async_trait::async_trait;
use axum::extract::{FromRef, FromRequestParts, Query};
use chronicle::{
    db::mongodb::collections::SortOrder,
    model::{
        tangle::{MilestoneIndex, MilestoneTimestamp},
        utxo::OutputId,
    },
};
use serde::Deserialize;

use crate::api::{config::ApiConfigData, error::RequestError, ApiError, DEFAULT_PAGE_SIZE};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerUpdatesByAddressPagination {
    pub page_size: usize,
    pub sort: SortOrder,
    pub cursor: Option<(MilestoneIndex, Option<(OutputId, bool)>)>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct LedgerUpdatesByAddressPaginationQuery {
    pub page_size: Option<usize>,
    pub sort: Option<String>,
    pub start_milestone_index: Option<MilestoneIndex>,
    pub cursor: Option<String>,
}

#[derive(Clone)]
pub struct LedgerUpdatesByAddressCursor {
    pub milestone_index: MilestoneIndex,
    pub output_id: OutputId,
    pub is_spent: bool,
    pub page_size: usize,
}

impl FromStr for LedgerUpdatesByAddressCursor {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split('.').collect();
        Ok(match parts[..] {
            [ms, o, sp, ps] => LedgerUpdatesByAddressCursor {
                milestone_index: ms.parse().map_err(RequestError::from)?,
                output_id: o.parse().map_err(RequestError::from)?,
                is_spent: sp.parse().map_err(RequestError::from)?,
                page_size: ps.parse().map_err(RequestError::from)?,
            },
            _ => return Err(ApiError::from(RequestError::BadPagingState)),
        })
    }
}

impl Display for LedgerUpdatesByAddressCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.milestone_index,
            self.output_id.to_hex(),
            self.is_spent,
            self.page_size
        )
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for LedgerUpdatesByAddressPagination
where
    ApiConfigData: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut axum::http::request::Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<LedgerUpdatesByAddressPaginationQuery>::from_request_parts(parts, state)
            .await
            .map_err(RequestError::from)?;
        let config = ApiConfigData::from_ref(state);

        let sort = query
            .sort
            .as_deref()
            .map_or(Ok(Default::default()), str::parse)
            .map_err(RequestError::SortOrder)?;

        let (page_size, cursor) = if let Some(cursor) = query.cursor {
            let cursor: LedgerUpdatesByAddressCursor = cursor.parse()?;
            (
                cursor.page_size,
                Some((cursor.milestone_index, Some((cursor.output_id, cursor.is_spent)))),
            )
        } else {
            (
                query.page_size.unwrap_or(DEFAULT_PAGE_SIZE),
                query.start_milestone_index.map(|i| (i, None)),
            )
        };

        Ok(LedgerUpdatesByAddressPagination {
            page_size: page_size.min(config.max_page_size),
            cursor,
            sort,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerUpdatesByMilestonePagination {
    pub page_size: usize,
    pub cursor: Option<(OutputId, bool)>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct LedgerUpdatesByMilestonePaginationQuery {
    pub page_size: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Clone)]
pub struct LedgerUpdatesByMilestoneCursor {
    pub output_id: OutputId,
    pub is_spent: bool,
    pub page_size: usize,
}

impl FromStr for LedgerUpdatesByMilestoneCursor {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split('.').collect();
        Ok(match parts[..] {
            [o, sp, ps] => LedgerUpdatesByMilestoneCursor {
                output_id: o.parse().map_err(RequestError::from)?,
                is_spent: sp.parse().map_err(RequestError::from)?,
                page_size: ps.parse().map_err(RequestError::from)?,
            },
            _ => return Err(ApiError::from(RequestError::BadPagingState)),
        })
    }
}

impl Display for LedgerUpdatesByMilestoneCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.output_id.to_hex(), self.is_spent, self.page_size)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for LedgerUpdatesByMilestonePagination
where
    ApiConfigData: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut axum::http::request::Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<LedgerUpdatesByMilestonePaginationQuery>::from_request_parts(parts, state)
            .await
            .map_err(RequestError::from)?;
        let config = ApiConfigData::from_ref(state);

        let (page_size, cursor) = if let Some(cursor) = query.cursor {
            let cursor: LedgerUpdatesByMilestoneCursor = cursor.parse()?;
            (cursor.page_size, Some((cursor.output_id, cursor.is_spent)))
        } else {
            (query.page_size.unwrap_or(DEFAULT_PAGE_SIZE), None)
        };

        Ok(LedgerUpdatesByMilestonePagination {
            page_size: page_size.min(config.max_page_size),
            cursor,
        })
    }
}

pub struct MilestonesPagination {
    pub start_timestamp: Option<MilestoneTimestamp>,
    pub end_timestamp: Option<MilestoneTimestamp>,
    pub sort: SortOrder,
    pub page_size: usize,
    pub cursor: Option<MilestoneIndex>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct MilestonesPaginationQuery {
    pub start_timestamp: Option<u32>,
    pub end_timestamp: Option<u32>,
    pub sort: Option<String>,
    pub page_size: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Clone)]
pub struct MilestonesCursor {
    pub milestone_index: MilestoneIndex,
    pub page_size: usize,
}

impl FromStr for MilestonesCursor {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split('.').collect();
        Ok(match parts[..] {
            [m, ps] => MilestonesCursor {
                milestone_index: m.parse().map_err(RequestError::from)?,
                page_size: ps.parse().map_err(RequestError::from)?,
            },
            _ => return Err(ApiError::from(RequestError::BadPagingState)),
        })
    }
}

impl Display for MilestonesCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.milestone_index, self.page_size)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for MilestonesPagination
where
    ApiConfigData: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut axum::http::request::Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<MilestonesPaginationQuery>::from_request_parts(parts, state)
            .await
            .map_err(RequestError::from)?;
        let config = ApiConfigData::from_ref(state);

        if matches!((query.start_timestamp, query.end_timestamp), (Some(start), Some(end)) if end < start) {
            return Err(ApiError::from(RequestError::BadTimeRange));
        }

        let sort = query
            .sort
            .as_deref()
            .map_or(Ok(Default::default()), str::parse)
            .map_err(RequestError::SortOrder)?;

        let (page_size, cursor) = if let Some(cursor) = query.cursor {
            let cursor: MilestonesCursor = cursor.parse()?;
            (cursor.page_size, Some(cursor.milestone_index))
        } else {
            (query.page_size.unwrap_or(DEFAULT_PAGE_SIZE), None)
        };

        Ok(MilestonesPagination {
            start_timestamp: query.start_timestamp.map(Into::into),
            end_timestamp: query.end_timestamp.map(Into::into),
            sort,
            page_size: page_size.min(config.max_page_size),
            cursor,
        })
    }
}

const DEFAULT_TOP_RICHLIST: usize = 100;

#[derive(Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RichestAddressesQuery {
    pub top: usize,
    pub ledger_index: Option<MilestoneIndex>,
}

impl Default for RichestAddressesQuery {
    fn default() -> Self {
        Self {
            top: DEFAULT_TOP_RICHLIST,
            ledger_index: None,
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for RichestAddressesQuery
where
    ApiConfigData: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut axum::http::request::Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(mut query) = Query::<RichestAddressesQuery>::from_request_parts(parts, state)
            .await
            .map_err(RequestError::from)?;
        let config = ApiConfigData::from_ref(state);
        query.top = query.top.min(config.max_page_size);
        Ok(query)
    }
}

#[derive(Copy, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct LedgerIndex {
    pub ledger_index: Option<MilestoneIndex>,
}

#[async_trait]
impl<S> FromRequestParts<S> for LedgerIndex
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut axum::http::request::Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<LedgerIndex>::from_request_parts(parts, state)
            .await
            .map_err(RequestError::from)?;
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
impl<S> FromRequestParts<S> for MilestoneRange
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut axum::http::request::Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(MilestoneRange { start_index, end_index }) =
            Query::<MilestoneRange>::from_request_parts(parts, state)
                .await
                .map_err(RequestError::from)?;
        if matches!((start_index, end_index), (Some(start), Some(end)) if end < start) {
            return Err(ApiError::from(RequestError::BadTimeRange));
        }
        Ok(MilestoneRange { start_index, end_index })
    }
}

pub struct BlocksByMilestoneIndexPagination {
    pub sort: SortOrder,
    pub page_size: usize,
    pub cursor: Option<u32>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct BlocksByMilestoneIndexPaginationQuery {
    pub sort: Option<String>,
    pub page_size: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Clone)]
pub struct BlocksByMilestoneCursor {
    pub white_flag_index: u32,
    pub page_size: usize,
}

impl FromStr for BlocksByMilestoneCursor {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split('.').collect();
        Ok(match parts[..] {
            [wfi, ps] => BlocksByMilestoneCursor {
                white_flag_index: wfi.parse().map_err(RequestError::from)?,
                page_size: ps.parse().map_err(RequestError::from)?,
            },
            _ => return Err(ApiError::from(RequestError::BadPagingState)),
        })
    }
}

impl Display for BlocksByMilestoneCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.white_flag_index, self.page_size)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for BlocksByMilestoneIndexPagination
where
    ApiConfigData: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut axum::http::request::Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<BlocksByMilestoneIndexPaginationQuery>::from_request_parts(parts, state)
            .await
            .map_err(RequestError::from)?;
        let config = ApiConfigData::from_ref(state);

        let sort = query
            .sort
            .as_deref()
            .map_or(Ok(Default::default()), str::parse)
            .map_err(RequestError::SortOrder)?;

        let (page_size, cursor) = if let Some(cursor) = query.cursor {
            let cursor: BlocksByMilestoneCursor = cursor.parse()?;
            (cursor.page_size, Some(cursor.white_flag_index))
        } else {
            (query.page_size.unwrap_or(DEFAULT_PAGE_SIZE), None)
        };

        Ok(BlocksByMilestoneIndexPagination {
            sort,
            page_size: page_size.min(config.max_page_size),
            cursor,
        })
    }
}

pub struct BlocksByMilestoneIdPagination {
    pub sort: SortOrder,
    pub page_size: usize,
    pub cursor: Option<u32>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct BlocksByMilestoneIdPaginationQuery {
    pub sort: Option<String>,
    pub page_size: Option<usize>,
    pub cursor: Option<String>,
}

#[async_trait]
impl<S> FromRequestParts<S> for BlocksByMilestoneIdPagination
where
    ApiConfigData: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut axum::http::request::Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<BlocksByMilestoneIdPaginationQuery>::from_request_parts(parts, state)
            .await
            .map_err(RequestError::from)?;
        let config = ApiConfigData::from_ref(state);

        let sort = query
            .sort
            .as_deref()
            .map_or(Ok(Default::default()), str::parse)
            .map_err(RequestError::SortOrder)?;

        let (page_size, cursor) = if let Some(cursor) = query.cursor {
            let cursor: BlocksByMilestoneCursor = cursor.parse()?;
            (cursor.page_size, Some(cursor.white_flag_index))
        } else {
            (query.page_size.unwrap_or(DEFAULT_PAGE_SIZE), None)
        };

        Ok(BlocksByMilestoneIdPagination {
            sort,
            page_size: page_size.min(config.max_page_size),
            cursor,
        })
    }
}

#[cfg(test)]
mod test {
    use axum::{extract::RequestParts, http::Request};

    use super::*;
    use crate::api::ApiConfig;

    #[test]
    fn ledger_updates_by_address_cursor_from_to_str() {
        let milestone_index = 164338324u32;
        let output_id_str = "0xfa0de75d225cca2799395e5fc340702fc7eac821d2bdd79911126f131ae097a20100";
        let is_spent_str = "false";
        let page_size_str = "1337";

        let cursor = format!("{milestone_index}.{output_id_str}.{is_spent_str}.{page_size_str}",);
        let parsed: LedgerUpdatesByAddressCursor = cursor.parse().unwrap();
        assert_eq!(parsed.to_string(), cursor);
    }

    #[test]
    fn ledger_updates_by_milestone_cursor_from_to_str() {
        let output_id_str = "0xfa0de75d225cca2799395e5fc340702fc7eac821d2bdd79911126f131ae097a20100";
        let is_spent_str = "false";
        let page_size_str = "1337";

        let cursor = format!("{output_id_str}.{is_spent_str}.{page_size_str}",);
        let parsed: LedgerUpdatesByMilestoneCursor = cursor.parse().unwrap();
        assert_eq!(parsed.to_string(), cursor);
    }

    #[tokio::test]
    async fn page_size_clamped() {
        let mut req = RequestParts::new(
            Request::builder()
                .method("GET")
                .uri("/ledger/updates/by-address/0x00?pageSize=9999999")
                .extension(ApiConfigData::try_from(ApiConfig::default()).unwrap())
                .body(())
                .unwrap(),
        );
        assert_eq!(
            LedgerUpdatesByAddressPagination::from_request(&mut req).await.unwrap(),
            LedgerUpdatesByAddressPagination {
                page_size: 1000,
                sort: Default::default(),
                cursor: Default::default()
            }
        );

        let mut req = RequestParts::new(
            Request::builder()
                .method("GET")
                .uri("/ledger/updates/by-milestone/0?pageSize=9999999")
                .extension(ApiConfigData::try_from(ApiConfig::default()).unwrap())
                .body(())
                .unwrap(),
        );
        assert_eq!(
            LedgerUpdatesByMilestonePagination::from_request(&mut req)
                .await
                .unwrap(),
            LedgerUpdatesByMilestonePagination {
                page_size: 1000,
                cursor: Default::default()
            }
        );
    }
}
