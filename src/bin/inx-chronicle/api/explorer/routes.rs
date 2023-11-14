// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{
    extract::{Path, State},
    routing::get,
};
use chronicle::db::{
    mongodb::collections::{
        ApplicationStateCollection, BlockCollection, CommittedSlotCollection, LedgerUpdateCollection, OutputCollection,
        ParentsCollection,
    },
    MongoDb,
};
use futures::{StreamExt, TryStreamExt};
use iota_sdk::types::block::{
    address::{Bech32Address, ToBech32Ext},
    slot::{SlotCommitmentId, SlotIndex},
    BlockId,
};

use super::{
    extractors::{
        BlocksBySlotCursor, BlocksBySlotIndexPagination, LedgerIndex, LedgerUpdatesByAddressCursor,
        LedgerUpdatesByAddressPagination, LedgerUpdatesBySlotCursor, LedgerUpdatesBySlotPagination,
        RichestAddressesQuery, SlotsCursor, SlotsPagination,
    },
    responses::{
        AddressStatDto, BalanceResponse, BlockChildrenResponse, BlockPayloadTypeDto, BlocksBySlotResponse,
        LedgerUpdateBySlotDto, LedgerUpdatesByAddressResponse, LedgerUpdatesBySlotResponse, RichestAddressesResponse,
        SlotDto, SlotsResponse, TokenDistributionResponse,
    },
};
use crate::api::{
    error::{CorruptStateError, MissingError},
    extractors::Pagination,
    router::Router,
    ApiResult, ApiState,
};

pub fn routes() -> Router<ApiState> {
    Router::new()
        .route("/balance/:address", get(balance))
        .route("/blocks/:block_id/children", get(block_children))
        .nest(
            "/commitments",
            Router::new()
                .route("/", get(commitments))
                .route("/:commitment_id/blocks", get(blocks_by_commitment_id))
                .route("/by-index/:index/blocks", get(blocks_by_slot_index)),
        )
        .nest(
            "/ledger",
            Router::new()
                .route("/richest-addresses", get(richest_addresses_ledger_analytics))
                .route("/token-distribution", get(token_distribution_ledger_analytics))
                .nest(
                    "/updates",
                    Router::new()
                        .route("/by-address/:address", get(ledger_updates_by_address))
                        .route("/by-slot-index/:index", get(ledger_updates_by_slot)),
                ),
        )
}

async fn ledger_updates_by_address(
    database: State<MongoDb>,
    Path(address): Path<Bech32Address>,
    LedgerUpdatesByAddressPagination {
        page_size,
        sort,
        cursor,
    }: LedgerUpdatesByAddressPagination,
) -> ApiResult<LedgerUpdatesByAddressResponse> {
    let mut record_stream = database
        .collection::<LedgerUpdateCollection>()
        .get_ledger_updates_by_address(
            &address,
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
            slot_index: rec.slot_index,
            output_id: rec.output_id,
            is_spent: rec.is_spent,
            page_size,
        }
        .to_string()
    });

    Ok(LedgerUpdatesByAddressResponse { address, items, cursor })
}

async fn ledger_updates_by_slot(
    database: State<MongoDb>,
    Path(index): Path<SlotIndex>,
    LedgerUpdatesBySlotPagination { page_size, cursor }: LedgerUpdatesBySlotPagination,
) -> ApiResult<LedgerUpdatesBySlotResponse> {
    let hrp = database
        .collection::<ApplicationStateCollection>()
        .get_protocol_parameters()
        .await?
        .ok_or(CorruptStateError::ProtocolParams)?
        .bech32_hrp();

    let mut record_stream = database
        .collection::<LedgerUpdateCollection>()
        .get_ledger_updates_by_slot(index, page_size + 1, cursor)
        .await?;

    // Take all of the requested records first
    let items = record_stream
        .by_ref()
        .take(page_size)
        .map_ok(|dto| LedgerUpdateBySlotDto {
            address: dto.address.to_bech32(hrp),
            output_id: dto.output_id,
            is_spent: dto.is_spent,
        })
        .try_collect()
        .await?;

    // If any record is left, use it to make the paging state
    let cursor = record_stream.try_next().await?.map(|rec| {
        LedgerUpdatesBySlotCursor {
            output_id: rec.output_id,
            page_size,
            is_spent: rec.is_spent,
        }
        .to_string()
    });

    Ok(LedgerUpdatesBySlotResponse {
        slot_index: index,
        items,
        cursor,
    })
}

async fn balance(database: State<MongoDb>, Path(address): Path<Bech32Address>) -> ApiResult<BalanceResponse> {
    let latest_slot = database
        .collection::<CommittedSlotCollection>()
        .get_latest_committed_slot()
        .await?
        .ok_or(MissingError::NoResults)?;

    let res = database
        .collection::<OutputCollection>()
        .get_address_balance(address.into_inner(), latest_slot.slot_index)
        .await?
        .ok_or(MissingError::NoResults)?;

    Ok(BalanceResponse {
        total_balance: res.total_balance,
        sig_locked_balance: res.sig_locked_balance,
        ledger_index: latest_slot.slot_index,
    })
}

