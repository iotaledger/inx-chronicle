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
