// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::unlock_block as bee;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::Signature;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum UnlockBlock {
    #[serde(rename = "signature")]
    Signature { signature: Signature },
    #[serde(rename = "reference")]
    Reference { index: u16 },
    #[serde(rename = "alias")]
    Alias { index: u16 },
    #[serde(rename = "nft")]
    Nft { index: u16 },
}

impl From<&bee::UnlockBlock> for UnlockBlock {
    fn from(value: &bee::UnlockBlock) -> Self {
        match value {
            bee::UnlockBlock::Signature(s) => Self::Signature {
                signature: s.signature().into(),
            },
            bee::UnlockBlock::Reference(r) => Self::Reference { index: r.index() },
            bee::UnlockBlock::Alias(a) => Self::Alias { index: a.index() },
            bee::UnlockBlock::Nft(n) => Self::Nft { index: n.index() },
        }
    }
}

impl TryFrom<UnlockBlock> for bee::UnlockBlock {
    type Error = crate::types::error::Error;

    fn try_from(value: UnlockBlock) -> Result<Self, Self::Error> {
        Ok(match value {
            UnlockBlock::Signature { signature } => {
                bee::UnlockBlock::Signature(bee::SignatureUnlockBlock::new(signature.try_into()?))
            }
            UnlockBlock::Reference { index } => bee::UnlockBlock::Reference(bee::ReferenceUnlockBlock::new(index)?),
            UnlockBlock::Alias { index } => bee::UnlockBlock::Alias(bee::AliasUnlockBlock::new(index)?),
            UnlockBlock::Nft { index } => bee::UnlockBlock::Nft(bee::NftUnlockBlock::new(index)?),
        })
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;
    use crate::types::stardust::block::signature::test::get_test_signature;

    #[test]
    fn test_unlock_block_bson() {
        let unlock_block = get_test_signature_unlock_block();
        let bson = to_bson(&unlock_block).unwrap();
        assert_eq!(unlock_block, from_bson::<UnlockBlock>(bson).unwrap());

        let unlock_block = UnlockBlock::Reference { index: 1 };
        let bson = to_bson(&unlock_block).unwrap();
        assert_eq!(unlock_block, from_bson::<UnlockBlock>(bson).unwrap());

        let unlock_block = UnlockBlock::Alias { index: 1 };
        let bson = to_bson(&unlock_block).unwrap();
        assert_eq!(unlock_block, from_bson::<UnlockBlock>(bson).unwrap());

        let unlock_block = UnlockBlock::Nft { index: 1 };
        let bson = to_bson(&unlock_block).unwrap();
        assert_eq!(unlock_block, from_bson::<UnlockBlock>(bson).unwrap());
    }

    pub(crate) fn get_test_signature_unlock_block() -> UnlockBlock {
        UnlockBlock::Signature {
            signature: get_test_signature(),
        }
    }

    pub(crate) fn get_test_reference_unlock_block() -> UnlockBlock {
        UnlockBlock::Reference { index: 0 }
    }

    pub(crate) fn get_test_alias_unlock_block() -> UnlockBlock {
        UnlockBlock::Alias { index: 0 }
    }

    pub(crate) fn get_test_nft_unlock_block() -> UnlockBlock {
        UnlockBlock::Nft { index: 0 }
    }
}
