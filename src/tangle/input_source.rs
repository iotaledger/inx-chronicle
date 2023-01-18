// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use futures::{stream::BoxStream, StreamExt, TryStreamExt};
use inx::proto::LedgerUpdate;

use crate::{
    db::{collections::OutputCollection, MongoDb},
    types::{
        ledger::{BlockMetadata, MilestoneIndexTimestamp},
        stardust::block::{
            output::OutputId,
            payload::{MilestoneId, MilestonePayload, TransactionEssence},
            Block, BlockId, Input, Output, Payload,
        },
        tangle::{MilestoneIndex, ProtocolParameters},
    },
};
