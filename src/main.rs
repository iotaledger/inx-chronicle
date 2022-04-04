// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![warn(missing_docs)]

//! TODO

use actix::{
    Actor,
    System,
};
use chronicle::{
    api::{
        ChronicleAPI,
        ShutdownAPI,
    },
    config::mongo::{
        Credential,
        MongoConfig,
    },
};

fn main() {
    dotenv::dotenv().unwrap();
    env_logger::init();
    let system = System::new();
    let mongo_config =
        MongoConfig::default().with_credential(Credential::default().with_username("root").with_password("pass"));
    let res: anyhow::Result<()> = system.block_on(async {
        let api_addr = ChronicleAPI::new(mongo_config)?.start();
        tokio::signal::ctrl_c().await.ok();
        api_addr.send(ShutdownAPI).await.ok();
        Ok(())
    });
    res.unwrap();
}
