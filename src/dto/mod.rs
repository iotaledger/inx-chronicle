// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

mod address;
mod conflict_reason;
mod error;
mod inclusion_state;
mod input;
mod message;
mod output;
mod payload;
mod signature;
mod unlock_block;

pub use self::{
    address::*, conflict_reason::*, inclusion_state::*, input::*, message::*, output::*, payload::*, signature::*,
    unlock_block::*,
};

pub mod stringify {
    use std::{fmt::Display, marker::PhantomData, str::FromStr};

    use serde::{de::Visitor, Deserializer, Serializer};

    /// Deserialize T using [`FromStr`]
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: FromStr,
        T::Err: Display,
    {
        struct Helper<S>(PhantomData<S>);

        impl<'de, S> Visitor<'de> for Helper<S>
        where
            S: FromStr,
            <S as FromStr>::Err: Display,
        {
            type Value = S;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "a string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                value.parse::<Self::Value>().map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_str(Helper(PhantomData))
    }

    /// Serialize T using [`Display`]
    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Display,
        S: Serializer,
    {
        serializer.collect_str(&value)
    }
}
