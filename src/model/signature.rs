// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module containing the [`Signature`] type.

use iota_sdk::types::block::signature as iota;
use serde::{Deserialize, Serialize};

use crate::model::bytify;

/// Represents a signature used to unlock an output.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Signature {
    /// An [`Ed25519`](https://en.wikipedia.org/wiki/EdDSA) signature.
    Ed25519 {
        /// The public key as bytes.
        #[serde(with = "bytify")]
        public_key: [u8; Self::PUBLIC_KEY_LENGTH],
        /// The signature as bytes.
        #[serde(with = "bytify")]
        signature: [u8; Self::SIGNATURE_LENGTH],
    },
}

impl Signature {
    const PUBLIC_KEY_LENGTH: usize = iota::Ed25519Signature::PUBLIC_KEY_LENGTH;
    const SIGNATURE_LENGTH: usize = iota::Ed25519Signature::SIGNATURE_LENGTH;
}

impl From<&iota::Signature> for Signature {
    fn from(value: &iota::Signature) -> Self {
        match value {
            iota::Signature::Ed25519(signature) => Self::Ed25519 {
                public_key: signature.public_key_bytes().to_bytes(),
                signature: signature.signature().to_bytes(),
            },
        }
    }
}

impl From<Signature> for iota::Signature {
    fn from(value: Signature) -> Self {
        match value {
            Signature::Ed25519 { public_key, signature } => {
                iota::Ed25519Signature::from_bytes(public_key, signature).into()
            }
        }
    }
}

impl From<Signature> for iota::dto::SignatureDto {
    fn from(value: Signature) -> Self {
        match value {
            Signature::Ed25519 { public_key, signature } => Self::Ed25519(
                iota::dto::Ed25519SignatureDto {
                    kind: iota::Ed25519Signature::KIND,
                    public_key: prefix_hex::encode(public_key),
                    signature: prefix_hex::encode(signature),
                }
                .into(),
            ),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use iota_sdk::types::block::rand::signature::rand_signature;

    use super::*;

    impl Signature {
        /// Generates a random [`Signature`] with an [`iota::Ed25519Signature`].
        pub fn rand() -> Self {
            Self::from(&rand_signature())
        }
    }
}

#[cfg(all(test, feature = "rand"))]
mod test {
    use mongodb::bson::{from_bson, to_bson};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_signature_bson() {
        let signature = Signature::rand();
        let bson = to_bson(&signature).unwrap();
        assert_eq!(signature, from_bson::<Signature>(bson).unwrap());
    }
}
