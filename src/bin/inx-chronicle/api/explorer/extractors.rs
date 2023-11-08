// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Display, str::FromStr};

use async_trait::async_trait;
use axum::{
    extract::{FromRequest, Query},
    Extension,
};
use chronicle::{self, db::mongodb::collections::SortOrder};
use iota_sdk::types::block::{output::OutputId, slot::SlotIndex, BlockId};
use serde::Deserialize;

use crate::api::{config::ApiConfigData, error::RequestError, ApiError, DEFAULT_PAGE_SIZE};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerUpdatesByAddressPagination {
    pub page_size: usize,
    pub sort: SortOrder,
    pub cursor: Option<(SlotIndex, Option<(OutputId, bool)>)>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct LedgerUpdatesByAddressPaginationQuery {
    pub page_size: Option<usize>,
    pub sort: Option<String>,
    pub start_slot: Option<SlotIndex>,
    pub cursor: Option<String>,
}

#[derive(Clone)]
pub struct LedgerUpdatesByAddressCursor {
    pub slot_index: SlotIndex,
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
                slot_index: ms.parse().map_err(RequestError::from)?,
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
            self.slot_index, self.output_id, self.is_spent, self.page_size
        )
    }
}

#[async_trait]
impl<B: Send> FromRequest<B> for LedgerUpdatesByAddressPagination {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<LedgerUpdatesByAddressPaginationQuery>::from_request(req)
            .await
            .map_err(RequestError::from)?;
        let Extension(config) = Extension::<ApiConfigData>::from_request(req).await?;

        let sort = query
            .sort
            .as_deref()
            .map_or(Ok(Default::default()), str::parse)
            .map_err(RequestError::SortOrder)?;

        let (page_size, cursor) = if let Some(cursor) = query.cursor {
            let cursor: LedgerUpdatesByAddressCursor = cursor.parse()?;
            (
                cursor.page_size,
                Some((cursor.slot_index, Some((cursor.output_id, cursor.is_spent)))),
            )
        } else {
            (
                query.page_size.unwrap_or(DEFAULT_PAGE_SIZE),
                query.start_slot.map(|i| (i, None)),
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
pub struct LedgerUpdatesBySlotPagination {
    pub page_size: usize,
    pub cursor: Option<(OutputId, bool)>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct LedgerUpdatesBySlotPaginationQuery {
    pub page_size: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Clone)]
pub struct LedgerUpdatesBySlotCursor {
    pub output_id: OutputId,
    pub is_spent: bool,
    pub page_size: usize,
}

impl FromStr for LedgerUpdatesBySlotCursor {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split('.').collect();
        Ok(match parts[..] {
            [o, sp, ps] => LedgerUpdatesBySlotCursor {
                output_id: o.parse().map_err(RequestError::from)?,
                is_spent: sp.parse().map_err(RequestError::from)?,
                page_size: ps.parse().map_err(RequestError::from)?,
            },
            _ => return Err(ApiError::from(RequestError::BadPagingState)),
        })
    }
}

impl Display for LedgerUpdatesBySlotCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.output_id, self.is_spent, self.page_size)
    }
}

#[async_trait]
impl<B: Send> FromRequest<B> for LedgerUpdatesBySlotPagination {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<LedgerUpdatesBySlotPaginationQuery>::from_request(req)
            .await
            .map_err(RequestError::from)?;
        let Extension(config) = Extension::<ApiConfigData>::from_request(req).await?;

        let (page_size, cursor) = if let Some(cursor) = query.cursor {
            let cursor: LedgerUpdatesBySlotCursor = cursor.parse()?;
            (cursor.page_size, Some((cursor.output_id, cursor.is_spent)))
        } else {
            (query.page_size.unwrap_or(DEFAULT_PAGE_SIZE), None)
        };

        Ok(LedgerUpdatesBySlotPagination {
            page_size: page_size.min(config.max_page_size),
            cursor,
        })
    }
}

