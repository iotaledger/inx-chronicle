// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Types and utilities to archive and recover milestone ranges.
//!
//! The format is the following:
//!
//! ```ignore
//! archived_milestones ::= first_index last_index (milestone)*
//! milestone := messages_len (message)*
//! ```
//! where:
//!
//! - `first_index` is the earliest milestone index.
//! - `last_index` is the latest milestone index.
//! - `messages_len` is the length in bytes of all the messages in the current milestone.

use std::{
    fs::File,
    io::{self, Seek, SeekFrom, Write},
    path::Path,
};

use bee_message::{milestone::MilestoneIndex, Message};
use packable::{
    error::UnpackError,
    packer::{CounterPacker, IoPacker},
    unpacker::{CounterUnpacker, IoUnpacker},
    Packable,
};

/// Archives an inclusive range of milestones into a file.
pub fn archive_milestones<P, E, I, F>(
    path: P,
    first_index: MilestoneIndex,
    last_index: MilestoneIndex,
    f: F,
) -> Result<(), E>
where
    P: AsRef<Path>,
    E: From<io::Error>,
    I: Iterator<Item = Result<Message, E>>,
    F: Fn(MilestoneIndex) -> Result<I, E>,
{
    let mut file = File::create(path)?;
    let mut packer = IoPacker::new(&mut file);

    // Write the first index
    first_index.pack(&mut packer)?;
    // Write the last index
    last_index.pack(&mut packer)?;

    for milestone_index in *first_index..=*last_index {
        let messages = f(MilestoneIndex(milestone_index))?;

        // Instead of computing the length of all the messages, we will write some value and backpatch
        // it later.
        u64::MAX.pack(&mut packer)?;

        // This is the length of all the messages in this milestone.
        let messages_len = {
            let mut packer = CounterPacker::new(&mut packer);

            // FIXME: maybe compress?
            for message in messages {
                // Write each message in this milestone
                message?.pack(&mut packer)?;
            }

            packer.counter()
        };

        drop(packer);

        // Panic: seek requires an `i64` as an argument. If the byte length of the messages in the
        // current milestone does not fit in an `i64` there is not much we can do.
        let offset = i64::try_from(messages_len).unwrap();
        // Panic: This is only an issue in 128-bit platforms.
        let bytes = u64::try_from(messages_len).unwrap().to_le_bytes();
        // Panic: This value always fits in an `i64`.
        let bytes_len = i64::try_from(bytes.len()).unwrap();

        file.seek(SeekFrom::Current(-(offset + bytes_len)))?;

        file.write_all(&bytes)?;

        file.seek(SeekFrom::Current(offset))?;

        packer = IoPacker::new(&mut file);
    }

    Ok(())
}

/// Type used to sequentially read an archive file containing a range of milestones.
pub struct Archive {
    file: File,
    first_index: MilestoneIndex,
    last_index: MilestoneIndex,
}

impl Archive {
    /// Opens an already existing archive file.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, io::Error> {
        let mut file = File::open(path)?;

        let mut unpacker = IoUnpacker::new(&mut file);

        let first_index = MilestoneIndex::unpack::<_, true>(&mut unpacker).map_err(UnpackError::into_unpacker_err)?;
        let last_index = MilestoneIndex::unpack::<_, true>(&mut unpacker).map_err(UnpackError::into_unpacker_err)?;

