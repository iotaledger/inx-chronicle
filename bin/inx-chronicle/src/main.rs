// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! TODO

mod cli;
mod config;

use chronicle::db::{MongoConfig, MongoDatabase};
use clap::Parser;
use futures::StreamExt;
use inx::{
    client::InxClient,
    proto::{MessageFilter, NoParams},
    Channel,
};
use log::{debug, error, info};

use self::cli::CliArgs;

async fn subscribe_messages(client: &mut InxClient<Channel>, db: &MongoDatabase) {
    let response = client.listen_to_messages(MessageFilter {}).await;
    info!("Subscribed to `ListenToMessages`.");
    let mut stream = response.unwrap().into_inner();

    while let Some(item) = stream.next().await {
        let msg = item.unwrap();
        debug!("INX received message.");
        if let Err(err) = db.insert_message_raw(msg).await {
            error!("Failed to insert raw message: {err}");
        }
    }
}

async fn subscribe_latest_milestone(client: &mut InxClient<Channel>, db: &MongoDatabase) {
    let response = client.listen_to_latest_milestone(NoParams {}).await;
    info!("Subscribed to `ListenToLatestMilestone`.");
    let mut stream = response.unwrap().into_inner();

    while let Some(item) = stream.next().await {
        let milestone = item.unwrap();
        debug!("INX received latest milestone: {}", milestone.milestone_index);
        if let Err(err) = db.insert_milestone(milestone).await {
            error!("Failed to insert milestone: {err}");
        }
    }
}

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

    {
        let mut inx = inx_client.clone();
        let db = db.clone();
        tokio::spawn(async move {
            subscribe_messages(&mut inx, &db).await;
        });
    }

    {
        let mut inx = inx_client.clone();
        let db = db.clone();
        tokio::spawn(async move {
            subscribe_latest_milestone(&mut inx, &db).await;
        });
    }

    tokio::signal::ctrl_c().await?;

    Ok(())
}
