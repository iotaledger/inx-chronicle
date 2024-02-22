// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{Stream, TryStreamExt};
use iota_sdk::types::block::{
    address::Address,
    output::{Output, OutputId},
    payload::signed_transaction::TransactionId,
    protocol::ProtocolParameters,
    slot::{SlotCommitmentId, SlotIndex},
    BlockId,
};
use mongodb::{
    bson::{doc, Document},
    options::{FindOptions, IndexOptions, InsertManyOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use super::SortOrder;
use crate::{
    db::{
        mongodb::{DbError, InsertIgnoreDuplicatesExt, MongoDbCollection, MongoDbCollectionExt},
        MongoDb,
    },
    model::{
        address::AddressDto,
        ledger::{LedgerOutput, LedgerSpent},
        raw::Raw,
        SerializeToBson,
    },
};

/// Contains all information related to an output.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LedgerUpdateDocument {
    _id: LedgerUpdateByAddressRecord,
    address: AddressDto,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerOutputRecord {
    pub output_id: OutputId,
    pub block_id: BlockId,
    pub slot_booked: SlotIndex,
    pub commitment_id_included: SlotCommitmentId,
    pub output: Raw<Output>,
}

impl From<LedgerOutputRecord> for LedgerOutput {
    fn from(value: LedgerOutputRecord) -> Self {
        Self {
            output_id: value.output_id,
            block_id: value.block_id,
            slot_booked: value.slot_booked,
            commitment_id_included: value.commitment_id_included,
            output: value.output,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerSpentRecord {
    pub output: LedgerOutputRecord,
    pub commitment_id_spent: SlotCommitmentId,
    pub transaction_id_spent: TransactionId,
    pub slot_spent: SlotIndex,
}

impl From<LedgerSpentRecord> for LedgerSpent {
    fn from(value: LedgerSpentRecord) -> Self {
        Self {
            output: value.output.into(),
            commitment_id_spent: value.commitment_id_spent,
            transaction_id_spent: value.transaction_id_spent,
            slot_spent: value.slot_spent,
        }
    }
}

/// The iota ledger updates collection.
pub struct LedgerUpdateCollection {
    collection: mongodb::Collection<LedgerUpdateDocument>,
}

#[async_trait::async_trait]
impl MongoDbCollection for LedgerUpdateCollection {
    const NAME: &'static str = "iota_ledger_updates";
    type Document = LedgerUpdateDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }

    async fn create_indexes(&self) -> Result<(), DbError> {
        self.create_index(
            IndexModel::builder()
                .keys(newest())
                .options(
                    IndexOptions::builder()
                        .unique(true)
                        .name("ledger_update_index".to_string())
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct LedgerUpdateByAddressRecord {
    pub slot_index: SlotIndex,
    pub output_id: OutputId,
    pub is_spent: bool,
}

#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub struct LedgerUpdateBySlotRecord {
    pub address: AddressDto,
    pub output_id: OutputId,
    pub is_spent: bool,
}

fn newest() -> Document {
    doc! { "address": -1, "_id.slot_index": -1, "_id.output_id": -1, "_id.is_spent": -1 }
}

fn oldest() -> Document {
    doc! { "address": 1, "_id.slot_index": 1, "_id.output_id": 1, "_id.is_spent": 1 }
}

/// Queries that are related to ledger updates.
impl LedgerUpdateCollection {
    /// Inserts spent ledger updates.
    #[instrument(skip_all, err, level = "trace")]
    pub async fn insert_spent_ledger_updates<'a, I>(
        &self,
        outputs: I,
        params: &ProtocolParameters,
    ) -> Result<(), DbError>
    where
        I: IntoIterator<Item = &'a LedgerSpent>,
        I::IntoIter: Send + Sync,
    {
        let ledger_updates = outputs.into_iter().map(|output| {
            // Ledger updates
            LedgerUpdateDocument {
                _id: LedgerUpdateByAddressRecord {
                    slot_index: output.slot_booked(),
                    output_id: output.output_id(),
                    is_spent: true,
                },
                address: output.locked_address(params).into(),
            }
        });
        self.insert_many_ignore_duplicates(ledger_updates, InsertManyOptions::builder().ordered(false).build())
            .await?;

        Ok(())
    }

    /// Inserts unspent ledger updates.
    #[instrument(skip_all, err, level = "trace")]
    pub async fn insert_unspent_ledger_updates<'a, I>(
        &self,
        outputs: I,
        params: &ProtocolParameters,
    ) -> Result<(), DbError>
    where
        I: IntoIterator<Item = &'a LedgerOutput>,
        I::IntoIter: Send + Sync,
    {
        let ledger_updates = outputs.into_iter().map(|output| {
            // Ledger updates
            LedgerUpdateDocument {
                _id: LedgerUpdateByAddressRecord {
                    slot_index: output.slot_booked,
                    output_id: output.output_id,
                    is_spent: false,
                },
                address: output.locked_address(params).into(),
            }
        });
        self.insert_many_ignore_duplicates(ledger_updates, InsertManyOptions::builder().ordered(false).build())
            .await?;

        Ok(())
    }

    /// Streams updates to the ledger for a given address.
    pub async fn get_ledger_updates_by_address(
        &self,
        address: &Address,
        page_size: usize,
        cursor: Option<(SlotIndex, Option<(OutputId, bool)>)>,
        order: SortOrder,
    ) -> Result<impl Stream<Item = Result<LedgerUpdateByAddressRecord, DbError>>, DbError> {
        let (sort, cmp1, cmp2) = match order {
            SortOrder::Newest => (newest(), "$lt", "$lte"),
            SortOrder::Oldest => (oldest(), "$gt", "$gte"),
        };

        let mut queries = vec![doc! { "address": address.to_bson() }];

        if let Some((slot_index, rest)) = cursor {
            let mut cursor_queries = vec![doc! { "_id.slot_index": { cmp1: slot_index.to_bson() } }];
            if let Some((output_id, is_spent)) = rest {
                cursor_queries.push(doc! {
                    "_id.slot_index": slot_index.to_bson(),
                    "_id.output_id": { cmp1: output_id.to_bson() }
                });
                cursor_queries.push(doc! {
                    "_id.slot_index": slot_index.to_bson(),
                    "_id.output_id": output_id.to_bson(),
                    "_id.is_spent": { cmp2: is_spent }
                });
            }
            queries.push(doc! { "$or": cursor_queries });
        }

        Ok(self
            .find::<LedgerUpdateDocument>(
                doc! { "$and": queries },
                FindOptions::builder().limit(page_size as i64).sort(sort).build(),
            )
            .await?
            .map_err(Into::into)
            .map_ok(|doc| LedgerUpdateByAddressRecord {
                slot_index: doc._id.slot_index,
                output_id: doc._id.output_id,
                is_spent: doc._id.is_spent,
            }))
    }

    /// Streams updates to the ledger for a given slot index (sorted by [`OutputId`]).
    pub async fn get_ledger_updates_by_slot(
        &self,
        slot_index: SlotIndex,
        page_size: usize,
        cursor: Option<(OutputId, bool)>,
    ) -> Result<impl Stream<Item = Result<LedgerUpdateBySlotRecord, DbError>>, DbError> {
        let (cmp1, cmp2) = ("$gt", "$gte");

        let mut queries = vec![doc! { "_id.slot_index": slot_index.to_bson() }];

        if let Some((output_id, is_spent)) = cursor {
            let mut cursor_queries = vec![doc! { "_id.output_id": { cmp1: output_id.to_bson() } }];
            cursor_queries.push(doc! {
                "_id.output_id": output_id.to_bson(),
                "_id.is_spent": { cmp2: is_spent }
            });
            queries.push(doc! { "$or": cursor_queries });
        }

        Ok(self
            .find::<LedgerUpdateDocument>(
                doc! { "$and": queries },
                FindOptions::builder().limit(page_size as i64).sort(oldest()).build(),
            )
            .await?
            .map_err(Into::into)
            .map_ok(|doc| LedgerUpdateBySlotRecord {
                address: doc.address,
                output_id: doc._id.output_id,
                is_spent: doc._id.is_spent,
            }))
    }
}
