// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "inx")]
mod influx;

use std::collections::{HashMap, HashSet};

use decimal::d128;
use mongodb::{bson::doc, error::Error};
use serde::{Deserialize, Serialize};

use super::{BlockCollection, OutputCollection};
use crate::{
    db::MongoDb,
    types::{
        ledger::{BlockMetadata, LedgerInclusionState, LedgerOutput, LedgerSpent},
        stardust::{
            block::{
                output::{AliasOutput, BasicOutput, FoundryOutput, NftOutput},
                Address, Block, Output, Payload,
            },
            milestone::MilestoneTimestamp,
        },
        tangle::{MilestoneIndex, ProtocolParameters},
    },
};

/// Holds analytics about stardust data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Analytics {
    pub addresses: AddressAnalytics,
    pub outputs: HashMap<String, OutputAnalytics>,
    pub unspent_outputs: HashMap<String, OutputAnalytics>,
    pub storage_deposits: StorageDepositAnalytics,
    pub claimed_tokens: ClaimedTokensAnalytics,
    pub milestone_activity: MilestoneActivityAnalytics,
}

impl Default for Analytics {
    fn default() -> Self {
        Self {
            addresses: Default::default(),
            outputs: [
                (BasicOutput::KIND.to_string(), Default::default()),
                (AliasOutput::KIND.to_string(), Default::default()),
                (NftOutput::KIND.to_string(), Default::default()),
                (FoundryOutput::KIND.to_string(), Default::default()),
            ]
            .into(),
            unspent_outputs: [
                (BasicOutput::KIND.to_string(), Default::default()),
                (AliasOutput::KIND.to_string(), Default::default()),
                (NftOutput::KIND.to_string(), Default::default()),
                (FoundryOutput::KIND.to_string(), Default::default()),
            ]
            .into(),
            storage_deposits: Default::default(),
            claimed_tokens: Default::default(),
            milestone_activity: Default::default(),
        }
    }
}

impl Analytics {
    /// Get a processor to update the analytics with new data.
    pub fn processor(self) -> AnalyticsProcessor {
        AnalyticsProcessor {
            analytics: self,
            addresses: Default::default(),
            sending_addresses: Default::default(),
            receiving_addresses: Default::default(),
            removed_outputs: Default::default(),
            removed_storage_deposits: Default::default(),
        }
    }
}

impl MongoDb {
    /// Gets all analytics for a milestone index, fetching the data from the collections.
    pub async fn get_all_analytics(&self, milestone_index: MilestoneIndex) -> Result<Analytics, Error> {
        let output_collection = self.collection::<OutputCollection>();
        let block_collection = self.collection::<BlockCollection>();
        let addresses = output_collection
            .get_address_analytics(milestone_index, milestone_index + 1)
            .await?;
        let mut outputs = HashMap::new();
        outputs.insert(
            BasicOutput::KIND.to_string(),
            output_collection
                .get_output_analytics::<BasicOutput>(milestone_index, milestone_index + 1)
                .await?,
        );
        outputs.insert(
            AliasOutput::KIND.to_string(),
            output_collection
                .get_output_analytics::<AliasOutput>(milestone_index, milestone_index + 1)
                .await?,
        );
        outputs.insert(
            NftOutput::KIND.to_string(),
            output_collection
                .get_output_analytics::<NftOutput>(milestone_index, milestone_index + 1)
                .await?,
        );
        outputs.insert(
            FoundryOutput::KIND.to_string(),
            output_collection
                .get_output_analytics::<FoundryOutput>(milestone_index, milestone_index + 1)
                .await?,
        );
        let mut unspent_outputs = HashMap::new();
        unspent_outputs.insert(
            BasicOutput::KIND.to_string(),
            output_collection
                .get_unspent_output_analytics::<BasicOutput>(milestone_index)
                .await?,
        );
        unspent_outputs.insert(
            AliasOutput::KIND.to_string(),
            output_collection
                .get_unspent_output_analytics::<AliasOutput>(milestone_index)
                .await?,
        );
        unspent_outputs.insert(
            NftOutput::KIND.to_string(),
            output_collection
                .get_unspent_output_analytics::<NftOutput>(milestone_index)
                .await?,
        );
        unspent_outputs.insert(
            FoundryOutput::KIND.to_string(),
            output_collection
                .get_unspent_output_analytics::<FoundryOutput>(milestone_index)
                .await?,
        );
        let storage_deposits = output_collection.get_storage_deposit_analytics(milestone_index).await?;
        let claimed_tokens = output_collection.get_claimed_token_analytics(milestone_index).await?;
        let milestone_activity = block_collection
            .get_milestone_activity_analytics(milestone_index)
            .await?;
        Ok(Analytics {
            addresses,
            outputs,
            unspent_outputs,
            storage_deposits,
            claimed_tokens,
            milestone_activity,
        })
    }
}

