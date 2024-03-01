// Copyright 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{prelude::stream::TryStreamExt, Stream};
use iota_sdk::types::block::{
    output::AccountId,
    protocol::ProtocolParameters,
    slot::{EpochIndex, SlotIndex},
};
use mongodb::{
    bson::doc,
    options::{IndexOptions, UpdateOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::{mongodb::DbError, MongoDb, MongoDbCollection, MongoDbCollectionExt},
    model::SerializeToBson,
};

/// The MongoDb document representation of address balances.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AccountCandidacyDocument {
    #[serde(rename = "_id")]
    pub account_id: AccountId,
    pub staking_start_epoch: EpochIndex,
    pub staking_end_epoch: EpochIndex,
    pub candidacy_slots: Option<Vec<SlotIndex>>,
}

/// A collection to store analytics address balances.
pub struct AccountCandidacyCollection {
    collection: mongodb::Collection<AccountCandidacyDocument>,
}

#[async_trait::async_trait]
impl MongoDbCollection for AccountCandidacyCollection {
    const NAME: &'static str = "analytics_candidacy_announcement";
    type Document = AccountCandidacyDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }

    async fn create_indexes(&self) -> Result<(), DbError> {
        self.create_index(
            IndexModel::builder()
                .keys(doc! { "staking_end_epoch": 1, "staking_start_epoch": 1 })
                .options(
                    IndexOptions::builder()
                        .name("candidate_index".to_string())
                        .partial_filter_expression(doc! {
                            "candidacy_slot": { "$exists": true },
                        })
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        Ok(())
    }
}

impl AccountCandidacyCollection {
    /// Add an account with a staking epoch range.
    pub async fn add_staking_account(
        &self,
        account_id: &AccountId,
        EpochIndex(staking_start_epoch): EpochIndex,
        EpochIndex(staking_end_epoch): EpochIndex,
    ) -> Result<(), DbError> {
        self.update_one(
            doc! { "_id": account_id.to_bson() },
            doc! { "$set": {
                "staking_start_epoch": staking_start_epoch,
                "staking_end_epoch": staking_end_epoch,
            } },
            UpdateOptions::builder().upsert(true).build(),
        )
        .await?;
        Ok(())
    }

    /// Add a candidacy announcement slot to an account.
    pub async fn add_candidacy_slot(
        &self,
        account_id: &AccountId,
        SlotIndex(candidacy_slot): SlotIndex,
    ) -> Result<(), DbError> {
        self.update_many(
            doc! {
                "_id.account_id": account_id.to_bson(),
            },
            doc! { "$addToSet": {
                "candidacy_slots": candidacy_slot,
            } },
            None,
        )
        .await?;
        Ok(())
    }

    /// Get all candidates at the candidate epoch.
    pub async fn get_candidates(
        &self,
        EpochIndex(candidate_epoch): EpochIndex,
        protocol_parameters: &ProtocolParameters,
    ) -> Result<impl Stream<Item = Result<AccountId, DbError>>, DbError> {
        let SlotIndex(start_slot) = protocol_parameters.first_slot_of(candidate_epoch.saturating_sub(1));
        let SlotIndex(registration_slot) = protocol_parameters.registration_slot(candidate_epoch.into());
        Ok(self
            .find::<AccountCandidacyDocument>(
                doc! {
                    "staking_start_epoch": { "$lte": candidate_epoch },
                    "staking_end_epoch": { "$gte": candidate_epoch },
                    "candidacy_slots": { "$exists": true },
                    "candidacy_slots": {
                        "$elemMatch": {
                            "$gte": start_slot,
                            "$lte": registration_slot,
                        }
                    },
                },
                None,
            )
            .await?
            .map_err(Into::into)
            .map_ok(|doc| doc.account_id))
    }

    /// Clears data that is outside of the range implied by the candidate epoch.
    pub async fn clear_expired_data(
        &self,
        EpochIndex(candidate_epoch): EpochIndex,
        protocol_parameters: &ProtocolParameters,
    ) -> Result<(), DbError> {
        let SlotIndex(start_slot) = protocol_parameters.first_slot_of(candidate_epoch.saturating_sub(1));
        self.collection()
            .delete_many(
                doc! {
                    "staking_end_epoch": { "$lt": candidate_epoch },
                },
                None,
            )
            .await?;
        self.update_many(
            doc! {
                "staking_start_epoch": { "$lte": candidate_epoch },
                "staking_end_epoch": { "$gte": candidate_epoch },
                "candidacy_slots": { "$exists": true },
            },
            doc! {
                "$pull": { "candidacy_slots": {
                    "$lt": start_slot,
                } }
            },
            None,
        )
        .await?;
        Ok(())
    }
}
