// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Display, str::FromStr};

use async_trait::async_trait;
use axum::{
    extract::{FromRequest, Query},
    Extension,
};
use chronicle::{
    db::mongodb::collections::{
        AccountOutputsQuery, AnchorOutputsQuery, BasicOutputsQuery, DelegationOutputsQuery, FoundryOutputsQuery,
        NftOutputsQuery, SortOrder,
    },
    model::tag::Tag,
};
use iota_sdk::types::block::{
    address::Bech32Address,
    output::{AccountId, OutputId, TokenId},
    slot::SlotIndex,
};
use mongodb::bson;
use serde::Deserialize;

use crate::api::{config::ApiConfigData, error::RequestError, ApiError, DEFAULT_PAGE_SIZE};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexedOutputsPagination<Q>
where
    bson::Document: From<Q>,
{
    pub query: Q,
    pub page_size: usize,
    pub cursor: Option<(SlotIndex, OutputId)>,
    pub sort: SortOrder,
    pub include_spent: bool,
}

#[derive(Clone)]
pub struct IndexedOutputsCursor {
    pub slot_index: SlotIndex,
    pub output_id: OutputId,
    pub page_size: usize,
}

impl FromStr for IndexedOutputsCursor {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split('.').collect();
        Ok(match parts[..] {
            [ms, o, ps] => IndexedOutputsCursor {
                slot_index: ms.parse().map_err(RequestError::from)?,
                output_id: o.parse().map_err(RequestError::from)?,
                page_size: ps.parse().map_err(RequestError::from)?,
            },
            _ => return Err(ApiError::from(RequestError::BadPagingState)),
        })
    }
}

impl Display for IndexedOutputsCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.slot_index, self.output_id, self.page_size)
    }
}

