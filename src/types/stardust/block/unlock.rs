// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::unlock as bee;
use serde::{Deserialize, Serialize};

use super::Signature;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Unlock {
    Signature { signature: Signature },
    Reference { index: u16 },
    Alias { index: u16 },
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
    type Error = bee_block_stardust::Error;

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
mod test {
    use mongodb::bson::{from_bson, to_bson};
    use test_util::unlock::{rand_alias_unlock, rand_nft_unlock, rand_reference_unlock, rand_signature_unlock};

    use super::*;

    #[test]
    fn test_unlock_bson() {
        let unlock = Unlock::from(&bee::Unlock::from(rand_signature_unlock()));
        let bson = to_bson(&unlock).unwrap();
        assert_eq!(unlock, from_bson::<Unlock>(bson).unwrap());

        let unlock = Unlock::from(&bee::Unlock::from(rand_reference_unlock()));
        let bson = to_bson(&unlock).unwrap();
        assert_eq!(unlock, from_bson::<Unlock>(bson).unwrap());

        let unlock = Unlock::from(&bee::Unlock::from(rand_alias_unlock()));
        let bson = to_bson(&unlock).unwrap();
        assert_eq!(unlock, from_bson::<Unlock>(bson).unwrap());

        let unlock = Unlock::from(&bee::Unlock::from(rand_nft_unlock()));
        let bson = to_bson(&unlock).unwrap();
        assert_eq!(unlock, from_bson::<Unlock>(bson).unwrap());
    }
}
