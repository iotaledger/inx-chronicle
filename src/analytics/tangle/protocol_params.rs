// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk::types::block::protocol::ProtocolParameters;

use crate::analytics::{Analytics, AnalyticsContext};

#[derive(Clone, Debug, Default)]
pub(crate) struct ProtocolParamsAnalytics {
    params: Option<ProtocolParameters>,
}

#[async_trait::async_trait]
impl Analytics for ProtocolParamsAnalytics {
    type Measurement = Option<ProtocolParameters>;

    async fn take_measurement(&mut self, ctx: &dyn AnalyticsContext) -> eyre::Result<Self::Measurement> {
        // Ensure that we record it if either the protocol changes or we had no params
        Ok(
            (!matches!(&self.params, Some(last_params) if last_params == ctx.protocol_parameters())).then(|| {
                self.params.replace(ctx.protocol_parameters().clone());
                ctx.protocol_parameters().clone()
            }),
        )
    }
}
