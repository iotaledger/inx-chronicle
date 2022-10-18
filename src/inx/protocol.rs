// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use inx::proto;
use iota_types::block as iota;

use super::raw::RawMessage;

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawProtocolParametersMessage {
    pub protocol_version: u8,
    pub params: RawMessage<iota::protocol::ProtocolParameters>,
}

impl From<proto::RawProtocolParameters> for RawProtocolParametersMessage {
    fn from(value: proto::RawProtocolParameters) -> Self {
        Self {
            protocol_version: value.protocol_version as u8,
            params: value.params.into(),
        }
    }
}

impl From<RawProtocolParametersMessage> for proto::RawProtocolParameters {
    fn from(value: RawProtocolParametersMessage) -> Self {
        Self {
            protocol_version: value.protocol_version as u32,
            params: value.params.data(),
        }
    }
}
