// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use tokio::sync::oneshot;

pub(crate) async fn shutdown_signal_listener() {
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    #[cfg(unix)]
    tokio::spawn(async move {
        use futures::future;
        use tokio::signal::unix::{signal, Signal, SignalKind};

        // Panic: none of the possible error conditions should happen.
        let mut signals = vec![SignalKind::interrupt(), SignalKind::terminate()]
            .iter()
            .map(|kind| signal(*kind).unwrap())
            .collect::<Vec<Signal>>();

        let signal_futures = signals.iter_mut().map(|signal| Box::pin(signal.recv()));

        let (signal_event, _, _) = future::select_all(signal_futures).await;

        if signal_event.is_none() {
            panic!("Shutdown signal stream failed, channel may have closed.");
        } else {
            send_shutdown_signal(shutdown_tx);
        }
    });

    #[cfg(not(unix))]
    tokio::spawn(async move {
        if let Err(e) = tokio::signal::ctrl_c().await {
            panic!("Failed to intercept CTRL-C: {:?}.", e);
        } else {
            send_shutdown_signal(shutdown_tx);
        }
    });

    if let Err(e) = shutdown_rx.await {
        log::warn!("awaiting shutdown failed: {:?}", e);
    }
}

fn send_shutdown_signal(shutdown_tx: oneshot::Sender<()>) {
    log::warn!("Gracefully shutting down Chronicle, this may take some time.");

    if let Err(e) = shutdown_tx.send(()) {
        panic!("Failed to send the shutdown signal: {:?}", e);
    }
}
