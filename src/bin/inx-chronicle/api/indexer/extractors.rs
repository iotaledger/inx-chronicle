// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Display, str::FromStr};

use async_trait::async_trait;
use axum::{
    extract::{FromRequest, Query},
    Extension,
};
use chronicle::{
    db::mongodb::collections::{AliasOutputsQuery, BasicOutputsQuery, FoundryOutputsQuery, NftOutputsQuery, SortOrder},
    model::{tangle::MilestoneIndex, utxo::OutputId, Address},
};
use mongodb::bson;
use primitive_types::U256;
use serde::Deserialize;

use crate::api::{config::ApiConfigData, error::RequestError, ApiError, DEFAULT_PAGE_SIZE};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexedOutputsPagination<Q>
where
    bson::Document: From<Q>,
{
    pub query: Q,
    pub page_size: usize,
    pub cursor: Option<(MilestoneIndex, OutputId)>,
    pub sort: SortOrder,
    pub include_spent: bool,
}

#[derive(Clone)]
pub struct IndexedOutputsCursor {
    pub milestone_index: MilestoneIndex,
    pub output_id: OutputId,
    pub page_size: usize,
}

impl FromStr for IndexedOutputsCursor {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split('.').collect();
        Ok(match parts[..] {
            [ms, o, ps] => IndexedOutputsCursor {
                milestone_index: ms.parse().map_err(RequestError::from)?,
                output_id: o.parse().map_err(RequestError::from)?,
                page_size: ps.parse().map_err(RequestError::from)?,
            },
            _ => return Err(ApiError::from(RequestError::BadPagingState)),
        })
    }
}

