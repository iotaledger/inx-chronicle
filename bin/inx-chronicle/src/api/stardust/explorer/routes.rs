// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use axum::{extract::Path, routing::get, Extension, Router};
use chronicle::db::{bson::DocExt, model::inclusion_state::LedgerInclusionState, MongoDb};
use futures::TryStreamExt;

use super::responses::TransactionHistoryResponse;
use crate::api::{
    extractors::{Pagination, TimeRange},
    responses::Transfer,
    stardust::{end_milestone, start_milestone},
    ApiError, ApiResult,
};

pub fn routes() -> Router {
    Router::new().nest(
        "/transactions",
        Router::new().route("/history/:address", get(transaction_history)),
    )
}

async fn transaction_history(
    database: Extension<MongoDb>,
    Path(address): Path<String>,
    Pagination { page_size, page }: Pagination,
    TimeRange {
        start_timestamp,
        end_timestamp,
    }: TimeRange,
) -> ApiResult<TransactionHistoryResponse> {
    let start_milestone = start_milestone(&database, start_timestamp).await?;
    let end_milestone = end_milestone(&database, end_timestamp).await?;

    let records = database
        .get_transaction_history(&address, page_size, page, start_milestone, end_milestone)
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    let transactions = records
        .into_iter()
        .map(|mut rec| {
            let mut payload = rec.take_document("message.payload.data")?;
            let spending_transaction = rec.take_document("spending_transaction").ok();
            let output = payload.take_document("essence.data.outputs")?;
            Ok(Transfer {
                transaction_id: payload.get_as_string("transaction_id")?,
                output_index: output.get_as_u16("idx")?,
                is_spending: spending_transaction.is_some(),
                inclusion_state: rec
                    .get_as_u8("inclusion_state")
                    .ok()
                    .map(LedgerInclusionState::try_from)
                    .transpose()?,
                message_id: rec.get_as_string("message_id")?,
                amount: output.get_as_u64("amount")?,
            })
        })
        .collect::<Result<_, ApiError>>()?;

    Ok(TransactionHistoryResponse { transactions, address })
}
