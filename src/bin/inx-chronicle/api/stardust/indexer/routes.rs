// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{
    extract::{Path, State},
    routing::get,
    Router,
};
use chronicle::{
    db::{
        collections::{
            AliasOutputsQuery, BasicOutputsQuery, FoundryOutputsQuery, IndexedId, MilestoneCollection, NftOutputsQuery,
            OutputCollection,
        },
        MongoDb,
    },
    types::stardust::block::output::{AliasId, FoundryId, NftId},
};
use mongodb::bson;

use super::{extractors::IndexedOutputsPagination, responses::IndexerOutputsResponse};
use crate::api::{
    error::{MissingError, RequestError},
    stardust::indexer::extractors::IndexedOutputsCursor,
    ApiResult, ApiState,
};

pub fn routes() -> Router<ApiState> {
    Router::new().nest(
        "/outputs",
        Router::new()
            .route("/basic", get(indexed_outputs::<BasicOutputsQuery>))
            .nest(
                "/alias",
                Router::new()
                    .route("/", get(indexed_outputs::<AliasOutputsQuery>))
                    .route("/:alias_id", get(indexed_output_by_id::<AliasId>)),
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
            ),
    )
}

async fn indexed_output_by_id<ID>(database: State<MongoDb>, Path(id): Path<String>) -> ApiResult<IndexerOutputsResponse>
where
    ID: Into<IndexedId> + FromStr,
    RequestError: From<ID::Err>,
{
    let ledger_index = database
        .collection::<MilestoneCollection>()
        .get_ledger_index()
        .await?
        .ok_or(MissingError::NoResults)?;
    let id = ID::from_str(&id).map_err(RequestError::from)?;
    let res = database
        .collection::<OutputCollection>()
        .get_indexed_output_by_id(id, ledger_index)
        .await?
        .ok_or(MissingError::NoResults)?;
    Ok(IndexerOutputsResponse {
        ledger_index,
        items: vec![res.output_id.to_hex()],
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
        .collection::<MilestoneCollection>()
        .get_ledger_index()
        .await?
        .ok_or(MissingError::NoResults)?;
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
    let items = iter.by_ref().take(page_size).map(|o| o.output_id.to_hex()).collect();

    // If any record is left, use it to make the cursor
    let cursor = iter.next().map(|rec| {
        IndexedOutputsCursor {
            milestone_index: rec.booked_index,
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
