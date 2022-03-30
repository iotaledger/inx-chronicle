// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use actix::prelude::*;
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
    let mut mongo_config = MongoConfig::default();
    mongo_config.credential = Credential {
        username: Some("root".to_string()),
        password: Some("pass".to_string()),
        ..Default::default()
    }
    .into();
    system.block_on(async {
        let api_addr = ChronicleAPI::new(mongo_config).start();
        tokio::signal::ctrl_c().await.ok();
        api_addr.send(ShutdownAPI).await.ok();
    });
}
