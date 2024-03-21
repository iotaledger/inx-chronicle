// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::prelude::stream::StreamExt;
use iota_sdk::{
    types::block::{
        output::{
            feature::{NativeTokenFeature, StakingFeature},
            AccountId, Feature,
        },
        payload::SignedTransactionPayload,
        Block,
    },
    utils::serde::string,
    U256,
};
use serde::{Deserialize, Serialize};

use super::CountAndAmount;
use crate::{
    analytics::{Analytics, AnalyticsContext},
    db::{mongodb::collections::AccountCandidacyCollection, MongoDb},
    model::{
        block_metadata::{BlockMetadata, TransactionMetadata},
        ledger::{LedgerOutput, LedgerSpent},
    },
};

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
#[allow(missing_docs)]
pub(crate) struct FeaturesMeasurement {
    pub(crate) native_tokens: NativeTokensCountAndAmount,
    pub(crate) block_issuer: CountAndAmount,
    pub(crate) staking: StakingCountAndAmount,
}

impl FeaturesMeasurement {
    fn wrapping_add(&mut self, rhs: Self) {
        self.native_tokens.wrapping_add(rhs.native_tokens);
        self.block_issuer.wrapping_add(rhs.block_issuer);
        self.staking.wrapping_add(rhs.staking);
    }

    fn wrapping_sub(&mut self, rhs: Self) {
        self.native_tokens.wrapping_sub(rhs.native_tokens);
        self.block_issuer.wrapping_sub(rhs.block_issuer);
        self.staking.wrapping_sub(rhs.staking);
    }

    /// Initialize the analytics by reading the current ledger state.
    pub(crate) async fn init<'a>(
        unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>,
        db: &MongoDb,
    ) -> eyre::Result<Self> {
        let mut measurement = Self::default();
        for output in unspent_outputs {
            if let Some(features) = output.output().features() {
                for feature in features.iter() {
                    match feature {
                        Feature::NativeToken(nt) => measurement.native_tokens.add_native_token(nt),
                        Feature::BlockIssuer(_) => measurement.block_issuer.add_output(output),
                        Feature::Staking(staking) => {
                            measurement
                                .staking
                                .add_staking(
                                    output.output().as_account().account_id_non_null(&output.output_id()),
                                    staking,
                                    db,
                                )
                                .await?
                        }
                        _ => (),
                    }
                }
            }
        }
        Ok(measurement)
    }
}

#[async_trait::async_trait]
impl Analytics for FeaturesMeasurement {
    type Measurement = Self;

    async fn handle_transaction(
        &mut self,
        _payload: &SignedTransactionPayload,
        _metadata: &TransactionMetadata,
        consumed: &[LedgerSpent],
        created: &[LedgerOutput],
        ctx: &dyn AnalyticsContext,
    ) -> eyre::Result<()> {
        let consumed = consumed.iter().map(|input| &input.output).collect::<Vec<_>>();
        let consumed = Self::init(consumed, ctx.database()).await?;
        let created = Self::init(created, ctx.database()).await?;

        self.wrapping_add(created);
        self.wrapping_sub(consumed);

        Ok(())
    }

    async fn handle_block(
        &mut self,
        block: &Block,
        _metadata: &BlockMetadata,
        ctx: &dyn AnalyticsContext,
    ) -> eyre::Result<()> {
        if block
            .body()
            .as_basic_opt()
            .and_then(|body| body.payload())
            .map_or(false, |payload| payload.is_candidacy_announcement())
        {
            ctx.database()
                .collection::<AccountCandidacyCollection>()
                .add_candidacy_slot(&block.issuer_id(), ctx.slot_index())
                .await?;
        }
        Ok(())
    }

    async fn take_measurement(&mut self, ctx: &dyn AnalyticsContext) -> eyre::Result<Self::Measurement> {
        self.staking.candidate_count = ctx
            .database()
            .collection::<AccountCandidacyCollection>()
            .get_candidates(ctx.epoch_index(), ctx.protocol_parameters())
            .await?
            .count()
            .await;
        if ctx.slot_index() == ctx.protocol_parameters().first_slot_of(ctx.epoch_index()) {
            ctx.database()
                .collection::<AccountCandidacyCollection>()
                .clear_expired_data(ctx.epoch_index(), ctx.protocol_parameters())
                .await?;
        }
        Ok(*self)
    }
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct NativeTokensCountAndAmount {
    pub(crate) count: usize,
    #[serde(with = "string")]
    pub(crate) amount: U256,
}

impl NativeTokensCountAndAmount {
    fn wrapping_add(&mut self, rhs: Self) {
        *self = Self {
            count: self.count.wrapping_add(rhs.count),
            amount: self.amount.overflowing_add(rhs.amount).0,
        }
    }

    fn wrapping_sub(&mut self, rhs: Self) {
        *self = Self {
            count: self.count.wrapping_sub(rhs.count),
            amount: self.amount.overflowing_sub(rhs.amount).0,
        }
    }

    fn add_native_token(&mut self, nt: &NativeTokenFeature) {
        self.count += 1;
        self.amount += nt.amount();
    }
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct StakingCountAndAmount {
    pub(crate) count: usize,
    pub(crate) candidate_count: usize,
    #[serde(with = "string")]
    pub(crate) staked_amount: u64,
}

impl StakingCountAndAmount {
    fn wrapping_add(&mut self, rhs: Self) {
        *self = Self {
            count: self.count.wrapping_add(rhs.count),
            candidate_count: self.candidate_count.wrapping_add(rhs.count),
            staked_amount: self.staked_amount.wrapping_add(rhs.staked_amount),
        }
    }

    fn wrapping_sub(&mut self, rhs: Self) {
        *self = Self {
            count: self.count.wrapping_sub(rhs.count),
            candidate_count: self.candidate_count.wrapping_sub(rhs.count),
            staked_amount: self.staked_amount.wrapping_sub(rhs.staked_amount),
        }
    }

    async fn add_staking(&mut self, account_id: AccountId, staking: &StakingFeature, db: &MongoDb) -> eyre::Result<()> {
        self.count += 1;
        self.staked_amount += staking.staked_amount();
        db.collection::<AccountCandidacyCollection>()
            .add_staking_account(&account_id, staking.start_epoch(), staking.end_epoch())
            .await?;
        Ok(())
    }
}
