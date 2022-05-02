// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use crate::dto;

pub type MilestoneId = Box<[u8]>;
pub type MilestoneIndex = u32;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MilestonePayload {
    pub essence: MilestoneEssence,
    pub signatures: Box<dto::Signature>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MilestoneEssence {
    pub index: MilestoneIndex,
    pub timestamp: u32,
    pub previous_milestone_id: MilestoneId,
    pub parents: Box<[dto::MessageId]>,
    pub confirmed_merkle_proof: [u8; 32],
    pub applied_merkle_proof: [u8; 32],
    pub metadata: Vec<u8>,
    pub options: Box<[MilestoneOption]>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum MilestoneOption {
    Receipt {
        migrated_at: MilestoneIndex,
        last: bool,
        funds: Box<[MigratedFundsEntry]>,
        transaction: dto::TreasuryTransactionPayload,
    },
    Pow {},
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MigratedFundsEntry {}
