// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use futures::prelude::stream::TryStreamExt;
use iota_sdk::types::block::{payload::SignedTransactionPayload, protocol::ProtocolParameters, slot::SlotIndex};
use serde::{Deserialize, Serialize};

use crate::{
    analytics::{Analytics, AnalyticsContext},
    db::{
        mongodb::{collections::AddressBalanceCollection, DbError},
        MongoDb, MongoDbCollection,
    },
    model::{
        address::AddressDto,
        block_metadata::TransactionMetadata,
        ledger::{LedgerOutput, LedgerSpent},
    },
};

#[derive(Debug, Default)]
pub(crate) struct AddressBalanceMeasurement {
    pub(crate) ed25519_address_with_balance_count: usize,
    pub(crate) account_address_with_balance_count: usize,
    pub(crate) nft_address_with_balance_count: usize,
    pub(crate) anchor_address_with_balance_count: usize,
    pub(crate) implicit_address_with_balance_count: usize,
    pub(crate) token_distribution: Vec<DistributionStat>,
}

/// Statistics for a particular logarithmic range of balances.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct DistributionStat {
    pub(crate) ed25519_count: usize,
    pub(crate) ed25519_amount: u64,
    pub(crate) account_count: usize,
    pub(crate) account_amount: u64,
    pub(crate) nft_count: usize,
    pub(crate) nft_amount: u64,
    pub(crate) anchor_count: usize,
    pub(crate) anchor_amount: u64,
    pub(crate) implicit_count: usize,
    pub(crate) implicit_amount: u64,
}

/// Computes the number of addresses the currently hold a balance.
#[derive(Serialize, Deserialize, Default)]
pub(crate) struct AddressBalancesAnalytics;

impl AddressBalancesAnalytics {
    /// Initialize the analytics by reading the current ledger state.
    pub(crate) async fn init<'a>(
        protocol_parameters: &ProtocolParameters,
        slot: SlotIndex,
        unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>,
        db: &MongoDb,
    ) -> Result<Self, DbError> {
        db.collection::<AddressBalanceCollection>()
            .collection()
            .drop(None)
            .await?;
        let mut map = HashMap::new();
        for output in unspent_outputs {
            *map.entry(output.locked_address_at(slot, protocol_parameters))
                .or_default() += output.amount();
        }
        for (address, balance) in map {
            db.collection::<AddressBalanceCollection>()
                .add_balance(&address, balance)
                .await?;
        }
        Ok(AddressBalancesAnalytics)
    }
}

#[async_trait::async_trait]
impl Analytics for AddressBalancesAnalytics {
    type Measurement = AddressBalanceMeasurement;

    async fn handle_transaction(
        &mut self,
        _payload: &SignedTransactionPayload,
        _metadata: &TransactionMetadata,
        consumed: &[LedgerSpent],
        created: &[LedgerOutput],
        ctx: &dyn AnalyticsContext,
    ) -> eyre::Result<()> {
        for output in consumed {
            ctx.database()
                .collection::<AddressBalanceCollection>()
                .remove_balance(
                    &output.output.locked_address(ctx.protocol_parameters()),
                    output.amount(),
                )
                .await?;
        }

        for output in created {
            ctx.database()
                .collection::<AddressBalanceCollection>()
                .add_balance(&output.locked_address(ctx.protocol_parameters()), output.amount())
                .await?;
        }
        Ok(())
    }

    async fn take_measurement(&mut self, ctx: &dyn AnalyticsContext) -> eyre::Result<Self::Measurement> {
        let bucket_max = ctx.protocol_parameters().token_supply().ilog10() as usize + 1;

        let mut balances = AddressBalanceMeasurement {
            token_distribution: vec![DistributionStat::default(); bucket_max],
            ..Default::default()
        };
        let mut balances_stream = ctx
            .database()
            .collection::<AddressBalanceCollection>()
            .get_all_balances()
            .await?;
        while let Some(rec) = balances_stream.try_next().await? {
            // Balances are partitioned into ranges defined by: [10^index..10^(index+1)).
            let index = rec.balance.ilog10() as usize;
            match rec.address {
                AddressDto::Ed25519(_) => {
                    balances.ed25519_address_with_balance_count += 1;
                    balances.token_distribution[index].ed25519_count += 1;
                    balances.token_distribution[index].ed25519_amount += rec.balance;
                }
                AddressDto::Account(_) => {
                    balances.account_address_with_balance_count += 1;
                    balances.token_distribution[index].account_count += 1;
                    balances.token_distribution[index].account_amount += rec.balance;
                }
                AddressDto::Nft(_) => {
                    balances.nft_address_with_balance_count += 1;
                    balances.token_distribution[index].nft_count += 1;
                    balances.token_distribution[index].nft_amount += rec.balance;
                }
                AddressDto::Anchor(_) => {
                    balances.anchor_address_with_balance_count += 1;
                    balances.token_distribution[index].anchor_count += 1;
                    balances.token_distribution[index].anchor_amount += rec.balance;
                }
                AddressDto::ImplicitAccountCreation(_) => {
                    balances.implicit_address_with_balance_count += 1;
                    balances.token_distribution[index].implicit_count += 1;
                    balances.token_distribution[index].implicit_amount += rec.balance;
                }
                _ => (),
            }
        }

        Ok(balances)
    }
}
