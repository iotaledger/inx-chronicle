// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use async_trait::async_trait;
use chronicle::db::{collections::ApplicationStateCollection, MongoDb};
use eyre::bail;

pub mod migrate_20230202;

pub const LATEST_VERSION: &str = migrate_20230202::Migrate::VERSION;

macro_rules! migration {
    ($($mig:expr),*$(,)?) => {
        migration([$(Box::new($mig) as _),*])
    };
}

lazy_static::lazy_static! {
    /// The list of migrations, in order.
    ///
    /// In order to add a new migration, change the `LATEST_VERSION` above and add an entry to this list
    /// in the form `migrate_YYYYMMDD`.
    static ref MIGRATIONS: HashMap<Option<&'static str>, Box<dyn DynMigration>> = migration![
        migrate_20230202::Migrate,
    ];
}

fn migration<const N: usize>(
    migrations: [Box<dyn DynMigration>; N],
) -> HashMap<Option<&'static str>, Box<dyn DynMigration>> {
    let mut map = HashMap::default();
    let mut prev_version = None;
    for migration in migrations {
        let version = migration.version();
        map.insert(prev_version, migration);
        prev_version = Some(version);
    }
    map
}

#[async_trait]
trait Migration {
    const VERSION: &'static str;

    async fn migrate(db: &MongoDb) -> eyre::Result<()>;
}

trait DynMigration: Send + Sync {
    fn version(&self) -> &'static str;

    fn migrate(&self, db: &MongoDb) -> eyre::Result<()>;
}

impl<T: Migration + Send + Sync> DynMigration for T {
    fn version(&self) -> &'static str {
        T::VERSION
    }

    fn migrate(&self, db: &MongoDb) -> eyre::Result<()> {
        tracing::info!("Migrating to version {}", T::VERSION);
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async {
                T::migrate(db).await?;
                db.collection::<ApplicationStateCollection>()
                    .set_last_migration(T::VERSION)
                    .await?;
                Ok(())
            })
        })
    }
}

pub async fn migrate(db: &MongoDb) -> eyre::Result<()> {
    loop {
        let last_migration = db
            .collection::<ApplicationStateCollection>()
            .get_last_migration()
            .await?;
        if matches!(last_migration.as_deref(), Some(LATEST_VERSION)) {
            break;
        }
        match MIGRATIONS.get(&last_migration.as_deref()) {
            Some(migration) => {
                migration.migrate(db)?;
            }
            None => {
                bail!("cannot migrate version {:?}", last_migration);
            }
        }
    }
    Ok(())
}
