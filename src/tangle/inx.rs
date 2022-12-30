// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use futures::{stream::BoxStream, StreamExt};

use super::Backend;
use crate::{
    inx::{BlockWithMetadataMessage, Inx, InxError},
    types::tangle::MilestoneIndex,
};

#[async_trait::async_trait]
impl Backend for Inx {
    type Error = InxError;

    async fn blocks(
        &mut self,
        milestone_index: MilestoneIndex,
    ) -> Result<BoxStream<Result<BlockWithMetadataMessage, Self::Error>>, Self::Error> {
        Ok(self.read_milestone_cone(milestone_index.into()).await?.boxed())
    }
}
