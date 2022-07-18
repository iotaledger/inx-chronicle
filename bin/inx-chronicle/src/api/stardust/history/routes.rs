// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{extract::Path, routing::get, Extension, Router};
use chronicle::{
    db::MongoDb,
    types::{stardust::block::Address, tangle::MilestoneIndex},
};
use futures::{StreamExt, TryStreamExt};

use super::{
    extractors::{HistoryByAddressCursor, HistoryByAddressPagination, HistoryByMilestonePagination},
    responses::{
        TransactionsPerAddressResponse, TransactionsPerMilestoneResponse, TransferByAddress, TransferByMilestone,
    },
};
use crate::api::{responses::SyncDataDto, ApiError, ApiResult};

pub fn routes() -> Router {
    Router::new().route("/gaps", get(sync)).nest(
        "/ledger/updates",
        Router::new()
            .route("/by-address/:address", get(transactions_by_address_history))
            .route("/by-milestone/:milestone_id", get(transactions_by_milestone_history)),
    )
}

async fn sync(database: Extension<MongoDb>) -> ApiResult<SyncDataDto> {
    Ok(SyncDataDto(database.get_sync_data(0.into()..=u32::MAX.into()).await?))
}

async fn transactions_by_address_history(
    database: Extension<MongoDb>,
    Path(address): Path<String>,
    HistoryByAddressPagination {
        page_size,
        sort,
        cursor,
    }: HistoryByAddressPagination,
) -> ApiResult<TransactionsPerAddressResponse> {
    let address_dto = Address::from_str(&address).map_err(ApiError::bad_parse)?;

    let mut record_stream = database
        .stream_ledger_updates_for_address(
            &address_dto,
            // Get one extra record so that we can create the cursor.
            page_size + 1,
            cursor,
            sort.into(),
        )
        .await?;

    // Take all of the requested records first
    let records = record_stream.by_ref().take(page_size).try_collect::<Vec<_>>().await?;

    // If any record is left, use it to make the cursor
    let cursor = record_stream.try_next().await?.map(|rec| {
        HistoryByAddressCursor {
            milestone_index: rec.at.milestone_index,
            output_id: rec.output_id,
            is_spent: rec.is_spent,
            page_size,
        }
        .to_string()
    });

    let transfers = records
        .into_iter()
        .map(|rec| {
            Ok(TransferByAddress {
                output_id: rec.output_id.to_hex(),
                is_spent: rec.is_spent,
                milestone_index: rec.at.milestone_index,
                milestone_timestamp: rec.at.milestone_timestamp,
            })
        })
        .collect::<Result<_, ApiError>>()?;

    Ok(TransactionsPerAddressResponse {
        address,
        items: transfers,
        cursor,
    })
}

async fn transactions_by_milestone_history(
    database: Extension<MongoDb>,
    Path(milestone_index): Path<String>,
    HistoryByMilestonePagination {
        page_size,
        sort,
        start_output_id,
    }: HistoryByMilestonePagination,
) -> ApiResult<TransactionsPerMilestoneResponse> {
    let milestone_index = MilestoneIndex::from_str(&milestone_index).map_err(ApiError::bad_parse)?;

    let mut record_stream = database
        .stream_ledger_updates_for_index_paginated(milestone_index, page_size + 1, start_output_id, sort.into())
        .await?;

    // Take all of the requested records first
    let records = record_stream.by_ref().take(page_size).try_collect::<Vec<_>>().await?;

    // If any record is left, use it to make the paging state
    let cursor = record_stream
        .try_next()
        .await?
        .map(|rec| format!("{}.{}", rec.output_id.to_hex(), page_size));

    let transfers = records
        .into_iter()
        .map(|rec| {
            Ok(TransferByMilestone {
                address: rec.address,
                output_id: rec.output_id.to_hex(),
                is_spent: rec.is_spent,
            })
        })
        .collect::<Result<_, ApiError>>()?;

    Ok(TransactionsPerMilestoneResponse {
        index: milestone_index,
        items: transfers,
        cursor,
    })
}
