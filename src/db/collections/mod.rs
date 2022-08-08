// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Module containing the Block document model.
mod block;
/// Module containing the LedgerUpdate model.
mod ledger_update;
/// Module containing the Milestone document model.
mod milestone;
/// Module containing Block outputs.
mod outputs;
/// Module containing the protocol parameters collection.
mod protocol_update;
/// Module containing the treasury model.
mod treasury;

pub use self::{
    ledger_update::{LedgerUpdateByAddressRecord, LedgerUpdateByMilestoneRecord, ParseSortError, SortOrder},
    milestone::SyncData,
    outputs::{
        AliasOutputsQuery, BasicOutputsQuery, FoundryOutputsQuery, IndexedId, NftOutputsQuery, OutputMetadataResult,
        OutputWithMetadataResult, OutputsResult, UtxoChangesResult,
    },
    treasury::TreasuryResult,
};
use crate::types::stardust::block::{
    AliasOutput, BasicOutput, FoundryOutput, MilestonePayload, NftOutput, TaggedDataPayload, TransactionPayload,
    TreasuryTransactionPayload,
};

/// Helper to specify a kind for an output type.
pub trait OutputKind {
    /// Gets the output kind.
    fn kind() -> Option<&'static str> {
        None
    }
}

impl OutputKind for () {}

macro_rules! impl_output_kind {
    ($t:ty, $s:literal) => {
        impl OutputKind for $t {
            fn kind() -> Option<&'static str> {
                Some($s)
            }
        }
    };
}
impl_output_kind!(BasicOutput, "basic");
impl_output_kind!(AliasOutput, "alias");
impl_output_kind!(NftOutput, "nft");
impl_output_kind!(FoundryOutput, "foundry");

/// Helper to specify a kind for a block payload type.
pub trait PayloadKind {
    /// Gets the payload kind.
    fn kind() -> Option<&'static str> {
        None
    }
}

impl PayloadKind for () {}

macro_rules! impl_payload_kind {
    ($t:ty, $s:literal) => {
        impl PayloadKind for $t {
            fn kind() -> Option<&'static str> {
                Some($s)
            }
        }
    };
}
impl_payload_kind!(TransactionPayload, "transaction");
impl_payload_kind!(MilestonePayload, "milestone");
impl_payload_kind!(TaggedDataPayload, "tagged_data");
impl_payload_kind!(TreasuryTransactionPayload, "treasury_transaction");
