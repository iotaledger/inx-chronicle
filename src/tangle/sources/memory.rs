// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, ops::RangeBounds};

use async_trait::async_trait;
use futures::stream::BoxStream;
use thiserror::Error;

use super::{BlockData, InputSource, MilestoneData};
use crate::{tangle::ledger_updates::LedgerUpdateStore, types::tangle::MilestoneIndex};

pub struct InMemoryData {
    pub milestone: MilestoneData,
    pub cone: BTreeMap<u32, BlockData>,
    pub ledger_updates: LedgerUpdateStore,
}

#[derive(Debug, Error)]
pub enum InMemoryInputSourceError {
    #[error("missing block data for milestone {0}")]
    MissingBlockData(MilestoneIndex),
}

#[async_trait]
impl InputSource for BTreeMap<MilestoneIndex, InMemoryData> {
    type Error = InMemoryInputSourceError;

    async fn milestone_stream(
        &self,
        range: impl RangeBounds<MilestoneIndex> + Send,
    ) -> Result<BoxStream<Result<MilestoneData, Self::Error>>, Self::Error> {
        Ok(Box::pin(futures::stream::iter(
            self.range(range).map(|(_, v)| Ok(v.milestone.clone())),
        )))
    }

    async fn cone_stream(
        &self,
        index: MilestoneIndex,
    ) -> Result<BoxStream<Result<BlockData, Self::Error>>, Self::Error> {
        let cone = &self
            .get(&index)
            .ok_or(InMemoryInputSourceError::MissingBlockData(index))?
            .cone;
        Ok(Box::pin(futures::stream::iter(cone.values().map(|v| Ok(v.clone())))))
    }

    async fn ledger_updates(&self, index: MilestoneIndex) -> Result<LedgerUpdateStore, Self::Error> {
        Ok(self
            .get(&index)
            .ok_or(InMemoryInputSourceError::MissingBlockData(index))?
            .ledger_updates
            .clone())
    }
}

#[cfg(test)]
pub(crate) mod test {
    use std::{fs::File, io::BufReader};

    use packable::PackableExt;
    use serde::Deserialize;

    use super::*;
    use crate::{
        tangle::Tangle,
        types::{
            ledger::{BlockMetadata, LedgerOutput, LedgerSpent, MilestoneIndexTimestamp},
            node::NodeConfiguration,
            stardust::block::{
                payload::{MilestoneId, MilestonePayload},
                BlockId,
            },
            tangle::ProtocolParameters,
        },
    };

    pub(crate) const IN_MEM_MILESTONE: MilestoneIndex = MilestoneIndex(17339);

    pub(crate) fn get_in_memory_data() -> Tangle<BTreeMap<MilestoneIndex, InMemoryData>> {
        let file = File::open(format!("tests/data/in_memory_ms_{IN_MEM_MILESTONE}.json",)).unwrap();
        let test_data: mongodb::bson::Bson = serde_json::from_reader(BufReader::new(file)).unwrap();

        #[derive(Deserialize)]
        struct BsonMilestoneData {
            milestone_id: MilestoneId,
            at: MilestoneIndexTimestamp,
            payload: MilestonePayload,
            protocol_params: ProtocolParameters,
            node_config: NodeConfiguration,
        }

        impl From<BsonMilestoneData> for MilestoneData {
            fn from(value: BsonMilestoneData) -> Self {
                Self {
                    milestone_id: value.milestone_id,
                    at: value.at,
                    payload: value.payload,
                    protocol_params: value.protocol_params,
                    node_config: value.node_config,
                }
            }
        }

        #[derive(Deserialize)]
        struct BsonBlockData {
            block_id: BlockId,
            #[serde(with = "serde_bytes")]
            raw: Vec<u8>,
            metadata: BlockMetadata,
        }

        impl From<BsonBlockData> for BlockData {
            fn from(value: BsonBlockData) -> Self {
                Self {
                    block_id: value.block_id,
                    block: iota_types::block::Block::unpack_unverified(value.raw.clone())
                        .unwrap()
                        .into(),
                    raw: value.raw,
                    metadata: value.metadata,
                }
            }
        }

        #[derive(Deserialize)]
        struct InMemoryBsonData {
            milestone: BsonMilestoneData,
            cone: BTreeMap<String, BsonBlockData>,
            created: Vec<LedgerOutput>,
            consumed: Vec<LedgerSpent>,
        }

        impl From<InMemoryBsonData> for InMemoryData {
            fn from(value: InMemoryBsonData) -> Self {
                Self {
                    milestone: value.milestone.into(),
                    cone: value
                        .cone
                        .into_iter()
                        .map(|(idx, data)| (idx.parse().unwrap(), data.into()))
                        .collect(),
                    ledger_updates: LedgerUpdateStore::init(value.consumed, value.created),
                }
            }
        }

        Tangle::from(BTreeMap::from([(
            IN_MEM_MILESTONE,
            mongodb::bson::from_bson::<InMemoryBsonData>(test_data).unwrap().into(),
        )]))
    }
}
