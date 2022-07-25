// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod feature;
mod native_token;
mod unlock_condition;

// The different output types
mod alias;
mod basic;
mod foundry;
mod nft;
pub(crate) mod treasury;

use std::str::FromStr;

use bee_block_stardust::output::{self as bee, Rent};
use mongodb::bson::{doc, Bson};
use serde::{Deserialize, Serialize};

pub use self::{
    alias::{AliasId, AliasOutput},
    basic::BasicOutput,
    feature::Feature,
    foundry::{FoundryId, FoundryOutput},
    native_token::{NativeToken, NativeTokenAmount, TokenScheme},
    nft::{NftId, NftOutput},
    treasury::TreasuryOutput,
};
use super::Address;
use crate::types::{ledger::RentStructureBytes, stardust::block::TransactionId};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, derive_more::From)]
pub struct OutputAmount(#[serde(with = "crate::types::util::stringify")] pub u64);

pub type OutputIndex = u16;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputId {
    pub transaction_id: TransactionId,
    pub index: OutputIndex,
}

impl OutputId {
    pub fn to_hex(&self) -> String {
        prefix_hex::encode([self.transaction_id.0.as_ref(), &self.index.to_le_bytes()].concat())
    }
}

impl From<bee::OutputId> for OutputId {
    fn from(value: bee::OutputId) -> Self {
        Self {
            transaction_id: (*value.transaction_id()).into(),
            index: value.index(),
        }
    }
}

impl TryFrom<OutputId> for bee::OutputId {
    type Error = bee_block_stardust::Error;

    fn try_from(value: OutputId) -> Result<Self, Self::Error> {
        bee::OutputId::new(value.transaction_id.into(), value.index)
    }
}

impl FromStr for OutputId {
    type Err = bee_block_stardust::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(bee::OutputId::from_str(s)?.into())
    }
}

