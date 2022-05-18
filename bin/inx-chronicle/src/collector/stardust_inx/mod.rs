// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod config;
mod error;
mod listener;
mod syncer;
mod worker;

use std::collections::{HashSet, VecDeque};

use chronicle::{
    runtime::{Actor, Addr},
    types::stardust::message::MessageId,
};

pub(super) use self::{
    config::InxConfig,
    error::InxWorkerError,
    worker::{InxWorker, MessageRequest, MetadataRequest},
};

#[derive(Debug)]
pub struct MilestoneState {
    pub milestone_index: u32,
    pub process_queue: VecDeque<MessageId>,
    pub visited: HashSet<MessageId>,
    pub sender: Option<tokio::sync::oneshot::Sender<u32>>,
}

impl MilestoneState {
    pub fn new(milestone_index: u32) -> Self {
        Self {
            milestone_index,
            process_queue: VecDeque::new(),
            visited: HashSet::new(),
            sender: None,
        }
    }

    pub fn requested(milestone_index: u32, sender: tokio::sync::oneshot::Sender<u32>) -> Self {
        Self {
            milestone_index,
            process_queue: VecDeque::new(),
            visited: HashSet::new(),
            sender: Some(sender),
        }
    }
}

#[derive(Debug)]
pub struct RequestedMessage<Sender: Actor> {
    pub raw: Option<inx::proto::RawMessage>,
    pub metadata: inx::proto::MessageMetadata,
    pub sender_addr: Addr<Sender>,
    pub ms_state: MilestoneState,
}

impl<Sender: Actor> RequestedMessage<Sender> {
    pub fn new(
        raw: Option<inx::proto::RawMessage>,
        metadata: inx::proto::MessageMetadata,
        sender_addr: Addr<Sender>,
        ms_state: MilestoneState,
    ) -> Self {
        Self {
            raw,
            metadata,
            sender_addr,
            ms_state,
        }
    }
}
