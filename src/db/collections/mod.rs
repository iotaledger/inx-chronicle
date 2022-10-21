// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Module containing the Block document model.
mod block;
/// Module containing the LedgerUpdate model.
mod ledger_update;
/// Module containing the Milestone document model.
mod milestone;
/// Module containing the NodeConfiguration document model.
mod node_configuration;
/// Module containing Block outputs.
mod outputs;
/// Module containing the protocol parameters collection.
mod protocol_update;
/// Module containing the treasury model.
mod treasury;
// Module containing the time-series analytics model.
mod analytics;

use std::str::FromStr;

use thiserror::Error;

pub use self::{
    analytics::{Analytics, AnalyticsProcessor},
    block::BlockCollection,
    ledger_update::{LedgerUpdateByAddressRecord, LedgerUpdateByMilestoneRecord, LedgerUpdateCollection},
    milestone::{MilestoneCollection, MilestoneResult, SyncData},
    node_configuration::NodeConfigurationCollection,
    outputs::{
        AddressStat, AliasOutputsQuery, BasicOutputsQuery, DistributionStat, FoundryOutputsQuery, IndexedId,
        NftOutputsQuery, OutputCollection, OutputMetadataResult, OutputWithMetadataResult, OutputsResult,
        UtxoChangesResult,
    },
    protocol_update::ProtocolUpdateCollection,
    treasury::{TreasuryCollection, TreasuryResult},
};
use crate::types::stardust::block::{
    output::{AliasOutput, BasicOutput, FoundryOutput, NftOutput},
    Output,
};

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
    ($t:ty) => {
        impl OutputKindQuery for $t {
            fn kind() -> Option<&'static str> {
                Some(<$t>::KIND)
            }
        }
    };
}
impl_output_kind_query!(BasicOutput);
impl_output_kind_query!(AliasOutput);
impl_output_kind_query!(NftOutput);
impl_output_kind_query!(FoundryOutput);

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