impl From<OutputId> for Bson {
    fn from(val: OutputId) -> Self {
        // Unwrap: Cannot fail as type is well defined
        mongodb::bson::to_bson(&val).unwrap()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Output {
    Treasury(TreasuryOutput),
    Basic(BasicOutput),
    Alias(AliasOutput),
    Foundry(FoundryOutput),
    Nft(NftOutput),
}

impl Output {
    pub fn owning_address(&self) -> Option<&Address> {
        Some(match self {
            Self::Treasury(_) => return None,
            Self::Basic(BasicOutput {
                address_unlock_condition,
                ..
            }) => &address_unlock_condition.address,
            Self::Alias(AliasOutput {
                state_controller_address_unlock_condition,
                ..
            }) => &state_controller_address_unlock_condition.address,
            Self::Foundry(FoundryOutput {
                immutable_alias_address_unlock_condition,
                ..
            }) => &immutable_alias_address_unlock_condition.address,
            Self::Nft(NftOutput {
                address_unlock_condition,
                ..
            }) => &address_unlock_condition.address,
        })
    }

    pub fn amount(&self) -> OutputAmount {
        match self {
            Self::Treasury(TreasuryOutput { amount, .. }) => *amount,
            Self::Basic(BasicOutput { amount, .. }) => *amount,
            Self::Alias(AliasOutput { amount, .. }) => *amount,
            Self::Nft(NftOutput { amount, .. }) => *amount,
            Self::Foundry(FoundryOutput { amount, .. }) => *amount,
        }
    }

    pub fn is_trivial_unlock(&self) -> bool {
        match self {
            Self::Treasury(_) => false,
            Self::Basic(BasicOutput {
                storage_deposit_return_unlock_condition,
                timelock_unlock_condition,
                expiration_unlock_condition,
                ..
            }) => {
                storage_deposit_return_unlock_condition.is_none()
                    && timelock_unlock_condition.is_none()
                    && expiration_unlock_condition.is_none()
            }
            Self::Alias(_) => true,
            Self::Nft(NftOutput {
                storage_deposit_return_unlock_condition,
                timelock_unlock_condition,
                expiration_unlock_condition,
                ..
            }) => {
                storage_deposit_return_unlock_condition.is_none()
                    && timelock_unlock_condition.is_none()
                    && expiration_unlock_condition.is_none()
            }
            Self::Foundry(_) => true,
        }
    }

    pub fn rent_structure(&self) -> RentStructureBytes {
        match self {
            output @ (Self::Basic(_) | Self::Alias(_) | Self::Foundry(_) | Self::Nft(_)) => {
                let bee_output =
                    bee::Output::try_from(output.clone()).expect("`Output` has to be convertible to `bee::Output`");

                // The following computations of `data_bytes` and `key_bytes` makec use of the fact that the byte cost
                // computation is a linear combination with respect to the type of the fields and their weight.

                let num_data_bytes = {
                    let config = bee::RentStructureBuilder::new()
                        .byte_cost(1)
                        .data_factor(1)
                        .key_factor(0)
                        .finish();
                    bee_output.rent_cost(&config)
                };

                let num_key_bytes = {
                    let config = bee::RentStructureBuilder::new()
                        .byte_cost(1)
                        .data_factor(0)
                        .key_factor(1)
                        .finish();
                    bee_output.rent_cost(&config)
                };

                RentStructureBytes {
                    num_data_bytes,
                    num_key_bytes,
                }
            }
            // The treasury output does not have an associated byte cost.
            Self::Treasury(_) => RentStructureBytes {
                num_key_bytes: 0,
                num_data_bytes: 0,
            },
        }
    }
}

impl From<&bee::Output> for Output {
    fn from(value: &bee::Output) -> Self {
        match value {
            bee::Output::Treasury(o) => Self::Treasury(o.into()),
            bee::Output::Basic(o) => Self::Basic(o.into()),
            bee::Output::Alias(o) => Self::Alias(o.into()),
            bee::Output::Foundry(o) => Self::Foundry(o.into()),
            bee::Output::Nft(o) => Self::Nft(o.into()),
        }
    }
}

impl TryFrom<Output> for bee::Output {
    type Error = bee_block_stardust::Error;

    fn try_from(value: Output) -> Result<Self, Self::Error> {
        Ok(match value {
            Output::Treasury(o) => bee::Output::Treasury(o.try_into()?),
            Output::Basic(o) => bee::Output::Basic(o.try_into()?),
            Output::Alias(o) => bee::Output::Alias(o.try_into()?),
            Output::Foundry(o) => bee::Output::Foundry(o.try_into()?),
            Output::Nft(o) => bee::Output::Nft(o.try_into()?),
        })
    }
}

impl TryFrom<Output> for bee::dto::OutputDto {
    type Error = bee_block_stardust::Error;

    fn try_from(value: Output) -> Result<Self, Self::Error> {
        let stardust = bee::Output::try_from(value)?;
        Ok(bee::dto::OutputDto::from(&stardust))
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::{alias, basic, foundry, nft, *};

    #[test]
    fn test_output_id_bson() {
        let output_id = OutputId::from(bee_block_stardust::rand::output::rand_output_id());
        let bson = to_bson(&output_id).unwrap();
        from_bson::<OutputId>(bson).unwrap();
    }

    #[test]
    fn test_output_bson() {
        let output = get_test_alias_output();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<Output>(bson).unwrap());

        let output = get_test_basic_output();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<Output>(bson).unwrap());

        let output = get_test_foundry_output();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<Output>(bson).unwrap());

        let output = get_test_nft_output();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<Output>(bson).unwrap());

        let output = Output::from(&bee::Output::Treasury(
            bee_block_stardust::rand::output::rand_treasury_output(),
        ));
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<Output>(bson).unwrap());
    }

    pub(crate) fn get_test_alias_output() -> Output {
        Output::Alias(alias::test::get_test_alias_output())
    }

    pub(crate) fn get_test_basic_output() -> Output {
        Output::Basic(basic::test::get_test_basic_output())
    }

    pub(crate) fn get_test_foundry_output() -> Output {
        Output::Foundry(foundry::test::get_test_foundry_output())
    }

    pub(crate) fn get_test_nft_output() -> Output {
        Output::Nft(nft::test::get_test_nft_output())
    }
}
