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

#[derive(Clone, Debug)]
pub enum Sort {
    Ascending,
    Descending,
}

impl Default for Sort {
    fn default() -> Self {
        Self::Ascending
    }
}

impl FromStr for Sort {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "asc" | "oldest" => Ok(Self::Ascending),
            "desc" | "newest" => Ok(Self::Descending),
            _ => Err(ParseError::BadSortDescriptor),
        }
    }
}

impl From<Sort> for SortOrder {
    fn from(sort: Sort) -> Self {
        match sort {
            Sort::Ascending => Self::Oldest,
            Sort::Descending => Self::Newest,
        }
    }
}

#[derive(Clone)]
pub struct HistoryByAddressCursor {
    pub milestone_index: MilestoneIndex,
    pub output_id: OutputId,
    pub is_spent: bool,
    pub page_size: usize,
}

impl FromStr for HistoryByAddressCursor {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split('.').collect();
        Ok(match parts[..] {
            [ms, o, sp, ps] => HistoryByAddressCursor {
                milestone_index: ms.parse().map_err(ApiError::bad_parse)?,
                output_id: o.parse().map_err(ApiError::bad_parse)?,
                is_spent: sp.parse().map_err(ApiError::bad_parse)?,
                page_size: ps.parse().map_err(ApiError::bad_parse)?,
            },
            _ => return Err(ApiError::bad_parse(ParseError::BadPagingState)),
        })
    }
}

impl Display for HistoryByAddressCursor {
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

#[derive(Clone, Default)]
pub struct HistoryByAddressPagination {
    pub page_size: usize,
    pub sort: Sort,
    pub cursor: Option<(MilestoneIndex, Option<(OutputId, bool)>)>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct HistoryByAddressPaginationQuery {
    pub page_size: Option<usize>,
    pub sort: Option<String>,
    pub start_milestone_index: Option<MilestoneIndex>,
    pub cursor: Option<String>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for HistoryByAddressPagination {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<HistoryByAddressPaginationQuery>::from_request(req)
            .await
            .map_err(ApiError::QueryError)?;

        let mut pagination = if let Some(cursor) = query.cursor {
            let cursor: HistoryByAddressCursor = cursor.parse()?;
            HistoryByAddressPagination {
                page_size: cursor.page_size,
                cursor: Some((cursor.milestone_index, Some((cursor.output_id, cursor.is_spent)))),
                ..Default::default()
            }
        } else {
            HistoryByAddressPagination {
                page_size: query.page_size.unwrap_or(DEFAULT_PAGE_SIZE),
                cursor: query.start_milestone_index.map(|i| (i, None)),
                ..Default::default()
            }
        };

        if let Some(sort) = query.sort {
            pagination.sort = sort.parse().map_err(ApiError::bad_parse)?;
        }

        Ok(pagination)
    }
}

#[derive(Clone)]
pub struct HistoryByMilestonePagination {
    pub page_size: usize,
    pub sort: Sort,
    pub start_output_id: Option<OutputId>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct HistoryByMilestonePaginationQuery {
    pub page_size: Option<usize>,
    pub sort: Option<String>,
    pub cursor: Option<String>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for HistoryByMilestonePagination {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(HistoryByMilestonePaginationQuery {
            mut page_size,
            sort,
            cursor,
        }) = Query::<HistoryByMilestonePaginationQuery>::from_request(req)
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
            sort: if let Some(sort) = sort {
                sort.parse().map_err(ApiError::bad_parse)?
            } else {
                Sort::default()
            },
            start_output_id,
        })
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

        let cursor = format!(
            "{:0>10}.{output_id_str}.{is_spent_str}.{page_size_str}",
            milestone_index
        );
        let parsed: HistoryByAddressCursor = cursor.parse().unwrap();
        assert_eq!(parsed.to_string(), cursor);
    }
}
