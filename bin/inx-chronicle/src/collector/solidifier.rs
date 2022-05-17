// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use chronicle::{
    db::MongoDb,
    runtime::{Actor, ActorContext, RuntimeError},
};
use mongodb::bson::document::ValueAccessError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SolidifierError {
    #[cfg(all(feature = "stardust", feature = "inx"))]
    #[error("the stardust INX requester is missing")]
    MissingStardustInxRequester,
    #[error(transparent)]
    MongoDb(#[from] mongodb::error::Error),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    ValueAccess(#[from] ValueAccessError),
}

pub struct Solidifier {
    pub id: usize,
    pub(crate) db: MongoDb,
    #[cfg(feature = "metrics")]
    pub(crate) counter: bee_metrics::metrics::counter::Counter,
}

impl Solidifier {
    pub fn new(
        id: usize,
        db: MongoDb,
        #[cfg(feature = "metrics")] counter: bee_metrics::metrics::counter::Counter,
    ) -> Self {
        Self {
            id,
            db,
            #[cfg(feature = "metrics")]
            counter,
        }
    }
}

impl std::fmt::Debug for Solidifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Solidifier")
            .field("id", &self.id)
            .field("db", &self.db)
            .finish()
    }
}

#[async_trait]
impl Actor for Solidifier {
    type State = ();
    type Error = SolidifierError;

    async fn init(&mut self, _cx: &mut ActorContext<Self>) -> Result<Self::State, Self::Error> {
        Ok(())
    }

    fn name(&self) -> std::borrow::Cow<'static, str> {
        format!("Solidifier {}", self.id).into()
    }
}
