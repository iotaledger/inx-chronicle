// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "api-analytics")]
pub mod analytics;
#[cfg(feature = "api-core")]
pub mod core;
#[cfg(feature = "api-history")]
pub mod history;

use axum::Router;

pub fn routes() -> Router {
    #[allow(unused_mut)]
    let mut router = Router::new();

    #[cfg(feature = "api-analytics")]
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

    router
}
