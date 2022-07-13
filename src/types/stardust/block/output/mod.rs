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

use bee_block_stardust::output::{
    ByteCost, ByteCostConfigBuilder, {self as bee},
};
use mongodb::bson::{doc, Bson};
use serde::{Deserialize, Serialize};

pub use self::{
    alias::{AliasId, AliasOutput},
    basic::BasicOutput,
    feature::Feature,
    foundry::FoundryOutput,
    native_token::{NativeToken, TokenScheme},
    nft::{NftId, NftOutput},
    treasury::TreasuryOutput,
    unlock_condition::{UnlockCondition, UnlockConditionType},
};
use super::Address;
use crate::types::stardust::block::TransactionId;

pub type OutputAmount = u64;
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

/// Describes the number of bytes in key and data fields for a given [`Output`].
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct RentStructureBytes {
    #[serde(with = "crate::types::util::stringify")]
    pub key_bytes: u64,
    #[serde(with = "crate::types::util::stringify")]
    pub data_bytes: u64,
}

impl Output {
    pub fn owning_addresses(&self) -> Vec<(Address, UnlockConditionType)> {
        match self {
            Self::Treasury(_) => Vec::new(),
            Self::Basic(BasicOutput { unlock_conditions, .. })
            | Self::Alias(AliasOutput { unlock_conditions, .. })
            | Self::Nft(NftOutput { unlock_conditions, .. })
            | Self::Foundry(FoundryOutput { unlock_conditions, .. }) => unlock_conditions
                .iter()
                .filter_map(UnlockCondition::owning_address)
                .collect(),
        }
    }

    /// Computes the amount of tokens in the [`Output`].
    pub fn amount(&self) -> OutputAmount {
        match self {
            Self::Treasury(TreasuryOutput { amount, .. }) => *amount,
            Self::Basic(BasicOutput { amount, .. }) => *amount,
            Self::Alias(AliasOutput { amount, .. }) => *amount,
            Self::Nft(NftOutput { amount, .. }) => *amount,
            Self::Foundry(FoundryOutput { amount, .. }) => *amount,
        }
    }

    /// Computes the [`RentStructure`] for the [`Output`].
    pub fn rent_structure(&self) -> RentStructureBytes {
        match self {
            output @ (Self::Basic(_) | Self::Alias(_) | Self::Foundry(_) | Self::Nft(_)) => {
                let bee_output =
                    bee::Output::try_from(output.clone()).expect("`Output` has to be convertible to `bee::Output`");

                // The following computations of `data_bytes` and `key_bytes` makec use of the fact that the byte cost
                // computation is a linear combination with respect to the type of the fields and their weight.

                let data_bytes = {
                    let config = ByteCostConfigBuilder::new()
                        .byte_cost(1)
                        .data_factor(1)
                        .key_factor(0)
                        .finish();
                    bee_output.byte_cost(&config)
                };

                let key_bytes = {
                    let config = ByteCostConfigBuilder::new()
                        .byte_cost(1)
                        .data_factor(0)
                        .key_factor(1)
                        .finish();
                    bee_output.byte_cost(&config)
                };

                RentStructureBytes { data_bytes, key_bytes }
            }
            // The treasury output does not have an associated byte cost.
            Self::Treasury(_) => RentStructureBytes {
                key_bytes: 0,
                data_bytes: 0,
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
        let output_id = OutputId::from(bee_test::rand::output::rand_output_id());
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

        let output = Output::from(&bee::Output::Treasury(bee_test::rand::output::rand_treasury_output()));
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
