// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod mpsc {
    #[cfg(feature = "metrics")]
    pub(crate) use bee_metrics::metrics::sync::mpsc::{unbounded_channel, UnboundedReceiverStream, UnboundedSender};
    #[cfg(not(feature = "metrics"))]
    pub(crate) use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
    #[cfg(not(feature = "metrics"))]
    pub(crate) use tokio_stream::wrappers::UnboundedReceiverStream;
}
