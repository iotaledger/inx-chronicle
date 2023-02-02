// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::db::{collections::ApplicationStateCollection, MongoDb};
use eyre::bail;

pub mod migrate_20230202;

const LATEST_VERSION: &str = "20230202";

pub async fn migrate(db: &MongoDb) -> eyre::Result<()> {
    loop {
        let last_migration = db
            .collection::<ApplicationStateCollection>()
            .get_application_state()
            .await?
            .map(|s| s.last_migration);
        match last_migration.as_deref() {
            // First migration using the method, so there is no current version
            None => {
                migrate_20230202::migrate(db).await?;
            }
            Some(LATEST_VERSION) => break,
            Some(v) => bail!("cannot migrate version {}", v),
        }
    }
    Ok(())
}
