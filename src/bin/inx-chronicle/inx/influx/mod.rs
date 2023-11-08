// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "analytics")]
pub mod analytics;

use chronicle::{analytics::AnalyticsContext, inx::Inx, tangle::Slot};

use super::{InxWorker, InxWorkerError};

impl InxWorker {
    pub async fn update_influx<'a>(
        &self,
        slot: &Slot<'a, Inx>,
        #[cfg(feature = "analytics")] analytics_info: Option<&mut analytics::AnalyticsInfo>,
        #[cfg(feature = "metrics")] slot_start_time: std::time::Instant,
    ) -> eyre::Result<()> {
        #[cfg(all(feature = "analytics", feature = "metrics"))]
        let analytics_start_time = std::time::Instant::now();
        #[cfg(feature = "analytics")]
        if let Some(analytics_info) = analytics_info {
            if slot.slot_index() >= analytics_info.synced_index {
                self.update_analytics(slot, analytics_info).await?;
            }
        }
        #[cfg(all(feature = "analytics", feature = "metrics"))]
        {
            if let Some(influx_db) = &self.influx_db {
                if influx_db.config().analytics_enabled {
                    let analytics_elapsed = analytics_start_time.elapsed();
                    influx_db
                        .metrics()
                        .insert(chronicle::metrics::AnalyticsMetrics {
                            time: chrono::Utc::now(),
                            slot_index: slot.slot_index().0,
                            analytics_time: analytics_elapsed.as_millis() as u64,
                            chronicle_version: std::env!("CARGO_PKG_VERSION").to_string(),
                        })
                        .await?;
                }
            }
        }

        #[cfg(feature = "metrics")]
        if let Some(influx_db) = &self.influx_db {
            if influx_db.config().metrics_enabled {
                let elapsed = slot_start_time.elapsed();
                influx_db
                    .metrics()
                    .insert(chronicle::metrics::SyncMetrics {
                        time: chrono::Utc::now(),
                        slot_index: slot.slot_index().0,
                        slot_time: elapsed.as_millis() as u64,
                        chronicle_version: std::env!("CARGO_PKG_VERSION").to_string(),
                    })
                    .await?;
            }
        }

        Ok(())
    }
}
