// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Various analytics that give insight into the usage of the tangle.

use futures::TryStreamExt;
use iota_sdk::types::{
    api::core::BlockState,
    block::{output::OutputId, protocol::ProtocolParameters, slot::SlotIndex, BlockId, SignedBlock},
};
use thiserror::Error;

use self::{
    influx::PrepareQuery,
    ledger::{
        AddressActivityAnalytics, AddressActivityMeasurement, AddressBalancesAnalytics, BaseTokenActivityMeasurement,
        LedgerOutputMeasurement, LedgerSizeAnalytics, OutputActivityMeasurement, TransactionSizeMeasurement,
        UnclaimedTokenMeasurement, UnlockConditionMeasurement,
    },
    tangle::{BlockActivityMeasurement, ProtocolParamsAnalytics, SlotSizeMeasurement},
};
use crate::{
    db::{
        influxdb::{config::IntervalAnalyticsChoice, AnalyticsChoice, InfluxDb},
        MongoDb,
    },
    inx::{
        ledger::{LedgerOutput, LedgerSpent},
        responses::BlockMetadata,
    },
    tangle::{sources::BlockData, InputSource, Slot},
};

mod influx;
mod ledger;
mod tangle;

/// Provides an API to access basic information used for analytics
#[allow(missing_docs)]
pub trait AnalyticsContext: Send + Sync {
    fn protocol_params(&self) -> &ProtocolParameters;

    fn slot_index(&self) -> SlotIndex;
}

impl<'a, I: InputSource> AnalyticsContext for Slot<'a, I> {
    fn protocol_params(&self) -> &ProtocolParameters {
        &self.protocol_params.parameters
    }

    fn slot_index(&self) -> SlotIndex {
        self.index()
    }
}

/// Defines how analytics are gathered.
pub trait Analytics {
    /// The resulting measurement.
    type Measurement;
    /// Handle a transaction consisting of inputs (consumed [`LedgerSpent`]) and outputs (created [`LedgerOutput`]).
    fn handle_transaction(
        &mut self,
        _consumed: &[LedgerSpent],
        _created: &[LedgerOutput],
        _ctx: &dyn AnalyticsContext,
    ) {
    }
    /// Handle a block.
    fn handle_block(
        &mut self,
        _block_id: BlockId,
        _block: &SignedBlock,
        _metadata: &BlockMetadata,
        _ctx: &dyn AnalyticsContext,
    ) {
    }
    /// Take the measurement from the analytic. This should prepare the analytic for the next slot.
    fn take_measurement(&mut self, ctx: &dyn AnalyticsContext) -> Self::Measurement;
}

// This trait allows using the above implementation dynamically
trait DynAnalytics: Send {
    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], ctx: &dyn AnalyticsContext);
    fn handle_block(
        &mut self,
        block_id: BlockId,
        block: &SignedBlock,
        metadata: &BlockMetadata,
        ctx: &dyn AnalyticsContext,
    );
    fn take_measurement(&mut self, ctx: &dyn AnalyticsContext) -> Box<dyn PrepareQuery>;
}

impl<T: Analytics + Send> DynAnalytics for T
where
    PerSlot<T::Measurement>: 'static + PrepareQuery,
{
    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], ctx: &dyn AnalyticsContext) {
        Analytics::handle_transaction(self, consumed, created, ctx)
    }

    fn handle_block(
        &mut self,
        block_id: BlockId,
        block: &SignedBlock,
        metadata: &BlockMetadata,
        ctx: &dyn AnalyticsContext,
    ) {
        Analytics::handle_block(self, block_id, block, metadata, ctx)
    }

    fn take_measurement(&mut self, ctx: &dyn AnalyticsContext) -> Box<dyn PrepareQuery> {
        Box::new(PerSlot {
            slot_timestamp: ctx.slot_index().to_timestamp(
                ctx.protocol_params().genesis_unix_timestamp(),
                ctx.protocol_params().slot_duration_in_seconds(),
            ),
            slot_index: ctx.slot_index(),
            inner: Analytics::take_measurement(self, ctx),
        }) as _
    }
}

