// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use chronicle::db::{
    collections::{ProfileFilter, SystemProfileCollection},
    MongoDb,
};
use futures::TryStreamExt;
use mongodb::{bson::DateTime, error::Error};

use crate::config::ChronicleConfig;

pub struct ProfilerWorker {
    db: MongoDb,
    config: ChronicleConfig,
}

impl ProfilerWorker {
    pub fn new(db: &MongoDb, config: &ChronicleConfig) -> Self {
        Self {
            db: db.clone(),
            config: config.clone(),
        }
    }

    pub async fn run(&self) -> Result<(), Error> {
        let mut ts = DateTime::now();
        let mut profile_filters = Vec::new();
        #[cfg(feature = "api")]
        {
            if self.config.api.enabled {
                profile_filters.push(ProfileFilter {
                    app_name: "API Worker".to_string(),
                    slow_millis: self.config.api.slow_query_millis,
                });
            }
        }
        #[cfg(feature = "inx")]
        {
            if self.config.inx.enabled {
                profile_filters.push(ProfileFilter {
                    app_name: "INX Worker".to_string(),
                    slow_millis: self.config.inx.slow_query_millis,
                });
            }
        }
        if !profile_filters.is_empty() {
            self.db.enable_query_profiler(profile_filters).await?;
            loop {
                let slow_queries = self
                    .db
                    .collection::<SystemProfileCollection>()
                    .get_latest_slow_queries(ts)
                    .await?
                    .try_collect::<Vec<_>>()
                    .await?;
                if let Some(last) = slow_queries.last() {
                    ts = last.timestamp;
                }
                for slow_query in slow_queries {
                    tracing::warn!("Slow query detected: {:#?}", slow_query);
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
        Ok(())
    }

    pub async fn clear(&self) -> Result<(), Error> {
        self.db.disable_query_profiler().await
    }
}