/// A processor for analytics which holds some state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AnalyticsProcessor {
    analytics: Analytics,
    addresses: HashSet<Address>,
    sending_addresses: HashSet<Address>,
    receiving_addresses: HashSet<Address>,
    removed_outputs: HashMap<String, OutputAnalytics>,
    removed_storage_deposits: StorageDepositAnalytics,
}

impl AnalyticsProcessor {
    /// Process a batch of created outputs.
    pub fn process_created_outputs<'a, I>(&mut self, outputs: I)
    where
        I: IntoIterator<Item = &'a LedgerOutput>,
    {
        for output in outputs {
            self.process_output(output, false);
        }
    }

    /// Process a batch of consumed outputs.
    pub fn process_consumed_outputs<'a, I>(&mut self, outputs: I)
    where
        I: IntoIterator<Item = &'a LedgerSpent>,
    {
        for output in outputs {
            self.process_output(&output.output, true);
        }
    }

    fn process_output(&mut self, output: &LedgerOutput, is_spent: bool) {
        if let Some(&address) = output.output.owning_address() {
            self.addresses.insert(address);
            if is_spent {
                self.sending_addresses.insert(address);
            } else {
                self.receiving_addresses.insert(address);
            }
        }
        if !is_spent {
            let output_analytics = self
                .analytics
                .outputs
                .entry(output.output.kind().to_string())
                .or_default();
            output_analytics.count += 1;
            output_analytics.total_value += output.output.amount().0.into();
        }

        let (unspent_output_analytics, storage_deposits) = if is_spent {
            // Spent outputs that were created by the genesis are claimed.
            if output.booked.milestone_index == 0 {
                self.analytics.claimed_tokens.count += 1;
                self.analytics.claimed_tokens.total_amount += output.output.amount().0.into();
            }
            // To workaround spent outputs being processed first, we keep track of a separate set
            // of values which will be subtracted at the end.
            (
                self.removed_outputs
                    .entry(output.output.kind().to_string())
                    .or_default(),
                &mut self.removed_storage_deposits,
            )
        } else {
            (
                self.analytics
                    .unspent_outputs
                    .entry(output.output.kind().to_string())
                    .or_default(),
                &mut self.analytics.storage_deposits,
            )
        };
        unspent_output_analytics.count += 1;
        unspent_output_analytics.total_value += output.output.amount().0.into();
        storage_deposits.total_data_bytes += output.rent_structure.num_data_bytes.into();
        storage_deposits.total_key_bytes += output.rent_structure.num_key_bytes.into();
        match output.output {
            Output::Basic(BasicOutput {
                storage_deposit_return_unlock_condition: Some(uc),
                ..
            })
            | Output::Nft(NftOutput {
                storage_deposit_return_unlock_condition: Some(uc),
                ..
            }) => {
                storage_deposits.storage_deposit_return_count += 1;
                storage_deposits.storage_deposit_return_total_value += uc.amount.0.into();
            }
            _ => (),
        }
    }

