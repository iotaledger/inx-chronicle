// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Stream, StreamExt};

/// Provides the ability to merge two streams, exiting when either stream is exhausted.
pub(crate) trait MergeExt: Stream {
    fn merge<S>(self, other: S) -> Merge<Self::Item>
    where
        Self: 'static + Sized + Send,
        S: 'static + Stream<Item = Self::Item> + Send,
    {
        Merge::new(self).merge(other)
    }
}
impl<S: ?Sized> MergeExt for S where S: Stream {}

/// Stream returned by the [`merge`](MergeExt::merge) method.
#[pin_project::pin_project]
pub(crate) struct Merge<I> {
    #[pin]
    streams: Vec<Box<dyn Stream<Item = I> + Send>>,
    idx: usize,
}

impl<I> Merge<I> {
    pub(crate) fn new(stream: impl Stream<Item = I> + Send + 'static) -> Merge<I> {
        Merge {
            streams: vec![Box::new(stream.fuse())],
            idx: 0,
        }
    }

    /// Merge a new stream.
    pub(crate) fn merge(&mut self, other: impl Stream<Item = I> + Send + 'static) -> &mut Self {
        self.streams.push(Box::new(other.fuse()));
        self
    }
}

impl<I> Stream for Merge<I> {
    type Item = I;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<I>> {
        use Poll::*;

        let me = self.project();

        let n = me.streams.len();
        let idx = *me.idx;
        *me.idx += 1;
        *me.idx %= n;
        let streams = me.streams.get_mut();
        for i in (0..n).cycle().skip(idx).take(n) {
            let stream = unsafe { Pin::new_unchecked(streams.get_mut(i).unwrap().as_mut()) };
            match stream.poll_next(cx) {
                Ready(Some(val)) => return Ready(Some(val)),
                Ready(None) => return Ready(None),
                Pending => (),
            }
        }
        Pending
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (mut low, mut high) = (0, Some(0usize));
        for stream in self.streams.iter() {
            let (l, h) = stream.size_hint();
            low += l;
            high = match (high, h) {
                (Some(h1), Some(h2)) => h1.checked_add(h2),
                _ => None,
            };
        }
        (low, high)
    }
}