async fn block_children(
    database: State<MongoDb>,
    Path(block_id): Path<BlockId>,
    Pagination { page_size, page }: Pagination,
) -> ApiResult<BlockChildrenResponse> {
    let children = database
        .collection::<ParentsCollection>()
        .get_block_children(&block_id, page_size, page)
        .await
        .map_err(|_| MissingError::NoResults)?
        .try_collect::<Vec<_>>()
        .await?;

    Ok(BlockChildrenResponse {
        block_id,
        max_results: page_size,
        count: children.len(),
        children,
    })
}

async fn commitments(
    database: State<MongoDb>,
    SlotsPagination {
        start_index,
        end_index,
        sort,
        page_size,
        cursor,
    }: SlotsPagination,
) -> ApiResult<SlotsResponse> {
    let mut record_stream = database
        .collection::<CommittedSlotCollection>()
        .get_commitments(start_index, end_index, sort, page_size + 1, cursor)
        .await?;

    // Take all of the requested records first
    let items = record_stream
        .by_ref()
        .take(page_size)
        .map_ok(|s| SlotDto {
            commitment_id: s.commitment_id,
            index: s.slot_index,
        })
        .try_collect()
        .await?;

    // If any record is left, use it to make the paging state
    let cursor = record_stream.try_next().await?.map(|rec| {
        SlotsCursor {
            slot_index: rec.slot_index,
            page_size,
        }
        .to_string()
    });

    Ok(SlotsResponse { items, cursor })
}

async fn blocks_by_slot_index(
    database: State<MongoDb>,
    Path(index): Path<SlotIndex>,
    BlocksBySlotIndexPagination {
        sort,
        page_size,
        cursor,
    }: BlocksBySlotIndexPagination,
) -> ApiResult<BlocksBySlotResponse> {
    let mut record_stream = database
        .collection::<BlockCollection>()
        .get_blocks_by_slot_index(index, page_size + 1, cursor, sort)
        .await?;

    // Take all of the requested records first
    let blocks = record_stream
        .by_ref()
        .take(page_size)
        .map_ok(|rec| BlockPayloadTypeDto {
            block_id: rec.block_id,
            payload_kind: rec.payload_type,
        })
        .try_collect()
        .await?;

    // If any record is left, use it to make the paging state
    let cursor = record_stream.try_next().await?.map(|rec| {
        BlocksBySlotCursor {
            block_id: rec.block_id,
            page_size,
        }
        .to_string()
    });

    Ok(BlocksBySlotResponse { blocks, cursor })
}

async fn blocks_by_commitment_id(
    database: State<MongoDb>,
    Path(commitment_id): Path<SlotCommitmentId>,
    BlocksBySlotIndexPagination {
        sort,
        page_size,
        cursor,
    }: BlocksBySlotIndexPagination,
) -> ApiResult<BlocksBySlotResponse> {
    blocks_by_slot_index(
        database,
        Path(commitment_id.slot_index()),
        BlocksBySlotIndexPagination {
            sort,
            page_size,
            cursor,
        },
    )
    .await
}

async fn richest_addresses_ledger_analytics(
    database: State<MongoDb>,
    RichestAddressesQuery { top, ledger_index }: RichestAddressesQuery,
) -> ApiResult<RichestAddressesResponse> {
    let ledger_index = resolve_ledger_index(&database, ledger_index).await?;
    let res = database
        .collection::<OutputCollection>()
        .get_richest_addresses(ledger_index, top)
        .await?;

    let hrp = database
        .collection::<ApplicationStateCollection>()
        .get_protocol_parameters()
        .await?
        .ok_or(CorruptStateError::ProtocolParams)?
        .bech32_hrp();

    Ok(RichestAddressesResponse {
        top: res
            .top
            .into_iter()
            .map(|stat| AddressStatDto {
                address: stat.address.to_bech32(hrp),
                balance: stat.balance,
            })
            .collect(),
        ledger_index,
    })
}

async fn token_distribution_ledger_analytics(
    database: State<MongoDb>,
    LedgerIndex { ledger_index }: LedgerIndex,
) -> ApiResult<TokenDistributionResponse> {
    let ledger_index = resolve_ledger_index(&database, ledger_index).await?;
    let res = database
        .collection::<OutputCollection>()
        .get_token_distribution(ledger_index)
        .await?;

    Ok(TokenDistributionResponse {
        distribution: res.distribution.into_iter().map(Into::into).collect(),
        ledger_index,
    })
}

/// This is just a helper fn to either unwrap an optional ledger index param or fetch the latest
/// index from the database.
async fn resolve_ledger_index(database: &MongoDb, ledger_index: Option<SlotIndex>) -> ApiResult<SlotIndex> {
    Ok(if let Some(ledger_index) = ledger_index {
        ledger_index
    } else {
        database
            .collection::<CommittedSlotCollection>()
            .get_latest_committed_slot()
            .await?
            .ok_or(MissingError::NoResults)?
            .slot_index
    })
}
