// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{routing::get, Extension};
use bee_api_types_stardust::responses::RentStructureResponse;
use chronicle::{
    db::{
        collections::{OutputKind, PayloadKind},
        MongoDb,
    },
    types::stardust::block::{
        AliasOutput, BasicOutput, FoundryOutput, MilestonePayload, NftOutput, TaggedDataPayload, TransactionPayload,
        TreasuryTransactionPayload,
    },
};

use super::{
    extractors::{LedgerIndex, MilestoneRange, RichestAddressesQuery},
    responses::{
        AddressAnalyticsResponse, BlockAnalyticsResponse, OutputAnalyticsResponse, OutputDiffAnalyticsResponse,
        RichestAddressesResponse, StorageDepositAnalyticsResponse, TokenDistributionResponse,
    },
};
use crate::api::{router::Router, ApiError, ApiResult};

pub fn routes() -> Router {
    Router::new()
        .nest(
            "/ledger",
            Router::new()
                .route("/storage-deposit", get(storage_deposit_ledger_analytics))
                .route("/native-tokens", get(unspent_output_ledger_analytics::<FoundryOutput>))
                .route("/nfts", get(unspent_output_ledger_analytics::<NftOutput>))
                .route("/richest-addresses", get(richest_addresses_ledger_analytics))
                .route("/token-distribution", get(token_distribution_ledger_analytics)),
        )
        .nest(
            "/activity",
            Router::new()
                .route("/addresses", get(address_activity_analytics))
                .route("/native-tokens", get(native_token_activity_analytics))
                .route("/nfts", get(nft_activity_analytics))
                .nest(
                    "/blocks",
                    Router::new()
                        .route("/", get(block_activity_analytics::<()>))
                        .route("/milestone", get(block_activity_analytics::<MilestonePayload>))
                        .route("/transaction", get(block_activity_analytics::<TransactionPayload>))
                        .route("/tagged-data", get(block_activity_analytics::<TaggedDataPayload>))
                        .route(
                            "/treasury-transaction",
                            get(block_activity_analytics::<TreasuryTransactionPayload>),
                        ),
                )
                .nest(
                    "/outputs",
                    Router::new()
                        .route("/", get(output_activity_analytics::<()>))
                        .route("/basic", get(output_activity_analytics::<BasicOutput>))
                        .route("/alias", get(output_activity_analytics::<AliasOutput>))
                        .route("/nft", get(output_activity_analytics::<NftOutput>))
                        .route("/foundry", get(output_activity_analytics::<FoundryOutput>)),
                ),
        )
}

async fn address_activity_analytics(
    database: Extension<MongoDb>,
    MilestoneRange { start_index, end_index }: MilestoneRange,
) -> ApiResult<AddressAnalyticsResponse> {
    let res = database.get_address_analytics(start_index, end_index).await?;

    Ok(AddressAnalyticsResponse {
        total_active_addresses: res.total_active_addresses.to_string(),
        receiving_addresses: res.receiving_addresses.to_string(),
        sending_addresses: res.sending_addresses.to_string(),
    })
}

async fn block_activity_analytics<B: PayloadKind>(
    database: Extension<MongoDb>,
    MilestoneRange { start_index, end_index }: MilestoneRange,
) -> ApiResult<BlockAnalyticsResponse> {
    let res = database.get_block_analytics::<B>(start_index, end_index).await?;

    Ok(BlockAnalyticsResponse {
        count: res.count.to_string(),
    })
}

async fn output_activity_analytics<O: OutputKind>(
    database: Extension<MongoDb>,
    MilestoneRange { start_index, end_index }: MilestoneRange,
) -> ApiResult<OutputAnalyticsResponse> {
    let res = database.get_output_analytics::<O>(start_index, end_index).await?;

    Ok(OutputAnalyticsResponse {
        count: res.count.to_string(),
        total_value: res.total_value,
    })
}

async fn unspent_output_ledger_analytics<O: OutputKind>(
    database: Extension<MongoDb>,
    LedgerIndex { ledger_index }: LedgerIndex,
) -> ApiResult<OutputAnalyticsResponse> {
    let res = database
        .get_unspent_output_analytics::<O>(ledger_index)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(OutputAnalyticsResponse {
        count: res.count.to_string(),
        total_value: res.total_value,
    })
}

async fn storage_deposit_ledger_analytics(
    database: Extension<MongoDb>,
    LedgerIndex { ledger_index }: LedgerIndex,
) -> ApiResult<StorageDepositAnalyticsResponse> {
    let res = database
        .get_storage_deposit_analytics(ledger_index)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(StorageDepositAnalyticsResponse {
        output_count: res.output_count.to_string(),
        storage_deposit_return_count: res.storage_deposit_return_count.to_string(),
        storage_deposit_return_total_value: res.storage_deposit_return_total_value,
        total_key_bytes: res.total_key_bytes,
        total_data_bytes: res.total_data_bytes,
        total_byte_cost: res.total_byte_cost,
        ledger_index: res.ledger_index.0,
        rent_structure: RentStructureResponse {
            v_byte_cost: res.rent_structure.v_byte_cost,
            v_byte_factor_key: res.rent_structure.v_byte_factor_key,
            v_byte_factor_data: res.rent_structure.v_byte_factor_data,
        },
    })
}

async fn nft_activity_analytics(
    database: Extension<MongoDb>,
    MilestoneRange { start_index, end_index }: MilestoneRange,
) -> ApiResult<OutputDiffAnalyticsResponse> {
    let res = database.get_nft_output_analytics(start_index, end_index).await?;

    Ok(OutputDiffAnalyticsResponse {
        created_count: res.created_count.to_string(),
        transferred_count: res.transferred_count.to_string(),
        burned_count: res.burned_count.to_string(),
    })
}

async fn native_token_activity_analytics(
    database: Extension<MongoDb>,
    MilestoneRange { start_index, end_index }: MilestoneRange,
) -> ApiResult<OutputDiffAnalyticsResponse> {
    let res = database.get_foundry_output_analytics(start_index, end_index).await?;

    Ok(OutputDiffAnalyticsResponse {
        created_count: res.created_count.to_string(),
        transferred_count: res.transferred_count.to_string(),
        burned_count: res.burned_count.to_string(),
    })
}

async fn richest_addresses_ledger_analytics(
    database: Extension<MongoDb>,
    RichestAddressesQuery { top, ledger_index }: RichestAddressesQuery,
) -> ApiResult<RichestAddressesResponse> {
    let res = database
        .get_richest_addresses(ledger_index, top)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(RichestAddressesResponse {
        top: res.top.into_iter().map(Into::into).collect(),
        ledger_index: res.ledger_index.0,
    })
}

async fn token_distribution_ledger_analytics(
    database: Extension<MongoDb>,
    LedgerIndex { ledger_index }: LedgerIndex,
) -> ApiResult<TokenDistributionResponse> {
    let res = database
        .get_token_distribution(ledger_index)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(TokenDistributionResponse {
        distribution: res.distribution.into_iter().map(Into::into).collect(),
        ledger_index: res.ledger_index.0,
    })
}
