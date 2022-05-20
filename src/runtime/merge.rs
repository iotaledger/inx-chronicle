// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{stream::Fuse, Stream, StreamExt};

/// Provides the ability to merge two streams, exiting when either stream is exhausted.
pub(crate) trait MergeExt: Stream {
    fn merge<S>(self, other: S) -> Merge<Self, S>
    where
        Self: Sized,
        S: Stream<Item = Self::Item>,
    {
        Merge::new(self, other)
    }
}
impl<S: ?Sized> MergeExt for S where S: Stream {}

/// Stream returned by the [`merge`](MergeExt::merge) method.
#[pin_project::pin_project]
pub(crate) struct Merge<T, U> {
    #[pin]
    a: Fuse<T>,
    #[pin]
    b: Fuse<U>,
    // When `true`, poll `a` first, otherwise, `poll` b`.
    a_first: bool,
}

impl<T, U> Merge<T, U> {
    pub(super) fn new(a: T, b: U) -> Merge<T, U>
    where
        T: Stream,
        U: Stream,
    {
        Merge {
            a: a.fuse(),
            b: b.fuse(),
            a_first: true,
        }
    }
}

impl<T, U> Stream for Merge<T, U>
where
    T: Stream,
    U: Stream<Item = T::Item>,
{
    type Item = T::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T::Item>> {
        let me = self.project();
        let a_first = *me.a_first;

        // Toggle the flag
        *me.a_first = !a_first;

        if a_first {
            poll_next(me.a, me.b, cx)
        } else {
            poll_next(me.b, me.a, cx)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        merge_size_hints(self.a.size_hint(), self.b.size_hint())
    }
}

fn poll_next<T, U>(first: Pin<&mut T>, second: Pin<&mut U>, cx: &mut Context<'_>) -> Poll<Option<T::Item>>
where
    T: Stream,
    U: Stream<Item = T::Item>,
{
    use Poll::*;

    match first.poll_next(cx) {
        Ready(Some(val)) => return Ready(Some(val)),
        Ready(None) => return Ready(None),
        Pending => (),
    }

    match second.poll_next(cx) {
        Ready(Some(val)) => return Ready(Some(val)),
        Ready(None) => return Ready(None),
        Pending => return Pending,
    }
}

/// Merge the size hints from two streams.
fn merge_size_hints(
    (left_low, left_high): (usize, Option<usize>),
    (right_low, right_hign): (usize, Option<usize>),
) -> (usize, Option<usize>) {
    let low = left_low.saturating_add(right_low);
    let high = match (left_high, right_hign) {
        (Some(h1), Some(h2)) => h1.checked_add(h2),
        _ => None,
    };
    (low, high)
}
