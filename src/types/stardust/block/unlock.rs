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

#[cfg(feature = "rand")]
mod rand {
    use bee_block_stardust::{rand::number::rand_number_range, unlock::UNLOCK_INDEX_RANGE};

    use super::*;

    impl Unlock {
        /// Generates a random [`Unlock`].
        pub fn rand() -> Self {
            match rand_number_range(0..4) {
                0 => Self::rand_signature(),
                1 => Self::rand_reference(),
                2 => Self::rand_alias(),
                3 => Self::rand_nft(),
                _ => unreachable!(),
            }
        }

        /// Generates a random signature [`Unlock`].
        pub fn rand_signature() -> Self {
            Self::Signature {
                signature: Signature::rand(),
            }
        }

        /// Generates a random reference [`Unlock`].
        pub fn rand_reference() -> Self {
            Self::Reference {
                index: rand_number_range(UNLOCK_INDEX_RANGE),
            }
        }

        /// Generates a random alias [`Unlock`].
        pub fn rand_alias() -> Self {
            Self::Alias {
                index: rand_number_range(UNLOCK_INDEX_RANGE),
            }
        }

        /// Generates a random nft [`Unlock`].
        pub fn rand_nft() -> Self {
            Self::Nft {
                index: rand_number_range(UNLOCK_INDEX_RANGE),
            }
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_signature_unlock_bson() {
        let unlock = Unlock::rand_signature();
        let bson = to_bson(&unlock).unwrap();
        assert_eq!(unlock, from_bson::<Unlock>(bson).unwrap());
    }

    #[test]
    fn test_reference_unlock_bson() {
        let unlock = Unlock::rand_reference();
        let bson = to_bson(&unlock).unwrap();
        assert_eq!(unlock, from_bson::<Unlock>(bson).unwrap());
    }

    #[test]
    fn test_alias_unlock_bson() {
        let unlock = Unlock::rand_alias();
        let bson = to_bson(&unlock).unwrap();
        assert_eq!(unlock, from_bson::<Unlock>(bson).unwrap());
    }

    #[test]
    fn test_nft_unlock_bson() {
        let unlock = Unlock::rand_nft();
        let bson = to_bson(&unlock).unwrap();
        assert_eq!(unlock, from_bson::<Unlock>(bson).unwrap());
    }
}
