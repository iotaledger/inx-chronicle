// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::unlock as bee;
use serde::{Deserialize, Serialize};

use super::Signature;
use crate::db;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Unlock {
    #[serde(rename = "signature")]
    Signature { signature: Signature },
    #[serde(rename = "reference")]
    Reference { index: u16 },
    #[serde(rename = "alias")]
    Alias { index: u16 },
    #[serde(rename = "nft")]
    Nft { index: u16 },
}

impl From<&bee::Unlock> for Unlock {
    fn from(value: &bee::Unlock) -> Self {
        match value {
            bee::Unlock::Signature(s) => Self::Signature {
                signature: s.signature().into(),
            },
            bee::Unlock::Reference(r) => Self::Reference { index: r.index() },
            bee::Unlock::Alias(a) => Self::Alias { index: a.index() },
            bee::Unlock::Nft(n) => Self::Nft { index: n.index() },
        }
    }
}

impl TryFrom<Unlock> for bee::Unlock {
    type Error = db::error::Error;

    fn try_from(value: Unlock) -> Result<Self, Self::Error> {
        Ok(match value {
            Unlock::Signature { signature } => bee::Unlock::Signature(bee::SignatureUnlock::new(signature.into())),
            Unlock::Reference { index } => bee::Unlock::Reference(bee::ReferenceUnlock::new(index)?),
            Unlock::Alias { index } => bee::Unlock::Alias(bee::AliasUnlock::new(index)?),
            Unlock::Nft { index } => bee::Unlock::Nft(bee::NftUnlock::new(index)?),
        })
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::{super::signature::test::get_test_signature, *};

    #[test]
    fn test_unlock_bson() {
        let unlock = get_test_signature_unlock();
        let bson = to_bson(&unlock).unwrap();
        assert_eq!(unlock, from_bson::<Unlock>(bson).unwrap());

        let unlock = Unlock::Reference { index: 1 };
        let bson = to_bson(&unlock).unwrap();
        assert_eq!(unlock, from_bson::<Unlock>(bson).unwrap());

        let unlock = Unlock::Alias { index: 1 };
        let bson = to_bson(&unlock).unwrap();
        assert_eq!(unlock, from_bson::<Unlock>(bson).unwrap());

        let unlock = Unlock::Nft { index: 1 };
        let bson = to_bson(&unlock).unwrap();
        assert_eq!(unlock, from_bson::<Unlock>(bson).unwrap());
    }

    pub(crate) fn get_test_signature_unlock() -> Unlock {
        Unlock::Signature {
            signature: get_test_signature(),
        }
    }

    pub(crate) fn get_test_reference_unlock() -> Unlock {
        Unlock::Reference { index: 0 }
    }

    pub(crate) fn get_test_alias_unlock() -> Unlock {
        Unlock::Alias { index: 0 }
    }

    pub(crate) fn get_test_nft_unlock() -> Unlock {
        Unlock::Nft { index: 0 }
    }
}
