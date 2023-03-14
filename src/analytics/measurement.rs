// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Influx Measurement implementations

use influxdb::{InfluxDbWriteable, WriteQuery};

use super::{AnalyticsInterval, PerInterval, PerMilestone};
use crate::db::influxdb::InfluxDb;

/// A trait that defines an InfluxDb measurement.
pub(crate) trait Measurement {
    const NAME: &'static str;

    fn add_fields(&self, query: WriteQuery) -> WriteQuery;
}

impl<M: Measurement + ?Sized> Measurement for &M {
    const NAME: &'static str = M::NAME;

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        (*self).add_fields(query)
    }
}

/// A trait that defines an InfluxDb measurement over an interval.
pub(crate) trait IntervalMeasurement: Measurement {
    fn name(interval: AnalyticsInterval) -> String;
}

trait AddFields<M: Measurement> {
    fn add_fields(self, measurement: &M) -> Self;
}

impl<M: Measurement> AddFields<M> for WriteQuery {
    fn add_fields(self, measurement: &M) -> Self {
        measurement.add_fields(self)
    }
}

pub trait PrepareQuery: Send + Sync {
    fn prepare_query(&self) -> Vec<WriteQuery>;
}

impl<T: PrepareQuery + ?Sized> PrepareQuery for Box<T> {
    fn prepare_query(&self) -> Vec<WriteQuery> {
        (**self).prepare_query()
    }
}

impl<M: Send + Sync> PrepareQuery for PerMilestone<M>
where
    M: Measurement,
{
    fn prepare_query(&self) -> Vec<WriteQuery> {
        vec![
            influxdb::Timestamp::from(self.at.milestone_timestamp)
                .into_query(M::NAME)
                .add_field("milestone_index", self.at.milestone_index)
                .add_fields(&self.inner),
        ]
    }
}

impl<T: PrepareQuery> PrepareQuery for PerMilestone<Vec<T>> {
    fn prepare_query(&self) -> Vec<WriteQuery> {
        self.inner.iter().flat_map(|inner| inner.prepare_query()).collect()
    }
}

impl<M: Send + Sync> PrepareQuery for PerMilestone<Option<M>>
where
    M: Measurement,
{
    fn prepare_query(&self) -> Vec<WriteQuery> {
        self.inner
            .iter()
            .flat_map(|inner| PerMilestone { at: self.at, inner }.prepare_query())
            .collect()
    }
}

impl<M: Send + Sync> PrepareQuery for PerInterval<M>
where
    M: IntervalMeasurement,
{
    fn prepare_query(&self) -> Vec<WriteQuery> {
        vec![
            influxdb::Timestamp::Seconds(self.start_date.midnight().assume_utc().unix_timestamp() as _)
                .into_query(M::name(self.interval))
                .add_fields(&self.inner),
        ]
    }
}

impl InfluxDb {
    /// Writes a [`Measurement`] to the InfluxDB database.
    pub(super) async fn insert_measurement(&self, measurement: impl PrepareQuery) -> Result<(), influxdb::Error> {
        self.analytics().query(measurement.prepare_query()).await?;
        Ok(())
    }
}
