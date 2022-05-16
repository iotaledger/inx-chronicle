// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::output::feature_block as stardust;
use serde::{Deserialize, Serialize};

use crate::types::stardust::message::Address;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Feature {
    #[serde(rename = "sender")]
    Sender(Address),
    #[serde(rename = "issuer")]
    Issuer(Address),
    #[serde(rename = "metadata")]
    Metadata(#[serde(with = "serde_bytes")] Box<[u8]>),
    #[serde(rename = "tag")]
    Tag(#[serde(with = "serde_bytes")] Box<[u8]>),
}

impl From<&stardust::Feature> for Feature {
    fn from(value: &stardust::Feature) -> Self {
        match value {
            stardust::Feature::Sender(a) => Self::Sender(a.address().into()),
            stardust::Feature::Issuer(a) => Self::Issuer(a.address().into()),
            stardust::Feature::Metadata(b) => Self::Metadata(b.data().to_vec().into_boxed_slice()),
            stardust::Feature::Tag(b) => Self::Tag(b.tag().to_vec().into_boxed_slice()),
        }
    }
}

impl TryFrom<Feature> for stardust::Feature {
    type Error = crate::types::error::Error;

    fn try_from(value: Feature) -> Result<Self, Self::Error> {
        Ok(match value {
            Feature::Sender(a) => stardust::Feature::Sender(stardust::SenderFeature::new(a.try_into()?)),
            Feature::Issuer(a) => stardust::Feature::Issuer(stardust::IssuerFeature::new(a.try_into()?)),
            Feature::Metadata(b) => {
                stardust::Feature::Metadata(stardust::MetadataFeature::new(b.into())?)
            }
            Feature::Tag(b) => stardust::Feature::Tag(stardust::TagFeature::new(b.into())?),
        })
    }
}
