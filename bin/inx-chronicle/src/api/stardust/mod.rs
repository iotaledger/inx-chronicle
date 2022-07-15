// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "analytics")]
pub mod analytics;
#[cfg(feature = "api-core")]
pub mod core;
#[cfg(feature = "api-history")]
pub mod history;

pub mod indexer;

use axum::Router;

pub fn routes() -> Router {
    let mut router = Router::new();

    #[cfg(feature = "analytics")]
    {
        router = router.nest("/analytics/v2", analytics::routes());
    }

    #[cfg(feature = "api-history")]
    {
        router = router.nest("/history/v2", history::routes());
    }

    #[cfg(feature = "api-core")]
    {
        router = router.nest("/core/v2", core::routes());
    }

    // TODO: Chain these above once features are removed
    router = router.nest("/indexer/v1", indexer::routes());

    router
}
