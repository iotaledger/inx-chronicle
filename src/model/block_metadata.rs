// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing block metadata types.

use iota_sdk::{
    types::{
        api::core::{BlockState, TransactionState},
        block::{
            self as iota, payload::signed_transaction::TransactionId, semantic::TransactionFailureReason, BlockId,
        },
    },
    utils::serde::option_string,
};
use serde::{Deserialize, Serialize};

use super::raw::Raw;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BlockMetadata {
    pub block_id: BlockId,
    #[serde(default, with = "option_strum_string")]
    pub block_state: Option<BlockState>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]

pub struct TransactionMetadata {
    pub transaction_id: TransactionId,
    #[serde(with = "option_strum_string")]
    pub transaction_state: Option<TransactionState>,
    #[serde(default, with = "option_string")]
    pub transaction_failure_reason: Option<TransactionFailureReason>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BlockWithMetadata {
    pub metadata: BlockMetadata,
    pub block: Raw<iota::Block>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BlockWithTransactionMetadata {
    pub block: BlockWithMetadata,
    pub transaction: Option<TransactionMetadata>,
}

/// Serializes types that `impl AsRef<str>`
#[allow(missing_docs)]
pub mod option_strum_string {
    use core::{fmt::Display, str::FromStr};

    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: AsRef<str>,
        S: Serializer,
    {
        match value {
            Some(value) => serializer.collect_str(value.as_ref()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        Option::<String>::deserialize(deserializer)?
            .map(|string| string.parse().map_err(de::Error::custom))
            .transpose()
    }
}
