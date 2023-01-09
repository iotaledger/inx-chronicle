use super::*;
use crate::db::influxdb::InfluxDbMeasurement;

impl InfluxDbMeasurement for SyncMetrics {
    const NAME: &'static str = "sync_metrics";
}

#[cfg(feature = "analytics")]
impl InfluxDbMeasurement for AnalyticsMetrics {
    const NAME: &'static str = "analytics_metrics";
}
