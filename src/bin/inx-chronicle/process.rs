// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use tokio::{
    signal::unix::{signal, SignalKind},
    sync::mpsc,
};

pub fn interrupt_or_terminate() -> mpsc::UnboundedReceiver<()> {
    let (shutdown_send, shutdown_recv) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).expect("cannot listen to `SIGTERM`");
        let mut sigint = signal(SignalKind::interrupt()).expect("cannot listen to `SIGINT`");

        tokio::select! {
            _ = sigterm.recv() => {
                tracing::info!("received `SIGTERM`, sending shutdown signal")
            }
            _ = sigint.recv() => {
                tracing::info!("received `SIGTERM`, sending shutdown signal")
            }
        }

        shutdown_send.send(()).expect("Could not send shutdown signal");
    });

    shutdown_recv
}
