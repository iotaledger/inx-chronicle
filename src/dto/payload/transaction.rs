// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::dto;

pub type TransactionId = Box<[u8]>;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct TransactionPayload {
    pub essence: TransactionEssence,
    pub unlock_blocks: Box<[dto::UnlockBlock]>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum TransactionEssence {
    Regular {
        network_id: u64,
        inputs: Box<[dto::Input]>,
        inputs_commitment: [u8; 32],
        outputs: Box<[dto::Output]>,
        payload: Option<dto::Payload>,
    },
}
