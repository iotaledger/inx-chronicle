// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::router::Router;

pub mod analytics;
pub mod core;
pub mod history;
pub mod indexer;

pub fn routes() -> Router {
    Router::new()
        .nest("/analytics/v2", analytics::routes())
        .nest("/history/v2", history::routes())
        .nest("/core/v2", core::routes())
        .nest("/indexer/v1", indexer::routes())
}
