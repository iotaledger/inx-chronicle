// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[derive(Clone, Debug, Default)]
pub(crate) struct ProtocolParamsMeasurement {
    params: Option<ProtocolParameters>,
}

impl Analytics for ProtocolParamsMeasurement {
    type Measurement = ProtocolParameters;

    fn end_milestone(&mut self, ctx: &dyn AnalyticsContext) -> Option<Self::Measurement> {
        // Ensure that we record it if either the protocol changes or we had no params
        (!matches!(&self.params, Some(last_params) if last_params == ctx.protocol_params())).then(|| {
            self.params.replace(ctx.protocol_params().clone());
            ctx.protocol_params().clone()
        })
    }
}