#[derive(Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct BasicOutputsPaginationQuery {
    pub address: Option<Bech32Address>,
    pub has_native_tokens: Option<bool>,
    pub native_token: Option<TokenId>,
    pub has_storage_deposit_return: Option<bool>,
    pub storage_deposit_return_address: Option<Bech32Address>,
    pub has_timelock: Option<bool>,
    pub timelocked_before: Option<SlotIndex>,
    pub timelocked_after: Option<SlotIndex>,
    pub has_expiration: Option<bool>,
    pub expires_before: Option<SlotIndex>,
    pub expires_after: Option<SlotIndex>,
    pub expiration_return_address: Option<Bech32Address>,
    pub sender: Option<Bech32Address>,
    pub tag: Option<Tag>,
    pub created_before: Option<SlotIndex>,
    pub created_after: Option<SlotIndex>,
    pub unlockable_by_address: Option<Bech32Address>,
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
            (Some((cursor.slot_index, cursor.output_id)), cursor.page_size)
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
                address: query.address.map(Bech32Address::into_inner),
                has_native_tokens: query.has_native_tokens,
                native_token: query.native_token,
                has_storage_deposit_return: query.has_storage_deposit_return,
                storage_deposit_return_address: query.storage_deposit_return_address.map(Bech32Address::into_inner),
                has_timelock: query.has_timelock,
                timelocked_before: query.timelocked_before,
                timelocked_after: query.timelocked_after,
                has_expiration: query.has_expiration,
                expires_before: query.expires_before,
                expires_after: query.expires_after,
                expiration_return_address: query.expiration_return_address.map(Bech32Address::into_inner),
                sender: query.sender.map(Bech32Address::into_inner),
                tag: query.tag,
                created_before: query.created_before,
                created_after: query.created_after,
                unlockable_by_address: query.unlockable_by_address.map(Bech32Address::into_inner),
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
pub struct AccountOutputsPaginationQuery {
    pub address: Option<Bech32Address>,
    pub issuer: Option<Bech32Address>,
    pub sender: Option<Bech32Address>,
    pub created_before: Option<SlotIndex>,
    pub created_after: Option<SlotIndex>,
    pub page_size: Option<usize>,
    pub cursor: Option<String>,
    pub sort: Option<String>,
    pub include_spent: Option<bool>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for IndexedOutputsPagination<AccountOutputsQuery> {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<AccountOutputsPaginationQuery>::from_request(req)
            .await
            .map_err(RequestError::from)?;
        let Extension(config) = Extension::<ApiConfigData>::from_request(req).await?;

        let (cursor, page_size) = if let Some(cursor) = query.cursor {
            let cursor: IndexedOutputsCursor = cursor.parse()?;
            (Some((cursor.slot_index, cursor.output_id)), cursor.page_size)
        } else {
            (None, query.page_size.unwrap_or(DEFAULT_PAGE_SIZE))
        };

        let sort = query
            .sort
            .as_deref()
            .map_or(Ok(Default::default()), str::parse)
            .map_err(RequestError::SortOrder)?;

        Ok(IndexedOutputsPagination {
            query: AccountOutputsQuery {
                address: query.address.map(Bech32Address::into_inner),
                issuer: query.issuer.map(Bech32Address::into_inner),
                sender: query.sender.map(Bech32Address::into_inner),
                created_before: query.created_before,
                created_after: query.created_after,
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
pub struct AnchorOutputsPaginationQuery {
    pub governor: Option<Bech32Address>,
    pub state_controller: Option<Bech32Address>,
    pub issuer: Option<Bech32Address>,
    pub sender: Option<Bech32Address>,
    pub created_before: Option<SlotIndex>,
    pub created_after: Option<SlotIndex>,
    pub unlockable_by_address: Option<Bech32Address>,
    pub page_size: Option<usize>,
    pub cursor: Option<String>,
    pub sort: Option<String>,
    pub include_spent: Option<bool>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for IndexedOutputsPagination<AnchorOutputsQuery> {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<AnchorOutputsPaginationQuery>::from_request(req)
            .await
            .map_err(RequestError::from)?;
        let Extension(config) = Extension::<ApiConfigData>::from_request(req).await?;

        let (cursor, page_size) = if let Some(cursor) = query.cursor {
            let cursor: IndexedOutputsCursor = cursor.parse()?;
            (Some((cursor.slot_index, cursor.output_id)), cursor.page_size)
        } else {
            (None, query.page_size.unwrap_or(DEFAULT_PAGE_SIZE))
        };

        let sort = query
            .sort
            .as_deref()
            .map_or(Ok(Default::default()), str::parse)
            .map_err(RequestError::SortOrder)?;

        Ok(IndexedOutputsPagination {
            query: AnchorOutputsQuery {
                governor: query.governor.map(Bech32Address::into_inner),
                state_controller: query.state_controller.map(Bech32Address::into_inner),
                issuer: query.issuer.map(Bech32Address::into_inner),
                sender: query.sender.map(Bech32Address::into_inner),
                created_before: query.created_before,
                created_after: query.created_after,
                unlockable_by_address: query.unlockable_by_address.map(Bech32Address::into_inner),
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
    pub account: Option<AccountId>,
    pub has_native_tokens: Option<bool>,
    pub native_token: Option<TokenId>,
    pub created_before: Option<SlotIndex>,
    pub created_after: Option<SlotIndex>,
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
            (Some((cursor.slot_index, cursor.output_id)), cursor.page_size)
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
                account: query.account,
                has_native_tokens: query.has_native_tokens,
                native_token: query.native_token,
                created_before: query.created_before,
                created_after: query.created_after,
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
    pub address: Option<Bech32Address>,
    pub has_native_tokens: Option<bool>,
    pub native_token: Option<TokenId>,
    pub has_storage_deposit_return: Option<bool>,
    pub storage_deposit_return_address: Option<Bech32Address>,
    pub has_timelock: Option<bool>,
    pub timelocked_before: Option<SlotIndex>,
    pub timelocked_after: Option<SlotIndex>,
    pub has_expiration: Option<bool>,
    pub expires_before: Option<SlotIndex>,
    pub expires_after: Option<SlotIndex>,
    pub expiration_return_address: Option<Bech32Address>,
    pub issuer: Option<Bech32Address>,
    pub sender: Option<Bech32Address>,
    pub tag: Option<Tag>,
    pub created_before: Option<SlotIndex>,
    pub created_after: Option<SlotIndex>,
    pub unlockable_by_address: Option<Bech32Address>,
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
            (Some((cursor.slot_index, cursor.output_id)), cursor.page_size)
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
                address: query.address.map(Bech32Address::into_inner),
                issuer: query.issuer.map(Bech32Address::into_inner),
                sender: query.sender.map(Bech32Address::into_inner),
                has_native_tokens: query.has_native_tokens,
                native_token: query.native_token,
                has_storage_deposit_return: query.has_storage_deposit_return,
                storage_deposit_return_address: query.storage_deposit_return_address.map(Bech32Address::into_inner),
                has_timelock: query.has_timelock,
                timelocked_before: query.timelocked_before,
                timelocked_after: query.timelocked_after,
                has_expiration: query.has_expiration,
                expires_before: query.expires_before,
                expires_after: query.expires_after,
                expiration_return_address: query.expiration_return_address.map(Bech32Address::into_inner),
                tag: query.tag,
                created_before: query.created_before,
                created_after: query.created_after,
                unlockable_by_address: query.unlockable_by_address.map(Bech32Address::into_inner),
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
pub struct DelegationOutputsPaginationQuery {
    pub address: Option<Bech32Address>,
    pub validator: Option<AccountId>,
    pub created_before: Option<SlotIndex>,
    pub created_after: Option<SlotIndex>,
    pub page_size: Option<usize>,
    pub cursor: Option<String>,
    pub sort: Option<String>,
    pub include_spent: Option<bool>,
}

#[async_trait]
impl<B: Send> FromRequest<B> for IndexedOutputsPagination<DelegationOutputsQuery> {
    type Rejection = ApiError;

    async fn from_request(req: &mut axum::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<DelegationOutputsPaginationQuery>::from_request(req)
            .await
            .map_err(RequestError::from)?;
        let Extension(config) = Extension::<ApiConfigData>::from_request(req).await?;

        let (cursor, page_size) = if let Some(cursor) = query.cursor {
            let cursor: IndexedOutputsCursor = cursor.parse()?;
            (Some((cursor.slot_index, cursor.output_id)), cursor.page_size)
        } else {
            (None, query.page_size.unwrap_or(DEFAULT_PAGE_SIZE))
        };

        let sort = query
            .sort
            .as_deref()
            .map_or(Ok(Default::default()), str::parse)
            .map_err(RequestError::SortOrder)?;

        Ok(IndexedOutputsPagination {
            query: DelegationOutputsQuery {
                address: query.address.map(Bech32Address::into_inner),
                validator: query.validator,
                created_before: query.created_before,
                created_after: query.created_after,
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
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::api::ApiConfig;

    #[test]
    fn indexed_outputs_cursor_from_to_str() {
        let slot_index = SlotIndex(164338324);
        let output_id_str = "0xfa0de75d225cca2799395e5fc340702fc7eac821d2bdd79911126f131ae097a20100";
        let page_size_str = "1337";

        let cursor = format!("{slot_index}.{output_id_str}.{page_size_str}",);
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
