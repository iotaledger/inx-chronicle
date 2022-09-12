// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{extract::Path, routing::get, Extension, Router};
use chronicle::{
    db::MongoDb,
    types::stardust::block::{payload::milestone::MilestoneId, Address, BlockId},
};
use futures::{StreamExt, TryStreamExt};

use super::{
    extractors::{
        LedgerUpdatesByAddressCursor, LedgerUpdatesByAddressPagination, LedgerUpdatesByMilestoneCursor,
        LedgerUpdatesByMilestonePagination,
    },
    responses::{
        BalanceResponse, BlockChildrenResponse, LedgerUpdatesByAddressResponse, LedgerUpdatesByMilestoneResponse,
    },
};
use crate::api::{extractors::Pagination, ApiError, ApiResult};

pub fn routes() -> Router {
    Router::new()
        .route("/balance/:address", get(balance))
        .route("/blocks/:block_id/children", get(block_children))
        .nest(
            "/ledger/updates",
            Router::new()
                .route("/by-address/:address", get(ledger_updates_by_address))
                .route("/by-milestone/:milestone_id", get(ledger_updates_by_milestone)),
        )
}

async fn ledger_updates_by_address(
    database: Extension<MongoDb>,
    Path(address): Path<String>,
    LedgerUpdatesByAddressPagination {
        page_size,
        sort,
        cursor,
    }: LedgerUpdatesByAddressPagination,
) -> ApiResult<LedgerUpdatesByAddressResponse> {
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
        .map_ok(Into::into)
        .try_collect()
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

    Ok(LedgerUpdatesByAddressResponse { address, items, cursor })
}

async fn ledger_updates_by_milestone(
    database: Extension<MongoDb>,
    Path(milestone_id): Path<String>,
    LedgerUpdatesByMilestonePagination { page_size, cursor }: LedgerUpdatesByMilestonePagination,
) -> ApiResult<LedgerUpdatesByMilestoneResponse> {
    let milestone_id = MilestoneId::from_str(&milestone_id).map_err(ApiError::bad_parse)?;

    let milestone_index = database
        .get_milestone_payload_by_id(&milestone_id)
        .await?
        .ok_or(ApiError::NotFound)?
        .essence
        .index;

    let mut record_stream = database
        .stream_ledger_updates_by_milestone(milestone_index, page_size + 1, cursor)
        .await?;

    // Take all of the requested records first
    let items = record_stream
        .by_ref()
        .take(page_size)
        .map_ok(Into::into)
        .try_collect()
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
        .get_address_balance(address)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(BalanceResponse {
        total_balance: res.total_balance,
        sig_locked_balance: res.sig_locked_balance,
        ledger_index: res.ledger_index,
    })
}

async fn block_children(
    database: Extension<MongoDb>,
    Path(block_id): Path<String>,
    Pagination { page_size, page }: Pagination,
) -> ApiResult<BlockChildrenResponse> {
    let block_id = BlockId::from_str(&block_id).map_err(ApiError::bad_parse)?;
    let mut block_children = database
        .get_block_children(&block_id, page_size, page)
        .await
        .map_err(|_| ApiError::NoResults)?;

    let mut children = Vec::new();
    while let Some(block_id) = block_children.try_next().await? {
        children.push(block_id.to_hex());
    }

    Ok(BlockChildrenResponse {
        block_id: block_id.to_hex(),
        max_results: page_size,
        count: children.len(),
        children,
    })
}
