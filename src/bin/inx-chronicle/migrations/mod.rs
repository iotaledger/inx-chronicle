// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use async_trait::async_trait;
use chronicle::db::{
    collections::{ApplicationStateCollection, MigrationVersion},
    MongoDb,
};
use eyre::bail;
use tracing::info;

pub mod migrate_0;

pub type LatestMigration = migrate_0::Migrate;

/// The list of migrations, in order.
const MIGRATIONS: &[&'static dyn DynMigration] = &[
    // In order to add a new migration, change the `LatestMigration` type above and add an entry at the bottom of this
    // list.
    &migrate_0::Migrate,
];

fn build_migrations(migrations: &[&'static dyn DynMigration]) -> HashMap<Option<usize>, &'static dyn DynMigration> {
    let mut map = HashMap::default();
    let mut prev_version = None;
    for &migration in migrations {
        let version = migration.version().id;
        map.insert(prev_version, migration);
        prev_version = Some(version);
    }
    map
}

#[async_trait]
pub trait Migration {
    const ID: usize;
    const APP_VERSION: &'static str;
    const DATE: time::Date;

    fn version() -> MigrationVersion {
        MigrationVersion {
            id: Self::ID,
            app_version: Self::APP_VERSION.to_string(),
            date: Self::DATE,
        }
    }

    async fn migrate(db: &MongoDb) -> eyre::Result<()>;
}

#[async_trait]
trait DynMigration: Send + Sync {
    fn version(&self) -> MigrationVersion;

    async fn migrate(&self, db: &MongoDb) -> eyre::Result<()>;
}

#[async_trait]
impl<T: Migration + Send + Sync> DynMigration for T {
    fn version(&self) -> MigrationVersion {
        T::version()
    }

    async fn migrate(&self, db: &MongoDb) -> eyre::Result<()> {
        let version = self.version();
        T::migrate(db).await?;
        db.collection::<ApplicationStateCollection>()
            .set_last_migration(version)
            .await?;
        Ok(())
    }
}

pub async fn check_migration_version(db: &MongoDb) -> eyre::Result<()> {
    let latest_version = <LatestMigration as Migration>::version();
    match db
        .collection::<ApplicationStateCollection>()
        .get_last_migration()
        .await?
    {
        None => {
            // Check if this is the first application run
            if db
                .collection::<ApplicationStateCollection>()
                .get_starting_index()
                .await?
                .is_none()
            {
                info!("Setting migration version to {}", latest_version);
                db.collection::<ApplicationStateCollection>()
                    .set_last_migration(latest_version)
                    .await?;
            } else {
                migrate(db).await?;
            }
        }
        Some(v) if v == latest_version => (),
        Some(_) => {
            migrate(db).await?;
        }
    }
    Ok(())
}

pub async fn migrate(db: &MongoDb) -> eyre::Result<()> {
    let migrations = build_migrations(MIGRATIONS);

    loop {
        let last_migration = db
            .collection::<ApplicationStateCollection>()
            .get_last_migration()
            .await?
            .map(|mig| mig.id);
        if matches!(last_migration, Some(v) if v == LatestMigration::ID) {
            break;
        }
        match migrations.get(&last_migration) {
            Some(migration) => {
                migration.migrate(db).await?;
            }
            None => {
                bail!(
                    "cannot migrate version {:?}, database is in invalid state",
                    last_migration
                );
            }
        }
    }
    Ok(())
}
