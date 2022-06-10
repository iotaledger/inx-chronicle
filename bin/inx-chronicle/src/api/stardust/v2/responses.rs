// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub mod bee {
    pub use bee_block_stardust::{
        output::{dto::OutputDto, Output},
        payload::{
            dto::PayloadDto,
            milestone::{
                dto::MilestonePayloadDto,
                option::{dto::MilestoneOptionDto, MilestoneOption},
            },
            MilestonePayload, Payload,
        },
        Block, BlockDto,
    };
    pub use bee_rest_api_stardust::types::{
        dtos::{LedgerInclusionStateDto, ReceiptDto},
        responses::{
            BlockChildrenResponse, BlockMetadataResponse, BlockResponse, MilestoneResponse, OutputMetadataResponse,
            OutputResponse, ReceiptsResponse, TreasuryResponse, UtxoChangesResponse,
        },
    };
}

/// NOTE: this module is only necessary until the PR #1239 is merged into the `shimmer-develop` branch of Bee.
mod temporary {
    macro_rules! create_response_wrapper {
        ($wrapped:ident => $wrapper:ident) => {
            #[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
            #[serde(transparent)]
            pub struct $wrapper(pub super::bee::$wrapped);

            impl From<super::bee::$wrapped> for $wrapper {
                fn from(value: super::bee::$wrapped) -> Self {
                    Self(value)
                }
            }

            crate::api::responses::impl_success_response!($wrapper);

            pub use $wrapper as $wrapped;
        };
    }

    // Response of `GET /api/v2/blocks/<block_id>`
    // and `GET /api/v2/transactions/<transaction_id>/included-block`.
    create_response_wrapper!(BlockResponse => BlockResponseWrapper);

    // Response of `GET /api/v2/blocks/<block_id>/metadata`.
    create_response_wrapper!(BlockMetadataResponse => BlockMetadataResponseWrapper);

    // Response of `GET /api/v2/blocks/<block_id>/children`.
    create_response_wrapper!(BlockChildrenResponse => BlockChildrenResponseWrapper);

    // Response of `GET /api/v2/outputs/<output_id>`.
    create_response_wrapper!(OutputResponse => OutputResponseWrapper);

    // Response of `GET /api/v2/outputs/<output_id>/metadata`.
    create_response_wrapper!(OutputMetadataResponse => OutputMetadataResponseWrapper);

    // Response of `GET /api/v2/receipts`
    // and `GET /api/v2/receipts/<migrated_at>`.
    create_response_wrapper!(ReceiptsResponse => ReceiptsResponseWrapper);

    // Response of `GET /api/v2/treasury`.
    create_response_wrapper!(TreasuryResponse => TreasuryResponseWrapper);

    // Response of `GET /api/v2/milestones/<milestone_id>`
    // and `GET /api/v2/milestones/by-index/<index>.
    create_response_wrapper!(MilestoneResponse => MilestoneResponseWrapper);

    // Response of `GET /api/v2/milestones/<milestone_id>/utxo-changes`
    // and `GET /api/v2/milestones/by-index/<index>/utxo-changes.
    create_response_wrapper!(UtxoChangesResponse => UtxoChangesResponseWrapper);
}

pub use temporary::{
    BlockChildrenResponse, BlockMetadataResponse, BlockResponse, MilestoneResponse, OutputMetadataResponse,
    OutputResponse, ReceiptsResponse, TreasuryResponse, UtxoChangesResponse,
};
