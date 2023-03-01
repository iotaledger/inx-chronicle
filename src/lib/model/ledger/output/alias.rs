// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`AliasOutput`].

use std::{borrow::Borrow, str::FromStr};

use iota_types::block::output as iota;
use mongodb::bson::{spec::BinarySubtype, Binary, Bson};
use serde::{Deserialize, Serialize};

use super::{
    feature::Feature,
    native_token::NativeToken,
    unlock_condition::{GovernorAddressUnlockCondition, StateControllerAddressUnlockCondition},
    OutputId, TokenAmount,
};
use crate::model::{serde::bytify, tangle::TryFromWithContext};

/// Uniquely identifies an Alias.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct AliasId(#[serde(with = "bytify")] pub [u8; Self::LENGTH]);

impl AliasId {
    const LENGTH: usize = iota::AliasId::LENGTH;

    /// The [`AliasId`] is derived from the [`OutputId`](super::OutputId) that created the alias.
    pub fn from_output_id_str(s: &str) -> Result<Self, iota_types::block::Error> {
        Ok(iota::AliasId::from(&iota::OutputId::from_str(s)?).into())
    }

    /// Get an implicit (zeroed) alias ID, for new alias outputs.
    pub fn implicit() -> Self {
        Self([0; Self::LENGTH])
    }
}

impl From<iota::AliasId> for AliasId {
    fn from(value: iota::AliasId) -> Self {
        Self(*value)
    }
}

impl From<AliasId> for iota::AliasId {
    fn from(value: AliasId) -> Self {
        iota::AliasId::new(value.0)
    }
}

impl From<AliasId> for iota::dto::AliasIdDto {
    fn from(value: AliasId) -> Self {
        Into::into(&iota::AliasId::from(value))
    }
}

impl From<OutputId> for AliasId {
    fn from(value: OutputId) -> Self {
        Self(value.hash())
    }
}

impl FromStr for AliasId {
    type Err = iota_types::block::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(iota::AliasId::from_str(s)?.into())
    }
}

impl From<AliasId> for Bson {
    fn from(val: AliasId) -> Self {
        Binary {
            subtype: BinarySubtype::Generic,
            bytes: val.0.to_vec(),
        }
        .into()
    }
}

/// Represents an alias in the UTXO model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AliasOutput {
    /// The output amount.
    pub amount: TokenAmount,
    /// The list of [`NativeTokens`](NativeToken).
    pub native_tokens: Box<[NativeToken]>,
    /// The associated id of the alias.
    pub alias_id: AliasId,
    /// The current state index.
    pub state_index: u32,
    /// The metadata corresponding to the current state.
    #[serde(with = "serde_bytes")]
    pub state_metadata: Box<[u8]>,
    /// A counter that denotes the number of foundries created by this alias account.
    pub foundry_counter: u32,
    // The governor address unlock condition and the state controller unlock conditions are mandatory for now, but this
    // could change in the protocol in the future for compression reasons.
    /// The state controller address unlock condition.
    pub state_controller_address_unlock_condition: StateControllerAddressUnlockCondition,
    /// The governer address unlock condition.
    pub governor_address_unlock_condition: GovernorAddressUnlockCondition,
    /// The corresponding list of [`Features`](Feature).
    pub features: Box<[Feature]>,
    /// The corresponding list of immutable [`Features`](Feature).
    pub immutable_features: Box<[Feature]>,
}

impl AliasOutput {
    /// A `&str` representation of the type.
    pub const KIND: &'static str = "alias";
}

impl<T: Borrow<iota::AliasOutput>> From<T> for AliasOutput {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            amount: value.amount().into(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            alias_id: (*value.alias_id()).into(),
            state_index: value.state_index(),
            state_metadata: value.state_metadata().to_vec().into_boxed_slice(),
            foundry_counter: value.foundry_counter(),
            // Panic: The state controller address unlock condition has to be present for now.
            state_controller_address_unlock_condition: value
                .unlock_conditions()
                .state_controller_address()
                .unwrap()
                .into(),
            // Panic: The governor address unlock condition has to be present for now.
            governor_address_unlock_condition: value.unlock_conditions().governor_address().unwrap().into(),
            features: value.features().iter().map(Into::into).collect(),
            immutable_features: value.immutable_features().iter().map(Into::into).collect(),
        }
    }
}

impl TryFromWithContext<AliasOutput> for iota::AliasOutput {
    type Error = iota_types::block::Error;

    fn try_from_with_context(
        ctx: &iota_types::block::protocol::ProtocolParameters,
        value: AliasOutput,
    ) -> Result<Self, Self::Error> {
        // The order of the conditions is important here because unlock conditions have to be sorted by type.
        let unlock_conditions = [
            Some(
                iota::unlock_condition::StateControllerAddressUnlockCondition::from(
                    value.state_controller_address_unlock_condition,
                )
                .into(),
            ),
            Some(
                iota::unlock_condition::GovernorAddressUnlockCondition::from(value.governor_address_unlock_condition)
                    .into(),
            ),
        ];

        Self::build_with_amount(value.amount.0, value.alias_id.into())?
            .with_native_tokens(
                value
                    .native_tokens
                    .into_vec()
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .with_state_index(value.state_index)
            .with_state_metadata(value.state_metadata.into())
            .with_foundry_counter(value.foundry_counter)
            .with_unlock_conditions(unlock_conditions.into_iter().flatten())
            .with_features(
                value
                    .features
                    .into_vec()
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .with_immutable_features(
                value
                    .immutable_features
                    .into_vec()
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .finish(ctx.token_supply())
    }
}

impl From<AliasOutput> for iota::dto::AliasOutputDto {
    fn from(value: AliasOutput) -> Self {
        let unlock_conditions = vec![
            iota::unlock_condition::dto::UnlockConditionDto::StateControllerAddress(
                value.state_controller_address_unlock_condition.into(),
            ),
            iota::unlock_condition::dto::UnlockConditionDto::GovernorAddress(
                value.governor_address_unlock_condition.into(),
            ),
        ];
        Self {
            kind: iota::AliasOutput::KIND,
            amount: value.amount.0.to_string(),
            native_tokens: value.native_tokens.into_vec().into_iter().map(Into::into).collect(),
            alias_id: value.alias_id.into(),
            state_index: value.state_index,
            state_metadata: prefix_hex::encode(value.state_metadata),
            foundry_counter: value.foundry_counter,
            unlock_conditions,
            features: value.features.into_vec().into_iter().map(Into::into).collect(),
            immutable_features: value
                .immutable_features
                .into_vec()
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use iota_types::block::rand::output::{rand_alias_id, rand_alias_output};

    use super::*;

    impl AliasId {
        /// Generates a random [`AliasId`].
        pub fn rand() -> Self {
            rand_alias_id().into()
        }
    }

    impl AliasOutput {
        /// Generates a random [`AliasOutput`].
        pub fn rand(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
            rand_alias_output(ctx.token_supply()).into()
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_alias_id_bson() {
        let alias_id = AliasId::rand();
        let bson = to_bson(&alias_id).unwrap();
        assert_eq!(Bson::from(alias_id), bson);
        assert_eq!(alias_id, from_bson::<AliasId>(bson).unwrap());
    }

    #[test]
    fn test_alias_output_bson() {
        let ctx = iota_types::block::protocol::protocol_parameters();
        let output = AliasOutput::rand(&ctx);
        iota::AliasOutput::try_from_with_context(&ctx, output.clone()).unwrap();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<AliasOutput>(bson).unwrap());
    }
}
