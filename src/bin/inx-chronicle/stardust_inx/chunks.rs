// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

pub trait ChunksExt: Iterator {
    fn chunks(self, size: usize) -> IntoChunks<Self>
    where
        Self: Sized,
    {
        IntoChunks {
            inner: Arc::new(Mutex::new(GroupInner {
                key: ChunkIndex::new(size),
                iter: self,
                current_key: None,
                current_elt: None,
                done: false,
                top_group: 0,
                oldest_buffered_group: 0,
                bottom_group: 0,
                buffer: Vec::new(),
                dropped_group: !0,
            })),
            index: AtomicUsize::new(0),
        }
    }
}
impl<T: Iterator> ChunksExt for T {}

pub struct IntoChunks<I: Iterator> {
    inner: Arc<Mutex<GroupInner<usize, I, ChunkIndex>>>,
    index: AtomicUsize,
}

impl<I: Iterator> IntoChunks<I> {
    /// `client`: Index of chunk that requests next element
    fn step(&self, client: usize) -> Option<I::Item> {
        self.inner.lock().unwrap().step(client)
    }

    /// `client`: Index of chunk
    fn drop_group(&self, client: usize) {
        self.inner.lock().unwrap().drop_group(client)
    }
}

impl<'a, I: Iterator> IntoIterator for &'a IntoChunks<I> {
    type Item = Chunk<'a, I>;
    type IntoIter = Chunks<'a, I>;

    fn into_iter(self) -> Self::IntoIter {
        Chunks { parent: self }
    }
}

pub struct Chunks<'a, I: Iterator> {
    parent: &'a IntoChunks<I>,
}

impl<'a, I: Iterator> Iterator for Chunks<'a, I> {
    type Item = Chunk<'a, I>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.parent.index.fetch_add(1, Ordering::Relaxed);
        let inner = &mut *self.parent.inner.lock().unwrap();
        inner.step(index).map(|elt| Chunk {
            parent: self.parent,
            index,
            first: Some(elt),
        })
    }
}

pub struct Chunk<'a, I: Iterator> {
    parent: &'a IntoChunks<I>,
    index: usize,
    first: Option<I::Item>,
}

impl<'a, I: Iterator> Drop for Chunk<'a, I> {
    fn drop(&mut self) {
        self.parent.drop_group(self.index);
    }
}

impl<'a, I: Iterator> Iterator for Chunk<'a, I> {
    type Item = I::Item;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let elt @ Some(..) = self.first.take() {
            return elt;
        }
        self.parent.step(self.index)
    }
}

struct GroupInner<K, I, F>
where
    I: Iterator,
{
    key: F,
    iter: I,
    current_key: Option<K>,
    current_elt: Option<I::Item>,
    /// flag set if iterator is exhausted
    done: bool,
    /// Index of group we are currently buffering or visiting
    top_group: usize,
    /// Least index for which we still have elements buffered
    oldest_buffered_group: usize,
    /// Group index for `buffer[0]` -- the slots
    /// bottom_group..oldest_buffered_group are unused and will be erased when
    /// that range is large enough.
    bottom_group: usize,
    /// Buffered groups, from `bottom_group` (index 0) to `top_group`.
    buffer: Vec<std::vec::IntoIter<I::Item>>,
    /// index of last group iter that was dropped, usize::MAX == none
    dropped_group: usize,
}

