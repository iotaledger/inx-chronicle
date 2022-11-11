// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::block::{payload::dto::MilestonePayloadDto, BlockDto};
use serde::{Deserialize, Serialize};

use super::proof::ProofDto;
use crate::api::responses::impl_success_response;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProofResponse {
    pub milestone: MilestonePayloadDto,
    pub block: BlockDto,
    pub proof: ProofDto,
}

impl_success_response!(CreateProofResponse);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateProofResponse {
    pub valid: bool,
}

impl_success_response!(ValidateProofResponse);
