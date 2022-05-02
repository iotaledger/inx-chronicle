// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TaggedDataPayload {
    tag: Box<[u8]>,
    data: Box<[u8]>,
}
