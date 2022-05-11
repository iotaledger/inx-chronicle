// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::output::feature_block as stardust;
use serde::{Deserialize, Serialize};

use crate::types::address::Address;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum FeatureBlock {
    #[serde(rename = "sender")]
    Sender(Address),
    #[serde(rename = "issuer")]
    Issuer(Address),
    #[serde(rename = "metadata")]
    Metadata(#[serde(with = "serde_bytes")] Box<[u8]>),
    #[serde(rename = "tag")]
    Tag(#[serde(with = "serde_bytes")] Box<[u8]>),
}

impl From<&stardust::FeatureBlock> for FeatureBlock {
    fn from(value: &stardust::FeatureBlock) -> Self {
        match value {
            stardust::FeatureBlock::Sender(a) => Self::Sender(a.address().into()),
            stardust::FeatureBlock::Issuer(a) => Self::Issuer(a.address().into()),
            stardust::FeatureBlock::Metadata(b) => Self::Metadata(b.data().to_vec().into_boxed_slice()),
            stardust::FeatureBlock::Tag(b) => Self::Tag(b.tag().to_vec().into_boxed_slice()),
        }
    }
}

impl TryFrom<FeatureBlock> for stardust::FeatureBlock {
    type Error = crate::types::error::Error;

    fn try_from(value: FeatureBlock) -> Result<Self, Self::Error> {
        Ok(match value {
            FeatureBlock::Sender(a) => stardust::FeatureBlock::Sender(stardust::SenderFeatureBlock::new(a.try_into()?)),
            FeatureBlock::Issuer(a) => stardust::FeatureBlock::Issuer(stardust::IssuerFeatureBlock::new(a.try_into()?)),
            FeatureBlock::Metadata(b) => {
                stardust::FeatureBlock::Metadata(stardust::MetadataFeatureBlock::new(b.into())?)
            }
            FeatureBlock::Tag(b) => stardust::FeatureBlock::Tag(stardust::TagFeatureBlock::new(b.into())?),
        })
    }
}
