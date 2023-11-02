// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use core::borrow::Borrow;

use iota_sdk::{
    types::block::{
        output::{self as iota, AccountId, DelegationId},
        slot::EpochIndex,
    },
    utils::serde::string,
};
use serde::{Deserialize, Serialize};

use super::unlock_condition::AddressUnlockConditionDto;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationOutputDto {
    /// Amount of IOTA coins to deposit with this output.
    #[serde(with = "string")]
    pub amount: u64,
    /// Amount of delegated IOTA coins.
    #[serde(with = "string")]
    pub delegated_amount: u64,
    /// Unique identifier of the delegation output.
    pub delegation_id: DelegationId,
    /// Account address of the validator to which this output is delegating.
    pub validator_address: AccountId,
    /// Index of the first epoch for which this output delegates.
    pub start_epoch: EpochIndex,
    /// Index of the last epoch for which this output delegates.
    pub end_epoch: EpochIndex,
    /// The address unlock condition.
    pub address_unlock_condition: AddressUnlockConditionDto,
}

impl DelegationOutputDto {
    /// A `&str` representation of the type.
    pub const KIND: &'static str = "basic";
}

impl<T: Borrow<iota::DelegationOutput>> From<T> for DelegationOutputDto {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            amount: value.amount(),
            delegated_amount: value.delegated_amount(),
            delegation_id: *value.delegation_id(),
            validator_address: value.validator_address().into_account_id(),
            start_epoch: value.start_epoch(),
            end_epoch: value.end_epoch(),
            address_unlock_condition: AddressUnlockConditionDto {
                address: value.address().into(),
            },
        }
    }
}
