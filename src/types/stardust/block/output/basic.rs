// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::output as bee;
use serde::{Deserialize, Serialize};

use super::{
    unlock_condition::{
        AddressUnlockCondition, ExpirationUnlockCondition, StorageDepositReturnUnlockCondition, TimelockUnlockCondition,
    },
    Feature, NativeToken, OutputAmount,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BasicOutput {
    pub amount: OutputAmount,
    pub native_tokens: Box<[NativeToken]>,
    pub address_unlock_condition: AddressUnlockCondition,
    pub storage_deposit_return_unlock_condition: Option<StorageDepositReturnUnlockCondition>,
    pub timelock_unlock_condition: Option<TimelockUnlockCondition>,
    pub expiration_unlock_condition: Option<ExpirationUnlockCondition>,
    pub features: Box<[Feature]>,
}

impl From<&bee::BasicOutput> for BasicOutput {
    fn from(value: &bee::BasicOutput) -> Self {
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

impl TryFrom<BasicOutput> for bee::BasicOutput {
    type Error = bee_block_stardust::Error;

    fn try_from(value: BasicOutput) -> Result<Self, Self::Error> {
        // The order of the conditions is imporant here because unlock conditions have to be sorted by type.
        let unlock_conditions = [
            Some(bee::unlock_condition::AddressUnlockCondition::from(value.address_unlock_condition).into()),
            value
                .storage_deposit_return_unlock_condition
                .map(bee::unlock_condition::StorageDepositReturnUnlockCondition::try_from)
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
                Vec::from(value.native_tokens)
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .with_unlock_conditions(unlock_conditions.into_iter().flatten())
            .with_features(
                Vec::from(value.features)
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .finish()
    }
}

#[cfg(test)]
pub(crate) mod test {
    use bee_test::rand::output::unlock_condition::rand_address_unlock_condition;
    use mongodb::bson::{from_bson, to_bson};

    use super::*;
    use crate::types::stardust::block::output::{
        feature::test::{get_test_metadata_block, get_test_sender_block, get_test_tag_block},
        native_token::test::get_test_native_token,
        unlock_condition::test::{
            rand_expiration_unlock_condition, rand_storage_deposit_return_unlock_condition,
            rand_timelock_unlock_condition,
        },
    };

    #[test]
    fn test_basic_output_bson() {
        let output = get_test_basic_output();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<BasicOutput>(bson).unwrap());
    }

    pub(crate) fn get_test_basic_output() -> BasicOutput {
        BasicOutput::from(
            &bee::BasicOutput::build_with_amount(100)
                .unwrap()
                .with_native_tokens(vec![get_test_native_token().try_into().unwrap()])
                .with_unlock_conditions([
                    rand_address_unlock_condition().into(),
                    rand_storage_deposit_return_unlock_condition().into(),
                    rand_timelock_unlock_condition().into(),
                    rand_expiration_unlock_condition().into(),
                ])
                .with_features(vec![
                    get_test_sender_block(bee_test::rand::address::rand_address().into())
                        .try_into()
                        .unwrap(),
                    get_test_metadata_block().try_into().unwrap(),
                    get_test_tag_block().try_into().unwrap(),
                ])
                .finish()
                .unwrap(),
        )
    }
}
