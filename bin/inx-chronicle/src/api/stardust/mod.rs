// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "analytics")]
pub mod analytics;
#[cfg(feature = "api-explorer")]
pub mod explorer;
#[cfg(feature = "api-node")]
pub mod core;

use axum::Router;

pub fn routes() -> Router {
    #[allow(unused_mut)]
    let mut router = Router::new();

    #[cfg(feature = "analytics")]
    {
        router = router.nest("/analytics/v2", analytics::routes());
    }

    #[cfg(feature = "api-explorer")]
    {
        router = router.nest("/history/v2", explorer::routes());
    }

    #[cfg(feature = "api-node")]
    {
        router = router.nest("/core/v2", core::routes());
    }

    router
}
