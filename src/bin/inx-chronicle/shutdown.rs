use futures::Future;
use tokio::sync::{broadcast, mpsc};

#[derive(Clone)]
pub struct ShutdownSignal {
    /// We use an mpsc channel to help notify when all currently running tasks have
    /// gracefully shut down.
    _drop_ref: mpsc::Sender<()>,
    shutdown_tx: broadcast::Sender<()>,
    tx: mpsc::Sender<()>, // TODO: Repurpose above?
}

impl ShutdownSignal {
    /// Listens for a shutdown signal.
    pub fn listen(&self) -> impl Future<Output = ()> + Unpin {
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        // We have to pin the future here for it to work with `.take_until`.
        Box::pin(async move {
            if let Err(e) = shutdown_rx.recv().await {
                tracing::error!("listening for shutdown failed: {:?}", e);
            }
        })
    }

    /// Signals the whole application to shut down.
    pub async fn signal(&self) {
        match self.tx.send(()).await {
            Ok(_) => tracing::info!("sent shutdown request"),
            Err(_) => tracing::error!("failed to send shutdown request"),
        }
    }
}

pub struct ShutdownEmit {
    shutdown_tx: broadcast::Sender<()>,
    drop_ref: mpsc::Sender<()>,
    wait: mpsc::Receiver<()>,
}

impl ShutdownEmit {
    pub async fn emit(mut self) {
        match self.shutdown_tx.send(()) {
            Ok(n) => tracing::info!("sent `notify_shutdown` to {n} tasks"),
            Err(_) => tracing::error!("failed to send `notify_shutdown`"),
        }

        drop(self.drop_ref);

        tracing::info!("dropped `shutdown_complete_tx`, now waiting for others...");
        match self.wait.recv().await {
            Some(_) => tracing::error!("received a message, that should not happen"),
            None => tracing::info!("all other tasks shut down and the sender was dropped"),
        };
    }
}

pub fn shutdown_handles() -> (mpsc::Receiver<()>, ShutdownEmit, ShutdownSignal) {
    let (shutdown_tx, _) = broadcast::channel(1);
    let (drop_ref, shutdown_complete_rx) = mpsc::channel(1);

    let (tx, rx) = mpsc::channel(1);

    (
        rx,
        ShutdownEmit {
            shutdown_tx: shutdown_tx.clone(),
            drop_ref: drop_ref.clone(),
            wait: shutdown_complete_rx,
        },
        ShutdownSignal {
            _drop_ref: drop_ref,
            shutdown_tx,
            tx,
        },
    )
}