        Ok(Self {
            file,
            first_index,
            last_index,
        })
    }

    /// Reads the next milestone available in the archive and returns its index and an [`Iterator`] with the
    /// messages in the milestone.
    pub fn read_next_milestone(
        &mut self,
    ) -> io::Result<Option<(MilestoneIndex, impl Iterator<Item = io::Result<Message>> + '_)>> {
        if self.first_index > self.last_index {
            return Ok(None);
        }

        let mut file = IoUnpacker::new(&mut self.file);

        let messages_len = u64::unpack::<_, true>(&mut file).map_err(UnpackError::into_unpacker_err)?;

        let milestone_index = self.first_index;
        self.first_index = self.first_index + 1;

        Ok(Some((
            milestone_index,
            ArchivedMilestoneIter {
                file: CounterUnpacker::new(file),
                // Panic: If this panics, it would mean that the archive has a milestone that
                // most likely will not fit in memory.
                len: messages_len.try_into().unwrap(),
            },
        )))
    }

    /// Finds a milestone in the archive and and an [`Iterator`] with the messages in the
    /// milestone.
    ///
    /// This function can only read forward, meaning that any milestone that was already read or
    /// skipped over cannot be found by it.
    pub fn read_milestone(
        &mut self,
        milestone_index: MilestoneIndex,
    ) -> io::Result<Option<impl Iterator<Item = io::Result<Message>> + '_>> {
        if milestone_index < self.first_index || milestone_index > self.last_index {
            Ok(None)
        } else {
            while milestone_index > self.first_index {
                let mut file = IoUnpacker::new(&mut self.file);

                let messages_len = u64::unpack::<_, true>(&mut file).map_err(UnpackError::into_unpacker_err)?;

                self.first_index = self.first_index + 1;

                // Panic: If this panics, it would mean that the archive has a milestone that
                // most likely will not fit in memory.
                self.file.seek(SeekFrom::Current(messages_len.try_into().unwrap()))?;
            }

            Ok(self.read_next_milestone()?.map(|(_, iter)| iter))
        }
    }
}

struct ArchivedMilestoneIter<'a> {
    file: CounterUnpacker<IoUnpacker<&'a mut File>>,
    len: usize,
}

impl<'a> Iterator for ArchivedMilestoneIter<'a> {
    type Item = io::Result<Message>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.file.counter() == self.len {
            return None;
        }

        Some(Message::unpack::<_, true>(&mut self.file).map_err(|e| match e {
            UnpackError::Packable(e) => io::Error::new(io::ErrorKind::Other, e),
            UnpackError::Unpacker(e) => e,
        }))
    }
}

#[cfg(test)]
mod tests {
    use bee_test::rand::message::rand_message;

    use super::*;

    fn archive_milestones_test(path: &'static str, start_index: u32, end_index: u32, milestone_len: usize) {
        let expected_milestones = (start_index..=end_index)
            .map(|_| (0..milestone_len).map(|_| rand_message()).collect::<Vec<_>>())
            .collect::<Vec<_>>();

        archive_milestones(path, MilestoneIndex(start_index), MilestoneIndex(end_index), |index| {
            let index = usize::try_from(*index - start_index).unwrap();

            io::Result::Ok(expected_milestones[index].clone().into_iter().map(Ok))
        })
        .unwrap();

        let mut archive = Archive::open(path).unwrap();

        for (expected_index, expected_messages) in (start_index..=end_index).zip(expected_milestones) {
            let (milestone_index, mut messages) = archive.read_next_milestone().unwrap().unwrap();

            assert_eq!(MilestoneIndex(expected_index), milestone_index);

            for expected_message in expected_messages {
                assert_eq!(messages.next().unwrap().unwrap(), expected_message);
            }

            assert!(messages.next().is_none());
        }

        assert!(archive.read_next_milestone().unwrap().is_none());
    }

    #[test]
    fn archive_zero_milestones() {
        archive_milestones::<_, _, std::vec::IntoIter<io::Result<Message>>, _>(
            "/tmp/archive",
            MilestoneIndex(1),
            MilestoneIndex(0),
            |_| unreachable!(),
        )
        .unwrap();

        let mut archive = Archive::open("/tmp/archive").unwrap();

        assert!(archive.read_next_milestone().unwrap().is_none());
    }

    #[test]
    fn archive_one_milestone_one_message() {
        archive_milestones_test("/tmp/archive_one_milestone_one_message", 0, 0, 1);
    }

    #[test]
    fn archive_one_milestone_several_messages() {
        archive_milestones_test("/tmp/archive_one_milestone_several_messages", 0, 0, 100);
    }

    #[test]
    fn archive_several_milestones_several_messages() {
        archive_milestones_test("/tmp/archive_several_milestone_several_messages", 0, 10, 100);
    }
}
