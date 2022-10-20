// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{extract::Path, routing::get, Extension};
use chronicle::{
    db::{
        collections::{
            BlockCollection, MilestoneCollection, OutputCollection, OutputKindQuery, ProtocolUpdateCollection,
        },
        MongoDb,
    },
    types::{
        stardust::block::{
            output::{AliasOutput, BasicOutput, FoundryOutput, NftOutput},
            payload::MilestoneId,
            Output,
        },
        tangle::MilestoneIndex,
    },
};
use iota_types::api::response::RentStructureResponse;

use super::{
    extractors::{LedgerIndex, MilestoneRange, RichestAddressesQuery},
    responses::{
        ActivityPerInclusionStateDto, ActivityPerPayloadTypeDto, AddressAnalyticsResponse, AddressStatDto,
        ClaimedTokensAnalyticsResponse, MilestoneAnalyticsResponse, OutputAnalyticsResponse,
        OutputDiffAnalyticsResponse, RichestAddressesResponse, StorageDepositAnalyticsResponse,
        TokenDistributionResponse,
    },
};
use crate::api::{error::InternalApiError, router::Router, ApiError, ApiResult};

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
                .route("/claimed-tokens", get(claimed_tokens_analytics))
                .nest(
                    "/milestones",
                    Router::new()
                        .route("/:milestone_id", get(milestone_activity_analytics_by_id))
                        .route("/by-index/:milestone_index", get(milestone_activity_analytics)),
                )
                .route("/native-tokens", get(native_token_activity_analytics))
                .route("/nfts", get(nft_activity_analytics))
                .nest(
                    "/outputs",
                    Router::new()
                        .route("/", get(output_activity_analytics::<Output>))
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
    let res = database
        .collection::<OutputCollection>()
        .get_address_analytics(start_index, end_index)
        .await?;

    Ok(AddressAnalyticsResponse {
        total_active_addresses: res.total_active_addresses.to_string(),
        receiving_addresses: res.receiving_addresses.to_string(),
        sending_addresses: res.sending_addresses.to_string(),
    })
}

async fn milestone_activity_analytics(
    database: Extension<MongoDb>,
    Path(milestone_index): Path<String>,
) -> ApiResult<MilestoneAnalyticsResponse> {
    let index = MilestoneIndex::from_str(&milestone_index).map_err(ApiError::bad_parse)?;

    let activity = database
        .collection::<BlockCollection>()
        .get_milestone_activity(index)
        .await?;

    Ok(MilestoneAnalyticsResponse {
        blocks_count: activity.num_blocks,
        per_payload_type: ActivityPerPayloadTypeDto {
            tx_payload_count: activity.num_tx_payload,
            treasury_tx_payload_count: activity.num_treasury_tx_payload,
            tagged_data_payload_count: activity.num_tagged_data_payload,
            milestone_payload_count: activity.num_milestone_payload,
            no_payload_count: activity.num_no_payload,
        },
        per_inclusion_state: ActivityPerInclusionStateDto {
            confirmed_tx_count: activity.num_confirmed_tx,
            conflicting_tx_count: activity.num_conflicting_tx,
            no_tx_count: activity.num_no_tx,
        },
    })
}

async fn milestone_activity_analytics_by_id(
    database: Extension<MongoDb>,
    Path(milestone_id): Path<String>,
) -> ApiResult<MilestoneAnalyticsResponse> {
    let milestone_id = MilestoneId::from_str(&milestone_id).map_err(ApiError::bad_parse)?;

    let index = database
        .collection::<MilestoneCollection>()
        .get_milestone_payload_by_id(&milestone_id)
        .await?
        .ok_or(ApiError::NotFound)?
        .essence
        .index;

    let activity = database
        .collection::<BlockCollection>()
        .get_milestone_activity(index)
        .await?;

    Ok(MilestoneAnalyticsResponse {
        blocks_count: activity.num_blocks,
        per_payload_type: ActivityPerPayloadTypeDto {
            tx_payload_count: activity.num_tx_payload,
            treasury_tx_payload_count: activity.num_treasury_tx_payload,
            tagged_data_payload_count: activity.num_tagged_data_payload,
            milestone_payload_count: activity.num_milestone_payload,
            no_payload_count: activity.num_no_payload,
        },
        per_inclusion_state: ActivityPerInclusionStateDto {
            confirmed_tx_count: activity.num_confirmed_tx,
            conflicting_tx_count: activity.num_conflicting_tx,
            no_tx_count: activity.num_no_tx,
        },
    })
}

async fn output_activity_analytics<O: OutputKindQuery>(
    database: Extension<MongoDb>,
    MilestoneRange { start_index, end_index }: MilestoneRange,
) -> ApiResult<OutputAnalyticsResponse> {
    let res = database
        .collection::<OutputCollection>()
        .get_output_analytics::<O>(start_index, end_index)
        .await?;

    Ok(OutputAnalyticsResponse {
        count: res.count.to_string(),
        total_value: res.total_value,
    })
}

async fn unspent_output_ledger_analytics<O: OutputKindQuery>(
    database: Extension<MongoDb>,
    LedgerIndex { ledger_index }: LedgerIndex,
) -> ApiResult<OutputAnalyticsResponse> {
    let res = database
        .collection::<OutputCollection>()
        .get_unspent_output_analytics::<O>(resolve_ledger_index(&database, ledger_index).await?)
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
    let ledger_index = resolve_ledger_index(&database, ledger_index).await?;
    let protocol_params = database
        .collection::<ProtocolUpdateCollection>()
        .get_protocol_parameters_for_ledger_index(ledger_index)
        .await?
        .ok_or(InternalApiError::CorruptState("no protocol parameters"))?
        .parameters;
    let res = database
        .collection::<OutputCollection>()
        .get_storage_deposit_analytics(ledger_index, protocol_params)
        .await?;

    Ok(StorageDepositAnalyticsResponse {
        output_count: res.output_count.to_string(),
        storage_deposit_return_count: res.storage_deposit_return_count.to_string(),
        storage_deposit_return_total_value: res.storage_deposit_return_total_value,
        total_key_bytes: res.total_key_bytes,
        total_data_bytes: res.total_data_bytes,
        total_byte_cost: res.total_byte_cost,
        ledger_index,
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
    let res = database
        .collection::<OutputCollection>()
        .get_nft_output_analytics(start_index, end_index)
        .await?;

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
    let res = database
        .collection::<OutputCollection>()
        .get_foundry_output_analytics(start_index, end_index)
        .await?;

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
    let ledger_index = resolve_ledger_index(&database, ledger_index).await?;
    let res = database
        .collection::<OutputCollection>()
        .get_richest_addresses(ledger_index, top)
        .await?;

    let hrp = database
        .collection::<ProtocolUpdateCollection>()
        .get_protocol_parameters_for_ledger_index(ledger_index)
        .await?
        .ok_or(InternalApiError::CorruptState("no protocol parameters"))?
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
            .ok_or(ApiError::NoResults)?
    })
}

async fn claimed_tokens_analytics(
    database: Extension<MongoDb>,
    LedgerIndex { ledger_index }: LedgerIndex,
) -> ApiResult<ClaimedTokensAnalyticsResponse> {
    let res = database
        .collection::<OutputCollection>()
        .get_claimed_token_analytics(ledger_index)
        .await?
        .ok_or(ApiError::NoResults)?;

    Ok(ClaimedTokensAnalyticsResponse { count: res.count })
}
