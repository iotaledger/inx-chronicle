// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{
    extract::{Path, State},
    routing::get,
};
use chronicle::db::{
    mongodb::collections::{
        AccountOutputsQuery, AnchorOutputsQuery, BasicOutputsQuery, CommittedSlotCollection, DelegationOutputsQuery,
        FoundryOutputsQuery, IndexedId, NftOutputsQuery, OutputCollection,
    },
    MongoDb,
};
use iota_sdk::types::block::output::{AccountId, AnchorId, DelegationId, FoundryId, NftId};
use mongodb::bson;

use super::{extractors::IndexedOutputsPagination, responses::IndexerOutputsResponse};
use crate::api::{
    error::{MissingError, RequestError},
    indexer::extractors::IndexedOutputsCursor,
    router::Router,
    ApiResult, ApiState,
};

pub fn routes() -> Router<ApiState> {
    Router::new().nest(
        "/outputs",
        Router::new()
            .route("/basic", get(indexed_outputs::<BasicOutputsQuery>))
            .nest(
                "/account",
                Router::new()
                    .route("/", get(indexed_outputs::<AccountOutputsQuery>))
                    .route("/:account_id", get(indexed_output_by_id::<AccountId>)),
            )
            .nest(
                "/anchor",
                Router::new()
                    .route("/", get(indexed_outputs::<AnchorOutputsQuery>))
                    .route("/:anchor_id", get(indexed_output_by_id::<AnchorId>)),
            )
            .nest(
                "/foundry",
                Router::new()
                    .route("/", get(indexed_outputs::<FoundryOutputsQuery>))
                    .route("/:foundry_id", get(indexed_output_by_id::<FoundryId>)),
            )
            .nest(
                "/nft",
                Router::new()
                    .route("/", get(indexed_outputs::<NftOutputsQuery>))
                    .route("/:nft_id", get(indexed_output_by_id::<NftId>)),
            )
            .nest(
                "/delegation",
                Router::new()
                    .route("/", get(indexed_outputs::<DelegationOutputsQuery>))
                    .route("/:delegation_id", get(indexed_output_by_id::<DelegationId>)),
            ),
    )
}

async fn indexed_output_by_id<ID>(database: State<MongoDb>, Path(id): Path<String>) -> ApiResult<IndexerOutputsResponse>
where
    ID: Into<IndexedId> + FromStr,
    RequestError: From<ID::Err>,
{
    let ledger_index = database
        .collection::<CommittedSlotCollection>()
        .get_latest_committed_slot()
        .await?
        .ok_or(MissingError::NoResults)?
        .slot_index;
    let id = ID::from_str(&id).map_err(RequestError::from)?;
    let res = database
        .collection::<OutputCollection>()
        .get_indexed_output_by_id(id, ledger_index)
        .await?
        .ok_or(MissingError::NoResults)?;
    Ok(IndexerOutputsResponse {
        ledger_index,
        items: vec![res.output_id],
        cursor: None,
    })
}

async fn indexed_outputs<Q>(
    database: State<MongoDb>,
    IndexedOutputsPagination {
        query,
        page_size,
        cursor,
        sort,
        include_spent,
    }: IndexedOutputsPagination<Q>,
) -> ApiResult<IndexerOutputsResponse>
where
    bson::Document: From<Q>,
{
    let ledger_index = database
        .collection::<CommittedSlotCollection>()
        .get_latest_committed_slot()
        .await?
        .ok_or(MissingError::NoResults)?
        .slot_index;
    let res = database
        .collection::<OutputCollection>()
        .get_indexed_outputs(
            query,
            // Get one extra record so that we can create the cursor.
            page_size + 1,
            cursor,
            sort,
            include_spent,
            ledger_index,
        )
        .await?;

    let mut iter = res.outputs.iter();

    // Take all of the requested records first
    let items = iter.by_ref().take(page_size).map(|o| o.output_id).collect();

    // If any record is left, use it to make the cursor
    let cursor = iter.next().map(|rec| {
        IndexedOutputsCursor {
            slot_index: rec.booked_index,
            output_id: rec.output_id,
            page_size,
        }
        .to_string()
    });

    Ok(IndexerOutputsResponse {
        ledger_index,
        items,
        cursor,
    })
}