#[async_trait::async_trait]
trait IntervalAnalytics {
    type Measurement;
    async fn handle_date_range(
        &mut self,
        start_date: time::Date,
        interval: AnalyticsInterval,
        db: &MongoDb,
    ) -> eyre::Result<Self::Measurement>;
}

// This trait allows using the above implementation dynamically
#[async_trait::async_trait]
trait DynIntervalAnalytics: Send {
    async fn handle_date_range(
        &mut self,
        start_date: time::Date,
        interval: AnalyticsInterval,
        db: &MongoDb,
    ) -> eyre::Result<Box<dyn PrepareQuery>>;
}

#[async_trait::async_trait]
impl<T: IntervalAnalytics + Send> DynIntervalAnalytics for T
where
    PerInterval<T::Measurement>: 'static + PrepareQuery,
{
    async fn handle_date_range(
        &mut self,
        start_date: time::Date,
        interval: AnalyticsInterval,
        db: &MongoDb,
    ) -> eyre::Result<Box<dyn PrepareQuery>> {
        IntervalAnalytics::handle_date_range(self, start_date, interval, db)
            .await
            .map(|r| {
                Box::new(PerInterval {
                    start_date,
                    interval,
                    inner: r,
                }) as _
            })
    }
}

#[allow(missing_docs)]
pub struct Analytic(Box<dyn DynAnalytics>);

impl Analytic {
    /// Init an analytic from a choice and ledger state.
    pub fn init<'a>(
        choice: &AnalyticsChoice,
        protocol_params: &ProtocolParameters,
        unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput>,
    ) -> Self {
        Self(match choice {
            AnalyticsChoice::AddressBalance => {
                Box::new(AddressBalancesAnalytics::init(unspent_outputs, &protocol_params)) as _
            }
            AnalyticsChoice::BaseTokenActivity => Box::<BaseTokenActivityMeasurement>::default() as _,
            AnalyticsChoice::BlockActivity => Box::<BlockActivityMeasurement>::default() as _,
            AnalyticsChoice::ActiveAddresses => Box::<AddressActivityAnalytics>::default() as _,
            AnalyticsChoice::LedgerOutputs => Box::new(LedgerOutputMeasurement::init(unspent_outputs)) as _,
            AnalyticsChoice::LedgerSize => {
                Box::new(LedgerSizeAnalytics::init(protocol_params.clone(), unspent_outputs)) as _
            }
            AnalyticsChoice::SlotSize => Box::<SlotSizeMeasurement>::default() as _,
            AnalyticsChoice::OutputActivity => Box::<OutputActivityMeasurement>::default() as _,
            AnalyticsChoice::ProtocolParameters => Box::<ProtocolParamsAnalytics>::default() as _,
            AnalyticsChoice::TransactionSizeDistribution => Box::<TransactionSizeMeasurement>::default() as _,
            AnalyticsChoice::UnclaimedTokens => Box::new(UnclaimedTokenMeasurement::init(unspent_outputs)) as _,
            AnalyticsChoice::UnlockConditions => Box::new(UnlockConditionMeasurement::init(unspent_outputs)) as _,
        })
    }
}

impl<T: AsMut<[Analytic]>> Analytics for T {
    type Measurement = Vec<Box<dyn PrepareQuery>>;

    fn handle_block(
        &mut self,
        block_id: BlockId,
        block: &SignedBlock,
        metadata: &BlockMetadata,
        ctx: &dyn AnalyticsContext,
    ) {
        for analytic in self.as_mut().iter_mut() {
            analytic.0.handle_block(block_id, block, metadata, ctx);
        }
    }

    fn handle_transaction(&mut self, consumed: &[LedgerSpent], created: &[LedgerOutput], ctx: &dyn AnalyticsContext) {
        for analytic in self.as_mut().iter_mut() {
            analytic.0.handle_transaction(consumed, created, ctx);
        }
    }