impl Display for IndexedOutputsCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}",
            self.milestone_index,
            self.output_id.to_hex(),
            self.page_size
        )
    }
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct BasicOutputsPaginationQuery {
    pub address: Option<String>,
    pub has_native_tokens: Option<bool>,
    pub min_native_token_count: Option<String>,
    pub max_native_token_count: Option<String>,
    pub has_storage_deposit_return: Option<bool>,
    pub storage_deposit_return_address: Option<String>,
    pub has_timelock: Option<bool>,
    pub timelocked_before: Option<u32>,
    pub timelocked_after: Option<u32>,
    pub has_expiration: Option<bool>,
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
    pub include_spent: Option<bool>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for IndexedOutputsPagination<BasicOutputsQuery> {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<BasicOutputsPaginationQuery>::from_request(req)
            .await
            .map_err(RequestError::from)?;
        let Extension(config) = Extension::<ApiConfigData>::from_request(req).await?;

        let (cursor, page_size) = if let Some(cursor) = query.cursor {
            let cursor: IndexedOutputsCursor = cursor.parse()?;
            (Some((cursor.milestone_index, cursor.output_id)), cursor.page_size)
        } else {
            (None, query.page_size.unwrap_or(DEFAULT_PAGE_SIZE))
        };

        let sort = query
            .sort
            .as_deref()
            .map_or(Ok(Default::default()), str::parse)
            .map_err(RequestError::SortOrder)?;

        Ok(IndexedOutputsPagination {
            query: BasicOutputsQuery {
                address: query
                    .address
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(RequestError::from)?,
                has_native_tokens: query.has_native_tokens,
                min_native_token_count: query
                    .min_native_token_count
                    .map(|c| U256::from_dec_str(&c))
                    .transpose()
                    .map_err(RequestError::from)?,
                max_native_token_count: query
                    .max_native_token_count
                    .map(|c| U256::from_dec_str(&c))
                    .transpose()
                    .map_err(RequestError::from)?,
                has_storage_deposit_return: query.has_storage_deposit_return,
                storage_deposit_return_address: query
                    .storage_deposit_return_address
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(RequestError::from)?,
                has_timelock: query.has_timelock,
                timelocked_before: query.timelocked_before.map(Into::into),
                timelocked_after: query.timelocked_after.map(Into::into),
                has_expiration: query.has_expiration,
                expires_before: query.expires_before.map(Into::into),
                expires_after: query.expires_after.map(Into::into),
                expiration_return_address: query
                    .expiration_return_address
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(RequestError::from)?,
                sender: query
                    .sender
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(RequestError::from)?,
                tag: query.tag,
                created_before: query.created_before.map(Into::into),
                created_after: query.created_after.map(Into::into),
            },
            page_size: page_size.min(config.max_page_size),
            cursor,
            sort,
            include_spent: query.include_spent.unwrap_or_default(),
        })
    }
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
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
    pub include_spent: Option<bool>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for IndexedOutputsPagination<AliasOutputsQuery> {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<AliasOutputsPaginationQuery>::from_request(req)
            .await
            .map_err(RequestError::from)?;
        let Extension(config) = Extension::<ApiConfigData>::from_request(req).await?;

        let (cursor, page_size) = if let Some(cursor) = query.cursor {
            let cursor: IndexedOutputsCursor = cursor.parse()?;
            (Some((cursor.milestone_index, cursor.output_id)), cursor.page_size)
        } else {
            (None, query.page_size.unwrap_or(DEFAULT_PAGE_SIZE))
        };

        let sort = query
            .sort
            .as_deref()
            .map_or(Ok(Default::default()), str::parse)
            .map_err(RequestError::SortOrder)?;

        Ok(IndexedOutputsPagination {
            query: AliasOutputsQuery {
                state_controller: query
                    .state_controller
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(RequestError::from)?,
                governor: query
                    .governor
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(RequestError::from)?,
                issuer: query
                    .issuer
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(RequestError::from)?,
                sender: query
                    .sender
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(RequestError::from)?,
                has_native_tokens: query.has_native_tokens,
                min_native_token_count: query
                    .min_native_token_count
                    .map(|c| U256::from_dec_str(&c))
                    .transpose()
                    .map_err(RequestError::from)?,
                max_native_token_count: query
                    .max_native_token_count
                    .map(|c| U256::from_dec_str(&c))
                    .transpose()
                    .map_err(RequestError::from)?,
                created_before: query.created_before.map(Into::into),
                created_after: query.created_after.map(Into::into),
            },
            page_size: page_size.min(config.max_page_size),
            cursor,
            sort,
            include_spent: query.include_spent.unwrap_or_default(),
        })
    }
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
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
    pub include_spent: Option<bool>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for IndexedOutputsPagination<FoundryOutputsQuery> {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<FoundryOutputsPaginationQuery>::from_request(req)
            .await
            .map_err(RequestError::from)?;
        let Extension(config) = Extension::<ApiConfigData>::from_request(req).await?;

        let (cursor, page_size) = if let Some(cursor) = query.cursor {
            let cursor: IndexedOutputsCursor = cursor.parse()?;
            (Some((cursor.milestone_index, cursor.output_id)), cursor.page_size)
        } else {
            (None, query.page_size.unwrap_or(DEFAULT_PAGE_SIZE))
        };

        let sort = query
            .sort
            .as_deref()
            .map_or(Ok(Default::default()), str::parse)
            .map_err(RequestError::SortOrder)?;

        Ok(IndexedOutputsPagination {
            query: FoundryOutputsQuery {
                alias_address: query
                    .alias_address
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(RequestError::from)?,
                has_native_tokens: query.has_native_tokens,
                min_native_token_count: query
                    .min_native_token_count
                    .map(|c| U256::from_dec_str(&c))
                    .transpose()
                    .map_err(RequestError::from)?,
                max_native_token_count: query
                    .max_native_token_count
                    .map(|c| U256::from_dec_str(&c))
                    .transpose()
                    .map_err(RequestError::from)?,
                created_before: query.created_before.map(Into::into),
                created_after: query.created_after.map(Into::into),
            },
            page_size: page_size.min(config.max_page_size),
            cursor,
            sort,
            include_spent: query.include_spent.unwrap_or_default(),
        })
    }
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct NftOutputsPaginationQuery {
    pub address: Option<String>,
    pub issuer: Option<String>,
    pub sender: Option<String>,
    pub has_native_tokens: Option<bool>,
    pub min_native_token_count: Option<String>,
    pub max_native_token_count: Option<String>,
    pub has_storage_deposit_return: Option<bool>,
    pub storage_deposit_return_address: Option<String>,
    pub has_timelock: Option<bool>,
    pub timelocked_before: Option<u32>,
    pub timelocked_after: Option<u32>,
    pub has_expiration: Option<bool>,
    pub expires_before: Option<u32>,
    pub expires_after: Option<u32>,
    pub expiration_return_address: Option<String>,
    pub tag: Option<String>,
    pub created_before: Option<u32>,
    pub created_after: Option<u32>,
    pub page_size: Option<usize>,
    pub cursor: Option<String>,
    pub sort: Option<String>,
    pub include_spent: Option<bool>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for IndexedOutputsPagination<NftOutputsQuery> {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<NftOutputsPaginationQuery>::from_request(req)
            .await
            .map_err(RequestError::from)?;
        let Extension(config) = Extension::<ApiConfigData>::from_request(req).await?;

        let (cursor, page_size) = if let Some(cursor) = query.cursor {
            let cursor: IndexedOutputsCursor = cursor.parse()?;
            (Some((cursor.milestone_index, cursor.output_id)), cursor.page_size)
        } else {
            (None, query.page_size.unwrap_or(DEFAULT_PAGE_SIZE))
        };

        let sort = query
            .sort
            .as_deref()
            .map_or(Ok(Default::default()), str::parse)
            .map_err(RequestError::SortOrder)?;

        Ok(IndexedOutputsPagination {
            query: NftOutputsQuery {
                address: query
                    .address
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(RequestError::from)?,
                issuer: query
                    .issuer
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(RequestError::from)?,
                sender: query
                    .sender
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(RequestError::from)?,
                has_native_tokens: query.has_native_tokens,
                min_native_token_count: query
                    .min_native_token_count
                    .map(|c| U256::from_dec_str(&c))
                    .transpose()
                    .map_err(RequestError::from)?,
                max_native_token_count: query
                    .max_native_token_count
                    .map(|c| U256::from_dec_str(&c))
                    .transpose()
                    .map_err(RequestError::from)?,
                has_storage_deposit_return: query.has_storage_deposit_return,
                storage_deposit_return_address: query
                    .storage_deposit_return_address
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(RequestError::from)?,
                has_timelock: query.has_timelock,
                timelocked_before: query.timelocked_before.map(Into::into),
                timelocked_after: query.timelocked_after.map(Into::into),
                has_expiration: query.has_expiration,
                expires_before: query.expires_before.map(Into::into),
                expires_after: query.expires_after.map(Into::into),
                expiration_return_address: query
                    .expiration_return_address
                    .map(|address| Address::from_str(&address))
                    .transpose()
                    .map_err(RequestError::from)?,
                tag: query.tag,
                created_before: query.created_before.map(Into::into),
                created_after: query.created_after.map(Into::into),
            },
            page_size: page_size.min(config.max_page_size),
            cursor,
            sort,
            include_spent: query.include_spent.unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod test {
    use axum::{extract::RequestParts, http::Request};

    use super::*;
    use crate::api::ApiConfig;

    #[test]
    fn indexed_outputs_cursor_from_to_str() {
        let milestone_index = 164338324u32;
        let output_id_str = "0xfa0de75d225cca2799395e5fc340702fc7eac821d2bdd79911126f131ae097a20100";
        let page_size_str = "1337";

        let cursor = format!("{milestone_index}.{output_id_str}.{page_size_str}",);
        let parsed: IndexedOutputsCursor = cursor.parse().unwrap();
        assert_eq!(parsed.to_string(), cursor);
    }

    #[tokio::test]
    async fn page_size_clamped() {
        let mut req = RequestParts::new(
            Request::builder()
                .method("GET")
                .uri("/outputs/basic?pageSize=9999999")
                .extension(ApiConfigData::try_from(ApiConfig::default()).unwrap())
                .body(())
                .unwrap(),
        );
        assert_eq!(
            IndexedOutputsPagination::<BasicOutputsQuery>::from_request(&mut req)
                .await
                .unwrap(),
            IndexedOutputsPagination {
                page_size: 1000,
                query: Default::default(),
                cursor: Default::default(),
                sort: Default::default(),
                include_spent: Default::default()
            }
        );
    }
}
