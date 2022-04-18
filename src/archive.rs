// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fs::File, io, path::Path};

use bee_message::{milestone::MilestoneIndex, Message, MessageId};
use packable::{packer::IoPacker, Packable};

/// Archives milestones into a file.
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
    let mut file = IoPacker::new(File::create(path)?);

    // Write the first index
    first_index.pack(&mut file)?;
    // Write the last index
    last_index.pack(&mut file)?;

    let mut milestone_index = first_index;

    while milestone_index <= last_index {
        let milestone_iter = f(milestone_index)?;

        // Write the current index
        milestone_index.pack(&mut file)?;

        for res in milestone_iter {
            let (message_id, message) = res?;

            // Write each message in this milestone
            message_id.pack(&mut file)?;
            // FIXME: maybe compress?
            message.pack(&mut file)?;
        }

        milestone_index = MilestoneIndex(*milestone_index + 1);
    }

    Ok(())
}
