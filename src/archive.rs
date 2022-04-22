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
    io::{self, BufReader, Read, Seek, SeekFrom, Write},
    path::Path,
};

use bee_message::{milestone::MilestoneIndex, Message};
use packable::{
    error::UnpackError,
    packer::IoPacker,
    unpacker::{CounterUnpacker, IoUnpacker},
    Packable,
};
use zstd::{Decoder, Encoder};

/// Type used to sequentially read an archive file containing a range of milestones.
#[derive(Debug)]
pub struct Archive {
    file: File,
    start_index: MilestoneIndex,
    end_index: MilestoneIndex,
}

impl Archive {
    /// Creates a new archive file starting at the specified milestone index and overwriting any
    /// existing file with the same path.
    ///
    /// In order for the file to be well-formed it is mandatory to call [`Archive::close`] when
    /// done writing to the file.
    pub fn create(path: impl AsRef<Path>, start_index: MilestoneIndex) -> io::Result<Self> {
        let mut file = File::create(path)?;

        let mut packer = IoPacker::new(&mut file);

        (start_index, u32::MAX).pack(&mut packer)?;

        Ok(Self {
            file,
            start_index,
            end_index: start_index,
        })
    }

    /// Writes a milestone into an archive file and returns the index of such milestone if
    /// successful.
    pub fn write_milestone(&mut self, messages: Vec<Message>) -> io::Result<MilestoneIndex> {
        let mut packer = IoPacker::new(Encoder::new(vec![], 0)?);

        for message in messages {
            message.pack(&mut packer)?;
        }

        let bytes = packer.into_inner().finish()?;
        let bytes_len = u64::try_from(bytes.len()).unwrap();

        self.file.write_all(&bytes_len.to_le_bytes())?;
        self.file.write_all(&bytes)?;
        self.file.sync_data()?;

        let milestone_index = self.end_index;
        self.end_index = self.end_index + 1;

        Ok(milestone_index)
    }

    /// Closes a milestone file.
    pub fn close(&mut self) -> io::Result<()> {
        self.file.rewind()?;
        (self.start_index, self.end_index).pack(&mut IoPacker::new(&mut self.file))?;

        Ok(())
    }

    /// Opens an already existing archive file in read mode.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut file = File::open(path)?;

        let mut unpacker = IoUnpacker::new(&mut file);

        let (start_index, end_index) = <(MilestoneIndex, MilestoneIndex)>::unpack::<_, true>(&mut unpacker)
            .map_err(UnpackError::into_unpacker_err)?;

