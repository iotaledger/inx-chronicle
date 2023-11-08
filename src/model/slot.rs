// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Module that contains slot types.

use iota_sdk::types::block::slot::{SlotCommitment, SlotCommitmentId};
use serde::{Deserialize, Serialize};

use super::raw::Raw;

/// A slot's commitment data.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]

pub struct Commitment {
    /// The identifier of the slot commitment.
    pub commitment_id: SlotCommitmentId,
    /// The commitment.
    pub commitment: Raw<SlotCommitment>,
}
