// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

use futures::{
    future::{self, FusedFuture},
    stream::{self, FusedStream},
    task::AtomicWaker,
    Future, FutureExt, Stream, StreamExt,
};

#[derive(Default, Debug)]
pub(crate) struct ShutdownFlag {
    waker: AtomicWaker,
    set: AtomicBool,
}

impl ShutdownFlag {
    pub(crate) fn signal(&self) {
        self.set.store(true, Ordering::SeqCst);
        self.waker.wake();
    }
}

/// A handle which can be invoked to shutdown an actor.
#[derive(Clone, Default, Debug)]
pub struct ShutdownHandle {
    flag: Arc<ShutdownFlag>,
}

impl ShutdownHandle {
    /// Notifies the listener of this handle that it should shut down.
    pub fn shutdown(&self) {
        self.flag.signal()
    }
}

impl Future for ShutdownHandle {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        // Quick check to avoid registration if already done.
        if self.flag.set.load(Ordering::SeqCst) {
            return Poll::Ready(());
        }

        self.flag.waker.register(cx.waker());

        // Need to check condition **after** `register` to avoid a race
        // condition that would result in lost notifications.
        if self.flag.set.load(Ordering::SeqCst) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

/// A stream with a shutdown.
///
/// This type wraps a shutdown receiver and a stream to produce a new stream that ends when the
/// shutdown receiver is triggered or when the stream ends.
#[derive(Debug)]
pub struct ShutdownStream<S> {
    shutdown: future::Fuse<ShutdownHandle>,
    stream: stream::Fuse<S>,
}

impl<S: Stream> ShutdownStream<S> {
    /// Create a new `ShutdownStream` from a shutdown receiver and an unfused stream.
    ///
    /// This method receives the stream to be wrapped and a `oneshot::Receiver` for the shutdown.
    /// Both the stream and the shutdown receiver are fused to avoid polling already completed
    /// futures.
    pub fn new(stream: S) -> (Self, ShutdownHandle) {
        let handle = ShutdownHandle::default();
        (
            Self {
                shutdown: handle.clone().fuse(),
                stream: stream.fuse(),
            },
            handle,
        )
    }
}

impl<S: Stream<Item = T> + Unpin, T> Stream for ShutdownStream<S> {
    type Item = T;
    /// The shutdown receiver is polled first, if it is not ready, the stream is polled. This
    /// guarantees that checking for shutdown always happens first.
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if self.is_terminated() {
            Poll::Ready(None)
        } else {
            if self.shutdown.poll_unpin(cx).is_ready() {
                return Poll::Ready(None);
            }

            self.stream.poll_next_unpin(cx)
        }
    }
}

impl<S: Stream<Item = T> + Unpin, T> FusedStream for ShutdownStream<S> {
    fn is_terminated(&self) -> bool {
        self.shutdown.is_terminated() || self.stream.is_terminated()
    }
}
