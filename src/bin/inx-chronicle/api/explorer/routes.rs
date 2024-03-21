// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{
    extract::{Path, State},
    routing::get,
};
use chronicle::db::{
    mongodb::collections::{
        AddressBalanceCollection, ApplicationStateCollection, BlockCollection, CommittedSlotCollection,
        LedgerUpdateCollection, OutputCollection, ParentsCollection,
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
        BlocksBySlotCursor, BlocksBySlotIndexPagination, LedgerUpdatesByAddressCursor,
        LedgerUpdatesByAddressPagination, LedgerUpdatesBySlotCursor, LedgerUpdatesBySlotPagination,
        RichestAddressesQuery, SlotsCursor, SlotsPagination,
    },
    responses::{
        AddressStatDto, Balance, BalanceResponse, BlockChildrenResponse, BlockPayloadTypeDto, BlocksBySlotResponse,
        DecayedMana, LedgerUpdateBySlotDto, LedgerUpdatesByAddressResponse, LedgerUpdatesBySlotResponse,
        RichestAddressesResponse, SlotDto, SlotsResponse, TokenDistributionResponse,
    },
};
use crate::api::{
    error::{CorruptStateError, MissingError},
    extractors::Pagination,
    router::Router,
    ApiResult, ApiState,
};

pub fn routes() -> Router<ApiState> {
    #[allow(unused_mut)]
    let mut routes = Router::new()
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
            Router::new().nest(
                "/updates",
                Router::new()
                    .route("/by-address/:address", get(ledger_updates_by_address))
                    .route("/by-slot-index/:index", get(ledger_updates_by_slot)),
            ),
        );

    #[cfg(feature = "analytics")]
    {
        routes = routes.merge(
            Router::new().nest(
                "/ledger",
                Router::new()
                    .route("/richest-addresses", get(richest_addresses_ledger_analytics))
                    .route("/token-distribution", get(token_distribution_ledger_analytics)),
            ),
        );
    }
    routes
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

    let protocol_params = database
        .collection::<ApplicationStateCollection>()
        .get_protocol_parameters()
        .await?
        .ok_or(CorruptStateError::ProtocolParams)?;

    let res = database
        .collection::<OutputCollection>()
        .get_address_balance(address.into_inner(), latest_slot.slot_index, &protocol_params)
        .await?
        .ok_or(MissingError::NoResults)?;

    Ok(BalanceResponse {
        total_balance: Balance {
            amount: res.total.amount,
            stored_mana: res.total.stored_mana,
            decayed_mana: DecayedMana {
                stored: res.total.decayed_mana.stored,
                potential: res.total.decayed_mana.potential,
            },
        },
        available_balance: Balance {
            amount: res.available.amount,
            stored_mana: res.available.stored_mana,
            decayed_mana: DecayedMana {
                stored: res.available.decayed_mana.stored,
                potential: res.available.decayed_mana.potential,
            },
        },
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
    let record_stream = database
        .collection::<BlockCollection>()
        .get_blocks_by_slot_index(index, page_size + 1, cursor, sort)
        .await?;
    let count = record_stream.count;
    let mut record_stream = record_stream.stream;

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

    Ok(BlocksBySlotResponse { count, blocks, cursor })
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

#[cfg(feature = "analytics")]
async fn richest_addresses_ledger_analytics(
    database: State<MongoDb>,
    RichestAddressesQuery { top }: RichestAddressesQuery,
) -> ApiResult<RichestAddressesResponse> {
    let ledger_index = database
        .collection::<CommittedSlotCollection>()
        .get_latest_committed_slot()
        .await?
        .ok_or(MissingError::NoResults)?
        .slot_index;
    let res = database
        .collection::<AddressBalanceCollection>()
        .get_richest_addresses(top)
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

#[cfg(feature = "analytics")]
async fn token_distribution_ledger_analytics(database: State<MongoDb>) -> ApiResult<TokenDistributionResponse> {
    let ledger_index = database
        .collection::<CommittedSlotCollection>()
        .get_latest_committed_slot()
        .await?
        .ok_or(MissingError::NoResults)?
        .slot_index;
    let res = database
        .collection::<AddressBalanceCollection>()
        .get_token_distribution()
        .await?;

    Ok(TokenDistributionResponse {
        distribution: res.distribution.into_iter().map(Into::into).collect(),
        ledger_index,
    })
}
