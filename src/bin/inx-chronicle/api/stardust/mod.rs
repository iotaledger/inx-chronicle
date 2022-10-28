// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::router::Router;

pub mod core;
pub mod explorer;
pub mod indexer;

pub fn routes() -> Router {
    Router::new()
        .nest("/explorer/v2", explorer::routes())
        .nest("/core/v2", core::routes())
        .nest("/indexer/v1", indexer::routes())
}
