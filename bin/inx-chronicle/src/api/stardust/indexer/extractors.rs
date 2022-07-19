// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use async_trait::async_trait;
use axum::extract::{FromRequest, Query};
use chronicle::{
    db::collections::{AliasOutputsQuery, BasicOutputsQuery, FoundryOutputsQuery, NftOutputsQuery, SortOrder},
    types::stardust::block::{Address, OutputId},
};
use mongodb::bson;
use primitive_types::U256;
use serde::Deserialize;

use crate::api::{
    error::ParseError,
    stardust::{sort_order_from_str, DEFAULT_PAGE_SIZE, DEFAULT_SORT_ORDER},
    ApiError,
};

#[derive(Clone)]
pub struct OutputsPagination<Q>
where
    bson::Document: From<Q>,
{
    pub query: Q,
    pub page_size: usize,
    pub cursor: Option<(u32, OutputId)>,
    pub sort: SortOrder,
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
    pub sort: Option<String>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for OutputsPagination<BasicOutputsQuery> {
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

        let sort = query.sort.map_or(Ok(DEFAULT_SORT_ORDER), sort_order_from_str)?;

        Ok(OutputsPagination {
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
                timelocked_before: query.timelocked_before.map(Into::into),
                timelocked_after: query.timelocked_after.map(Into::into),
                has_expiration_condition: query.has_expiration_condition,
                expires_before: query.expires_before.map(Into::into),
                expires_after: query.expires_after.map(Into::into),
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
                created_before: query.created_before.map(Into::into),
                created_after: query.created_after.map(Into::into),
            },
            page_size: query.page_size.unwrap_or(DEFAULT_PAGE_SIZE),
            cursor,
            sort,
        })
    }
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct AliasOutputsPaginationQuery {
    pub state_controller: Option<String>,
    pub governor: Option<String>,
    pub issuer: Option<String>,
    pub sender: Option<String>,
    pub has_native_tokens: Option<bool>,
    pub min_native_token_count: Option<String>,
    pub max_native_token_count: Option<String>,
    pub created_before: Option<u32>,
    pub created_after: Option<u32>,
    pub page_size: Option<usize>,
    pub cursor: Option<String>,
    pub sort: Option<String>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for OutputsPagination<AliasOutputsQuery> {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(mut query) = Query::<AliasOutputsPaginationQuery>::from_request(req)
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

        let sort = query.sort.map_or(Ok(DEFAULT_SORT_ORDER), sort_order_from_str)?;

        Ok(OutputsPagination {
            query: AliasOutputsQuery {
                state_controller: query
                    .state_controller
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(ApiError::bad_parse)?,
                governor: query
                    .governor
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(ApiError::bad_parse)?,
                issuer: query
                    .issuer
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(ApiError::bad_parse)?,
                sender: query
                    .sender
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
                created_before: query.created_before.map(Into::into),
                created_after: query.created_after.map(Into::into),
            },
            page_size: query.page_size.unwrap_or(DEFAULT_PAGE_SIZE),
            cursor,
            sort,
        })
    }
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct FoundryOutputsPaginationQuery {
    pub alias_address: Option<String>,
    pub has_native_tokens: Option<bool>,
    pub min_native_token_count: Option<String>,
    pub max_native_token_count: Option<String>,
    pub created_before: Option<u32>,
    pub created_after: Option<u32>,
    pub page_size: Option<usize>,
    pub cursor: Option<String>,
    pub sort: Option<String>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for OutputsPagination<FoundryOutputsQuery> {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(mut query) = Query::<FoundryOutputsPaginationQuery>::from_request(req)
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

        let sort = query.sort.map_or(Ok(DEFAULT_SORT_ORDER), sort_order_from_str)?;

        Ok(OutputsPagination {
            query: FoundryOutputsQuery {
                alias_address: query
                    .alias_address
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
                created_before: query.created_before.map(Into::into),
                created_after: query.created_after.map(Into::into),
            },
            page_size: query.page_size.unwrap_or(DEFAULT_PAGE_SIZE),
            cursor,
            sort,
        })
    }
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct NftOutputsPaginationQuery {
    pub address: Option<String>,
    pub issuer: Option<String>,
    pub sender: Option<String>,
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
    pub tag: Option<String>,
    pub created_before: Option<u32>,
    pub created_after: Option<u32>,
    pub page_size: Option<usize>,
    pub cursor: Option<String>,
    pub sort: Option<String>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for OutputsPagination<NftOutputsQuery> {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(mut query) = Query::<NftOutputsPaginationQuery>::from_request(req)
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

        let sort = query.sort.map_or(Ok(DEFAULT_SORT_ORDER), sort_order_from_str)?;

        Ok(OutputsPagination {
            query: NftOutputsQuery {
                address: query
                    .address
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(ApiError::bad_parse)?,
                issuer: query
                    .issuer
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(ApiError::bad_parse)?,
                sender: query
                    .sender
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
                timelocked_before: query.timelocked_before.map(Into::into),
                timelocked_after: query.timelocked_after.map(Into::into),
                has_expiration_condition: query.has_expiration_condition,
                expires_before: query.expires_before.map(Into::into),
                expires_after: query.expires_after.map(Into::into),
                expiration_return_address: query
                    .expiration_return_address
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(ApiError::bad_parse)?,
                tag: query.tag,
                created_before: query.created_before.map(Into::into),
                created_after: query.created_after.map(Into::into),
            },
            page_size: query.page_size.unwrap_or(DEFAULT_PAGE_SIZE),
            cursor,
            sort,
        })
    }
}
