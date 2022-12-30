// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::Router;

use super::ApiWorker;

pub mod core;
pub mod explorer;
pub mod indexer;
#[cfg(feature = "poi")]
pub mod poi;

#[allow(clippy::let_and_return)]
pub fn routes() -> Router<ApiWorker> {
    #[allow(unused_mut)]
    let mut router = Router::new()
        .nest("/core/v2", core::routes())
        .nest("/explorer/v2", explorer::routes())
        .nest("/indexer/v1", indexer::routes());

    #[cfg(feature = "poi")]
    {
        router = router.nest("/poi/v1", poi::routes());
    }

    router
}
