// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::unlock as stardust;
use serde::{Deserialize, Serialize};

use crate::types::stardust::message::Signature;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Unlock {
    #[serde(rename = "signature")]
    Signature(Signature),
    #[serde(rename = "reference")]
    Reference { index: u16 },
    #[serde(rename = "alias")]
    Alias { index: u16 },
    #[serde(rename = "nft")]
    Nft { index: u16 },
}

impl From<&stardust::Unlock> for Unlock {
    fn from(value: &stardust::Unlock) -> Self {
        match value {
            stardust::Unlock::Signature(s) => Self::Signature(s.signature().into()),
            stardust::Unlock::Reference(r) => Self::Reference { index: r.index() },
            stardust::Unlock::Alias(a) => Self::Alias { index: a.index() },
            stardust::Unlock::Nft(n) => Self::Nft { index: n.index() },
        }
    }
}

impl TryFrom<Unlock> for stardust::Unlock {
    type Error = crate::types::error::Error;

    fn try_from(value: Unlock) -> Result<Self, Self::Error> {
        Ok(match value {
            Unlock::Signature(s) => {
                stardust::Unlock::Signature(stardust::SignatureUnlock::new(s.try_into()?))
            }
            Unlock::Reference { index } => {
                stardust::Unlock::Reference(stardust::ReferenceUnlock::new(index)?)
            }
            Unlock::Alias { index } => stardust::Unlock::Alias(stardust::AliasUnlock::new(index)?),
            Unlock::Nft { index } => stardust::Unlock::Nft(stardust::NftUnlock::new(index)?),
        })
    }
}
