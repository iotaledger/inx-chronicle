// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub mod analytics;
pub mod core;
pub mod history;

pub mod indexer;

use axum::Router;
use chronicle::db::collections::SortOrder;

use super::{error::ParseError, ApiError};

pub const DEFAULT_PAGE_SIZE: usize = 100;
pub const DEFAULT_SORT_ORDER: SortOrder = SortOrder::Newest;

pub fn routes() -> Router {
    Router::new()
        .nest("/analytics/v2", analytics::routes())
        .nest("/history/v2", history::routes())
        .nest("/core/v2", core::routes())
        .nest("/indexer/v1", indexer::routes())
}

fn sort_order_from_str(s: String) -> Result<SortOrder, ApiError> {
    match s.as_ref() {
        "asc" | "oldest" => Ok(SortOrder::Oldest),
        "desc" | "newest" => Ok(SortOrder::Newest),
        _ => Err(ParseError::BadSortDescriptor).map_err(ApiError::bad_parse),
    }
}
