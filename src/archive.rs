// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! The format is the following:
//!
//! ```ignore
//! archived_milestones ::= first_index last_index (milestone)*
//! milestone := messages_len (message_id message)*
//! ```
//! where:
//!
//! - `first_index` is the earliest milestone index.
//! - `last_index` is the latest milestone index.
//! - `messages_len` is the length in bytes of all the message ID and message pairs in the current
//! milestone.

use std::{
    fs::File,
    io::{self, Seek, SeekFrom, Write},
    path::Path,
};

use bee_message::{milestone::MilestoneIndex, Message, MessageId};
use packable::{
    error::UnpackError,
    packer::{CounterPacker, IoPacker},
    unpacker::{CounterUnpacker, IoUnpacker},
    Packable,
};

/// Archives milestones into a file.
pub fn archive_milestones<P, E, I, F>(
    path: P,
    first_index: MilestoneIndex,
    last_index: MilestoneIndex,
    f: F,
) -> Result<(), E>
where
    P: AsRef<Path>,
    E: From<io::Error>,
    I: Iterator<Item = Result<(MessageId, Message), E>>,
    F: Fn(MilestoneIndex) -> Result<I, E>,
{
    let mut file = CounterPacker::new(IoPacker::new(File::create(path)?));

    // Write the first index
    first_index.pack(&mut file)?;
    // Write the last index
    last_index.pack(&mut file)?;

    let mut milestone_index = first_index;

    let mut backpatches = Vec::new();

    while milestone_index <= last_index {
        let milestone_iter = f(milestone_index)?;

        // The position where the total length of the messages will be written. We store it as a
        // `u64` because that is what `SeekFrom` uses.
        //
        // Panic: this is only an issue in a platform with 128-bit memory addresses.
        let pos = u64::try_from(file.counter()).unwrap();

        // Instead of computing the length of all the messages, we will write a zero and backpatch
        // it later.
        0u64.pack(&mut file)?;

        // The position before writing the messages.
        let start_pos = file.counter();

        for res in milestone_iter {
            let (message_id, message) = res?;

            // Write each message in this milestone
            message_id.pack(&mut file)?;
            // FIXME: maybe compress?
            message.pack(&mut file)?;
        }

        // The length of all the messages in this milestone. We store it as a `u64` because we will
        // use `Write::write_all` instead of `Packable::pack` to backpatch the value.
        //
        // Panic: If this panics, it would mean that the archive has a milestone that
        // most likely will not fit in memory.
        let len = u64::try_from(file.counter() - start_pos).unwrap();

        backpatches.push((pos, len));

        milestone_index = MilestoneIndex(*milestone_index + 1);
    }

    let mut file = file.into_inner().into_inner();

    for (pos, len) in backpatches {
        // Jump to the position.
        file.seek(SeekFrom::Start(pos))?;
        // Overwrite the value.
        file.write_all(&len.to_le_bytes())?;
    }

    Ok(())
}

/// FIXME: docs
pub struct Archive {
    file: File,
    first_index: MilestoneIndex,
    last_index: MilestoneIndex,
}

impl Archive {
    /// FIXME: docs
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

    /// FIXME: docs
    pub fn read_milestone(
        &mut self,
        milestone_index: MilestoneIndex,
    ) -> Result<Option<impl Iterator<Item = Result<(MessageId, Message), io::Error>> + '_>, io::Error> {
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

    /// FIXME: docs
    pub fn read_next_milestone(
        &mut self,
    ) -> Result<
        Option<(
            MilestoneIndex,
            impl Iterator<Item = Result<(MessageId, Message), io::Error>> + '_,
        )>,
        io::Error,
    > {
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
}

struct ArchivedMilestoneIter<'a> {
    file: CounterUnpacker<IoUnpacker<&'a mut File>>,
    len: usize,
}

impl<'a> Iterator for ArchivedMilestoneIter<'a> {
    type Item = Result<(MessageId, Message), io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.file.counter() == self.len {
            return None;
        }

        Some((|| {
            let message_id = MessageId::unpack::<_, true>(&mut self.file).map_err(UnpackError::into_unpacker_err)?;

            let message = Message::unpack::<_, true>(&mut self.file).map_err(|e| match e {
                UnpackError::Packable(e) => io::Error::new(io::ErrorKind::Other, e),
                UnpackError::Unpacker(e) => e,
            })?;

            Ok((message_id, message))
        })())
    }
}

#[cfg(test)]
mod tests {
    use bee_test::rand::message::rand_message;

    use super::*;

    fn generate_data(start_index: u32, end_index: u32, milestone_len: usize) -> Vec<Vec<(MessageId, Message)>> {
        (start_index..=end_index)
            .map(|_| {
                (0..milestone_len)
                    .map(|_| {
                        let msg = rand_message();
                        (msg.id(), msg)
                    })
                    .collect()
            })
            .collect()
    }

    fn archive_milestones_test(path: &'static str, start_index: u32, end_index: u32, milestone_len: usize) {
        let expected_milestones = generate_data(start_index, end_index, milestone_len);

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
        archive_milestones::<_, _, std::vec::IntoIter<Result<(MessageId, Message), io::Error>>, _>(
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
