// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::{
    address::Address,
    output::{AccountId, TokenId},
    slot::SlotIndex,
};
use mongodb::bson::{doc, Document};

use crate::model::{address::AddressDto, tag::Tag, SerializeToBson};

/// Defines how a query is appended to a list of `$and` queries.
pub(super) trait AppendToQuery {
    fn append_to(self, queries: &mut Vec<Document>);
}

pub(super) trait AppendQuery<Q> {
    fn append_query(&mut self, query: Q);
}

impl<Q: AppendToQuery> AppendQuery<Q> for Vec<Document> {
    fn append_query(&mut self, query: Q) {
        query.append_to(self)
    }
}

/// Queries for a feature of type `issuer`.
pub(super) struct IssuerQuery(pub(super) Option<Address>);

impl AppendToQuery for IssuerQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(address) = self.0 {
            queries.push(doc! {
                "details.issuer": AddressDto::from(address)
            });
        }
    }
}

/// Queries for a feature of type `sender`.
pub(super) struct SenderQuery(pub(super) Option<Address>);

impl AppendToQuery for SenderQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(address) = self.0 {
            queries.push(doc! {
                "details.sender": AddressDto::from(address)
            });
        }
    }
}

/// Queries for a feature of type `tag`.
pub(super) struct TagQuery(pub(super) Option<Tag>);

impl AppendToQuery for TagQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(tag) = self.0 {
            queries.push(doc! {
                "details.tag": tag
            });
        }
    }
}

/// Queries for native tokens.
pub(super) struct NativeTokensQuery {
    pub(super) has_native_tokens: Option<bool>,
    pub(super) native_token: Option<TokenId>,
}

impl AppendToQuery for NativeTokensQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(false) = self.has_native_tokens {
            queries.push(doc! {
                "details.native_tokens": { "$exists": false }
            });
        } else {
            if matches!(self.has_native_tokens, Some(true)) || self.native_token.is_some() {
                queries.push(doc! {
                    "details.native_tokens": { "$exists": true }
                });
            }
            if let Some(native_token) = self.native_token {
                queries.push(doc! {
                    "details.native_tokens": {
                        "$elemMatch": {
                            "token_id": native_token.to_bson()
                        }
                    }
                });
            }
        }
    }
}

/// Queries for an unlock condition of type `address`.
pub(super) struct AddressQuery(pub(super) Option<Address>);

impl AppendToQuery for AddressQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(address) = self.0 {
            queries.push(doc! {
                "details.address": AddressDto::from(address)
            });
        }
    }
}

/// Queries for an a unlocking address.
pub(super) struct UnlockableByAddressQuery {
    pub(super) address: Option<Address>,
    pub(super) slot_index: Option<SlotIndex>,
}

impl AppendToQuery for UnlockableByAddressQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        match (self.address, self.slot_index) {
            (Some(address), Some(SlotIndex(slot_index))) => {
                queries.push(doc! {
                    "$or": [
                        // If this output is trivially unlocked by this address
                        { "$and": [
                            { "details.address": address.to_bson() },
                            // And the output has no expiration or is not expired
                            { "$or": [
                                { "$lte": [ "$details.expiration", null ] },
                                { "$gt": [ "$details.expiration.slot_index", slot_index ] }
                            ] },
                            // and has no timelock or is past the lock period
                            { "$or": [
                                { "$lte": [ "$details.timelock", null ] },
                                { "$lte": [ "$details.timelock", slot_index ] }
                            ] }
                        ] },
                        // Otherwise, if this output has expiring funds that will be returned to this address
                        { "$and": [
                            { "details.expiration.return_address": address.to_bson() },
                            // And the output is expired
                            { "$lte": [ "$details.expiration.slot_index", slot_index ] },
                        ] },
                    ]
                });
            }
            (Some(address), None) => {
                queries.push(doc! {
                    "$or": [
                        { "details.address": address.to_bson() },
                        { "details.expiration.return_address": address.to_bson() },
                    ]
                });
            }
            (None, Some(SlotIndex(slot_index))) => {
                queries.push(doc! {
                    "$or": [
                        { "$lte": [ "$details.timelock", null ] },
                        { "$lte": [ "$details.timelock", slot_index ] }
                    ]
                });
            }
            _ => (),
        }
    }
}

