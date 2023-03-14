// Copyright 2023 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use influxdb::WriteQuery;

use super::*;
use crate::analytics::measurement::Measurement;

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

impl Measurement for ProtocolParameters {
    const NAME: &'static str = "stardust_protocol_params";

    fn add_fields(&self, query: WriteQuery) -> WriteQuery {
        query
            .add_field("token_supply", self.token_supply)
            .add_field("min_pow_score", self.min_pow_score)
            .add_field("below_max_depth", self.below_max_depth)
            .add_field("v_byte_cost", self.rent_structure.v_byte_cost)
            .add_field("v_byte_factor_key", self.rent_structure.v_byte_factor_key)
            .add_field("v_byte_factor_data", self.rent_structure.v_byte_factor_data)
    }
}
