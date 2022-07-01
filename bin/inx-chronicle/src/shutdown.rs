// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub(crate) async fn shutdown_signal_listener() {
    #[cfg(unix)]
    {
        use futures::future;
        use tokio::signal::unix::{signal, Signal, SignalKind};

        // Panic: none of the possible error conditions should happen.
        let mut signals = vec![SignalKind::interrupt(), SignalKind::terminate()]
            .iter()
            .map(|kind| signal(*kind).unwrap())
            .collect::<Vec<Signal>>();
        let signal_futs = signals.iter_mut().map(|signal| Box::pin(signal.recv()));
        let (signal_event, _, _) = future::select_all(signal_futs).await;

        if signal_event.is_none() {
            panic!("Shutdown signal stream failed, channel may have closed.");
        }
    }
    #[cfg(not(unix))]
    {
        if let Err(e) = tokio::signal::ctrl_c().await {
            panic!("Failed to intercept CTRL-C: {:?}.", e);
        }
    }
}
