// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{extract::Path, routing::get, Extension};
use chronicle::{
    db::{
        mongodb::collections::{
            BlockCollection, LedgerUpdateCollection, MilestoneCollection, OutputCollection, ProtocolUpdateCollection,
        },
        MongoDb,
    },
    model::{
        Address, BlockId, MilestoneId, MilestoneIndex, MilestonePayload, TaggedDataPayload, TransactionPayload,
        TreasuryTransactionPayload,
    },
};
use futures::{StreamExt, TryStreamExt};

use super::{
    extractors::{
        BlocksByMilestoneCursor, BlocksByMilestoneIdPagination, BlocksByMilestoneIndexPagination, LedgerIndex,
        LedgerUpdatesByAddressCursor, LedgerUpdatesByAddressPagination, LedgerUpdatesByMilestoneCursor,
        LedgerUpdatesByMilestonePagination, MilestonesCursor, MilestonesPagination, RichestAddressesQuery,
    },
    responses::{
        AddressStatDto, BalanceResponse, BlockChildrenResponse, BlockPayloadTypeDto, BlocksByMilestoneResponse,
        LedgerUpdatesByAddressResponse, LedgerUpdatesByMilestoneResponse, MilestonesResponse, RichestAddressesResponse,
        TokenDistributionResponse,
    },
};
use crate::api::{
    error::{CorruptStateError, MissingError, RequestError},
    extractors::Pagination,
    router::Router,
    ApiResult,
};

pub fn routes() -> Router {
    Router::new()
        .route("/balance/:address", get(balance))
        .route("/blocks/:block_id/children", get(block_children))
        .nest(
            "/milestones",
            Router::new()
                .route("/", get(milestones))
                .route("/:milestone_id/blocks", get(blocks_by_milestone_id))
                .route("/by-index/:milestone_index/blocks", get(blocks_by_milestone_index)),
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
                        .route("/by-milestone/:milestone_id", get(ledger_updates_by_milestone)),
                ),
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
    let address_dto = Address::from_str(&address).map_err(RequestError::from)?;

    let mut record_stream = database
        .collection::<LedgerUpdateCollection>()
        .get_ledger_updates_by_address(
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
    let milestone_id = MilestoneId::from_str(&milestone_id).map_err(RequestError::from)?;

    let milestone_index = database
        .collection::<MilestoneCollection>()
        .get_milestone_payload_by_id(&milestone_id)
        .await?
        .ok_or(MissingError::NotFound)?
        .essence
        .index;

    let mut record_stream = database
        .collection::<LedgerUpdateCollection>()
        .get_ledger_updates_by_milestone(milestone_index, page_size + 1, cursor)
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
    let ledger_index = database
        .collection::<MilestoneCollection>()
        .get_ledger_index()
        .await?
        .ok_or(MissingError::NoResults)?;
    let address = Address::from_str(&address).map_err(RequestError::from)?;
    let res = database
        .collection::<OutputCollection>()
        .get_address_balance(address, ledger_index)
        .await?
        .ok_or(MissingError::NoResults)?;

    Ok(BalanceResponse {
        total_balance: res.total_balance,
        sig_locked_balance: res.sig_locked_balance,
        ledger_index,
    })
}

async fn block_children(
    database: Extension<MongoDb>,
    Path(block_id): Path<String>,
    Pagination { page_size, page }: Pagination,
) -> ApiResult<BlockChildrenResponse> {
    let block_id = BlockId::from_str(&block_id).map_err(RequestError::from)?;
    let mut block_children = database
        .collection::<BlockCollection>()
        .get_block_children(&block_id, page_size, page)
        .await
        .map_err(|_| MissingError::NoResults)?;

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

async fn milestones(
    database: Extension<MongoDb>,
    MilestonesPagination {
        start_timestamp,
        end_timestamp,
        sort,
        page_size,
        cursor,
    }: MilestonesPagination,
) -> ApiResult<MilestonesResponse> {
    let mut record_stream = database
        .collection::<MilestoneCollection>()
        .get_milestones(start_timestamp, end_timestamp, sort, page_size + 1, cursor)
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
        MilestonesCursor {
            milestone_index: rec.index,
            page_size,
        }
        .to_string()
    });

    Ok(MilestonesResponse { items, cursor })
}

