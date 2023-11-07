// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that contains the types.

pub mod block;
pub mod raw;

pub use block::*;

pub mod utxo {
    //! A logical grouping of UTXO types for convenience.
    #![allow(ambiguous_glob_reexports)]
    pub use super::block::payload::transaction::{
        input::*,
        output::{address::*, unlock_condition::*, *},
        unlock::*,
    };
}
use iota_sdk::types::ValidationParams;
// Bring this module up to the top level for convenience
use mongodb::bson::Bson;
use serde::{de::DeserializeOwned, Serialize};

/// Helper trait for serializable types
pub trait SerializeToBson: Serialize {
    /// Serializes values to Bson infallibly
    fn to_bson(&self) -> Bson {
        mongodb::bson::to_bson(self).unwrap()
    }
}
impl<T: Serialize> SerializeToBson for T {}

/// Helper trait for deserializable types
pub trait DeserializeFromBson: DeserializeOwned {
    /// Serializes values to Bson infallibly
    fn from_bson(bson: Bson) -> mongodb::bson::de::Result<Self>
    where
        Self: Sized,
    {
        mongodb::bson::from_bson(bson)
    }
}
impl<T: DeserializeOwned> DeserializeFromBson for T {}

pub trait TryFromDto<Dto>: Sized {
    type Error;

    fn try_from_dto(dto: Dto) -> Result<Self, Self::Error> {
        Self::try_from_dto_with_params(dto, ValidationParams::default())
    }

    fn try_from_dto_with_params<'a>(
        dto: Dto,
        params: impl Into<ValidationParams<'a>> + Send,
    ) -> Result<Self, Self::Error> {
        Self::try_from_dto_with_params_inner(dto, params.into())
    }

    fn try_from_dto_with_params_inner(dto: Dto, params: ValidationParams<'_>) -> Result<Self, Self::Error>;
}
