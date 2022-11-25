// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use chronicle::types::stardust::block::BlockId;
use serde::{Deserialize, Serialize};

use super::{audit_path::Hashable, MerkleAuditPath};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleAuditPathDto {
    #[serde(rename = "l")]
    left: HashableDto,
    #[serde(rename = "r")]
    right: HashableDto,
}

impl From<MerkleAuditPath> for MerkleAuditPathDto {
    fn from(value: MerkleAuditPath) -> Self {
        Self {
            left: value.left.into(),
            right: value.right.into(),
        }
    }
}

impl TryFrom<MerkleAuditPathDto> for MerkleAuditPath {
    type Error = prefix_hex::Error;

    fn try_from(audit_path: MerkleAuditPathDto) -> Result<Self, Self::Error> {
        Ok(Self {
            left: Hashable::try_from(audit_path.left)?,
            right: Hashable::try_from(audit_path.right)?,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HashableDto {
    Node {
        #[serde(rename = "h")]
        hash: String,
    },
    Path(Box<MerkleAuditPathDto>),
    Value {
        #[serde(rename = "value")]
        block_id_hex: String,
    },
}

impl From<Hashable> for HashableDto {
    fn from(value: Hashable) -> Self {
        match value {
            Hashable::Node(hash) => Self::Node {
                hash: prefix_hex::encode(hash.as_slice()),
            },
            Hashable::Path(path) => Self::Path(Box::new((*path).into())),
            Hashable::Value(block_id) => Self::Value {
                block_id_hex: block_id.to_hex(),
            },
        }
    }
}

impl TryFrom<HashableDto> for Hashable {
    type Error = prefix_hex::Error;

    fn try_from(hashed: HashableDto) -> Result<Self, Self::Error> {
        use iota_types::block::payload::milestone::MerkleRoot;
        Ok(match hashed {
            HashableDto::Node { hash } => Hashable::Node(prefix_hex::decode::<[u8; MerkleRoot::LENGTH]>(&hash)?.into()),
            HashableDto::Path(path) => Hashable::Path(Box::new(MerkleAuditPath::try_from(*path)?)),
            HashableDto::Value { block_id_hex } => {
                Hashable::Value(BlockId(prefix_hex::decode::<[u8; BlockId::LENGTH]>(&block_id_hex)?))
            }
        })
    }
}
