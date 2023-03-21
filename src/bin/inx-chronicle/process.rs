// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub async fn interrupt_or_terminate() -> eyre::Result<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sigint = signal(SignalKind::interrupt())?;

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

        ctrl_c().await?;
        tracing::info!("received `CTRL-C`, sending shutdown signal")
    }

    Ok(())
}
