// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::signature as bee;
use serde::{Deserialize, Serialize};

use crate::types::util::bytify;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Signature {
    #[serde(rename = "ed25519")]
    Ed25519 {
        #[serde(with = "bytify")]
        public_key: [u8; Self::PUBLIC_KEY_LENGTH],
        #[serde(with = "bytify")]
        signature: [u8; Self::SIGNATURE_LENGTH],
    },
}

impl Signature {
    const PUBLIC_KEY_LENGTH: usize = bee::Ed25519Signature::PUBLIC_KEY_LENGTH;
    const SIGNATURE_LENGTH: usize = bee::Ed25519Signature::SIGNATURE_LENGTH;
}

impl From<&bee::Signature> for Signature {
    fn from(value: &bee::Signature) -> Self {
        match value {
            bee::Signature::Ed25519(signature) => Self::Ed25519 {
                public_key: *signature.public_key(),
                signature: *signature.signature(),
            },
        }
    }
}

impl From<Signature> for bee::Signature {
    fn from(value: Signature) -> Self {
        match value {
            Signature::Ed25519 { public_key, signature } => {
                bee::Signature::Ed25519(bee::Ed25519Signature::new(public_key, signature))
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod test {
    use mongodb::bson::{from_bson, to_bson};

    use super::*;

    #[test]
    fn test_signature_bson() {
        let signature = get_test_signature();
        let bson = to_bson(&signature).unwrap();
        assert_eq!(signature, from_bson::<Signature>(bson).unwrap());
    }

    pub(crate) fn get_test_signature() -> Signature {
        Signature::from(&bee::Signature::Ed25519(bee::Ed25519Signature::new(
            bee_test::rand::bytes::rand_bytes_array(),
            bee_test::rand::bytes::rand_bytes_array(),
        )))
    }
}
