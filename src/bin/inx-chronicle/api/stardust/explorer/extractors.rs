// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Display, str::FromStr};

use async_trait::async_trait;
use axum::{
    extract::{FromRequest, Query},
    Extension,
};
use chronicle::{
    db::collections::SortOrder,
    types::{stardust::block::OutputId, tangle::MilestoneIndex},
};
use serde::Deserialize;

use crate::api::{config::ApiData, error::ParseError, ApiError, DEFAULT_PAGE_SIZE};

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
                milestone_index: ms.parse().map_err(ApiError::bad_parse)?,
                output_id: o.parse().map_err(ApiError::bad_parse)?,
                is_spent: sp.parse().map_err(ApiError::bad_parse)?,
                page_size: ps.parse().map_err(ApiError::bad_parse)?,
            },
            _ => return Err(ApiError::bad_parse(ParseError::BadPagingState)),
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
impl<B: Send> FromRequest<B> for LedgerUpdatesByAddressPagination {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<LedgerUpdatesByAddressPaginationQuery>::from_request(req)
            .await
            .map_err(ApiError::QueryError)?;
        let Extension(config) = Extension::<ApiData>::from_request(req).await?;

        let sort = query
            .sort
            .as_deref()
            .map_or(Ok(Default::default()), str::parse)
            .map_err(ParseError::SortOrder)?;

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
                output_id: o.parse().map_err(ApiError::bad_parse)?,
                is_spent: sp.parse().map_err(ApiError::bad_parse)?,
                page_size: ps.parse().map_err(ApiError::bad_parse)?,
            },
            _ => return Err(ApiError::bad_parse(ParseError::BadPagingState)),
        })
    }
}

impl Display for LedgerUpdatesByMilestoneCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.output_id.to_hex(), self.is_spent, self.page_size)
    }
}

#[async_trait]
impl<B: Send> FromRequest<B> for LedgerUpdatesByMilestonePagination {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<LedgerUpdatesByMilestonePaginationQuery>::from_request(req)
            .await
            .map_err(ApiError::QueryError)?;
        let Extension(config) = Extension::<ApiData>::from_request(req).await?;

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
                .extension(ApiData::try_from(ApiConfig::default()).unwrap())
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
                .extension(ApiData::try_from(ApiConfig::default()).unwrap())
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
