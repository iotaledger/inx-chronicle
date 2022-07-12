// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{collections::VecDeque, ops::RangeInclusive};

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, ActorError, HandleEvent, Report},
    types::tangle::MilestoneIndex,
};
use inx::{client::InxClient, tonic::Channel};

use super::{InxConfig, InxError, LedgerUpdateStream};

// The Syncer starts at a certain milestone index in the past and moves forwards in time trying to sync as many
// milestones as possible - including their cones.
pub struct Syncer {
    gaps: Gaps,
    db: MongoDb,
    inx_client: InxClient<Channel>,
}

impl Syncer {
    pub fn new(
        gaps: Vec<RangeInclusive<MilestoneIndex>>,
        db: MongoDb,
        inx_client: InxClient<Channel>,
        config: &InxConfig,
    ) -> Self {
        Self {
            gaps: Gaps {
                gaps: gaps.into(),
                batch_size: config.sync_batch_size.map(|v| v.max(1)), // Minimum of 1 at a time
            },
            db,
            inx_client,
        }
    }
}

#[derive(Debug, Default)]
pub struct Gaps {
    gaps: VecDeque<RangeInclusive<MilestoneIndex>>,
    batch_size: Option<u32>,
}

impl Iterator for Gaps {
    type Item = RangeInclusive<MilestoneIndex>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(range) = self.gaps.pop_front() {
            if range.start() <= range.end() {
                if let Some(batch_size) = self.batch_size {
                    let batch_end = (*range.end()).min(*range.start() + MilestoneIndex(batch_size - 1));
                    let res = *range.start()..=batch_end;
                    if batch_end < *range.end() {
                        self.gaps.push_front(batch_end + 1..=*range.end());
                    }
                    return Some(res);
                } else {
                    return Some(range);
                }
            }
        }
        None
    }
}

#[async_trait]
impl Actor for Syncer {
    type State = ();
    type Error = InxError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(())
    }

    fn name(&self) -> std::borrow::Cow<'static, str> {
        "Syncer".into()
    }
}

#[async_trait]
impl HandleEvent<Report<LedgerUpdateStream>> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<LedgerUpdateStream>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        // Start syncing the next milestone range
        cx.delay(SyncNext, None)?;
        match event {
            Report::Success(report) => {
                log::info!(
                    "Synced milestones {}..={}",
                    report.actor.range.start(),
                    report.actor.range.end()
                );
            }
            Report::Error(report) => match report.error {
                ActorError::Result(e) => {
                    Err(e)?;
                }
                ActorError::Aborted | ActorError::Panic => {
                    cx.abort().await;
                }
            },
        }
        Ok(())
    }
}

pub struct SyncNext;

#[async_trait]
impl HandleEvent<SyncNext> for Syncer {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        _evt: SyncNext,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        if let Some(milestone_range) = self.gaps.next() {
            log::info!(
                "Requesting unsynced milestone range {}..{}.",
                milestone_range.start(),
                milestone_range.end()
            );
            cx.spawn_child(LedgerUpdateStream::new(
                self.db.clone(),
                self.inx_client.clone(),
                *milestone_range.start()..=*milestone_range.end(),
            ))
            .await;
        } else {
            log::info!("Successfully finished synchronization with node.");
            cx.shutdown().await;
        }
        Ok(())
    }
}