        Ok(Self {
            file,
            start_index,
            end_index,
        })
    }

    /// Reads the next milestone available in the archive and returns its index and an [`Iterator`] with the
    /// messages in the milestone.
    pub fn read_next_milestone(
        &mut self,
    ) -> io::Result<Option<(MilestoneIndex, impl Iterator<Item = io::Result<Message>> + '_)>> {
        if self.start_index >= self.end_index {
            return Ok(None);
        }

        let mut file = IoUnpacker::new(&mut self.file);

        let messages_len = u64::unpack::<_, true>(&mut file).map_err(UnpackError::into_unpacker_err)?;

        let milestone_index = self.start_index;
        self.start_index = self.start_index + 1;

        Ok(Some((
            milestone_index,
            ArchivedMilestoneIter {
                file: CounterUnpacker::new(IoUnpacker::new(Decoder::new(file.into_inner().take(messages_len))?)),
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
        if milestone_index < self.start_index || milestone_index >= self.end_index {
            Ok(None)
        } else {
            while milestone_index > self.start_index {
                let mut file = IoUnpacker::new(&mut self.file);

                let messages_len = u64::unpack::<_, true>(&mut file).map_err(UnpackError::into_unpacker_err)?;

                self.start_index = self.start_index + 1;

                // Panic: If this panics, it would mean that the archive has a milestone that
                // most likely will not fit in memory.
                self.file.seek(SeekFrom::Current(messages_len.try_into().unwrap()))?;
            }

            Ok(self.read_next_milestone()?.map(|(_, iter)| iter))
        }
    }
}

struct ArchivedMilestoneIter<'a> {
    file: CounterUnpacker<IoUnpacker<Decoder<'a, BufReader<io::Take<&'a mut File>>>>>,
}

impl<'a> Iterator for ArchivedMilestoneIter<'a> {
    type Item = io::Result<Message>;

    fn next(&mut self) -> Option<Self::Item> {
        match Message::unpack::<_, true>(&mut self.file) {
            Ok(message) => Some(Ok(message)),
            Err(err) => match err {
                UnpackError::Packable(err) => Some(Err(io::Error::new(io::ErrorKind::Other, err))),
                UnpackError::Unpacker(err) => {
                    if let io::ErrorKind::UnexpectedEof = err.kind() {
                        None
                    } else {
                        Some(Err(err))
                    }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use bee_test::rand::message::rand_message;

    use super::*;

    struct FileJanitor(&'static str);

    impl Drop for FileJanitor {
        fn drop(&mut self) {
            std::fs::remove_file(self.0).ok();
        }
    }

    fn archive_milestones_test(path: &'static str, start_index: u32, end_index: u32, milestone_len: usize) {
        let janitor = FileJanitor(path);

        let expected_milestones = (start_index..end_index)
            .map(|_| (0..milestone_len).map(|_| rand_message()).collect::<Vec<_>>())
            .collect::<Vec<_>>();

        let mut archive = Archive::create(path, MilestoneIndex(start_index)).unwrap();

        for expected_messages in &expected_milestones {
            archive.write_milestone(expected_messages.clone()).unwrap();
        }

        archive.close().unwrap();

        let mut archive = Archive::open(path).unwrap();

        for (expected_index, expected_messages) in (start_index..end_index).zip(expected_milestones) {
            let (milestone_index, mut messages) = archive.read_next_milestone().unwrap().unwrap();

            assert_eq!(MilestoneIndex(expected_index), milestone_index);

            for expected_message in expected_messages {
                assert_eq!(messages.next().unwrap().unwrap(), expected_message);
            }

            assert!(messages.next().is_none());
        }

        assert!(archive.read_next_milestone().unwrap().is_none());

        drop(janitor);
    }

    #[test]
    fn archive_zero_milestones() {
        let path = "archive_zero_milestones";

        let janitor = FileJanitor(path);

        Archive::create(path, MilestoneIndex(0)).unwrap().close().unwrap();

        let mut archive = Archive::open(path).unwrap();

        assert!(archive.read_next_milestone().unwrap().is_none());

        drop(janitor);
    }

    #[test]
    fn archive_one_milestone_one_message() {
        archive_milestones_test("archive_one_milestone_one_message", 0, 1, 1);
    }

    #[test]
    fn archive_one_milestone_several_messages() {
        archive_milestones_test("archive_one_milestone_several_messages", 0, 1, 100);
    }

    #[test]
    fn archive_several_milestones_several_messages() {
        archive_milestones_test("archive_several_milestone_several_messages", 0, 10, 100);
    }

    #[test]
    fn archive_incomplete_milestone() {
        let path = "archive_incomplete_milestone";

        let janitor = FileJanitor(path);

        let expected_messages = (0..100).map(|_| rand_message()).collect::<Vec<_>>();

        let mut archive = Archive::create(path, MilestoneIndex(0)).unwrap();

        archive.write_milestone(expected_messages.clone()).unwrap();

        let mut archive = Archive::open(path).unwrap();

        assert_eq!(0, *archive.start_index);
        assert_eq!(u32::MAX, *archive.end_index);

        let (_, mut messages) = archive.read_next_milestone().unwrap().unwrap();

        for expected_message in expected_messages {
            assert_eq!(messages.next().unwrap().unwrap(), expected_message);
        }

        drop(janitor);
    }
}