async fn blocks_by_milestone_index(
    database: Extension<MongoDb>,
    Path(milestone_index): Path<MilestoneIndex>,
    BlocksByMilestoneIndexPagination {
        sort,
        page_size,
        cursor,
    }: BlocksByMilestoneIndexPagination,
) -> ApiResult<BlocksByMilestoneResponse> {
    let mut record_stream = database
        .collection::<BlockCollection>()
        .get_blocks_by_milestone_index(milestone_index, page_size + 1, cursor, sort)
        .await?;

    // Take all of the requested records first
    let blocks = record_stream
        .by_ref()
        .take(page_size)
        .map_ok(|rec| BlockPayloadTypeDto {
            block_id: rec.block_id.to_hex(),
            payload_kind: rec.payload_kind.map(|kind| match kind.as_str() {
                TransactionPayload::KIND => iota_types::block::payload::TransactionPayload::KIND,
                MilestonePayload::KIND => iota_types::block::payload::MilestonePayload::KIND,
                TreasuryTransactionPayload::KIND => iota_types::block::payload::TreasuryTransactionPayload::KIND,
                TaggedDataPayload::KIND => iota_types::block::payload::TaggedDataPayload::KIND,
                _ => panic!("Unknown payload type."),
            }),
        })
        .try_collect()
        .await?;

    // If any record is left, use it to make the paging state
    let cursor = record_stream.try_next().await?.map(|rec| {
        BlocksByMilestoneCursor {
            white_flag_index: rec.white_flag_index,
            page_size,
        }
        .to_string()
    });

    Ok(BlocksByMilestoneResponse { blocks, cursor })
}

async fn blocks_by_milestone_id(
    database: Extension<MongoDb>,
    Path(milestone_id): Path<String>,
    BlocksByMilestoneIdPagination {
        sort,
        page_size,
        cursor,
    }: BlocksByMilestoneIdPagination,
) -> ApiResult<BlocksByMilestoneResponse> {
    let milestone_id = MilestoneId::from_str(&milestone_id).map_err(RequestError::from)?;
    let milestone_index = database
        .collection::<MilestoneCollection>()
        .get_milestone_payload_by_id(&milestone_id)
        .await?
        .ok_or(MissingError::NoResults)?
        .essence
        .index;
    blocks_by_milestone_index(
        database,
        Path(milestone_index),
        BlocksByMilestoneIndexPagination {
            sort,
            page_size,
            cursor,
        },
    )
    .await
}

async fn richest_addresses_ledger_analytics(
    database: Extension<MongoDb>,
    RichestAddressesQuery { top, ledger_index }: RichestAddressesQuery,
) -> ApiResult<RichestAddressesResponse> {
    let ledger_index = resolve_ledger_index(&database, ledger_index).await?;
    let res = database
        .collection::<OutputCollection>()
        .get_richest_addresses(ledger_index, top)
        .await?;

    let hrp = database
        .collection::<ProtocolUpdateCollection>()
        .get_protocol_parameters_for_ledger_index(ledger_index)
        .await?
        .ok_or(CorruptStateError::ProtocolParams)?
        .parameters
        .bech32_hrp;

    Ok(RichestAddressesResponse {
        top: res
            .top
            .into_iter()
            .map(|stat| AddressStatDto {
                address: iota_types::block::address::Address::from(stat.address).to_bech32(hrp.clone()),
                balance: stat.balance,
            })
            .collect(),
        ledger_index,
    })
}

async fn token_distribution_ledger_analytics(
    database: Extension<MongoDb>,
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
async fn resolve_ledger_index(database: &MongoDb, ledger_index: Option<MilestoneIndex>) -> ApiResult<MilestoneIndex> {
    Ok(if let Some(ledger_index) = ledger_index {
        ledger_index
    } else {
        database
            .collection::<MilestoneCollection>()
            .get_ledger_index()
            .await?
            .ok_or(MissingError::NoResults)?
    })
}
