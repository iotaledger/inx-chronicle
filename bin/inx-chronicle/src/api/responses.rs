// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

macro_rules! impl_success_response {
    ($($type:ty),*) => {
        $(
            impl axum::response::IntoResponse for $type {
                fn into_response(self) -> axum::response::Response {
                    axum::Json(self).into_response()
                }
            }
        )*
    };
}

pub(crate) use impl_success_response;
use serde_json::Value;

/// Response of `GET /api/info`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoResponse {
    pub name: String,
    pub version: String,
    pub is_healthy: bool,
}

impl_success_response!(InfoResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaybeSpentOutput {
    pub output: Value,
    pub spending_block_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unlock {
    pub block_id: String,
    pub block: Value,
}

/// An aggregation type that represents the ranges of completed milestones and gaps.
#[cfg(feature = "stardust")]
mod stardust {
    use chronicle::{
        db::collections::SyncData,
        types::{ledger::LedgerInclusionState, tangle::MilestoneIndex},
    };
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub struct SyncDataDto(pub SyncData);

    impl_success_response!(SyncDataDto);

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Record {
        pub id: String,
        pub inclusion_state: Option<LedgerInclusionState>,
        pub milestone_index: Option<MilestoneIndex>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Transfer {
        pub transaction_id: String,
        pub output_index: u16,
        pub is_spending: bool,
        pub inclusion_state: Option<LedgerInclusionState>,
        pub block_id: String,
        pub amount: u64,
    }
}

#[cfg(feature = "stardust")]
pub use stardust::*;
