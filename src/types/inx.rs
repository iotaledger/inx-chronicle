// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// The different errors that can happen with INX.
pub enum InxError {
    InvalidByteLength { actual: usize, expected: usize },
    InvalidRawBytes(String),
    MissingField(&'static str),
}

#[macro_export]
macro_rules! maybe_missing {
    ($object:ident.$field:ident) => {
        $object.$field.ok_or(InxError::MissingField(stringify!($field)))?
    };
}
