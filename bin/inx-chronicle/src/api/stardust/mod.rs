// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub mod analytics;
pub mod core;
pub mod history;

use axum::Router;

pub fn routes() -> Router {
    #[allow(unused_mut)]
    let mut router = Router::new();

    #[cfg(feature = "api")]
    {
        router = router
            .nest("/analytics/v2", analytics::routes())
            .nest("/history/v2", history::routes())
            .nest("/core/v2", core::routes());
    }

    router
}
