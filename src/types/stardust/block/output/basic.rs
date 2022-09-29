// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;

use bee_block_stardust::output as bee;
use serde::{Deserialize, Serialize};

use super::{
    unlock_condition::{
        AddressUnlockCondition, ExpirationUnlockCondition, StorageDepositReturnUnlockCondition, TimelockUnlockCondition,
    },
    Feature, NativeToken, OutputAmount,
};
use crate::types::context::TryFromWithContext;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BasicOutput {
    pub amount: OutputAmount,
    pub native_tokens: Box<[NativeToken]>,
    pub address_unlock_condition: AddressUnlockCondition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_deposit_return_unlock_condition: Option<StorageDepositReturnUnlockCondition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timelock_unlock_condition: Option<TimelockUnlockCondition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_unlock_condition: Option<ExpirationUnlockCondition>,
    pub features: Box<[Feature]>,
}

impl<T: Borrow<bee::BasicOutput>> From<T> for BasicOutput {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            amount: value.amount().into(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            // Panic: The address unlock condition has to be present.
            address_unlock_condition: value.unlock_conditions().address().unwrap().into(),
            storage_deposit_return_unlock_condition: value.unlock_conditions().storage_deposit_return().map(Into::into),
            timelock_unlock_condition: value.unlock_conditions().timelock().map(Into::into),
            expiration_unlock_condition: value.unlock_conditions().expiration().map(Into::into),
            features: value.features().iter().map(Into::into).collect(),
        }
    }
}

impl TryFromWithContext<BasicOutput> for bee::BasicOutput {
    type Error = bee_block_stardust::Error;

    fn try_from_with_context(
        ctx: &bee_block_stardust::protocol::ProtocolParameters,
        value: BasicOutput,
    ) -> Result<Self, Self::Error> {
        // The order of the conditions is imporant here because unlock conditions have to be sorted by type.
        let unlock_conditions = [
            Some(bee::unlock_condition::AddressUnlockCondition::from(value.address_unlock_condition).into()),
            value
                .storage_deposit_return_unlock_condition
                .map(|x| bee::unlock_condition::StorageDepositReturnUnlockCondition::try_from_with_context(ctx, x))
                .transpose()?
                .map(Into::into),
            value
                .timelock_unlock_condition
                .map(bee::unlock_condition::TimelockUnlockCondition::try_from)
                .transpose()?
                .map(Into::into),
            value
                .expiration_unlock_condition
                .map(bee::unlock_condition::ExpirationUnlockCondition::try_from)
                .transpose()?
                .map(Into::into),
        ];

        Self::build_with_amount(value.amount.0)?
            .with_native_tokens(
                value
                    .native_tokens
                    .into_vec()
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .with_unlock_conditions(unlock_conditions.into_iter().flatten())
            .with_features(
                value
                    .features
                    .into_vec()
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .finish(ctx.token_supply())
    }
}

impl From<BasicOutput> for bee::dto::BasicOutputDto {
    fn from(value: BasicOutput) -> Self {
        let mut unlock_conditions = vec![bee::unlock_condition::dto::UnlockConditionDto::Address(
            value.address_unlock_condition.into(),
        )];
        if let Some(uc) = value.storage_deposit_return_unlock_condition {
            unlock_conditions.push(bee::unlock_condition::dto::UnlockConditionDto::StorageDepositReturn(
                uc.into(),
            ));
        }
        if let Some(uc) = value.timelock_unlock_condition {
            unlock_conditions.push(bee::unlock_condition::dto::UnlockConditionDto::Timelock(uc.into()));
        }
        if let Some(uc) = value.expiration_unlock_condition {
            unlock_conditions.push(bee::unlock_condition::dto::UnlockConditionDto::Expiration(uc.into()));
        }
        Self {
            kind: bee::BasicOutput::KIND,
            amount: value.amount.0.to_string(),
            native_tokens: value.native_tokens.into_vec().into_iter().map(Into::into).collect(),
            unlock_conditions,
            features: value.features.into_vec().into_iter().map(Into::into).collect(),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use bee_block_stardust::rand::output::rand_basic_output;

    use super::*;

    impl BasicOutput {
        /// Generates a random [`BasicOutput`].
        pub fn rand(ctx: &bee_block_stardust::protocol::ProtocolParameters) -> Self {
            rand_basic_output(ctx.token_supply()).into()
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_basic_output_bson() {
        let ctx = bee_block_stardust::protocol::protocol_parameters();
        let output = BasicOutput::rand(&ctx);
        bee::BasicOutput::try_from_with_context(&ctx, output.clone()).unwrap();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<BasicOutput>(bson).unwrap());
    }
}
