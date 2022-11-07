// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::router::Router;

pub mod core;
pub mod explorer;
pub mod indexer;
#[cfg(feature = "poi")]
pub mod poi;

pub fn routes() -> Router {
    let router = Router::new()
        .nest("/core/v2", core::routes())
        .nest("/explorer/v2", explorer::routes())
        .nest("/indexer/v1", indexer::routes());

    #[cfg(feature = "poi")]
    let router = router.nest("/poi/v1", poi::routes());

    router
}
