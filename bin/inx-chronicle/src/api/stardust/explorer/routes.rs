// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{extract::Path, routing::get, Extension, Router};
use chronicle::{
    db::{collections::SortOrder, MongoDb},
    types::stardust::block::{Address, OutputId},
};
use futures::TryStreamExt;

use super::{
    extractors::HistoryPagination,
    responses::{TransactionHistoryResponse, Transfer},
};
use crate::api::{ApiError, ApiResult};

pub fn routes() -> Router {
    Router::new().nest(
        "/transactions",
        Router::new().route("/history/:address", get(transaction_history)),
    )
}

async fn transaction_history(
    database: Extension<MongoDb>,
    Path(address): Path<String>,
    HistoryPagination {
        page_size,
        start_milestone_index,
        start_output_id,
    }: HistoryPagination,
) -> ApiResult<TransactionHistoryResponse> {
    let start_output_id = start_output_id
        .as_ref()
        .map(|output_id| OutputId::from_str(output_id).map_err(ApiError::bad_parse))
        .transpose()?;
    let address_dto = Address::from_str(&address).map_err(ApiError::bad_parse)?;

    let records = database
        .get_ledger_updates(
            &address_dto,
            page_size,
            start_milestone_index.map(Into::into),
            start_output_id,
            // TODO: Allow specifying sort in query
            SortOrder::Newest,
        )
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    let transactions = records
        .into_iter()
        .map(|rec| {
            Ok(Transfer {
                transaction_id: rec.output_id.transaction_id.to_hex(),
                output_index: rec.output_id.index,
                is_spent: rec.is_spent,
                milestone_index: rec.milestone_index,
            })
        })
        .collect::<Result<_, ApiError>>()?;

    Ok(TransactionHistoryResponse { transactions, address })
}