    fn take_measurement(&mut self, ctx: &dyn AnalyticsContext) -> Self::Measurement {
        self.as_mut()
            .iter_mut()
            .map(|analytic| analytic.0.take_measurement(ctx))
            .collect()
    }
}

#[allow(missing_docs)]
pub struct IntervalAnalytic(Box<dyn DynIntervalAnalytics>);

impl IntervalAnalytic {
    /// Init an analytic from a choice and ledger state.
    pub fn init(choice: &IntervalAnalyticsChoice) -> Self {
        Self(match choice {
            IntervalAnalyticsChoice::ActiveAddresses => Box::<AddressActivityMeasurement>::default() as _,
        })
    }
}

#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum AnalyticsError {
    #[error("missing created output ({output_id}) in slot {slot_index}")]
    MissingLedgerOutput { output_id: OutputId, slot_index: SlotIndex },
    #[error("missing consumed output ({output_id}) in slot {slot_index}")]
    MissingLedgerSpent { output_id: OutputId, slot_index: SlotIndex },
}

impl<'a, I: InputSource> Slot<'a, I> {
    /// Update a list of analytics with this slot
    pub async fn update_analytics<A: Analytics + Send>(
        &self,
        analytics: &mut A,
        influxdb: &InfluxDb,
    ) -> eyre::Result<()>
    where
        PerSlot<A::Measurement>: 'static + PrepareQuery,
    {
        let mut block_stream = self.confirmed_block_stream().await?;

        while let Some(block_data) = block_stream.try_next().await? {
            self.handle_block(analytics, &block_data)?;
        }

        influxdb
            .insert_measurement((analytics as &mut dyn DynAnalytics).take_measurement(self))
            .await?;

        Ok(())
    }

    fn handle_block<A: Analytics + Send>(&self, analytics: &mut A, block_data: &BlockData) -> eyre::Result<()> {
        let block = block_data.block.clone().inner_unverified().unwrap();
        if block_data.metadata.block_state == BlockState::Confirmed {
            if let Some(payload) = block
                .block()
                .as_basic_opt()
                .and_then(|b| b.payload())
                .and_then(|p| p.as_signed_transaction_opt())
            {
                let consumed = payload
                    .transaction()
                    .inputs()
                    .iter()
                    .map(|input| input.as_utxo().output_id())
                    .map(|output_id| {
                        Ok(self
                            .ledger_updates()
                            .get_consumed(output_id)
                            .ok_or(AnalyticsError::MissingLedgerSpent {
                                output_id: *output_id,
                                slot_index: block.slot_commitment_id().slot_index(),
                            })?
                            .clone())
                    })
                    .collect::<eyre::Result<Vec<_>>>()?;
                let created = payload
                    .transaction()
                    .outputs()
                    .iter()
                    .enumerate()
                    .map(|(index, _)| {
                        let output_id = payload.transaction().id().into_output_id(index as _).unwrap();
                        Ok(self
                            .ledger_updates()
                            .get_created(&output_id)
                            .ok_or(AnalyticsError::MissingLedgerOutput {
                                output_id,
                                slot_index: block.slot_commitment_id().slot_index(),
                            })?
                            .clone())
                    })
                    .collect::<eyre::Result<Vec<_>>>()?;
                analytics.handle_transaction(&consumed, &created, self)
            }
        }
        analytics.handle_block(block_data.block_id, &block, &block_data.metadata, self);
        Ok(())
    }
}

