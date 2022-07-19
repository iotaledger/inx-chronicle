// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Display, str::FromStr};

use async_trait::async_trait;
use axum::extract::{FromRequest, Query};
use chronicle::{
    db::collections::SortOrder,
    types::{stardust::block::OutputId, tangle::MilestoneIndex},
};
use serde::Deserialize;

use crate::api::{error::ParseError, ApiError};

const DEFAULT_PAGE_SIZE: usize = 100;
const DEFAULT_SORT_ORDER: SortOrder = SortOrder::Newest;

#[derive(Clone)]
pub struct LedgerUpdatesByAddressPagination {
    pub page_size: usize,
    pub sort: SortOrder,
    pub cursor: Option<(MilestoneIndex, Option<(OutputId, bool)>)>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
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

fn sort_order_from_str(s: String) -> Result<SortOrder, ApiError> {
    match s.as_ref() {
        "asc" | "oldest" => Ok(SortOrder::Oldest),
        "desc" | "newest" => Ok(SortOrder::Newest),
        _ => Err(ParseError::BadSortDescriptor).map_err(ApiError::bad_parse),
    }
}

#[async_trait]
impl<B: Send> FromRequest<B> for LedgerUpdatesByAddressPagination {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<LedgerUpdatesByAddressPaginationQuery>::from_request(req)
            .await
            .map_err(ApiError::QueryError)?;

        let sort = query.sort.map_or(Ok(DEFAULT_SORT_ORDER), sort_order_from_str)?;

        let pagination = if let Some(cursor) = query.cursor {
            let cursor: LedgerUpdatesByAddressCursor = cursor.parse()?;
            LedgerUpdatesByAddressPagination {
                page_size: cursor.page_size,
                cursor: Some((cursor.milestone_index, Some((cursor.output_id, cursor.is_spent)))),
                sort,
            }
        } else {
            LedgerUpdatesByAddressPagination {
                page_size: query.page_size.unwrap_or(DEFAULT_PAGE_SIZE),
                cursor: query.start_milestone_index.map(|i| (i, None)),
                sort,
            }
        };

        Ok(pagination)
    }
}

#[derive(Clone)]
pub struct LedgerUpdatesByMilestonePagination {
    pub page_size: usize,
    pub cursor: Option<(OutputId, bool)>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
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

        let pagination = if let Some(cursor) = query.cursor {
            let cursor: LedgerUpdatesByMilestoneCursor = cursor.parse()?;
            LedgerUpdatesByMilestonePagination {
                page_size: cursor.page_size,
                cursor: Some((cursor.output_id, cursor.is_spent)),
            }
        } else {
            LedgerUpdatesByMilestonePagination {
                page_size: query.page_size.unwrap_or(DEFAULT_PAGE_SIZE),
                cursor: None,
            }
        };

        Ok(pagination)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn address_cursor_from_to_str() {
        let milestone_index = 164338324u32;
        let output_id_str = "0xfa0de75d225cca2799395e5fc340702fc7eac821d2bdd79911126f131ae097a20100";
        let is_spent_str = "false";
        let page_size_str = "1337";

        let cursor = format!("{milestone_index}.{output_id_str}.{is_spent_str}.{page_size_str}",);
        let parsed: LedgerUpdatesByAddressCursor = cursor.parse().unwrap();
        assert_eq!(parsed.to_string(), cursor);
    }
}
