// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use axum::{extract::Path, routing::get, Extension, Router};
use chronicle::{
    db::{bson::DocExt, MongoDb},
    types::{ledger::LedgerInclusionState, stardust::message::Address},
};
use futures::TryStreamExt;

use super::responses::TransactionHistoryResponse;
use crate::api::{
    error::ParseError,
    extractors::{Pagination, TimeRange},
    responses::Transfer,
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
    let address_dto = Address::from_str(&address).map_err(ParseError::BeeMessageStardust)?;
    let start_milestone = database
        .find_first_milestone(start_timestamp)
        .await?
        .ok_or(ApiError::NoResults)?;
    let end_milestone = database
        .find_last_milestone(end_timestamp)
        .await?
        .ok_or(ApiError::NoResults)?;

    let records = database
        .get_transaction_history(&address_dto, page_size, page, start_milestone, end_milestone)
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    let transactions = records
        .into_iter()
        .map(|mut rec| {
            let mut payload = rec.take_document("message.payload")?;
            let spending_transaction = rec.take_document("spending_transaction").ok();
            let output = payload.take_document("essence.outputs")?;
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
