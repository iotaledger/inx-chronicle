// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use ed25519::pkcs8::{DecodePrivateKey, KeypairBytes};
use thiserror::Error;
use zeroize::Zeroize;

#[derive(Error, Debug)]
pub enum SecretKeyError {
    #[error("failed to read file: {0}")]
    FileRead(#[from] std::io::Error),
    #[error("failed to decode key: {0}")]
    Decode(ed25519_dalek::SignatureError),
    #[error("failed to read key bytes")]
    Read,
}

/// An Ed25519 secret key.
#[derive(Clone, Zeroize)]
pub struct SecretKey(ed25519_dalek::SecretKey);

/// View the bytes of the secret key.
impl AsRef<[u8]> for SecretKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretKey")
    }
}

impl SecretKey {
    /// Generate a new Ed25519 secret key.
    pub fn generate() -> Self {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        Self::from_bytes(bytes)
    }

    /// Create an Ed25519 secret key from a 32 byte array.
    pub fn from_bytes(sk_bytes: ed25519_dalek::SecretKey) -> Self {
        Self(sk_bytes)
    }

    /// Create an Ed25519 secret key from a PEM file.
    pub fn from_file(path: &str) -> Result<Self, SecretKeyError> {
        let bytes = KeypairBytes::from_pkcs8_pem(&std::fs::read_to_string(std::path::Path::new(path))?)
            .map_err(|_| SecretKeyError::Read)?;
        Ok(Self(bytes.secret_key))
    }
}
