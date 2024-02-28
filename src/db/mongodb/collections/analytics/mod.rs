// Copyright 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{prelude::stream::TryStreamExt, Stream};
use iota_sdk::{types::block::address::Address, utils::serde::string};
use mongodb::{
    bson::doc,
    options::{IndexOptions, UpdateOptions},
    IndexModel,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::{mongodb::DbError, MongoDb, MongoDbCollection, MongoDbCollectionExt},
    model::address::AddressDto,
};

/// The MongoDb document representation of address balances.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AddressBalanceDocument {
    #[serde(rename = "_id")]
    pub address: AddressDto,
    #[serde(with = "string")]
    pub balance: u64,
}

/// A collection to store analytics address balances.
pub struct AddressBalanceCollection {
    collection: mongodb::Collection<AddressBalanceDocument>,
}

#[async_trait::async_trait]
impl MongoDbCollection for AddressBalanceCollection {
    const NAME: &'static str = "analytics_address_balance";
    type Document = AddressBalanceDocument;

    fn instantiate(_db: &MongoDb, collection: mongodb::Collection<Self::Document>) -> Self {
        Self { collection }
    }

    fn collection(&self) -> &mongodb::Collection<Self::Document> {
        &self.collection
    }

    async fn create_indexes(&self) -> Result<(), DbError> {
        self.create_index(
            IndexModel::builder()
                .keys(doc! { "balance": 1 })
                .options(
                    IndexOptions::builder()
                        .name("address_balance_index".to_string())
                        .build(),
                )
                .build(),
            None,
        )
        .await?;

        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RichestAddresses {
    pub top: Vec<AddressStat>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AddressStat {
    pub address: Address,
    pub balance: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenDistribution {
    pub distribution: Vec<DistributionStat>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Statistics for a particular logarithmic range of balances
pub struct DistributionStat {
    /// The logarithmic index the balances are contained between: \[10^index..10^(index+1)\]
    pub index: u32,
    /// The number of unique addresses in this range
    pub address_count: u64,
    /// The total balance of the addresses in this range
    pub total_balance: u64,
}

impl AddressBalanceCollection {
    /// Add an amount of balance to the given address.
    pub async fn add_balance(&self, address: &Address, amount: u64) -> Result<(), DbError> {
        self.update_one(
            doc! { "_id": AddressDto::from(address) },
            doc! { "$set": {
                "amount": {
                    "$toString": { "$add": [
                        { "$toDecimal": "$amount" },
                        { "$toDecimal": amount.to_string() }
                    ] }
                }
            } },
            UpdateOptions::builder().upsert(true).build(),
        )
        .await?;
        Ok(())
    }

    /// Remove an amount of balance from the given address.
    pub async fn remove_balance(&self, address: &Address, amount: u64) -> Result<(), DbError> {
        let address_dto = AddressDto::from(address);
        self.update_one(
            doc! { "_id": &address_dto },
            doc! { "$set": {
                "amount": {
                    "$toString": { "$subtract": [
                        { "$toDecimal": "$amount" },
                        { "$toDecimal": amount.to_string() }
                    ] }
                }
            } },
            None,
        )
        .await?;
        if self.get_balance(address).await? == 0 {
            self.collection().delete_one(doc! { "_id": address_dto }, None).await?;
        }
        Ok(())
    }

    /// Get the balance of an address.
    pub async fn get_balance(&self, address: &Address) -> Result<u64, DbError> {
        Ok(self
            .find_one::<AddressBalanceDocument>(doc! { "_id": AddressDto::from(address) }, None)
            .await?
            .map(|b| b.balance)
            .unwrap_or_default())
    }

    /// Get all balances.
    pub async fn get_all_balances(
        &self,
    ) -> Result<impl Stream<Item = Result<AddressBalanceDocument, DbError>>, DbError> {
        Ok(self
            .find::<AddressBalanceDocument>(doc! {}, None)
            .await?
            .map_err(Into::into))
    }

    /// Gets the top richest addresses.
    pub async fn get_richest_addresses(&self, top: usize) -> Result<RichestAddresses, DbError> {
        let top = self
            .aggregate(
                [
                    doc! { "$sort": { "balance": -1 } },
                    doc! { "$limit": top as i64 },
                    doc! { "$project": {
                        "_id": 0,
                        "address": "$_id",
                        "balance": 1,
                    } },
                ],
                None,
            )
            .await?
            .try_collect()
            .await?;
        Ok(RichestAddresses { top })
    }

    /// Get the token distribution.
    pub async fn get_token_distribution(&self) -> Result<TokenDistribution, DbError> {
        let distribution = self
            .aggregate(
                [
                    doc! { "$set": { "index": { "$toInt": { "$log10": "$balance" } } } },
                    doc! { "$group" : {
                        "_id": "$index",
                        "address_count": { "$sum": 1 },
                        "total_balance": { "$sum": "$balance" },
                    } },
                    doc! { "$sort": { "_id": 1 } },
                    doc! { "$project": {
                        "_id": 0,
                        "index": "$_id",
                        "address_count": 1,
                        "total_balance": { "$toString": "$total_balance" },
                    } },
                ],
                None,
            )
            .await?
            .try_collect()
            .await?;
        Ok(TokenDistribution { distribution })
    }
}
