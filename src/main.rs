// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![warn(missing_docs)]

use futures::stream::StreamExt;
use inx::{client::InxClient, proto::NoParams};
use tonic::transport::Channel;

async fn milestones(client: &mut InxClient<Channel>, num: usize) {
    let response = client.listen_to_latest_milestone(NoParams {}).await;
    println!("{:#?}", response);
    let stream = response.unwrap().into_inner();

    // stream is infinite - take just `num` elements and then disconnect
    let mut stream = stream.take(num);
    while let Some(item) = stream.next().await {
        println!("\trecived: {:#?}", item.unwrap());
    }
    // stream is droped here and the disconnect info is send to server
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start Hornet with `./hornet --inx.bindAddress=127.0.0.1:50001 --deleteAll --config=config.json`
    let mut client = InxClient::connect("http://127.0.0.1:50001").await?;

    milestones(&mut client, 5).await;

    Ok(())
}
