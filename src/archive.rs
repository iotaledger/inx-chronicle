// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{
    fs::File,
    io::{self, Seek, SeekFrom, Write},
    path::Path,
};

use bee_message::{milestone::MilestoneIndex, Message, MessageId};
use packable::{
    packer::{IoPacker, Packer},
    unpacker::{IoUnpacker, Unpacker},
    Packable,
};

/// An unpacker backed by a `File` that tracks the position of the file cursor.
struct FileUnpacker<'a> {
    inner: IoUnpacker<&'a mut File>,
    pos: usize,
}

impl<'a> FileUnpacker<'a> {
    fn new(inner: IoUnpacker<&'a mut File>, pos: usize) -> Self {
        Self { inner, pos }
    }
}

impl<'a> Unpacker for FileUnpacker<'a> {
    type Error = <IoUnpacker<&'a mut File> as Unpacker>::Error;

    #[inline]
    fn unpack_bytes<B: AsMut<[u8]>>(&mut self, mut bytes: B) -> Result<(), Self::Error> {
        let bytes = bytes.as_mut();
        self.pos -= bytes.len();

        self.inner.unpack_bytes(bytes)?;

        Ok(())
    }
}

/// A packer backed by a `File` that tracks the position of the file cursor.
struct FilePacker {
    inner: IoPacker<File>,
    pos: usize,
}

impl FilePacker {
    fn create<P: AsRef<Path>>(path: &P) -> Result<Self, io::Error> {
        Ok(Self {
            inner: IoPacker::new(File::create(path)?),
            pos: 0,
        })
    }
}

impl Packer for FilePacker {
    type Error = <IoPacker<File> as Packer>::Error;

    #[inline]
    fn pack_bytes<B: AsRef<[u8]>>(&mut self, bytes: B) -> Result<(), Self::Error> {
        let bytes = bytes.as_ref();
        self.pos += bytes.len();

        self.inner.pack_bytes(bytes)?;

        Ok(())
    }
}

/// Archives milestones into a file.
///
/// The format is the following:
///
/// ```ignore
/// archived_milestones ::= first_index last_index (milestone)*
/// milestone := messages_len (message_id message)*
/// ```
/// where:
///
/// - `first_index` is the earliest milestone index.
/// - `last_index` is the latest milestone index.
/// - `messages_len` is the length in bytes of all the message ID and message pairs in the current
/// milestone.
pub fn archive_milestones<P, E, I, F>(
    path: &P,
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
    let mut file = FilePacker::create(path)?;

    // Write the first index
    first_index.pack(&mut file)?;
    // Write the last index
    last_index.pack(&mut file)?;

    let mut milestone_index = first_index;

    let mut backpatches = Vec::new();

    while milestone_index <= last_index {
        let milestone_iter = f(milestone_index)?;

        // FIXME: unwrap
        // The position where the total length of the messages will be written. We store it as a
        // `u64` because that is what `SeekFrom` uses.
        let pos = u64::try_from(file.pos).unwrap();

        // Instead of computing the length of all the messages, we will write a zero and backpatch
        // it later.
        0u64.pack(&mut file)?;

        // The position before writing the messages.
        let start_pos = file.pos;

        for res in milestone_iter {
            let (message_id, message) = res?;

            // Write each message in this milestone
            message_id.pack(&mut file)?;
            // FIXME: maybe compress?
            message.pack(&mut file)?;
        }

        // FIXME: unwrap
        // The length of all the messages in this milestone. We store it as a `u64` because we will
        // use `Write::write_all` instead of `Packable::pack` to backpatch the value.
        let len = u64::try_from(file.pos - start_pos).unwrap();

        backpatches.push((pos, len));

        milestone_index = MilestoneIndex(*milestone_index + 1);
    }

    let mut file = file.inner.into_inner();

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
    pub fn open<P: AsRef<Path>>(path: &P) -> Result<Self, io::Error> {
        let mut file = File::open(path)?;

        let mut unpacker = IoUnpacker::new(&mut file);
        // FIXME: unwrap
        let first_index = MilestoneIndex::unpack::<_, true>(&mut unpacker).unwrap();
        // FIXME: unwrap
        let last_index = MilestoneIndex::unpack::<_, true>(&mut unpacker).unwrap();

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
    ) -> Option<Result<impl Iterator<Item = Result<(MessageId, Message), io::Error>> + '_, io::Error>> {
        if milestone_index < self.first_index || milestone_index > self.last_index {
            None
        } else {
            while milestone_index > self.first_index {
                let mut file = IoUnpacker::new(&mut self.file);

                // FIXME: unwrap
                let len = usize::try_from(u64::unpack::<_, true>(&mut file).unwrap()).unwrap();

                self.first_index = MilestoneIndex(*self.first_index + 1);

                // FIXME: unwrap
                self.file.seek(SeekFrom::Current(len.try_into().unwrap())).unwrap();
            }

            Some(self.read_next_milestone().unwrap().map(|(_, iter)| iter))
        }
    }

    /// FIXME: docs
    pub fn read_next_milestone(
        &mut self,
    ) -> Option<
        Result<
            (
                MilestoneIndex,
                impl Iterator<Item = Result<(MessageId, Message), io::Error>> + '_,
            ),
            io::Error,
        >,
    > {
        if self.first_index > self.last_index {
            return None;
        }

        let mut file = IoUnpacker::new(&mut self.file);

        // FIXME: unwrap
        let len = usize::try_from(u64::unpack::<_, true>(&mut file).unwrap()).unwrap();

        let milestone_index = self.first_index;
        self.first_index = MilestoneIndex(*self.first_index + 1);

        Some(Ok((
            milestone_index,
            ArchivedMilestoneIter {
                file: FileUnpacker::new(file, len),
            },
        )))
    }
}

struct ArchivedMilestoneIter<'a> {
    file: FileUnpacker<'a>,
}

impl<'a> Iterator for ArchivedMilestoneIter<'a> {
    type Item = Result<(MessageId, Message), io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.file.pos == 0 {
            return None;
        }

        // FIXME: unwrap
        let message_id = MessageId::unpack::<_, true>(&mut self.file).unwrap();
        // FIXME: unwrap
        let message = Message::unpack::<_, true>(&mut self.file).unwrap();

        Some(Ok((message_id, message)))
    }
}
