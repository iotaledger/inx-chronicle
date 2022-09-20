use tokio::{
    signal::unix::{signal, SignalKind},
    sync::mpsc,
};

pub async fn interupt_or_terminate() -> mpsc::UnboundedReceiver<()> {
    let (shutdown_send, shutdown_recv) = mpsc::unbounded_channel();

    let _ = tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).expect("cannot listen to `SIGTERM`");
        let mut sigint = signal(SignalKind::interrupt()).expect("cannot listen to `SIGINT`");

        tokio::select! {
            _ = sigterm.recv() => {
                tracing::trace!("recived `SIGTERM`, sending shutdown signal")
            }
            _ = sigint.recv() => {
                tracing::trace!("recived `SIGTERM`, sending shutdown signal")
            }
        }

        shutdown_send.send(()).expect("Could not send shutdown signal");
    })
    .await;

    shutdown_recv
}
