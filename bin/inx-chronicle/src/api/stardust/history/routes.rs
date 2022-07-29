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
    extractors::{
        LedgerUpdatesByAddressCursor, LedgerUpdatesByAddressPagination, LedgerUpdatesByMilestoneCursor,
        LedgerUpdatesByMilestonePagination,
    },
    responses::{
        BalanceResponse, LederUpdatesByAddressResponse, LedgerUpdateByAddressResponse, LedgerUpdateByMilestoneResponse,
        LedgerUpdatesByMilestoneResponse,
    },
};
use crate::api::{responses::SyncDataDto, ApiError, ApiResult};

pub fn routes() -> Router {
    Router::new()
        .route("/gaps", get(sync))
        .route("/balance/:address", get(balance))
        .nest(
            "/ledger/updates",
            Router::new()
                .route("/by-address/:address", get(ledger_updates_by_address))
                .route("/by-milestone/:milestone_id", get(ledger_updates_by_milestone)),
        )
}

async fn sync(database: Extension<MongoDb>) -> ApiResult<SyncDataDto> {
    Ok(SyncDataDto(database.get_sync_data(0.into()..=u32::MAX.into()).await?))
}

async fn ledger_updates_by_address(
    database: Extension<MongoDb>,
    Path(address): Path<String>,
    LedgerUpdatesByAddressPagination {
        page_size,
        sort,
        cursor,
    }: LedgerUpdatesByAddressPagination,
) -> ApiResult<LederUpdatesByAddressResponse> {
    let address_dto = Address::from_str(&address).map_err(ApiError::bad_parse)?;

    let mut record_stream = database
        .stream_ledger_updates_by_address(
            &address_dto,
            // Get one extra record so that we can create the cursor.
            page_size + 1,
            cursor,
            sort,
        )
        .await?;

    // Take all of the requested records first
    let items = record_stream
        .by_ref()
        .take(page_size)
        .map(|record_result| record_result.map(LedgerUpdateByAddressResponse::from))
        .try_collect::<Vec<_>>()
        .await?;

    // If any record is left, use it to make the cursor
    let cursor = record_stream.try_next().await?.map(|rec| {
        LedgerUpdatesByAddressCursor {
            milestone_index: rec.at.milestone_index,
            output_id: rec.output_id,
            is_spent: rec.is_spent,
            page_size,
        }
        .to_string()
    });

    Ok(LederUpdatesByAddressResponse { address, items, cursor })
}

async fn ledger_updates_by_milestone(
    database: Extension<MongoDb>,
    Path(milestone_index): Path<String>,
    LedgerUpdatesByMilestonePagination { page_size, cursor }: LedgerUpdatesByMilestonePagination,
) -> ApiResult<LedgerUpdatesByMilestoneResponse> {
    let milestone_index = MilestoneIndex::from_str(&milestone_index).map_err(ApiError::bad_parse)?;

    let mut record_stream = database
        .stream_ledger_updates_by_milestone(milestone_index, page_size + 1, cursor)
        .await?;

    // Take all of the requested records first
    let items = record_stream
        .by_ref()
        .take(page_size)
        .map(|record_result| record_result.map(LedgerUpdateByMilestoneResponse::from))
        .try_collect::<Vec<_>>()
        .await?;

    // If any record is left, use it to make the paging state
    let cursor = record_stream.try_next().await?.map(|rec| {
        LedgerUpdatesByMilestoneCursor {
            output_id: rec.output_id,
            page_size,
            is_spent: rec.is_spent,
        }
        .to_string()
    });

    Ok(LedgerUpdatesByMilestoneResponse {
        milestone_index,
        items,
        cursor,
    })
}

async fn balance(database: Extension<MongoDb>, Path(address): Path<String>) -> ApiResult<BalanceResponse> {
    let address = Address::from_str(&address).map_err(ApiError::bad_parse)?;
    let res = database
        .sum_balances_owned_by_address(address)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(BalanceResponse {
        total_balance: res.total_balance as u64,
        sig_locked_balance: res.sig_locked_balance as u64,
        ledger_index: res.ledger_index,
    })
}
