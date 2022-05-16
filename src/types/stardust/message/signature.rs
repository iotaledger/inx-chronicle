// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::signature as stardust;
use serde::{Deserialize, Serialize};

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

impl From<&stardust::Signature> for Signature {
    fn from(value: &stardust::Signature) -> Self {
        match value {
            stardust::Signature::Ed25519(signature) => Self::Ed25519 {
                public_key: signature.public_key().to_vec().into_boxed_slice(),
                signature: signature.signature().to_vec().into_boxed_slice(),
            },
        }
    }
}

impl TryFrom<Signature> for stardust::Signature {
    type Error = crate::types::error::Error;

    fn try_from(value: Signature) -> Result<Self, Self::Error> {
        Ok(match value {
            Signature::Ed25519 { public_key, signature } => stardust::Signature::Ed25519(
                stardust::Ed25519Signature::new(public_key.as_ref().try_into()?, signature.as_ref().try_into()?),
            ),
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
        Signature::from(&stardust::Signature::Ed25519(stardust::Ed25519Signature::new(
            bee_test::rand::bytes::rand_bytes_array(),
            bee_test::rand::bytes::rand_bytes_array(),
        )))
    }
}
