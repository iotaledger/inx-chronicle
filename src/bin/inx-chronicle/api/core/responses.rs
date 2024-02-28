// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::{
    types::{
        api::core::{BaseTokenResponse, ProtocolParametersResponse},
        block::{
            address::Bech32Address,
            output::{Output, OutputIdProof, OutputMetadata},
            protocol::ProtocolParametersHash,
            slot::{EpochIndex, SlotCommitmentId},
        },
    },
    utils::serde::string,
};
use serde::{Deserialize, Serialize};

use crate::api::responses::impl_success_response;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoResponse {
    pub name: String,
    pub version: String,
    pub is_healthy: bool,
    pub latest_commitment_id: SlotCommitmentId,
    pub protocol_parameters: Vec<ProtocolParametersResponse>,
    pub base_token: BaseTokenResponse,
}

impl_success_response!(InfoResponse);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullOutputResponse {
    pub output: Output,
    pub output_id_proof: OutputIdProof,
    pub metadata: OutputMetadata,
}

impl_success_response!(FullOutputResponse);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatorResponse {
    /// Account address of the validator.
    pub address: Bech32Address,
    /// The epoch index until which the validator registered to stake.
    pub staking_end_epoch: EpochIndex,
    /// The total stake of the pool, including delegators.
    #[serde(with = "string")]
    pub pool_stake: u64,
    /// The stake of a validator.
    #[serde(with = "string")]
    pub validator_stake: u64,
    /// The fixed cost of the validator, which it receives as part of its Mana rewards.
    #[serde(with = "string")]
    pub fixed_cost: u64,
    /// Shows whether the validator was active recently.
    pub active: bool,
    /// The latest protocol version the validator supported.
    pub latest_supported_protocol_version: u8,
    /// The protocol hash of the latest supported protocol of the validator.
    pub latest_supported_protocol_hash: ProtocolParametersHash,
}

impl_success_response!(ValidatorResponse);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatorsResponse {
    /// List of registered validators ready for the next epoch.
    pub stakers: Vec<ValidatorResponse>,
    /// The number of validators returned per one API request with pagination.
    pub page_size: u32,
    /// The cursor that needs to be provided as cursor query parameter to request the next page. If empty, this was the
    /// last page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

impl_success_response!(ValidatorsResponse);

/// A wrapper struct that allows us to implement [`IntoResponse`](axum::response::IntoResponse) for the foreign
/// responses from [`iota_sdk`](iota_sdk::types::api::core).
#[derive(Clone, Debug, Serialize, derive_more::From)]
pub struct IotaResponse<T: Serialize>(T);

impl<T: Serialize> axum::response::IntoResponse for IotaResponse<T> {
    fn into_response(self) -> axum::response::Response {
        axum::Json(self.0).into_response()
    }
}

/// A wrapper struct that allows us to implement [`IntoResponse`](axum::response::IntoResponse) for the foreign
/// raw responses from [`iota_sdk`](iota_sdk::types::api::core).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IotaRawResponse<T: Serialize> {
    Json(T),
    Raw(Vec<u8>),
}

impl<T: Serialize> axum::response::IntoResponse for IotaRawResponse<T> {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Json(res) => axum::Json(res).into_response(),
            Self::Raw(bytes) => bytes.into_response(),
        }
    }
}