impl<K, I, F> GroupInner<K, I, F>
where
    I: Iterator,
    F: for<'a> KeyFunction<&'a I::Item, Key = K>,
    K: PartialEq,
{
    /// `client`: Index of group that requests next element
    #[inline(always)]
    fn step(&mut self, client: usize) -> Option<I::Item> {
        // println!("client={}, bottom_group={}, oldest_buffered_group={}, top_group={}, buffers=[{}]",
        // client, self.bottom_group, self.oldest_buffered_group,
        // self.top_group,
        // self.buffer.iter().map(|elt| elt.len()).format(", "));
        if client < self.oldest_buffered_group {
            None
        } else if client < self.top_group
            || (client == self.top_group && self.buffer.len() > self.top_group - self.bottom_group)
        {
            self.lookup_buffer(client)
        } else if self.done {
            None
        } else if self.top_group == client {
            self.step_current()
        } else {
            self.step_buffering(client)
        }
    }

    #[inline(never)]
    fn lookup_buffer(&mut self, client: usize) -> Option<I::Item> {
        // if `bufidx` doesn't exist in self.buffer, it might be empty
        let bufidx = client - self.bottom_group;
        if client < self.oldest_buffered_group {
            return None;
        }
        let elt = self.buffer.get_mut(bufidx).and_then(|queue| queue.next());
        if elt.is_none() && client == self.oldest_buffered_group {
            // FIXME: VecDeque is unfortunately not zero allocation when empty,
            // so we do this job manually.
            // `bottom_group..oldest_buffered_group` is unused, and if it's large enough, erase it.
            self.oldest_buffered_group += 1;
            // skip forward further empty queues too
            while self
                .buffer
                .get(self.oldest_buffered_group - self.bottom_group)
                .map_or(false, |buf| buf.len() == 0)
            {
                self.oldest_buffered_group += 1;
            }

            let nclear = self.oldest_buffered_group - self.bottom_group;
            if nclear > 0 && nclear >= self.buffer.len() / 2 {
                let mut i = 0;
                self.buffer.retain(|buf| {
                    i += 1;
                    debug_assert!(buf.len() == 0 || i > nclear);
                    i > nclear
                });
                self.bottom_group = self.oldest_buffered_group;
            }
        }
        elt
    }

    /// Take the next element from the iterator, and set the done
    /// flag if exhausted. Must not be called after done.
    #[inline(always)]
    fn next_element(&mut self) -> Option<I::Item> {
        debug_assert!(!self.done);
        match self.iter.next() {
            None => {
                self.done = true;
                None
            }
            otherwise => otherwise,
        }
    }

    #[inline(never)]
    fn step_buffering(&mut self, client: usize) -> Option<I::Item> {
        // requested a later group -- walk through the current group up to
        // the requested group index, and buffer the elements (unless
        // the group is marked as dropped).
        // Because the `Groups` iterator is always the first to request
        // each group index, client is the next index efter top_group.
        debug_assert!(self.top_group + 1 == client);
        let mut group = Vec::new();

        if let Some(elt) = self.current_elt.take() {
            if self.top_group != self.dropped_group {
                group.push(elt);
            }
        }
        let mut first_elt = None; // first element of the next group

        while let Some(elt) = self.next_element() {
            let key = self.key.call_mut(&elt);
            match self.current_key.take() {
                None => {}
                Some(old_key) => {
                    if old_key != key {
                        self.current_key = Some(key);
                        first_elt = Some(elt);
                        break;
                    }
                }
            }
            self.current_key = Some(key);
            if self.top_group != self.dropped_group {
                group.push(elt);
            }
        }

        if self.top_group != self.dropped_group {
            self.push_next_group(group);
        }
        if first_elt.is_some() {
            self.top_group += 1;
            debug_assert!(self.top_group == client);
        }
        first_elt
    }

    fn push_next_group(&mut self, group: Vec<I::Item>) {
        // When we add a new buffered group, fill up slots between oldest_buffered_group and top_group
        while self.top_group - self.bottom_group > self.buffer.len() {
            if self.buffer.is_empty() {
                self.bottom_group += 1;
                self.oldest_buffered_group += 1;
            } else {
                self.buffer.push(Vec::new().into_iter());
            }
        }
        self.buffer.push(group.into_iter());
        debug_assert!(self.top_group + 1 - self.bottom_group == self.buffer.len());
    }

    /// This is the immediate case, where we use no buffering
    #[inline]
    fn step_current(&mut self) -> Option<I::Item> {
        debug_assert!(!self.done);
        if let elt @ Some(..) = self.current_elt.take() {
            return elt;
        }
        match self.next_element() {
            None => None,
            Some(elt) => {
                let key = self.key.call_mut(&elt);
                match self.current_key.take() {
                    None => {}
                    Some(old_key) => {
                        if old_key != key {
                            self.current_key = Some(key);
                            self.current_elt = Some(elt);
                            self.top_group += 1;
                            return None;
                        }
                    }
                }
                self.current_key = Some(key);
                Some(elt)
            }
        }
    }
}

impl<K, I, F> GroupInner<K, I, F>
where
    I: Iterator,
{
    /// Called when a group is dropped
    fn drop_group(&mut self, client: usize) {
        // It's only useful to track the maximal index
        if self.dropped_group == !0 || client > self.dropped_group {
            self.dropped_group = client;
        }
    }
}

#[derive(Debug)]
struct ChunkIndex {
    size: usize,
    index: usize,
    key: usize,
}

impl ChunkIndex {
    #[inline(always)]
    fn new(size: usize) -> Self {
        ChunkIndex { size, index: 0, key: 0 }
    }
}

trait KeyFunction<A> {
    type Key;
    fn call_mut(&mut self, arg: A) -> Self::Key;
}

impl<A, K, F: ?Sized> KeyFunction<A> for F
where
    F: FnMut(A) -> K,
{
    type Key = K;
    #[inline]
    fn call_mut(&mut self, arg: A) -> Self::Key {
        (*self)(arg)
    }
}

impl<A> KeyFunction<A> for ChunkIndex {
    type Key = usize;
    #[inline(always)]
    fn call_mut(&mut self, _arg: A) -> Self::Key {
        if self.index == self.size {
            self.key += 1;
            self.index = 0;
        }
        self.index += 1;
        self.key
    }
}
