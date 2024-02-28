// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Range;

use chronicle::db::mongodb::collections::{DistributionStat, LedgerUpdateByAddressRecord};
use iota_sdk::{
    types::block::{
        address::Bech32Address,
        output::OutputId,
        slot::{SlotCommitmentId, SlotIndex},
        BlockId,
    },
    utils::serde::string,
};
use serde::{Deserialize, Serialize};

use crate::api::responses::impl_success_response;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerUpdatesByAddressResponse {
    pub address: Bech32Address,
    pub items: Vec<LedgerUpdateByAddressDto>,
    pub cursor: Option<String>,
}

impl_success_response!(LedgerUpdatesByAddressResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerUpdateByAddressDto {
    pub output_id: OutputId,
    pub is_spent: bool,
    pub slot_index: SlotIndex,
}

impl From<LedgerUpdateByAddressRecord> for LedgerUpdateByAddressDto {
    fn from(value: LedgerUpdateByAddressRecord) -> Self {
        Self {
            output_id: value.output_id,
            is_spent: value.is_spent,
            slot_index: value.slot_index,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerUpdatesBySlotResponse {
    pub slot_index: SlotIndex,
    pub items: Vec<LedgerUpdateBySlotDto>,
    pub cursor: Option<String>,
}

impl_success_response!(LedgerUpdatesBySlotResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerUpdateBySlotDto {
    pub address: Bech32Address,
    pub output_id: OutputId,
    pub is_spent: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceResponse {
    pub total_balance: Balance,
    pub available_balance: Balance,
    pub ledger_index: SlotIndex,
}

impl_success_response!(BalanceResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Balance {
    #[serde(with = "string")]
    pub amount: u64,
    #[serde(with = "string")]
    pub stored_mana: u64,
    pub decayed_mana: DecayedMana,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecayedMana {
    #[serde(with = "string")]
    pub stored: u64,
    #[serde(with = "string")]
    pub potential: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockChildrenResponse {
    pub block_id: BlockId,
    pub max_results: usize,
    pub count: usize,
    pub children: Vec<BlockId>,
}

impl_success_response!(BlockChildrenResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotsResponse {
    pub items: Vec<SlotDto>,
    pub cursor: Option<String>,
}

impl_success_response!(SlotsResponse);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockPayloadTypeDto {
    pub block_id: BlockId,
    #[serde(rename = "payloadType")]
    pub payload_kind: Option<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlocksBySlotResponse {
    pub blocks: Vec<BlockPayloadTypeDto>,
    pub cursor: Option<String>,
}

impl_success_response!(BlocksBySlotResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotDto {
    pub commitment_id: SlotCommitmentId,
    pub index: SlotIndex,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RichestAddressesResponse {
    pub top: Vec<AddressStatDto>,
    pub ledger_index: SlotIndex,
}

impl_success_response!(RichestAddressesResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddressStatDto {
    pub address: Bech32Address,
    #[serde(with = "string")]
    pub balance: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenDistributionResponse {
    pub distribution: Vec<DistributionStatDto>,
    pub ledger_index: SlotIndex,
}

impl_success_response!(TokenDistributionResponse);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionStatDto {
    pub range: Range<u64>,
    pub address_count: String,
    #[serde(with = "string")]
    pub total_balance: u64,
}

impl From<DistributionStat> for DistributionStatDto {
    fn from(s: DistributionStat) -> Self {
        Self {
            range: 10_u64.pow(s.index)..10_u64.pow(s.index + 1),
            address_count: s.address_count.to_string(),
            total_balance: s.total_balance,
        }
    }
}
