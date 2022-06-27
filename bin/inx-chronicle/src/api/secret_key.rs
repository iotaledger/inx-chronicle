// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use ed25519::pkcs8::{DecodePrivateKey, KeypairBytes};
use thiserror::Error;

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
pub struct SecretKey(ed25519_dalek::SecretKey);

/// View the bytes of the secret key.
impl AsRef<[u8]> for SecretKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl Clone for SecretKey {
    fn clone(&self) -> SecretKey {
        let mut sk_bytes = self.0.to_bytes();
        Self::from_bytes(&mut sk_bytes).expect("ed25519_dalek::SecretKey::from_bytes(to_bytes(k)) != k")
    }
}

impl std::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretKey")
    }
}

impl SecretKey {
    /// Generate a new Ed25519 secret key.
    pub fn generate() -> SecretKey {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        SecretKey(ed25519_dalek::SecretKey::from_bytes(&bytes).unwrap())
    }

    /// Create an Ed25519 secret key from a byte slice, zeroing the input on success.
    /// If the bytes do not constitute a valid Ed25519 secret key, an error is
    /// returned.
    pub fn from_bytes(mut sk_bytes: impl AsMut<[u8]>) -> Result<SecretKey, SecretKeyError> {
        use zeroize::Zeroize;
        let sk_bytes = sk_bytes.as_mut();
        let secret = ed25519_dalek::SecretKey::from_bytes(&*sk_bytes).map_err(SecretKeyError::Decode)?;
        sk_bytes.zeroize();
        Ok(SecretKey(secret))
    }

    /// Create an Ed25519 secret key from a PEM file.
    pub fn from_file(path: &str) -> Result<SecretKey, SecretKeyError> {
        let mut bytes = KeypairBytes::from_pkcs8_pem(&std::fs::read_to_string(std::path::Path::new(path))?)
            .map_err(|_| SecretKeyError::Read)?;
        SecretKey::from_bytes(&mut bytes.secret_key)
    }
}
