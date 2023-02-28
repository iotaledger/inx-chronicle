// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module contain utility functions.

/// A Serde helper module for converting values to [`String`].
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

/// `serde_bytes` cannot be used with sized arrays, so this works around that limitation.
pub mod bytify {
    use std::marker::PhantomData;

    use serde::{de::Visitor, Deserializer, Serializer};

    /// Deserialize T from bytes
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: for<'a> TryFrom<&'a [u8]>,
    {
        struct Helper<S>(PhantomData<S>);

        impl<'de, S> Visitor<'de> for Helper<S>
        where
            S: for<'a> TryFrom<&'a [u8]>,
        {
            type Value = S;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "bytes")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.try_into().map_err(|_| serde::de::Error::custom("invalid bytes"))
            }
        }

        deserializer.deserialize_bytes(Helper(PhantomData))
    }

    /// Serialize T as bytes
    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: AsRef<[u8]>,
        S: Serializer,
    {
        serde_bytes::Serialize::serialize(value.as_ref(), serializer)
    }
}
