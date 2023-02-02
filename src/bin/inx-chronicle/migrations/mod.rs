// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use chronicle::db::{collections::ApplicationStateCollection, MongoDb};
use eyre::bail;

pub mod migrate_20230202;

pub const LATEST_VERSION: &str = migrate_20230202::VERSION;

const MIGRATIONS: [(Option<&str>, fn(&MongoDb) -> eyre::Result<()>); 1] = [
    // Initial migration
    (None, migrate_20230202::migrate),
];

pub async fn migrate(db: &MongoDb) -> eyre::Result<()> {
    let migrations = HashMap::from(MIGRATIONS);
    loop {
        let last_migration = db
            .collection::<ApplicationStateCollection>()
            .get_last_migration()
            .await?;
        if matches!(last_migration.as_deref(), Some(LATEST_VERSION)) {
            break;
        }
        match migrations.get(&last_migration.as_deref()) {
            Some(migrate) => {
                (migrate)(db)?;
            }
            None => {
                bail!("cannot migrate version {:?}", last_migration);
            }
        }
    }
    Ok(())
}
