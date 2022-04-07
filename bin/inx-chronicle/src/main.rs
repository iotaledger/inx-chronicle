// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! TODO

mod broker;
mod cli;
mod config;
mod listener;

use chronicle::db::MongoConfig;
use clap::Parser;
use inx::client::InxClient;

use self::cli::CliArgs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    env_logger::init();

    let cli_args = CliArgs::parse();

    let (db, inx_client) = if let Some(config_path) = cli_args.config {
        let config = config::Config::from_file(config_path)?;
        (config.mongodb.build().await?, config.inx.build().await?)
    } else {
        (
            MongoConfig::new("mongodb://localhost:27017".into()).build().await?,
            InxClient::connect("http://localhost:9029").await?,
        )
    };

    let broker_addr = broker::Broker::register(db);
    let inx_listener = listener::InxListener::new(inx_client, broker_addr);

    tokio::signal::ctrl_c().await?;

    inx_listener.shutdown();

    Ok(())
}
