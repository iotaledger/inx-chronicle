// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

pub type TokenAmount = [u8; 32];
pub type TokenId = Box<[u8]>;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct NativeToken {
    pub token_id: TokenId,
    pub amount: TokenAmount,
}