impl MongoDb {
    /// Update a list of interval analytics with this date.
    pub async fn update_interval_analytics(
        &self,
        analytics: &mut [IntervalAnalytic],
        influxdb: &InfluxDb,
        start: time::Date,
        interval: AnalyticsInterval,
    ) -> eyre::Result<()> {
        for analytic in analytics {
            influxdb
                .insert_measurement(analytic.0.handle_date_range(start, interval, self).await?)
                .await?;
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
#[allow(missing_docs)]
pub enum AnalyticsInterval {
    Day,
    Week,
    Month,
    Year,
}

impl AnalyticsInterval {
    /// Get the duration based on the start date and interval.
    pub fn to_duration(&self, start_date: &time::Date) -> time::Duration {
        match self {
            AnalyticsInterval::Day => time::Duration::days(1),
            AnalyticsInterval::Week => time::Duration::days(7),
            AnalyticsInterval::Month => {
                time::Duration::days(time::util::days_in_year_month(start_date.year(), start_date.month()) as _)
            }
            AnalyticsInterval::Year => time::Duration::days(time::util::days_in_year(start_date.year()) as _),
        }
    }

    /// Get the exclusive end date based on the start date and interval.
    pub fn end_date(&self, start_date: &time::Date) -> time::Date {
        *start_date + self.to_duration(start_date)
    }
}

impl std::fmt::Display for AnalyticsInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AnalyticsInterval::Day => "daily",
                AnalyticsInterval::Week => "weekly",
                AnalyticsInterval::Month => "monthly",
                AnalyticsInterval::Year => "yearly",
            }
        )
    }
}

#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct PerSlot<M> {
    slot_timestamp: u64,
    slot_index: SlotIndex,
    inner: M,
}

#[derive(Clone, Debug)]
#[allow(missing_docs)]
struct PerInterval<M> {
    start_date: time::Date,
    interval: AnalyticsInterval,
    inner: M,
}

#[cfg(test)]
mod test {
    use std::{
        collections::{BTreeMap, HashMap},
        fs::File,
        io::{BufReader, BufWriter},
    };

    use futures::TryStreamExt;
    use iota_sdk::types::block::{
        output::{Output, OutputId},
        payload::signed_transaction::TransactionId,
        protocol::ProtocolParameters,
        slot::{SlotCommitment, SlotCommitmentId, SlotIndex},
        BlockId, SignedBlock,
    };
    use serde::{de::DeserializeOwned, Deserialize, Serialize};

    use super::{
        ledger::{
            AddressActivityAnalytics, AddressActivityMeasurement, AddressBalanceMeasurement, AddressBalancesAnalytics,
            BaseTokenActivityMeasurement, LedgerOutputMeasurement, LedgerSizeAnalytics, LedgerSizeMeasurement,
            OutputActivityMeasurement, TransactionSizeMeasurement, UnclaimedTokenMeasurement,
            UnlockConditionMeasurement,
        },
        tangle::{BlockActivityMeasurement, SlotSizeMeasurement},
        Analytics, AnalyticsContext,
    };
    use crate::{
        inx::{
            ledger::{LedgerOutput, LedgerSpent, LedgerUpdateStore},
            responses::{BlockMetadata, Commitment, NodeConfiguration},
        },
        model::raw::Raw,
        tangle::{
            sources::{memory::InMemoryData, BlockData, SlotData},
            Tangle,
        },
    };

    pub(crate) struct TestContext {
        pub(crate) slot_index: SlotIndex,
        pub(crate) params: ProtocolParameters,
    }

    impl AnalyticsContext for TestContext {
        fn protocol_params(&self) -> &ProtocolParameters {
            &self.params
        }

        fn slot_index(&self) -> SlotIndex {
            self.slot_index
        }
    }

    #[derive(Serialize, Deserialize)]
    struct TestAnalytics {
        #[serde(skip)]
        active_addresses: AddressActivityAnalytics,
        address_balance: AddressBalancesAnalytics,
        #[serde(skip)]
        base_tokens: BaseTokenActivityMeasurement,
        ledger_outputs: LedgerOutputMeasurement,
        ledger_size: LedgerSizeAnalytics,
        #[serde(skip)]
        output_activity: OutputActivityMeasurement,
        #[serde(skip)]
        transaction_size: TransactionSizeMeasurement,
        unclaimed_tokens: UnclaimedTokenMeasurement,
        unlock_conditions: UnlockConditionMeasurement,
        #[serde(skip)]
        block_activity: BlockActivityMeasurement,
        #[serde(skip)]
        slot_size: SlotSizeMeasurement,
    }

