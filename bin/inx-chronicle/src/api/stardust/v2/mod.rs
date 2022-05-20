// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod responses;
mod routes;

pub use self::routes::routes;

#[cfg(test)]
mod test {
    use axum::Extension;
    use chronicle::db::MongoDb;
    use hyper::{Body, Request};
    use tower::ServiceExt;

    use crate::{api::test::setup_database, config::ChronicleConfig};

    #[ignore]
    #[tokio::test]
    async fn test_node_api() -> Result<(), Box<dyn std::error::Error>> {
        dotenv::dotenv().ok();
        let config = if let Ok(path) = std::env::var("CONFIG_PATH") {
            ChronicleConfig::from_file(path)?
        } else {
            ChronicleConfig::default()
        };
        let db = MongoDb::connect(&config.mongodb.with_suffix("test")).await?;
        let data = setup_database(&db).await?;

        let app = crate::api::routes::routes().layer(Extension(db.clone()));
        for block_id in data.block_ids.iter() {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/v2/blocks/{}", block_id.to_hex()))
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), hyper::StatusCode::OK);

            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/v2/blocks/{}/raw", block_id.to_hex()))
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), hyper::StatusCode::OK);

            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/v2/blocks/{}/metadata", block_id.to_hex()))
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), hyper::StatusCode::OK);

            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/v2/blocks/{}/children", block_id.to_hex()))
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), hyper::StatusCode::OK);
        }
        db.drop().await?;
        Ok(())
    }
}
