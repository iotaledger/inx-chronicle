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
    Packable,
};

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
/// milestone := milestone_index messages_len (message_id message)*
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

        // Write the current index
        milestone_index.pack(&mut file)?;

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