    impl TestAnalytics {
        #[allow(dead_code)]
        fn init<'a>(
            protocol_params: ProtocolParameters,
            unspent_outputs: impl IntoIterator<Item = &'a LedgerOutput> + Copy,
        ) -> Self {
            Self {
                active_addresses: Default::default(),
                address_balance: AddressBalancesAnalytics::init(unspent_outputs, &protocol_params),
                base_tokens: Default::default(),
                ledger_outputs: LedgerOutputMeasurement::init(unspent_outputs),
                ledger_size: LedgerSizeAnalytics::init(protocol_params, unspent_outputs),
                output_activity: Default::default(),
                transaction_size: Default::default(),
                unclaimed_tokens: UnclaimedTokenMeasurement::init(unspent_outputs),
                unlock_conditions: UnlockConditionMeasurement::init(unspent_outputs),
                block_activity: Default::default(),
                slot_size: Default::default(),
            }
        }
    }

    #[derive(Debug)]
    struct TestMeasurements {
        active_addresses: AddressActivityMeasurement,
        address_balance: AddressBalanceMeasurement,
        base_tokens: BaseTokenActivityMeasurement,
        ledger_outputs: LedgerOutputMeasurement,
        ledger_size: LedgerSizeMeasurement,
        output_activity: OutputActivityMeasurement,
        transaction_size: TransactionSizeMeasurement,
        unclaimed_tokens: UnclaimedTokenMeasurement,
        unlock_conditions: UnlockConditionMeasurement,
        block_activity: BlockActivityMeasurement,
        slot_size: SlotSizeMeasurement,
    }

    impl Analytics for TestAnalytics {
        type Measurement = TestMeasurements;

        fn handle_block(
            &mut self,
            block_id: BlockId,
            block: &SignedBlock,
            metadata: &BlockMetadata,
            ctx: &dyn AnalyticsContext,
        ) {
            self.active_addresses.handle_block(block_id, block, metadata, ctx);
            self.address_balance.handle_block(block_id, block, metadata, ctx);
            self.base_tokens.handle_block(block_id, block, metadata, ctx);
            self.ledger_outputs.handle_block(block_id, block, metadata, ctx);
            self.ledger_size.handle_block(block_id, block, metadata, ctx);
            self.output_activity.handle_block(block_id, block, metadata, ctx);
            self.transaction_size.handle_block(block_id, block, metadata, ctx);
            self.unclaimed_tokens.handle_block(block_id, block, metadata, ctx);
            self.unlock_conditions.handle_block(block_id, block, metadata, ctx);
            self.block_activity.handle_block(block_id, block, metadata, ctx);
            self.slot_size.handle_block(block_id, block, metadata, ctx);
        }

        fn handle_transaction(
            &mut self,
            consumed: &[LedgerSpent],
            created: &[LedgerOutput],
            ctx: &dyn AnalyticsContext,
        ) {
            self.active_addresses.handle_transaction(consumed, created, ctx);
            self.address_balance.handle_transaction(consumed, created, ctx);
            self.base_tokens.handle_transaction(consumed, created, ctx);
            self.ledger_outputs.handle_transaction(consumed, created, ctx);
            self.ledger_size.handle_transaction(consumed, created, ctx);
            self.output_activity.handle_transaction(consumed, created, ctx);
            self.transaction_size.handle_transaction(consumed, created, ctx);
            self.unclaimed_tokens.handle_transaction(consumed, created, ctx);
            self.unlock_conditions.handle_transaction(consumed, created, ctx);
            self.block_activity.handle_transaction(consumed, created, ctx);
            self.slot_size.handle_transaction(consumed, created, ctx);
        }

        fn take_measurement(&mut self, ctx: &dyn AnalyticsContext) -> Self::Measurement {
            TestMeasurements {
                active_addresses: self.active_addresses.take_measurement(ctx),
                address_balance: self.address_balance.take_measurement(ctx),
                base_tokens: self.base_tokens.take_measurement(ctx),
                ledger_outputs: self.ledger_outputs.take_measurement(ctx),
                ledger_size: self.ledger_size.take_measurement(ctx),
                output_activity: self.output_activity.take_measurement(ctx),
                transaction_size: self.transaction_size.take_measurement(ctx),
                unclaimed_tokens: self.unclaimed_tokens.take_measurement(ctx),
                unlock_conditions: self.unlock_conditions.take_measurement(ctx),
                block_activity: self.block_activity.take_measurement(ctx),
                slot_size: self.slot_size.take_measurement(ctx),
            }
        }
    }

    #[tokio::test]
    async fn test_in_memory_analytics() {
        let analytics_map = gather_in_memory_analytics().await.unwrap();
        let expected: HashMap<SlotIndex, HashMap<String, usize>> =
            ron::de::from_reader(File::open("tests/data/measurements.ron").unwrap()).unwrap();
        for (slot_index, analytics) in analytics_map {
            let expected = &expected[&slot_index];

            macro_rules! assert_expected {
                ($path:expr) => {
                    assert_eq!($path as usize, expected[stringify!($path)]);
                };
            }
            assert_expected!(analytics.active_addresses.count);

            assert_expected!(analytics.address_balance.address_with_balance_count);

            assert_expected!(analytics.base_tokens.booked_amount);
            assert_expected!(analytics.base_tokens.transferred_amount);

            assert_expected!(analytics.ledger_outputs.basic.count);
            assert_expected!(analytics.ledger_outputs.basic.amount);
            assert_expected!(analytics.ledger_outputs.account.count);
            assert_expected!(analytics.ledger_outputs.account.amount);
            assert_expected!(analytics.ledger_outputs.anchor.count);
            assert_expected!(analytics.ledger_outputs.anchor.amount);
            assert_expected!(analytics.ledger_outputs.nft.count);
            assert_expected!(analytics.ledger_outputs.nft.amount);
            assert_expected!(analytics.ledger_outputs.foundry.count);
            assert_expected!(analytics.ledger_outputs.foundry.amount);
            assert_expected!(analytics.ledger_outputs.delegation.count);
            assert_expected!(analytics.ledger_outputs.delegation.amount);

            assert_expected!(analytics.ledger_size.total_key_bytes);
            assert_expected!(analytics.ledger_size.total_data_bytes);
            assert_expected!(analytics.ledger_size.total_storage_deposit_amount);

            assert_expected!(analytics.output_activity.nft.created_count);
            assert_expected!(analytics.output_activity.nft.transferred_count);
            assert_expected!(analytics.output_activity.nft.destroyed_count);
            assert_expected!(analytics.output_activity.account.created_count);
            assert_expected!(analytics.output_activity.account.destroyed_count);
            assert_expected!(analytics.output_activity.anchor.created_count);
            assert_expected!(analytics.output_activity.anchor.governor_changed_count);
            assert_expected!(analytics.output_activity.anchor.state_changed_count);
            assert_expected!(analytics.output_activity.anchor.destroyed_count);
            assert_expected!(analytics.output_activity.foundry.created_count);
            assert_expected!(analytics.output_activity.foundry.transferred_count);
            assert_expected!(analytics.output_activity.foundry.destroyed_count);
            assert_expected!(analytics.output_activity.delegation.created_count);
            assert_expected!(analytics.output_activity.delegation.destroyed_count);

            assert_expected!(analytics.transaction_size.input_buckets.single(1));
            assert_expected!(analytics.transaction_size.input_buckets.single(2));
            assert_expected!(analytics.transaction_size.input_buckets.single(3));
            assert_expected!(analytics.transaction_size.input_buckets.single(4));
            assert_expected!(analytics.transaction_size.input_buckets.single(5));
            assert_expected!(analytics.transaction_size.input_buckets.single(6));
            assert_expected!(analytics.transaction_size.input_buckets.single(7));
            assert_expected!(analytics.transaction_size.input_buckets.small);
            assert_expected!(analytics.transaction_size.input_buckets.medium);
            assert_expected!(analytics.transaction_size.input_buckets.large);
            assert_expected!(analytics.transaction_size.input_buckets.huge);
            assert_expected!(analytics.transaction_size.output_buckets.single(1));
            assert_expected!(analytics.transaction_size.output_buckets.single(2));
            assert_expected!(analytics.transaction_size.output_buckets.single(3));
            assert_expected!(analytics.transaction_size.output_buckets.single(4));
            assert_expected!(analytics.transaction_size.output_buckets.single(5));
            assert_expected!(analytics.transaction_size.output_buckets.single(6));
            assert_expected!(analytics.transaction_size.output_buckets.single(7));
            assert_expected!(analytics.transaction_size.output_buckets.small);
            assert_expected!(analytics.transaction_size.output_buckets.medium);
            assert_expected!(analytics.transaction_size.output_buckets.large);
            assert_expected!(analytics.transaction_size.output_buckets.huge);

            assert_expected!(analytics.unclaimed_tokens.unclaimed_count);
            assert_expected!(analytics.unclaimed_tokens.unclaimed_amount);

            assert_expected!(analytics.unlock_conditions.expiration.count);
            assert_expected!(analytics.unlock_conditions.expiration.amount);
            assert_expected!(analytics.unlock_conditions.timelock.count);
            assert_expected!(analytics.unlock_conditions.timelock.amount);
            assert_expected!(analytics.unlock_conditions.storage_deposit_return.count);
            assert_expected!(analytics.unlock_conditions.storage_deposit_return.amount);
            assert_expected!(analytics.unlock_conditions.storage_deposit_return_inner_amount);

            assert_expected!(analytics.block_activity.no_payload_count);
            assert_expected!(analytics.block_activity.tagged_data_count);
            assert_expected!(analytics.block_activity.transaction_count);
            assert_expected!(analytics.block_activity.candidacy_announcement_count);
            assert_expected!(analytics.block_activity.pending_count);
            assert_expected!(analytics.block_activity.confirmed_count);
            assert_expected!(analytics.block_activity.finalized_count);
            assert_expected!(analytics.block_activity.rejected_count);
            assert_expected!(analytics.block_activity.failed_count);

            assert_expected!(analytics.slot_size.total_tagged_data_payload_bytes);
            assert_expected!(analytics.slot_size.total_transaction_payload_bytes);
            assert_expected!(analytics.slot_size.total_candidacy_announcement_payload_bytes);
            assert_expected!(analytics.slot_size.total_slot_bytes);
        }
    }

    async fn gather_in_memory_analytics() -> eyre::Result<BTreeMap<SlotIndex, TestMeasurements>> {
        let mut analytics = decode_file::<TestAnalytics>("tests/data/ms_17338_analytics_compressed")?;
        let data = get_in_memory_data();
        let mut stream = data.slot_stream(..).await?;
        let mut res = BTreeMap::new();
        while let Some(slot) = stream.try_next().await? {
            let mut blocks_stream = slot.confirmed_block_stream().await?;

            while let Some(block_data) = blocks_stream.try_next().await? {
                slot.handle_block(&mut analytics, &block_data)?;
            }

            res.insert(slot.slot_index(), analytics.take_measurement(&slot));
        }

        Ok(res)
    }

    fn get_in_memory_data() -> Tangle<BTreeMap<SlotIndex, InMemoryData>> {
        #[derive(Deserialize)]
        struct BsonSlotData {
            commitment_id: SlotCommitmentId,
            commitment: Raw<SlotCommitment>,
            node_config: NodeConfiguration,
        }

        impl From<BsonSlotData> for SlotData {
            fn from(value: BsonSlotData) -> Self {
                Self {
                    commitment: Commitment {
                        commitment_id: value.commitment_id,
                        commitment: value.commitment,
                    },
                    node_config: value.node_config,
                }
            }
        }

        #[derive(Deserialize)]
        struct BsonBlockData {
            block_id: BlockId,
            block: Raw<SignedBlock>,
            metadata: BlockMetadata,
        }

        impl From<BsonBlockData> for BlockData {
            fn from(value: BsonBlockData) -> Self {
                Self {
                    block_id: value.block_id,
                    block: value.block,
                    metadata: value.metadata,
                }
            }
        }

        #[derive(Deserialize)]
        pub struct BsonLedgerOutput {
            pub output_id: OutputId,
            pub block_id: BlockId,
            pub slot_booked: SlotIndex,
            pub commitment_id_included: SlotCommitmentId,
            pub output: Raw<Output>,
        }

        impl From<BsonLedgerOutput> for LedgerOutput {
            fn from(value: BsonLedgerOutput) -> Self {
                Self {
                    output_id: value.output_id,
                    block_id: value.block_id,
                    slot_booked: value.slot_booked,
                    commitment_id_included: value.commitment_id_included,
                    output: value.output.inner_unverified().unwrap(),
                }
            }
        }

        #[derive(Deserialize)]
        pub struct BsonLedgerSpent {
            pub output: BsonLedgerOutput,
            pub commitment_id_spent: SlotCommitmentId,
            pub transaction_id_spent: TransactionId,
            pub slot_spent: SlotIndex,
        }

        impl From<BsonLedgerSpent> for LedgerSpent {
            fn from(value: BsonLedgerSpent) -> Self {
                Self {
                    output: value.output.into(),
                    commitment_id_spent: value.commitment_id_spent,
                    transaction_id_spent: value.transaction_id_spent,
                    slot_spent: value.slot_spent,
                }
            }
        }

        #[derive(Deserialize)]
        struct InMemoryBsonData {
            slot_data: BsonSlotData,
            confirmed_blocks: BTreeMap<BlockId, BsonBlockData>,
            created: Vec<BsonLedgerOutput>,
            consumed: Vec<BsonLedgerSpent>,
        }

        impl From<InMemoryBsonData> for InMemoryData {
            fn from(value: InMemoryBsonData) -> Self {
                Self {
                    slot_data: value.slot_data.into(),
                    confirmed_blocks: value
                        .confirmed_blocks
                        .into_iter()
                        .map(|(block_id, data)| (block_id, data.into()))
                        .collect(),
                    ledger_updates: LedgerUpdateStore::init(
                        value.consumed.into_iter().map(Into::into).collect(),
                        value.created.into_iter().map(Into::into).collect(),
                    ),
                }
            }
        }

        let file = File::open("tests/data/in_memory_data.json").unwrap();
        let test_data: mongodb::bson::Bson = serde_json::from_reader(BufReader::new(file)).unwrap();
        Tangle::from(
            mongodb::bson::from_bson::<BTreeMap<String, InMemoryBsonData>>(test_data)
                .unwrap()
                .into_iter()
                .map(|(k, v)| (k.parse().unwrap(), v.into()))
                .collect::<BTreeMap<_, _>>(),
        )
    }

    fn decode_file<T: DeserializeOwned>(file_name: &str) -> eyre::Result<T> {
        let file = File::open(file_name)?;
        let mut decoder = yazi::Decoder::boxed();
        let mut bytes = Vec::new();
        let mut stream = decoder.stream(&mut bytes);
        std::io::copy(&mut BufReader::new(file), &mut stream)?;
        stream.finish().map_err(|e| eyre::eyre!("{:?}", e))?;
        Ok(bincode::deserialize(&bytes)?)
    }

    #[allow(unused)]
    // This is here so that we can compress in the future if needed.
    fn encode_file(value: &impl Serialize, file_name: &str) -> eyre::Result<()> {
        let mut file = BufWriter::new(File::create(file_name)?);
        let mut compressor = yazi::Encoder::boxed();
        compressor.set_level(yazi::CompressionLevel::BestSize);
        let mut stream = compressor.stream(&mut file);
        bincode::serialize_into(&mut stream, value)?;
        let n_bytes = stream.finish().map_err(|e| eyre::eyre!("{:?}", e))?;
        println!("compressed {file_name} to {:.2}mb", n_bytes as f32 / 1000000.0);
        Ok(())
    }
}
