// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::dto;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum UnlockBlock {
    Signature(dto::Signature),
    Reference { index: u16 },
    Alias { index: u16 },
    Nft { index: u16 },
}
