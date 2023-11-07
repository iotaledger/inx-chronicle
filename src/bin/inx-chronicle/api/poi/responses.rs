// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::BlockDto;
use serde::{Deserialize, Serialize};

use super::merkle_proof::MerkleAuditPathDto;
use crate::api::responses::impl_success_response;

// #[derive(Clone, Debug, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct CreateProofResponse {
//     pub milestone: MilestonePayloadDto,
//     pub block: BlockDto,
//     #[serde(rename = "proof")]
//     pub audit_path: MerkleAuditPathDto,
// }

// impl_success_response!(CreateProofResponse);

// #[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub struct ValidateProofResponse {
//     pub valid: bool,
// }

// impl_success_response!(ValidateProofResponse);
