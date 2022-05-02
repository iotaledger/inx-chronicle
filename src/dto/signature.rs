// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use bee_message_stardust::signature as stardust;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Signature {
    Ed25519Signature {
        public_key: Box<[u8]>,
        signature: Box<[u8]>,
    },
}

impl From<stardust::Signature> for Signature {
    fn from(value: stardust::Signature) -> Self {
        match value {
            stardust::Signature::Ed25519(signature) => Self::Ed25519Signature {
                public_key: signature.public_key().to_vec().into_boxed_slice(),
                signature: signature.signature().to_vec().into_boxed_slice(),
            },
        }
    }
}
