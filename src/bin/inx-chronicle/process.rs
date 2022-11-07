// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub async fn interrupt_or_terminate() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm = signal(SignalKind::terminate()).expect("cannot listen to `SIGTERM`");
        let mut sigint = signal(SignalKind::interrupt()).expect("cannot listen to `SIGINT`");

        tokio::select! {
            _ = sigterm.recv() => {
                tracing::info!("received `SIGTERM`, sending shutdown signal")
            }
            _ = sigint.recv() => {
                tracing::info!("received `SIGINT`, sending shutdown signal")
            }
        }
    }
    #[cfg(not(unix))]
    {
        use tokio::signal::ctrl_c;

        ctrl_c().await.expect("cannot listen to `CTRL-C`");
        tracing::info!("received `CTRL-C`, sending shutdown signal")
    }
}
