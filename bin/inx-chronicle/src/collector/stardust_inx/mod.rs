// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod config;
mod error;
mod listener;
mod worker;

use std::collections::{HashSet, VecDeque};

use async_trait::async_trait;
use chronicle::{
    db::model::{
        ledger::Metadata,
        stardust::{
            block::{BlockId, BlockRecord},
            milestone::MilestoneRecord,
        },
        sync::SyncRecord,
        tangle::MilestoneIndex,
    },
    runtime::{ActorContext, ActorError, Addr, HandleEvent, Report},
};

pub(super) use self::{config::InxConfig, worker::InxWorker};
use self::{error::InxWorkerError, worker::InxRequest};
use super::{solidifier::Solidifier, Collector};
use crate::collector::solidifier::SolidifierError;

#[async_trait]
impl HandleEvent<Report<InxWorker>> for Collector {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        event: Report<InxWorker>,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match &event {
            Report::Success(_) => {
                cx.shutdown();
            }
            Report::Error(e) => match &e.error {
                ActorError::Result(e) => match e {
                    InxWorkerError::ConnectionError(_) => {
                        let wait_interval = self.config.inx.connection_retry_interval;
                        log::info!("Retrying INX connection in {} seconds.", wait_interval.as_secs_f32());
                        cx.delay(
                            chronicle::runtime::SpawnActor::new(InxWorker::new(self.config.inx.clone())),
                            wait_interval,
                        )?;
                    }
                    // TODO: This is stupid, but we can't use the ErrorKind enum so :shrug:
                    InxWorkerError::TransportFailed(e) => match e.to_string().as_ref() {
                        "transport error" => {
                            cx.spawn_child(InxWorker::new(self.config.inx.clone())).await;
                        }
                        _ => {
                            cx.shutdown();
                        }
                    },
                    InxWorkerError::MissingCollector => {
                        cx.delay(
                            chronicle::runtime::SpawnActor::new(InxWorker::new(self.config.inx.clone())),
                            None,
                        )?;
                    }
                    InxWorkerError::ListenerError(_)
                    | InxWorkerError::Runtime(_)
                    | InxWorkerError::Read(_)
                    | InxWorkerError::ParsingAddressFailed(_)
                    | InxWorkerError::InvalidAddress(_)
                    | InxWorkerError::FailedToAnswerRequest => {
                        cx.shutdown();
                    }
                },
                ActorError::Panic | ActorError::Aborted => {
                    cx.shutdown();
                }
            },
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct MilestoneState {
    pub milestone_index: MilestoneIndex,
    pub process_queue: VecDeque<BlockId>,
    pub visited: HashSet<BlockId>,
}

impl MilestoneState {
    pub fn new(milestone_index: MilestoneIndex) -> Self {
        Self {
            milestone_index,
            process_queue: VecDeque::new(),
            visited: HashSet::new(),
        }
    }
}

#[derive(Debug)]
pub struct RequestedBlock {
    raw: Option<inx::proto::RawBlock>,
    metadata: inx::proto::BlockMetadata,
    solidifier: Addr<Solidifier>,
    ms_state: MilestoneState,
}

impl RequestedBlock {
    pub fn new(
        raw: Option<inx::proto::RawBlock>,
        metadata: inx::proto::BlockMetadata,
        solidifier: Addr<Solidifier>,
        ms_state: MilestoneState,
    ) -> Self {
        Self {
            raw,
            metadata,
            solidifier,
            ms_state,
        }
    }
}

#[async_trait]
impl HandleEvent<inx::proto::Block> for Collector {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        block: inx::proto::Block,
        _solidifiers: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::trace!("Received Stardust Block Event");
        match BlockRecord::try_from(block) {
            Ok(rec) => {
                self.db.upsert_block_record(&rec).await?;
            }
            Err(e) => {
                log::error!("Could not read block: {:?}", e);
            }
        };
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<inx::proto::BlockMetadata> for Collector {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        metadata: inx::proto::BlockMetadata,
        _solidifiers: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::trace!("Received Stardust Block Referenced Event");
        match inx::BlockMetadata::try_from(metadata) {
            Ok(rec) => {
                let block_id = rec.block_id;
                self.db
                    .update_block_metadata(&block_id.into(), &Metadata::from(rec))
                    .await?;
            }
            Err(e) => {
                log::error!("Could not read block metadata: {:?}", e);
            }
        };
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<inx::proto::Milestone> for Collector {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        milestone: inx::proto::Milestone,
        solidifiers: &mut Self::State,
    ) -> Result<(), Self::Error> {
        log::trace!("Received Stardust Milestone Event");
        match MilestoneRecord::try_from(milestone) {
            Ok(rec) => {
                self.db.upsert_milestone_record(&rec).await?;
                // Get or create the milestone state
                let mut state = MilestoneState::new(rec.milestone_index);
                state
                    .process_queue
                    .extend(Vec::from(rec.payload.essence.parents).into_iter());
                solidifiers
                    // Divide solidifiers fairly by milestone
                    .get(*rec.milestone_index as usize % self.config.solidifier_count)
                    // Unwrap: We can never remove a `Solidifier` from the boxed slice, so they should always exist.
                    .unwrap()
                    .send(state)?;
            }
            Err(e) => {
                log::error!("Could not read milestone: {:?}", e);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl HandleEvent<RequestedBlock> for Collector {
    async fn handle_event(
        &mut self,
        _cx: &mut ActorContext<Self>,
        RequestedBlock {
            raw,
            metadata,
            solidifier,
            ms_state,
        }: RequestedBlock,
        _solidifiers: &mut Self::State,
    ) -> Result<(), Self::Error> {
        match raw {
            Some(raw) => {
                log::trace!("Received Stardust Requested Block and Metadata");
                match BlockRecord::try_from((raw, metadata)) {
                    Ok(rec) => {
                        self.db.upsert_block_record(&rec).await?;
                        // Send this directly to the solidifier that requested it
                        solidifier.send(ms_state)?;
                    }
                    Err(e) => {
                        log::error!("Could not read block: {:?}", e);
                    }
                };
            }
            None => {
                log::trace!("Received Stardust Requested Metadata");
                match inx::BlockMetadata::try_from(metadata) {
                    Ok(rec) => {
                        let block_id = rec.block_id;
                        self.db
                            .update_block_metadata(&block_id.into(), &Metadata::from(rec))
                            .await?;
                        // Send this directly to the solidifier that requested it
                        solidifier.send(ms_state)?;
                    }
                    Err(e) => {
                        log::error!("Could not read block metadata: {:?}", e);
                    }
                };
            }
        }

        Ok(())
    }
}

#[async_trait]
impl HandleEvent<MilestoneState> for Solidifier {
    async fn handle_event(
        &mut self,
        cx: &mut ActorContext<Self>,
        mut ms_state: MilestoneState,
        _state: &mut Self::State,
    ) -> Result<(), Self::Error> {
        // Process by iterating the queue until we either complete the milestone or fail to find a block
        while let Some(block_id) = ms_state.process_queue.front() {
            // First check if we already processed this block in this run
            if ms_state.visited.contains(block_id) {
                ms_state.process_queue.pop_front();
            } else {
                // Try the database
                match self.db.get_block(block_id).await? {
                    Some(block_rec) => {
                        match block_rec
                            .metadata
                            .map(|metadata| metadata.referenced_by_milestone_index)
                        {
                            Some(ms_index) => {
                                log::trace!("Block {} is referenced by milestone {}", block_id.to_hex(), ms_index);

                                // We add the current block to the list of visited blocks.
                                ms_state.visited.insert(block_id.clone());

                                // We may have reached a different milestone, in which case there is nothing to
                                // do for this block
                                if ms_state.milestone_index == ms_index {
                                    let parents = Vec::from(block_rec.inner.parents);
                                    ms_state.process_queue.extend(parents);
                                }
                                ms_state.process_queue.pop_front();
                            }
                            // If the block has not been referenced, we can't proceed
                            None => {
                                log::trace!("Requesting metadata for block {}", block_id.to_hex());
                                // Send the state and everything. If the requester finds the block, it will circle
                                // back.
                                cx.addr::<InxWorker>()
                                    .await
                                    .send(InxRequest::get_metadata(
                                        block_id.clone(),
                                        cx.handle().clone(),
                                        ms_state,
                                    ))
                                    .map_err(|_| SolidifierError::MissingStardustInxRequester)?;
                                return Ok(());
                            }
                        }
                    }
                    // Otherwise, send a block to the requester
                    None => {
                        log::trace!("Requesting block {}", block_id.to_hex());
                        // Send the state and everything. If the requester finds the block, it will circle
                        // back.
                        cx.addr::<InxWorker>()
                            .await
                            .send(InxRequest::get_block(block_id.clone(), cx.handle().clone(), ms_state))
                            .map_err(|_| SolidifierError::MissingStardustInxRequester)?;
                        return Ok(());
                    }
                }
            }
        }

        // If we finished all the parents, that means we have a complete milestone
        // so we should mark it synced
        self.db
            .upsert_sync_record(&SyncRecord {
                milestone_index: ms_state.milestone_index,
                logged: false,
                synced: true,
            })
            .await?;
        #[cfg(feature = "metrics")]
        self.counter.inc();

        Ok(())
    }
}
