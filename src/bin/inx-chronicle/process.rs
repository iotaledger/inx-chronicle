// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub async fn interrupt_or_terminate() -> eyre::Result<()> {
    #[cfg(unix)]
    {
        use eyre::eyre;
        use tokio::signal::unix::{signal, SignalKind};
        let mut terminate = signal(SignalKind::terminate()).map_err(|e| eyre!("cannot listen to `SIGTERM`: {e}"))?;
        let mut interrupt = signal(SignalKind::interrupt()).map_err(|e| eyre!("cannot listen to `SIGINT`: {e}"))?;
        tokio::select! {
            _ = terminate.recv() => {}
            _ = interrupt.recv() => {}
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await?;

    Ok(())
}
