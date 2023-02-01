// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::db::MongoDb;
use eyre::bail;

pub mod migrate_1_0_0_beta_31;

pub async fn migrate(version: &str, db: &MongoDb) -> eyre::Result<()> {
    let curr_version = std::env!("CARGO_PKG_VERSION");
    match version {
        "1.0.0-beta.31" => {
            if migrate_1_0_0_beta_31::PREV_VERSION == curr_version {
                migrate_1_0_0_beta_31::migrate(db).await?;
            } else {
                bail!("cannot migrate to {} from {}", version, curr_version);
            }
        }
        _ => bail!("cannot migrate version {}", version),
    }
    Ok(())
}
