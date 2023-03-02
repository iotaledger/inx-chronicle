// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`NftOutput`].

use std::{borrow::Borrow, str::FromStr};

use iota_types::block::output as iota;
use mongodb::bson::{spec::BinarySubtype, Binary, Bson};
use serde::{Deserialize, Serialize};

use super::{
    unlock_condition::{
        AddressUnlockCondition, ExpirationUnlockCondition, StorageDepositReturnUnlockCondition, TimelockUnlockCondition,
    },
    Feature, NativeToken, OutputId, TokenAmount,
};
use crate::types::{context::TryFromWithContext, util::bytify};

/// Uniquely identifies an NFT.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct NftId(#[serde(with = "bytify")] pub [u8; Self::LENGTH]);

impl NftId {
    const LENGTH: usize = iota::NftId::LENGTH;

    /// The [`NftId`] is derived from the [`super::OutputId`] that created the alias.
    pub fn from_output_id_str(s: &str) -> Result<Self, iota_types::block::Error> {
        Ok(iota::NftId::from(&iota::OutputId::from_str(s)?).into())
    }

    /// Get an implicit (zeroed) nft ID, for new nft outputs.
    pub fn implicit() -> Self {
        Self([0; Self::LENGTH])
    }
}

impl From<iota::NftId> for NftId {
    fn from(value: iota::NftId) -> Self {
        Self(*value)
    }
}

impl From<OutputId> for NftId {
    fn from(value: OutputId) -> Self {
        Self(value.hash())
    }
}

impl From<NftId> for iota::NftId {
    fn from(value: NftId) -> Self {
        iota::NftId::new(value.0)
    }
}

impl From<NftId> for iota::dto::NftIdDto {
    fn from(value: NftId) -> Self {
        Into::into(&iota::NftId::from(value))
    }
}

impl FromStr for NftId {
    type Err = iota_types::block::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(iota::NftId::from_str(s)?.into())
    }
}

impl From<NftId> for Bson {
    fn from(val: NftId) -> Self {
        Binary {
            subtype: BinarySubtype::Generic,
            bytes: val.0.to_vec(),
        }
        .into()
    }
}

/// Represents an NFT in the UTXO model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NftOutput {
    /// The output amount.
    pub amount: TokenAmount,
    /// The list of [`NativeToken`]s.
    pub native_tokens: Box<[NativeToken]>,
    /// The associated id of the NFT.
    pub nft_id: NftId,
    /// The address unlock condition.
    pub address_unlock_condition: AddressUnlockCondition,
    /// The storage deposit return unlock condition (SDRUC).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_deposit_return_unlock_condition: Option<StorageDepositReturnUnlockCondition>,
    /// The timelock unlock condition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timelock_unlock_condition: Option<TimelockUnlockCondition>,
    /// The expiration unlock condition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_unlock_condition: Option<ExpirationUnlockCondition>,
    /// The corresponding list of [`Feature`]s.
    pub features: Box<[Feature]>,
    /// The corresponding list of immutable [`Feature`]s.
    pub immutable_features: Box<[Feature]>,
}

impl NftOutput {
    /// A `&str` representation of the type.
    pub const KIND: &'static str = "nft";
}

impl<T: Borrow<iota::NftOutput>> From<T> for NftOutput {
    fn from(value: T) -> Self {
        let value = value.borrow();
        Self {
            amount: value.amount().into(),
            native_tokens: value.native_tokens().iter().map(Into::into).collect(),
            nft_id: (*value.nft_id()).into(),
            // Panic: The address unlock condition has to be present.
            address_unlock_condition: value.unlock_conditions().address().unwrap().into(),
            storage_deposit_return_unlock_condition: value.unlock_conditions().storage_deposit_return().map(Into::into),
            timelock_unlock_condition: value.unlock_conditions().timelock().map(Into::into),
            expiration_unlock_condition: value.unlock_conditions().expiration().map(Into::into),
            features: value.features().iter().map(Into::into).collect(),
            immutable_features: value.immutable_features().iter().map(Into::into).collect(),
        }
    }
}

impl TryFromWithContext<NftOutput> for iota::NftOutput {
    type Error = iota_types::block::Error;

    fn try_from_with_context(
        ctx: &iota_types::block::protocol::ProtocolParameters,
        value: NftOutput,
    ) -> Result<Self, Self::Error> {
        // The order of the conditions is imporant here because unlock conditions have to be sorted by type.
        let unlock_conditions = [
            Some(iota::unlock_condition::AddressUnlockCondition::from(value.address_unlock_condition).into()),
            value
                .storage_deposit_return_unlock_condition
                .map(|x| iota::unlock_condition::StorageDepositReturnUnlockCondition::try_from_with_context(ctx, x))
                .transpose()?
                .map(Into::into),
            value
                .timelock_unlock_condition
                .map(iota::unlock_condition::TimelockUnlockCondition::try_from)
                .transpose()?
                .map(Into::into),
            value
                .expiration_unlock_condition
                .map(iota::unlock_condition::ExpirationUnlockCondition::try_from)
                .transpose()?
                .map(Into::into),
        ];

        Self::build_with_amount(value.amount.0, value.nft_id.into())?
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

impl From<NftOutput> for iota::dto::NftOutputDto {
    fn from(value: NftOutput) -> Self {
        let mut unlock_conditions = vec![iota::unlock_condition::dto::UnlockConditionDto::Address(
            value.address_unlock_condition.into(),
        )];
        if let Some(uc) = value.storage_deposit_return_unlock_condition {
            unlock_conditions.push(iota::unlock_condition::dto::UnlockConditionDto::StorageDepositReturn(
                uc.into(),
            ));
        }
        if let Some(uc) = value.timelock_unlock_condition {
            unlock_conditions.push(iota::unlock_condition::dto::UnlockConditionDto::Timelock(uc.into()));
        }
        if let Some(uc) = value.expiration_unlock_condition {
            unlock_conditions.push(iota::unlock_condition::dto::UnlockConditionDto::Expiration(uc.into()));
        }
        Self {
            kind: iota::NftOutput::KIND,
            amount: value.amount.0.to_string(),
            native_tokens: value.native_tokens.into_vec().into_iter().map(Into::into).collect(),
            nft_id: value.nft_id.into(),
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
    use iota_types::block::rand::{bytes::rand_bytes_array, output::rand_nft_output};

    use super::*;

    impl NftId {
        /// Generates a random [`NftId`].
        pub fn rand() -> Self {
            Self(rand_bytes_array())
        }
    }

    impl NftOutput {
        /// Generates a random [`NftOutput`].
        pub fn rand(ctx: &iota_types::block::protocol::ProtocolParameters) -> Self {
            rand_nft_output(ctx.token_supply()).into()
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_nft_id_bson() {
        let nft_id = NftId::rand();
        let bson = to_bson(&nft_id).unwrap();
        assert_eq!(Bson::from(nft_id), bson);
        assert_eq!(nft_id, from_bson::<NftId>(bson).unwrap());
    }

    #[test]
    fn test_nft_output_bson() {
        let ctx = iota_types::block::protocol::protocol_parameters();
        let output = NftOutput::rand(&ctx);
        iota::NftOutput::try_from_with_context(&ctx, output.clone()).unwrap();
        let bson = to_bson(&output).unwrap();
        assert_eq!(output, from_bson::<NftOutput>(bson).unwrap());
    }
}
