// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod application_state;
/// Module containing the block collection.
mod block;
/// Module containing the committed slot collection.
mod committed_slot;
/// Module containing the node configuration collection.
mod configuration_update;
/// Module containing the ledger update collection.
mod ledger_update;
/// Module containing the outputs collection.
mod outputs;
/// Module containing the protocol parameters collection.
mod protocol_update;

use std::str::FromStr;

use iota_sdk::types::block::output::{
    AccountOutput, AnchorOutput, BasicOutput, DelegationOutput, FoundryOutput, NftOutput, Output,
};
use thiserror::Error;

pub use self::{
    application_state::{ApplicationStateCollection, MigrationVersion},
    block::BlockCollection,
    committed_slot::CommittedSlotCollection,
    configuration_update::ConfigurationUpdateCollection,
    ledger_update::{LedgerUpdateByAddressRecord, LedgerUpdateBySlotRecord, LedgerUpdateCollection},
    outputs::{
        AddressStat, AliasOutputsQuery, BasicOutputsQuery, DistributionStat, FoundryOutputsQuery, IndexedId,
        NftOutputsQuery, OutputCollection, OutputMetadata, OutputMetadataResult, OutputWithMetadataResult,
        OutputsResult, UtxoChangesResult,
    },
    protocol_update::ProtocolUpdateCollection,
};
// use crate::model::utxo::{AliasOutput, BasicOutput, FoundryOutput, NftOutput, Output};

/// Helper to specify a kind for an output type.
pub trait OutputKindQuery {
    /// Gets the output kind.
    fn kind() -> Option<&'static str>;
}

impl OutputKindQuery for Output {
    fn kind() -> Option<&'static str> {
        None
    }
}

macro_rules! impl_output_kind_query {
    ($t:ty, $kind:literal) => {
        impl OutputKindQuery for $t {
            fn kind() -> Option<&'static str> {
                Some($kind)
            }
        }
    };
}
impl_output_kind_query!(BasicOutput, "basic");
impl_output_kind_query!(AccountOutput, "account");
impl_output_kind_query!(FoundryOutput, "foundry");
impl_output_kind_query!(NftOutput, "nft");
impl_output_kind_query!(DelegationOutput, "delegation");
impl_output_kind_query!(AnchorOutput, "anchor");

#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SortOrder {
    Newest,
    Oldest,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Newest
    }
}

#[derive(Debug, Error)]
#[error("Invalid sort order descriptor. Expected `oldest` or `newest`, found `{0}`")]
#[allow(missing_docs)]
pub struct ParseSortError(String);

impl FromStr for SortOrder {
    type Err = ParseSortError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "oldest" => SortOrder::Oldest,
            "newest" => SortOrder::Newest,
            _ => Err(ParseSortError(s.to_string()))?,
        })
    }
}
