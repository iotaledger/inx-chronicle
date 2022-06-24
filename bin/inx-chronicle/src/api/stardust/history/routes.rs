// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{extract::Path, routing::get, Extension, Router};
use chronicle::{
    db::{collections::SortOrder, MongoDb},
    types::stardust::block::{Address, MilestoneId},
};
use futures::{StreamExt, TryStreamExt};

use super::{
    extractors::{HistoryByAddressPagination, HistoryByMilestonePagination},
    responses::{TransactionsPerAddressResponse, TransactionsPerMilestoneResponse, Transfer},
};
use crate::api::{routes::sync, ApiError, ApiResult};

pub fn routes() -> Router {
    Router::new().route("/gaps", get(sync)).nest(
        "/ledger/updates",
        Router::new()
            .route("/by-address/:address", get(transactions_by_address_history))
            .route("/by-milestone/:milestone_id", get(transactions_by_milestone_history)),
    )
}

async fn transactions_by_address_history(
    database: Extension<MongoDb>,
    Path(address): Path<String>,
    HistoryByAddressPagination {
        page_size,
        start_milestone_index,
        start_output_id,
    }: HistoryByAddressPagination,
) -> ApiResult<TransactionsPerAddressResponse> {
    let address_dto = Address::from_str(&address).map_err(ApiError::bad_parse)?;

    let mut records_iter = database
        .get_ledger_updates(
            &address_dto,
            // Get one extra record so that we can create the cursor.
            page_size + 1,
            start_milestone_index.map(Into::into),
            start_output_id,
            // TODO: Allow specifying sort in query
            SortOrder::Newest,
        )
        .await?;

    // Take all of the requested records first
    let records = records_iter.by_ref().take(page_size).try_collect::<Vec<_>>().await?;
    // If any record is left, use it to make the cursor
    let cursor = records_iter
        .try_next()
        .await?
        .map(|doc| format!("{}.{}.{}", doc.at.milestone_index, doc.output_id.to_hex(), page_size));

    let transactions = records
        .into_iter()
        .map(|rec| {
            Ok(Transfer {
                output_id: rec.output_id.to_hex(),
                is_spent: rec.is_spent,
                milestone_index: rec.at.milestone_index,
                milestone_timestamp: rec.at.milestone_timestamp,
            })
        })
        .collect::<Result<_, ApiError>>()?;

    Ok(TransactionsPerAddressResponse {
        address,
        items: transactions,
        cursor,
    })
}

async fn transactions_by_milestone_history(
    database: Extension<MongoDb>,
    Path(milestone_id): Path<String>,
    HistoryByMilestonePagination {
        page_size,
        start_output_id,
    }: HistoryByMilestonePagination,
) -> ApiResult<TransactionsPerMilestoneResponse> {
    let milestone_id = MilestoneId::from_str(&milestone_id).map_err(ApiError::bad_parse)?;
    let milestone_index = database
        .get_milestone_index_by_id(milestone_id)
        .await?
        .ok_or(ApiError::NoResults)?;

    let mut records_iter = database
        .get_ledger_updates_at_index_paginated(milestone_index, page_size + 1, start_output_id, SortOrder::Newest)
        .await?;

    // Take all of the requested records first
    let records = records_iter.by_ref().take(page_size).try_collect::<Vec<_>>().await?;
    // If any record is left, use it to make the paging state
    let cursor = records_iter
        .try_next()
        .await?
        .map(|doc| format!("{}.{}", doc.output_id.to_hex(), page_size));

    let transactions = records
        .into_iter()
        .map(|rec| {
            Ok(Transfer {
                output_id: rec.output_id.to_hex(),
                is_spent: rec.is_spent,
                milestone_index: rec.at.milestone_index,
                milestone_timestamp: rec.at.milestone_timestamp,
            })
        })
        .collect::<Result<_, ApiError>>()?;

    Ok(TransactionsPerMilestoneResponse {
        id: milestone_id.to_hex(),
        items: transactions,
        cursor,
    })
}
