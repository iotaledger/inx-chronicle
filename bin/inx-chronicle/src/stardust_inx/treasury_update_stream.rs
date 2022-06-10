// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::RangeInclusive;

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, HandleEvent},
    types::tangle::MilestoneIndex,
};
use inx::tonic::Status;

use super::InxError;

#[derive(Debug)]
pub struct TreasuryUpdateStream {
    db: MongoDb,
    range: RangeInclusive<MilestoneIndex>,
}

impl TreasuryUpdateStream {
    pub fn new(db: MongoDb, range: RangeInclusive<MilestoneIndex>) -> Self {
        Self { db, range }
    }
}

#[async_trait]
impl Actor for TreasuryUpdateStream {
    type State = ();
    type Error = InxError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(())
    }

    fn name(&self) -> std::borrow::Cow<'static, str> {
        if *self.range.end() == u32::MAX {
            format!("TreasuryUpdateStream ({}..)", self.range.start()).into()
        } else {
            format!("TreasuryUpdateStream ({}..={})", self.range.start(), self.range.end()).into()
        }
    }
}

#[async_trait]
impl HandleEvent<Result<inx::proto::TreasuryUpdate, Status>> for TreasuryUpdateStream {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        treasury_update_result: Result<inx::proto::TreasuryUpdate, Status>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::trace!("Received treasury update event {:#?}", treasury_update_result);

        let treasury_update = inx::TreasuryUpdate::try_from(treasury_update_result?)?;

        self.db.upsert_treasury(treasury_update).await?;

        Ok(())
    }
}
