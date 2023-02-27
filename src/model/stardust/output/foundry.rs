// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`FoundryOutput`].

use std::{borrow::Borrow, str::FromStr};

use iota_types::block::output as iota;
use mongodb::bson::{spec::BinarySubtype, Binary, Bson};
use serde::{Deserialize, Serialize};

use super::{unlock_condition::ImmutableAliasAddressUnlockCondition, Feature, NativeToken, TokenAmount, TokenScheme};
use crate::model::{
    context::TryFromWithContext,
    util::{bytify, stringify},
};

/// The id of a foundry.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct FoundryId(#[serde(with = "bytify")] pub [u8; Self::LENGTH]);

impl FoundryId {
    const LENGTH: usize = iota::FoundryId::LENGTH;
}

impl FoundryId {
    /// Get an implicit (zeroed) foundry ID, for new foundry outputs.
    pub fn implicit() -> Self {
        Self([0; Self::LENGTH])
    }
}

impl From<iota::FoundryId> for FoundryId {
    fn from(value: iota::FoundryId) -> Self {
        Self(*value)
    }
}

impl From<FoundryId> for iota::FoundryId {
    fn from(value: FoundryId) -> Self {
        iota::FoundryId::new(value.0)
    }
}

impl FromStr for FoundryId {
    type Err = iota_types::block::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(iota::FoundryId::from_str(s)?.into())
    }
}

impl From<FoundryId> for Bson {
    fn from(val: FoundryId) -> Self {
        Binary {
            subtype: BinarySubtype::Generic,
            bytes: val.0.to_vec(),
        }
        .into()
    }
}

/// Represents a foundry in the UTXO model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryOutput {
    /// The output amount.
    pub amount: TokenAmount,
    /// The list of [`NativeToken`]s.
    pub native_tokens: Box<[NativeToken]>,
    /// The associated id of the foundry.
    pub foundry_id: FoundryId,
    /// The serial number of the foundry.
    #[serde(with = "stringify")]
    pub serial_number: u32,
    /// The [`TokenScheme`] of the underlying token.
    pub token_scheme: TokenScheme,
    /// The immutable alias address unlock condition.
    pub immutable_alias_address_unlock_condition: ImmutableAliasAddressUnlockCondition,
    /// The corresponding list of [`Feature`]s.
    pub features: Box<[Feature]>,
    /// The corresponding list of immutable [`Feature`]s.
    pub immutable_features: Box<[Feature]>,
}

impl FoundryOutput {
    /// A `&str` representation of the type.
    pub const KIND: &'static str = "foundry";
}

impl<T: Borrow<iota::FoundryOutput>> From<T> for FoundryOutput {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            amount: value.amount().into(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            foundry_id: value.id().into(),
            serial_number: value.serial_number(),
            token_scheme: value.token_scheme().into(),
            // Panic: The immutable alias address unlock condition has to be present.
            immutable_alias_address_unlock_condition: value
                .unlock_conditions()
                .immutable_alias_address()
                .unwrap()
                .into(),
            features: value.features().iter().map(Into::into).collect(),
            immutable_features: value.immutable_features().iter().map(Into::into).collect(),
        }
    }
}

impl TryFromWithContext<FoundryOutput> for iota::FoundryOutput {
    type Error = iota_types::block::Error;

    fn try_from_with_context(
        ctx: &iota_types::block::protocol::ProtocolParameters,
        value: FoundryOutput,
    ) -> Result<Self, Self::Error> {
        let u: iota::UnlockCondition = iota::unlock_condition::ImmutableAliasAddressUnlockCondition::try_from(
            value.immutable_alias_address_unlock_condition,
        )?
        .into();

        Self::build_with_amount(value.amount.0, value.serial_number, value.token_scheme.try_into()?)?
            .with_native_tokens(
                value
                    .native_tokens
                    .into_vec()
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .with_unlock_conditions([u])
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

impl From<FoundryOutput> for iota::dto::FoundryOutputDto {
    fn from(value: FoundryOutput) -> Self {
        let unlock_conditions = vec![iota::unlock_condition::dto::UnlockConditionDto::ImmutableAliasAddress(
            value.immutable_alias_address_unlock_condition.into(),
        )];
        Self {
            kind: iota::FoundryOutput::KIND,
            amount: value.amount.0.to_string(),
            native_tokens: value.native_tokens.into_vec().into_iter().map(Into::into).collect(),
            serial_number: value.serial_number,
            token_scheme: value.token_scheme.into(),
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
    use iota_types::block::rand::{bytes::rand_bytes_array, output::rand_foundry_output};

    use super::*;

    impl FoundryId {
        /// Generates a random [`FoundryId`].
        pub fn rand() -> Self {
            Self(rand_bytes_array())
        }
    }

    impl FoundryOutput {
        /// Generates a random [`FoundryOutput`].
        pub fn rand(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
            rand_foundry_output(ctx.token_supply()).into()
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_foundry_output_bson() {
        let ctx = iota_types::block::protocol::protocol_parameters();
        let output = FoundryOutput::rand(&ctx);
        iota::FoundryOutput::try_from_with_context(&ctx, output.clone()).unwrap();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<FoundryOutput>(bson).unwrap());
    }
}
