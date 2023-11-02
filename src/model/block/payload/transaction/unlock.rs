// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`Unlock`] types.

use iota_sdk::types::block::{signature::Ed25519Signature, unlock as iota};
use serde::{Deserialize, Serialize};

/// The different types of [`Unlock`]s.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum UnlockDto {
    /// A signature unlock.
    Signature {
        /// The [`Ed25519Signature`] of the unlock.
        signature: Ed25519Signature,
    },
    /// A reference unlock.
    Reference {
        /// The index of the unlock.
        index: u16,
    },
    /// An account unlock.
    Account {
        /// The index of the unlock.
        index: u16,
    },
    /// An anchor unlock.
    Anchor {
        /// The index of the unlock.
        index: u16,
    },
    /// An NFT unlock.
    Nft {
        /// The index of the unlock.
        index: u16,
    },
}

impl From<&iota::Unlock> for UnlockDto {
    fn from(value: &iota::Unlock) -> Self {
        match value {
            iota::Unlock::Signature(s) => Self::Signature {
                signature: *s.signature().as_ed25519(),
            },
            iota::Unlock::Reference(r) => Self::Reference { index: r.index() },
            iota::Unlock::Account(a) => Self::Account { index: a.index() },
            iota::Unlock::Anchor(a) => Self::Anchor { index: a.index() },
            iota::Unlock::Nft(n) => Self::Nft { index: n.index() },
        }
    }
}

impl TryFrom<UnlockDto> for iota::Unlock {
    type Error = iota_sdk::types::block::Error;

    fn try_from(value: UnlockDto) -> Result<Self, Self::Error> {
        Ok(match value {
            UnlockDto::Signature { signature } => {
                iota::Unlock::Signature(Box::new(iota::SignatureUnlock::new(signature.into())))
            }
            UnlockDto::Reference { index } => iota::Unlock::Reference(iota::ReferenceUnlock::new(index)?),
            UnlockDto::Account { index } => iota::Unlock::Account(iota::AccountUnlock::new(index)?),
            UnlockDto::Anchor { index } => iota::Unlock::Anchor(iota::AnchorUnlock::new(index)?),
            UnlockDto::Nft { index } => iota::Unlock::Nft(iota::NftUnlock::new(index)?),
        })
    }
}

// #[cfg(all(test, feature = "rand"))]
// mod test {
//     use mongodb::bson::{from_bson, to_bson};
//     use pretty_assertions::assert_eq;

//     use super::*;

//     #[test]
//     fn test_signature_unlock_bson() {
//         let unlock = Unlock::rand_signature();
//         let bson = to_bson(&unlock).unwrap();
//         assert_eq!(unlock, from_bson::<Unlock>(bson).unwrap());
//     }

//     #[test]
//     fn test_reference_unlock_bson() {
//         let unlock = Unlock::rand_reference();
//         let bson = to_bson(&unlock).unwrap();
//         assert_eq!(unlock, from_bson::<Unlock>(bson).unwrap());
//     }

//     #[test]
//     fn test_alias_unlock_bson() {
//         let unlock = Unlock::rand_alias();
//         let bson = to_bson(&unlock).unwrap();
//         assert_eq!(unlock, from_bson::<Unlock>(bson).unwrap());
//     }

//     #[test]
//     fn test_nft_unlock_bson() {
//         let unlock = Unlock::rand_nft();
//         let bson = to_bson(&unlock).unwrap();
//         assert_eq!(unlock, from_bson::<Unlock>(bson).unwrap());
//     }
// }