    /// Process a batch of blocks.
    pub fn process_blocks<'a, I>(&mut self, blocks: I)
    where
        I: IntoIterator<Item = (&'a Block, &'a BlockMetadata)>,
    {
        for (block, metadata) in blocks {
            self.analytics.milestone_activity.count += 1;
            match &block.payload {
                Some(payload) => match payload {
                    Payload::Transaction(_) => self.analytics.milestone_activity.transaction_count += 1,
                    Payload::Milestone(_) => self.analytics.milestone_activity.milestone_count += 1,
                    Payload::TreasuryTransaction(_) => {
                        self.analytics.milestone_activity.treasury_transaction_count += 1
                    }
                    Payload::TaggedData(_) => self.analytics.milestone_activity.tagged_data_count += 1,
                },
                None => self.analytics.milestone_activity.no_payload_count += 1,
            }
            match &metadata.inclusion_state {
                LedgerInclusionState::Conflicting => self.analytics.milestone_activity.conflicting_count += 1,
                LedgerInclusionState::Included => self.analytics.milestone_activity.confirmed_count += 1,
                LedgerInclusionState::NoTransaction => self.analytics.milestone_activity.no_transaction_count += 1,
            }
        }
    }

    /// Complete processing and return the analytics.
    pub fn finish(mut self) -> Analytics {
        self.analytics.addresses.total_active_addresses = self.addresses.len() as _;
        self.analytics.addresses.receiving_addresses = self.receiving_addresses.len() as _;
        self.analytics.addresses.sending_addresses = self.sending_addresses.len() as _;
        for (key, val) in self.removed_outputs {
            *self.analytics.unspent_outputs.get_mut(&key).unwrap() -= val;
        }
        self.analytics.storage_deposits.storage_deposit_return_count -=
            self.removed_storage_deposits.storage_deposit_return_count;
        self.analytics.storage_deposits.storage_deposit_return_total_value -=
            self.removed_storage_deposits.storage_deposit_return_total_value;
        self.analytics.storage_deposits.total_data_bytes -= self.removed_storage_deposits.total_data_bytes;
        self.analytics.storage_deposits.total_key_bytes -= self.removed_storage_deposits.total_key_bytes;
        self.analytics
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressAnalytics {
    /// The number of addresses used in the time period.
    pub total_active_addresses: u64,
    /// The number of addresses that received tokens in the time period.
    pub receiving_addresses: u64,
    /// The number of addresses that sent tokens in the time period.
    pub sending_addresses: u64,
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct AddressAnalyticsSchema {
    pub milestone_timestamp: MilestoneTimestamp,
    pub milestone_index: MilestoneIndex,
    pub analytics: AddressAnalytics,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OutputAnalytics {
    pub count: u64,
    pub total_value: d128,
}

impl std::ops::Sub<OutputAnalytics> for OutputAnalytics {
    type Output = OutputAnalytics;

    fn sub(self, rhs: OutputAnalytics) -> Self::Output {
        Self {
            count: self.count - rhs.count,
            total_value: self.total_value - rhs.total_value,
        }
    }
}

impl std::ops::SubAssign<OutputAnalytics> for OutputAnalytics {
    fn sub_assign(&mut self, rhs: OutputAnalytics) {
        *self = *self - rhs
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OutputAnalyticsSchema {
    pub milestone_timestamp: MilestoneTimestamp,
    pub milestone_index: MilestoneIndex,
    pub kind: String,
    pub analytics: OutputAnalytics,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StorageDepositAnalytics {
    pub storage_deposit_return_count: u64,
    pub storage_deposit_return_total_value: d128,
    pub total_key_bytes: d128,
    pub total_data_bytes: d128,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StorageDepositAnalyticsSchema {
    pub milestone_timestamp: MilestoneTimestamp,
    pub milestone_index: MilestoneIndex,
    pub analytics: StorageDepositAnalytics,
}

impl StorageDepositAnalytics {
    pub fn total_byte_cost(&self, protocol_params: &ProtocolParameters) -> d128 {
        let rent_structure = protocol_params.rent_structure;
        d128::from(rent_structure.v_byte_cost)
            * ((self.total_data_bytes * d128::from(rent_structure.v_byte_factor_data as u32))
                + (self.total_data_bytes * d128::from(rent_structure.v_byte_factor_data as u32)))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ClaimedTokensAnalytics {
    pub count: u64,
    pub total_amount: d128,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ClaimedTokensAnalyticsSchema {
    pub milestone_timestamp: MilestoneTimestamp,
    pub milestone_index: MilestoneIndex,
    pub analytics: ClaimedTokensAnalytics,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MilestoneActivityAnalytics {
    /// The number of blocks referenced by a milestone.
    pub count: u32,
    /// The number of blocks referenced by a milestone that contain a payload.
    pub transaction_count: u32,
    /// The number of blocks containing a treasury transaction payload.
    pub treasury_transaction_count: u32,
    /// The number of blocks containing a milestone payload.
    pub milestone_count: u32,
    /// The number of blocks containing a tagged data payload.
    pub tagged_data_count: u32,
    /// The number of blocks referenced by a milestone that contain no payload.
    pub no_payload_count: u32,
    /// The number of blocks containing a confirmed transaction.
    pub confirmed_count: u32,
    /// The number of blocks containing a conflicting transaction.
    pub conflicting_count: u32,
    /// The number of blocks containing no transaction.
    pub no_transaction_count: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MilestoneActivityAnalyticsSchema {
    pub milestone_timestamp: MilestoneTimestamp,
    pub milestone_index: MilestoneIndex,
    pub analytics: MilestoneActivityAnalytics,
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use std::collections::HashSet;

    use decimal::d128;
    use rand::Rng;

    use super::Analytics;
    use crate::{
        db::collections::analytics::{
            AddressAnalytics, ClaimedTokensAnalytics, MilestoneActivityAnalytics, OutputAnalytics,
            StorageDepositAnalytics,
        },
        types::{
            ledger::{
                BlockMetadata, ConflictReason, LedgerInclusionState, LedgerOutput, LedgerSpent,
                MilestoneIndexTimestamp, RentStructureBytes, SpentMetadata,
            },
            stardust::block::{
                output::{AliasOutput, BasicOutput, FoundryOutput, NftOutput, OutputId},
                payload::TransactionId,
                Block, BlockId, Output, Payload,
            },
        },
    };

    #[test]
    fn test_analytics_processor() {
        let protocol_params = iota_types::block::protocol::protocol_parameters();

        let outputs = std::iter::repeat_with(|| LedgerOutput {
            output_id: OutputId::rand(),
            rent_structure: RentStructureBytes {
                num_key_bytes: 0,
                num_data_bytes: 100,
            },
            output: Output::rand(&protocol_params),
            block_id: BlockId::rand(),
            booked: MilestoneIndexTimestamp {
                milestone_index: rand::thread_rng().gen_range(0..2).into(),
                milestone_timestamp: 12345.into(),
            },
        })
        .take(100)
        .collect::<Vec<_>>();

        let spent_outputs = outputs
            .iter()
            .filter_map(|output| {
                if rand::random::<bool>() {
                    Some(LedgerSpent {
                        output: output.clone(),
                        spent_metadata: SpentMetadata {
                            transaction_id: TransactionId::rand(),
                            spent: MilestoneIndexTimestamp {
                                milestone_index: output.booked.milestone_index + rand::thread_rng().gen_range(0..2),
                                milestone_timestamp: 23456.into(),
                            },
                        },
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let blocks = (0..100)
            .map(|i| {
                let block = Block::rand(&protocol_params);
                let parents = block.parents.clone();
                (
                    block,
                    BlockMetadata {
                        parents,
                        is_solid: true,
                        should_promote: false,
                        should_reattach: false,
                        referenced_by_milestone_index: 1.into(),
                        milestone_index: 0.into(),
                        inclusion_state: LedgerInclusionState::Included,
                        conflict_reason: ConflictReason::None,
                        white_flag_index: i as u32,
                    },
                )
            })
            .collect::<Vec<_>>();

        let mut analytics = Analytics::default().processor();

        analytics.process_consumed_outputs(&spent_outputs);
        analytics.process_created_outputs(&outputs);
        analytics.process_blocks(blocks.iter().map(|(block, metadata)| (block, metadata)));

        let analytics = analytics.clone().finish();

        assert_eq!(
            analytics.addresses,
            AddressAnalytics {
                total_active_addresses: spent_outputs
                    .iter()
                    .map(|o| &o.output.output)
                    .chain(outputs.iter().map(|o| &o.output))
                    .filter_map(|o| o.owning_address().cloned())
                    .collect::<HashSet<_>>()
                    .len() as _,
                receiving_addresses: outputs
                    .iter()
                    .filter_map(|o| o.output.owning_address().cloned())
                    .collect::<HashSet<_>>()
                    .len() as _,
                sending_addresses: spent_outputs
                    .iter()
                    .filter_map(|o| o.output.output.owning_address().cloned())
                    .collect::<HashSet<_>>()
                    .len() as _,
            }
        );
        assert_eq!(
            analytics.outputs,
            [
                (
                    BasicOutput::KIND.to_string(),
                    output_analytics(BasicOutput::KIND, outputs.iter().map(|o| &o.output))
                ),
                (
                    AliasOutput::KIND.to_string(),
                    output_analytics(AliasOutput::KIND, outputs.iter().map(|o| &o.output))
                ),
                (
                    NftOutput::KIND.to_string(),
                    output_analytics(NftOutput::KIND, outputs.iter().map(|o| &o.output))
                ),
                (
                    FoundryOutput::KIND.to_string(),
                    output_analytics(FoundryOutput::KIND, outputs.iter().map(|o| &o.output))
                ),
            ]
            .into()
        );
        assert_eq!(
            analytics.unspent_outputs,
            [
                (
                    BasicOutput::KIND.to_string(),
                    output_analytics(BasicOutput::KIND, outputs.iter().map(|o| &o.output))
                        - output_analytics(BasicOutput::KIND, spent_outputs.iter().map(|o| &o.output.output))
                ),
                (
                    AliasOutput::KIND.to_string(),
                    output_analytics(AliasOutput::KIND, outputs.iter().map(|o| &o.output))
                        - output_analytics(AliasOutput::KIND, spent_outputs.iter().map(|o| &o.output.output))
                ),
                (
                    NftOutput::KIND.to_string(),
                    output_analytics(NftOutput::KIND, outputs.iter().map(|o| &o.output))
                        - output_analytics(NftOutput::KIND, spent_outputs.iter().map(|o| &o.output.output))
                ),
                (
                    FoundryOutput::KIND.to_string(),
                    output_analytics(FoundryOutput::KIND, outputs.iter().map(|o| &o.output))
                        - output_analytics(FoundryOutput::KIND, spent_outputs.iter().map(|o| &o.output.output))
                ),
            ]
            .into()
        );
        assert_eq!(
            analytics.storage_deposits,
            StorageDepositAnalytics {
                storage_deposit_return_count: outputs
                    .iter()
                    .filter(|o| matches!(
                        o.output,
                        Output::Basic(BasicOutput {
                            storage_deposit_return_unlock_condition: Some(_),
                            ..
                        }) | Output::Nft(NftOutput {
                            storage_deposit_return_unlock_condition: Some(_),
                            ..
                        })
                    ))
                    .count() as u64
                    - spent_outputs
                        .iter()
                        .filter(|o| matches!(
                            o.output.output,
                            Output::Basic(BasicOutput {
                                storage_deposit_return_unlock_condition: Some(_),
                                ..
                            }) | Output::Nft(NftOutput {
                                storage_deposit_return_unlock_condition: Some(_),
                                ..
                            })
                        ))
                        .count() as u64,
                storage_deposit_return_total_value: outputs
                    .iter()
                    .filter_map(|o| match o.output {
                        Output::Basic(BasicOutput {
                            storage_deposit_return_unlock_condition: Some(uc),
                            ..
                        })
                        | Output::Nft(NftOutput {
                            storage_deposit_return_unlock_condition: Some(uc),
                            ..
                        }) => Some(d128::from(uc.amount.0)),
                        _ => None,
                    })
                    .sum::<d128>()
                    - spent_outputs
                        .iter()
                        .filter_map(|o| match o.output.output {
                            Output::Basic(BasicOutput {
                                storage_deposit_return_unlock_condition: Some(uc),
                                ..
                            })
                            | Output::Nft(NftOutput {
                                storage_deposit_return_unlock_condition: Some(uc),
                                ..
                            }) => Some(d128::from(uc.amount.0)),
                            _ => None,
                        })
                        .sum::<d128>(),
                total_key_bytes: outputs
                    .iter()
                    .map(|o| d128::from(o.rent_structure.num_key_bytes))
                    .sum::<d128>()
                    - spent_outputs
                        .iter()
                        .map(|o| d128::from(o.output.rent_structure.num_key_bytes))
                        .sum::<d128>(),
                total_data_bytes: outputs
                    .iter()
                    .map(|o| d128::from(o.rent_structure.num_data_bytes))
                    .sum::<d128>()
                    - spent_outputs
                        .iter()
                        .map(|o| d128::from(o.output.rent_structure.num_data_bytes))
                        .sum::<d128>(),
            }
        );
        assert_eq!(
            analytics.claimed_tokens,
            ClaimedTokensAnalytics {
                count: spent_outputs
                    .iter()
                    .filter(|o| o.output.booked.milestone_index == 0)
                    .count() as _,
                total_amount: spent_outputs
                    .iter()
                    .filter_map(|o| if o.output.booked.milestone_index == 0 {
                        Some(d128::from(o.output.output.amount().0))
                    } else {
                        None
                    })
                    .sum::<d128>()
            }
        );
        assert_eq!(
            analytics.milestone_activity,
            MilestoneActivityAnalytics {
                count: blocks.len() as _,
                transaction_count: blocks
                    .iter()
                    .filter(|(block, _)| matches!(block.payload, Some(Payload::Transaction(_))))
                    .count() as _,
                treasury_transaction_count: blocks
                    .iter()
                    .filter(|(block, _)| matches!(block.payload, Some(Payload::TreasuryTransaction(_))))
                    .count() as _,
                milestone_count: blocks
                    .iter()
                    .filter(|(block, _)| matches!(block.payload, Some(Payload::Milestone(_))))
                    .count() as _,
                tagged_data_count: blocks
                    .iter()
                    .filter(|(block, _)| matches!(block.payload, Some(Payload::TaggedData(_))))
                    .count() as _,
                no_payload_count: blocks.iter().filter(|(block, _)| matches!(block.payload, None)).count() as _,
                confirmed_count: blocks
                    .iter()
                    .filter(|(_, metadata)| matches!(metadata.inclusion_state, LedgerInclusionState::Included))
                    .count() as _,
                conflicting_count: blocks
                    .iter()
                    .filter(|(_, metadata)| matches!(metadata.inclusion_state, LedgerInclusionState::Conflicting))
                    .count() as _,
                no_transaction_count: blocks
                    .iter()
                    .filter(|(_, metadata)| matches!(metadata.inclusion_state, LedgerInclusionState::NoTransaction))
                    .count() as _
            }
        )
    }

    fn output_analytics<'a, I: Iterator<Item = &'a Output> + Clone>(ty: &str, outputs: I) -> OutputAnalytics {
        OutputAnalytics {
            count: outputs.clone().filter(|o| o.kind() == ty).count() as u64,
            total_value: outputs
                .filter_map(|o| {
                    if o.kind() == ty {
                        Some(d128::from(o.amount().0))
                    } else {
                        None
                    }
                })
                .sum::<d128>(),
        }
    }
}
