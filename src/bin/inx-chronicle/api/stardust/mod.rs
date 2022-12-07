// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::Router;

use super::{ApiState, RegisteredRoutes};

pub mod core;
pub mod explorer;
pub mod indexer;

pub(crate) fn routes(routes: &mut RegisteredRoutes) -> Router<ApiState> {
    Router::new()
        .nest(&routes.register("/explorer/v2"), explorer::routes())
        .nest(&routes.register("/core/v2"), core::routes())
        .nest(&routes.register("/indexer/v1"), indexer::routes())
}
