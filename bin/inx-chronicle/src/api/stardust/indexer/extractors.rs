// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use async_trait::async_trait;
use axum::extract::{FromRequest, Query};
use chronicle::{
    db::collections::BasicOutputsQuery,
    types::stardust::block::{Address, OutputId},
};
use primitive_types::U256;
use serde::Deserialize;

use crate::api::{error::ParseError, ApiError};

const DEFAULT_PAGE_SIZE: usize = 100;

#[derive(Clone)]
pub struct BasicOutputsPagination {
    pub query: BasicOutputsQuery,
    pub page_size: usize,
    pub cursor: Option<(u32, OutputId)>,
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct BasicOutputsPaginationQuery {
    pub address: Option<String>,
    pub has_native_tokens: Option<bool>,
    pub min_native_token_count: Option<String>,
    pub max_native_token_count: Option<String>,
    pub has_storage_return_condition: Option<bool>,
    pub storage_return_address: Option<String>,
    pub has_timelock_condition: Option<bool>,
    pub timelocked_before: Option<u32>,
    pub timelocked_after: Option<u32>,
    pub has_expiration_condition: Option<bool>,
    pub expires_before: Option<u32>,
    pub expires_after: Option<u32>,
    pub expiration_return_address: Option<String>,
    pub sender: Option<String>,
    pub tag: Option<String>,
    pub created_before: Option<u32>,
    pub created_after: Option<u32>,
    pub page_size: Option<usize>,
    pub cursor: Option<String>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for BasicOutputsPagination {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(mut query) = Query::<BasicOutputsPaginationQuery>::from_request(req)
            .await
            .map_err(ApiError::QueryError)?;
        let mut cursor = None;
        if let Some(in_cursor) = query.cursor {
            let parts = in_cursor.split('.').collect::<Vec<_>>();
            if parts.len() != 3 {
                return Err(ApiError::bad_parse(ParseError::BadPagingState));
            } else {
                cursor.replace((
                    parts[0].parse().map_err(ApiError::bad_parse)?,
                    parts[1].parse().map_err(ApiError::bad_parse)?,
                ));
                query.page_size.replace(parts[2].parse().map_err(ApiError::bad_parse)?);
            }
        }
        Ok(BasicOutputsPagination {
            query: BasicOutputsQuery {
                address: query
                    .address
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(ApiError::bad_parse)?,
                has_native_tokens: query.has_native_tokens,
                min_native_token_count: query
                    .min_native_token_count
                    .map(|c| U256::from_dec_str(&c))
                    .transpose()
                    .map_err(ApiError::bad_parse)?,
                max_native_token_count: query
                    .max_native_token_count
                    .map(|c| U256::from_dec_str(&c))
                    .transpose()
                    .map_err(ApiError::bad_parse)?,
                has_storage_return_condition: query.has_storage_return_condition,
                storage_return_address: query
                    .storage_return_address
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(ApiError::bad_parse)?,
                has_timelock_condition: query.has_timelock_condition,
                timelocked_before: query.timelocked_before,
                timelocked_after: query.timelocked_after,
                has_expiration_condition: query.has_expiration_condition,
                expires_before: query.expires_before,
                expires_after: query.expires_after,
                expiration_return_address: query
                    .expiration_return_address
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(ApiError::bad_parse)?,
                sender: query
                    .sender
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(ApiError::bad_parse)?,
                tag: query.tag,
                created_before: query.created_before,
                created_after: query.created_after,
            },
            page_size: query.page_size.unwrap_or(DEFAULT_PAGE_SIZE),
            cursor,
        })
    }
}
