// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::runtime::actor::{context::ActorContext, event::HandleEvent, Actor};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArchiverError {}

#[derive(Debug)]
pub struct Archiver;

#[async_trait]
impl Actor for Archiver {
    type State = ();
    type Error = ArchiverError;

    async fn init(&mut self, cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        todo!()
    }
}

#[cfg(feature = "stardust")]
mod stardust {
    use chronicle::stardust::milestone::MilestoneIndex;

    use super::*;

    #[async_trait]
    impl HandleEvent<MilestoneIndex> for Archiver {
        async fn handle_event(
            &mut self,
            cx: &mut ActorContext<Self>,
            event: MilestoneIndex,
            state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            // The archiver's job is to retrieve all the messages of a milestone from the database
            // then archive them in order
            todo!()
        }
    }
}
