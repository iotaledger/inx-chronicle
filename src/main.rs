// Copyright 2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![warn(missing_docs)]

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Extension, Router,
};
use futures::stream::StreamExt;
use inx::{client::InxClient, proto::NoParams};
use tokio::sync::RwLock;
use tonic::transport::Channel;

#[derive(Debug)]
struct Error;

async fn root(Extension(handl): Extension<Arc<RwLock<i32>>>) -> Result<impl IntoResponse, Error> {
    let v = (*handl).read().await;
    Ok(format!("Milestone count: {:#?}", v))
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (StatusCode::FORBIDDEN, "Bad bad bad").into_response()
    }
}

async fn milestones(client: &mut InxClient<Channel>, lock: Arc<RwLock<i32>>) {
    let response = client.listen_to_latest_milestone(NoParams {}).await;
    println!("{:#?}", response);
    let mut stream = response.unwrap().into_inner();

    while let Some(item) = stream.next().await {
        println!("\trecived: {:#?}", item.unwrap());
        let mut counter = lock.write().await;
        *counter += 1;
    }
    // stream is droped here and the disconnect info is send to server
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lock = Arc::new(RwLock::new(5));
    let c_lock = lock.clone();

    tokio::spawn(async move {
        // build our application with a route
        let app = Router::new()
            // `GET /` goes to `root`
            .route("/milestones", get(root))
            .layer(Extension(c_lock.clone()));

        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
        println!("listening on {}", addr);
        axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
    });

    println!("Wait for INX");

    tokio::spawn(async move {
        let mut client = InxClient::connect("http://localhost:9029").await.unwrap();

        milestones(&mut client, lock).await;

    });

    loop {}

    Ok(())
}