/// Queries for an unlock condition of type `state_controller`.
pub(super) struct StateControllerQuery(pub(super) Option<Address>);

impl AppendToQuery for StateControllerQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(address) = self.0 {
            queries.push(doc! {
                "details.state_controller_address": AddressDto::from(address)
            });
        }
    }
}

/// Queries for an unlock condition of type `governor_address`.
pub(super) struct GovernorQuery(pub(super) Option<Address>);

impl AppendToQuery for GovernorQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(address) = self.0 {
            queries.push(doc! {
                "details.governor_address": AddressDto::from(address)
            });
        }
    }
}

/// Queries for a validator account.
pub(super) struct ValidatorQuery(pub(super) Option<AccountId>);

impl AppendToQuery for ValidatorQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(account_id) = self.0 {
            queries.push(doc! {
                "details.validator": account_id.to_bson()
            });
        }
    }
}

/// Queries for an account address.
pub(super) struct AccountAddressQuery(pub(super) Option<AccountId>);

impl AppendToQuery for AccountAddressQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(account_id) = self.0 {
            queries.push(doc! {
                "details.account_address": account_id.to_bson()
            });
        }
    }
}

/// Queries for an unlock condition of type `storage_deposit_return`.
pub(super) struct StorageDepositReturnQuery {
    pub(super) has_storage_return_condition: Option<bool>,
    pub(super) storage_return_address: Option<Address>,
}

impl AppendToQuery for StorageDepositReturnQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(has_storage_return_condition) = self.has_storage_return_condition {
            queries.push(doc! {
                "details.storage_deposit_return_address": { "$exists": has_storage_return_condition }
            });
        }
        if let Some(address) = self.storage_return_address {
            queries.push(doc! {
                "details.storage_deposit_return_address": AddressDto::from(address)
            });
        }
    }
}

/// Queries for an unlock condition of type `timelock`.
pub(super) struct TimelockQuery {
    pub(super) has_timelock_condition: Option<bool>,
    pub(super) timelocked_before: Option<SlotIndex>,
    pub(super) timelocked_after: Option<SlotIndex>,
}

impl AppendToQuery for TimelockQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(has_timelock_condition) = self.has_timelock_condition {
            queries.push(doc! {
                "details.timelock": { "$exists": has_timelock_condition }
            });
        }
        if let Some(timelocked_before) = self.timelocked_before {
            queries.push(doc! {
                "details.timelock": { "$lt": timelocked_before.0 }
            });
        }
        if let Some(timelocked_after) = self.timelocked_after {
            queries.push(doc! {
                "details.timelock": { "$gt": timelocked_after.0 }
            });
        }
    }
}

/// Queries for an unlock condition of type `expiration`.
pub(super) struct ExpirationQuery {
    pub(super) has_expiration_condition: Option<bool>,
    pub(super) expires_before: Option<SlotIndex>,
    pub(super) expires_after: Option<SlotIndex>,
    pub(super) expiration_return_address: Option<Address>,
}

impl AppendToQuery for ExpirationQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(has_expiration_condition) = self.has_expiration_condition {
            queries.push(doc! {
                "details.expiration": { "$exists": has_expiration_condition }
            });
        }
        if let Some(expires_before) = self.expires_before {
            queries.push(doc! {
                "details.expiration": { "$lt": expires_before.0 }
            });
        }
        if let Some(expires_after) = self.expires_after {
            queries.push(doc! {
                "details.expiration": { "$gt": expires_after.0 }
            });
        }
        if let Some(address) = self.expiration_return_address {
            queries.push(doc! {
                "details.expiration_return_address": AddressDto::from(address)
            });
        }
    }
}

/// Queries for created (booked) time.
pub(super) struct CreatedQuery {
    pub(super) created_before: Option<SlotIndex>,
    pub(super) created_after: Option<SlotIndex>,
}

impl AppendToQuery for CreatedQuery {
    fn append_to(self, queries: &mut Vec<Document>) {
        if let Some(created_before) = self.created_before {
            queries.push(doc! {
                "metadata.slot_booked": { "$lt": created_before.0 }
            });
        }
        if let Some(created_after) = self.created_after {
            queries.push(doc! {
                "metadata.slot_booked": { "$gt": created_after.0 }
            });
        }
    }
}
