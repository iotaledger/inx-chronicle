// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub mod analytics;
pub mod core;
pub mod explorer;
pub mod indexer;

use axum::Router;

pub fn routes() -> Router {
    Router::new()
        .nest("/analytics/v2", analytics::routes())
        .nest("/explorer/v2", explorer::routes())
        .nest("/core/v2", core::routes())
        .nest("/indexer/v1", indexer::routes())
}
