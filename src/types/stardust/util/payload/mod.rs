// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::types::stardust::block::Payload;

pub mod milestone;
pub mod tagged_data;
pub mod transaction;
pub mod treasury_transaction;

pub fn get_test_transaction_payload() -> Payload {
    Payload::Transaction(Box::new(transaction::get_test_transaction_payload()))
}

pub fn get_test_milestone_payload() -> Payload {
    Payload::Milestone(Box::new(milestone::get_test_milestone_payload()))
}

pub fn get_test_treasury_transaction_payload() -> Payload {
    Payload::TreasuryTransaction(Box::new(treasury_transaction::get_test_treasury_transaction_payload()))
}

pub fn get_test_tagged_data_payload() -> Payload {
    Payload::TaggedData(Box::new(tagged_data::get_test_tagged_data_payload()))
}
