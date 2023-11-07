// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::protocol::ProtocolParameters;

use super::*;

#[derive(Clone, Debug, Default)]
pub(crate) struct ProtocolParamsAnalytics {
    params: Option<ProtocolParameters>,
}

impl Analytics for ProtocolParamsAnalytics {
    type Measurement = Option<ProtocolParameters>;

    fn take_measurement(&mut self, ctx: &dyn AnalyticsContext) -> Self::Measurement {
        // Ensure that we record it if either the protocol changes or we had no params
        (!matches!(&self.params, Some(last_params) if last_params == ctx.protocol_params())).then(|| {
            self.params.replace(ctx.protocol_params().clone());
            ctx.protocol_params().clone()
        })
    }
}
