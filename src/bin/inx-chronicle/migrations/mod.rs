// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use chronicle::db::{collections::ApplicationStateCollection, MongoDb};
use eyre::bail;

pub mod migrate_20230202;

type MigrateFn = fn(&MongoDb) -> eyre::Result<()>;

pub const LATEST_VERSION: &str = migrate_20230202::VERSION;

/// The list of migrations, specified as key-value tuples.
/// - `key`: The `last_migration` value that can be migrated to the next iteration.
/// - `value`: The migration fn pointer that migrates from `last_migration == key` to the next iteration.
///
/// In order to add a new migration, change the `LATEST_VERSION` above and add an entry to this list.
///
/// ## Example
/// ```
/// (Some(migrate_YYYYMMDD::VERSION), migrate_YYYYMMDD::migrate)
/// ```
const MIGRATIONS: [(Option<&str>, MigrateFn); 1] = [
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
