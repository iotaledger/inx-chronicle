// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "analytics")]
pub mod analytics;

use chronicle::{inx::Inx, tangle::Milestone};

use super::{InxWorker, InxWorkerError};

impl InxWorker {
    pub async fn update_influx<'a>(
        &self,
        milestone: &Milestone<'a, Inx>,
        #[cfg(feature = "analytics")] analytics_info: &mut Option<&mut analytics::AnalyticsInfo>,
        #[cfg(feature = "metrics")] milestone_start_time: std::time::Instant,
    ) -> eyre::Result<()> {
        #[cfg(all(feature = "analytics", feature = "metrics"))]
        let analytics_start_time = std::time::Instant::now();
        #[cfg(feature = "analytics")]
        if let Some(analytics_info) = analytics_info {
            if milestone.at.milestone_index >= analytics_info.synced_index {
                self.update_analytics(milestone, analytics_info).await?;
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
                            milestone_index: milestone.at.milestone_index,
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
                let elapsed = milestone_start_time.elapsed();
                influx_db
                    .metrics()
                    .insert(chronicle::metrics::SyncMetrics {
                        time: chrono::Utc::now(),
                        milestone_index: milestone.at.milestone_index,
                        milestone_time: elapsed.as_millis() as u64,
                        chronicle_version: std::env!("CARGO_PKG_VERSION").to_string(),
                    })
                    .await?;
            }
        }

        Ok(())
    }
}