pub struct SlotsPagination {
    pub start_index: Option<SlotIndex>,
    pub end_index: Option<SlotIndex>,
    pub sort: SortOrder,
    pub page_size: usize,
    pub cursor: Option<SlotIndex>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct SlotsPaginationQuery {
    pub start_index: Option<SlotIndex>,
    pub end_index: Option<SlotIndex>,
    pub sort: Option<String>,
    pub page_size: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Clone)]
pub struct SlotsCursor {
    pub slot_index: SlotIndex,
    pub page_size: usize,
}

impl FromStr for SlotsCursor {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split('.').collect();
        Ok(match parts[..] {
            [m, ps] => SlotsCursor {
                slot_index: m.parse().map_err(RequestError::from)?,
                page_size: ps.parse().map_err(RequestError::from)?,
            },
            _ => return Err(ApiError::from(RequestError::BadPagingState)),
        })
    }
}

impl Display for SlotsCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.slot_index, self.page_size)
    }
}

#[async_trait]
impl<B: Send> FromRequest<B> for SlotsPagination {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<SlotsPaginationQuery>::from_request(req)
            .await
            .map_err(RequestError::from)?;
        let Extension(config) = Extension::<ApiConfigData>::from_request(req).await?;

        if matches!((query.start_index, query.end_index), (Some(start), Some(end)) if end < start) {
            return Err(ApiError::from(RequestError::BadTimeRange));
        }

        let sort = query
            .sort
            .as_deref()
            .map_or(Ok(Default::default()), str::parse)
            .map_err(RequestError::SortOrder)?;

        let (page_size, cursor) = if let Some(cursor) = query.cursor {
            let cursor: SlotsCursor = cursor.parse()?;
            (cursor.page_size, Some(cursor.slot_index))
        } else {
            (query.page_size.unwrap_or(DEFAULT_PAGE_SIZE), None)
        };

        Ok(SlotsPagination {
            start_index: query.start_index,
            end_index: query.end_index,
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
    pub ledger_index: Option<SlotIndex>,
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
impl<B: Send> FromRequest<B> for RichestAddressesQuery {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(mut query) = Query::<RichestAddressesQuery>::from_request(req)
            .await
            .map_err(RequestError::from)?;
        let Extension(config) = Extension::<ApiConfigData>::from_request(req).await?;
        query.top = query.top.min(config.max_page_size);
        Ok(query)
    }
}

#[derive(Copy, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct LedgerIndex {
    pub ledger_index: Option<SlotIndex>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for LedgerIndex {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<LedgerIndex>::from_request(req)
            .await
            .map_err(RequestError::from)?;
        Ok(query)
    }
}

#[derive(Copy, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct SlotRange {
    pub start_index: Option<SlotIndex>,
    pub end_index: Option<SlotIndex>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for SlotRange {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(SlotRange { start_index, end_index }) = Query::<SlotRange>::from_request(req)
            .await
            .map_err(RequestError::from)?;
        if matches!((start_index, end_index), (Some(start), Some(end)) if end < start) {
            return Err(ApiError::from(RequestError::BadTimeRange));
        }
        Ok(SlotRange { start_index, end_index })
    }
}

pub struct BlocksBySlotIndexPagination {
    pub sort: SortOrder,
    pub page_size: usize,
    pub cursor: Option<BlockId>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct BlocksBySlotIndexPaginationQuery {
    pub sort: Option<String>,
    pub page_size: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Clone)]
pub struct BlocksBySlotCursor {
    pub block_id: BlockId,
    pub page_size: usize,
}

impl FromStr for BlocksBySlotCursor {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split('.').collect();
        Ok(match parts[..] {
            [wfi, ps] => BlocksBySlotCursor {
                block_id: wfi.parse().map_err(RequestError::from)?,
                page_size: ps.parse().map_err(RequestError::from)?,
            },
            _ => return Err(ApiError::from(RequestError::BadPagingState)),
        })
    }
}

