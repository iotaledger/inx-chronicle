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

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        // TODO
        Ok(())
    }
}

#[cfg(feature = "stardust")]
mod stardust {
    use chronicle::stardust::milestone::MilestoneIndex;

    use super::*;

    #[async_trait]
    impl HandleEvent<(MilestoneIndex, Vec<Vec<u8>>)> for Archiver {
        async fn handle_event(
            &mut self,
            _cx: &mut ActorContext<Self>,
            (milestone_index, _messages): (MilestoneIndex, Vec<Vec<u8>>),
            _state: &mut Self::State,
        ) -> Result<(), Self::Error> {
            log::info!("Archiving milestone {}", milestone_index);
            // TODO
            Ok(())
        }
    }
}
