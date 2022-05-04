// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use derive_more::From;
use serde::{Deserialize, Serialize};
use tower_http::cors::AllowOrigin;

use super::error::ConfigError;

/// API configuration
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    pub port: u16,
    pub allow_origins: Option<SingleOrMultiple<String>>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            port: 9092,
            allow_origins: Some(String::from("*").into()),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, From)]
#[serde(untagged)]
pub enum SingleOrMultiple<T> {
    Single(T),
    Multiple(Vec<T>),
}

impl TryFrom<SingleOrMultiple<String>> for AllowOrigin {
    type Error = ConfigError;

    fn try_from(value: SingleOrMultiple<String>) -> Result<Self, Self::Error> {
        Ok(match value {
            SingleOrMultiple::Single(value) => AllowOrigin::exact(value.parse()?),
            SingleOrMultiple::Multiple(value) => {
                AllowOrigin::list(value.into_iter().map(|v| v.parse()).collect::<Result<Vec<_>, _>>()?)
            }
        })
    }
}