impl Display for BlocksBySlotCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.block_id, self.page_size)
    }
}

#[async_trait]
impl<B: Send> FromRequest<B> for BlocksBySlotIndexPagination {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<BlocksBySlotIndexPaginationQuery>::from_request(req)
            .await
            .map_err(RequestError::from)?;
        let Extension(config) = Extension::<ApiConfigData>::from_request(req).await?;

        let sort = query
            .sort
            .as_deref()
            .map_or(Ok(Default::default()), str::parse)
            .map_err(RequestError::SortOrder)?;

        let (page_size, cursor) = if let Some(cursor) = query.cursor {
            let cursor: BlocksBySlotCursor = cursor.parse()?;
            (cursor.page_size, Some(cursor.block_id))
        } else {
            (query.page_size.unwrap_or(DEFAULT_PAGE_SIZE), None)
        };

        Ok(BlocksBySlotIndexPagination {
            sort,
            page_size: page_size.min(config.max_page_size),
            cursor,
        })
    }
}

pub struct BlocksBySlotCommitmentIdPagination {
    pub sort: SortOrder,
    pub page_size: usize,
    pub cursor: Option<BlockId>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct BlocksBySlotCommitmentIdPaginationQuery {
    pub sort: Option<String>,
    pub page_size: Option<usize>,
    pub cursor: Option<String>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for BlocksBySlotCommitmentIdPagination {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<BlocksBySlotCommitmentIdPaginationQuery>::from_request(req)
            .await
            .map_err(RequestError::from)?;
        let Extension(config) = Extension::<ApiConfigData>::from_request(req).await?;

        let sort = query
            .sort
            .as_deref()
            .map_or(Ok(Default::default()), str::parse)
            .map_err(RequestError::SortOrder)?;

        let (page_size, cursor) = if let Some(cursor) = query.cursor {
            let cursor: BlocksBySlotCursor = cursor.parse()?;
            (cursor.page_size, Some(cursor.block_id))
        } else {
            (query.page_size.unwrap_or(DEFAULT_PAGE_SIZE), None)
        };

        Ok(BlocksBySlotCommitmentIdPagination {
            sort,
            page_size: page_size.min(config.max_page_size),
            cursor,
        })
    }
}

#[cfg(test)]
mod test {
    use axum::{extract::RequestParts, http::Request};
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::api::ApiConfig;

    #[test]
    fn ledger_updates_by_address_cursor_from_to_str() {
        let slot_index = 164338324u32;
        let output_id_str = "0xfa0de75d225cca2799395e5fc340702fc7eac821d2bdd79911126f131ae097a2010000000000";
        let is_spent_str = "false";
        let page_size_str = "1337";

        let cursor = format!("{slot_index}.{output_id_str}.{is_spent_str}.{page_size_str}",);
        let parsed: LedgerUpdatesByAddressCursor = cursor.parse().unwrap();
        assert_eq!(parsed.to_string(), cursor);
    }

    #[test]
    fn ledger_updates_by_slot_cursor_from_to_str() {
        let output_id_str = "0xfa0de75d225cca2799395e5fc340702fc7eac821d2bdd79911126f131ae097a2010000000000";
        let is_spent_str = "false";
        let page_size_str = "1337";

        let cursor = format!("{output_id_str}.{is_spent_str}.{page_size_str}",);
        let parsed: LedgerUpdatesBySlotCursor = cursor.parse().unwrap();
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
                .uri("/ledger/updates/by-slot-index/0?pageSize=9999999")
                .extension(ApiConfigData::try_from(ApiConfig::default()).unwrap())
                .body(())
                .unwrap(),
        );
        assert_eq!(
            LedgerUpdatesBySlotPagination::from_request(&mut req).await.unwrap(),
            LedgerUpdatesBySlotPagination {
                page_size: 1000,
                cursor: Default::default()
            }
        );
    }
}
