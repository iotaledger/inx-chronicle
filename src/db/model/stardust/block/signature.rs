// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_block_stardust::signature as bee;
use serde::{Deserialize, Serialize};

use crate::db;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Signature {
    #[serde(rename = "ed25519")]
    Ed25519 {
        #[serde(with = "serde_bytes")]
        public_key: Box<[u8]>,
        #[serde(with = "serde_bytes")]
        signature: Box<[u8]>,
    },
}

impl From<&bee::Signature> for Signature {
    fn from(value: &bee::Signature) -> Self {
        match value {
            bee::Signature::Ed25519(signature) => Self::Ed25519 {
                public_key: signature.public_key().to_vec().into_boxed_slice(),
                signature: signature.signature().to_vec().into_boxed_slice(),
            },
        }
    }
}

impl TryFrom<Signature> for bee::Signature {
    type Error = db::error::Error;

    fn try_from(value: Signature) -> Result<Self, Self::Error> {
        Ok(match value {
            Signature::Ed25519 { public_key, signature } => bee::Signature::Ed25519(bee::Ed25519Signature::new(
                public_key.as_ref().try_into()?,
                signature.as_ref().try_into()?,
            )),
        })
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
