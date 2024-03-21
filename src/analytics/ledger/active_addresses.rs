// Copyright 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use iota_sdk::types::block::{
    address::{AccountAddress, Address, AnchorAddress, Ed25519Address, ImplicitAccountCreationAddress, NftAddress},
    payload::SignedTransactionPayload,
};

use crate::{
    analytics::{Analytics, AnalyticsContext, AnalyticsInterval, IntervalAnalytics},
    db::{mongodb::collections::OutputCollection, MongoDb},
    model::{
        block_metadata::TransactionMetadata,
        ledger::{LedgerOutput, LedgerSpent},
    },
};

#[derive(Debug, Default)]
pub(crate) struct AddressActivityMeasurement {
    pub(crate) ed25519_count: usize,
    pub(crate) account_count: usize,
    pub(crate) nft_count: usize,
    pub(crate) anchor_count: usize,
    pub(crate) implicit_count: usize,
}

/// Computes the number of addresses that were active during a given time interval.
#[allow(missing_docs)]
#[derive(Debug, Default)]
pub(crate) struct AddressActivityAnalytics {
    ed25519_addresses: HashSet<Ed25519Address>,
    account_addresses: HashSet<AccountAddress>,
    nft_addresses: HashSet<NftAddress>,
    anchor_addresses: HashSet<AnchorAddress>,
    implicit_addresses: HashSet<ImplicitAccountCreationAddress>,
}

#[async_trait::async_trait]
impl IntervalAnalytics for AddressActivityMeasurement {
    type Measurement = Self;

    async fn handle_date_range(
        &mut self,
        start_date: time::Date,
        interval: AnalyticsInterval,
        db: &MongoDb,
    ) -> eyre::Result<Self::Measurement> {
        let res = db
            .collection::<OutputCollection>()
            .get_address_activity_count_in_range(start_date, interval.end_date(&start_date))
            .await?;
        Ok(AddressActivityMeasurement {
            ed25519_count: res.ed25519_count,
            account_count: res.account_count,
            nft_count: res.nft_count,
            anchor_count: res.anchor_count,
            implicit_count: res.implicit_count,
        })
    }
}

#[async_trait::async_trait]
impl Analytics for AddressActivityAnalytics {
    type Measurement = AddressActivityMeasurement;

    async fn handle_transaction(
        &mut self,
        _payload: &SignedTransactionPayload,
        _metadata: &TransactionMetadata,
        consumed: &[LedgerSpent],
        created: &[LedgerOutput],
        ctx: &dyn AnalyticsContext,
    ) -> eyre::Result<()> {
        for output in consumed {
            self.add_address(output.output.locked_address(ctx.protocol_parameters()));
        }

        for output in created {
            self.add_address(output.locked_address(ctx.protocol_parameters()));
        }
        Ok(())
    }

    async fn take_measurement(&mut self, _ctx: &dyn AnalyticsContext) -> eyre::Result<Self::Measurement> {
        Ok(AddressActivityMeasurement {
            ed25519_count: std::mem::take(&mut self.ed25519_addresses).len(),
            account_count: std::mem::take(&mut self.account_addresses).len(),
            nft_count: std::mem::take(&mut self.nft_addresses).len(),
            anchor_count: std::mem::take(&mut self.anchor_addresses).len(),
            implicit_count: std::mem::take(&mut self.implicit_addresses).len(),
        })
    }
}

impl AddressActivityAnalytics {
    fn add_address(&mut self, address: Address) {
        match address {
            Address::Ed25519(a) => {
                self.ed25519_addresses.insert(a);
            }
            Address::Account(a) => {
                self.account_addresses.insert(a);
            }
            Address::Nft(a) => {
                self.nft_addresses.insert(a);
            }
            Address::Anchor(a) => {
                self.anchor_addresses.insert(a);
            }
            Address::ImplicitAccountCreation(a) => {
                self.implicit_addresses.insert(a);
            }
            _ => (),
        }
    }
}
