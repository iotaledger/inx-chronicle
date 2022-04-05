// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "chrysalis")]
pub mod cpt2 {
    use std::ops::{Deref, DerefMut};

    use bee_message_cpt2::{Message, MessageId};
    use bee_rest_api_cpt2::types::responses::MessageMetadataResponse;
    use serde::{Deserialize, Serialize};

    use crate::types::LedgerInclusionState;
    /// Chronicle Message record
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct MessageRecord {
        pub message_id: MessageId,
        pub message: Message,
        pub milestone_index: Option<u32>,
        pub inclusion_state: Option<LedgerInclusionState>,
        pub conflict_reason: Option<u8>,
    }

    impl MessageRecord {
        /// Create new message record
        pub fn new(message_id: MessageId, message: Message) -> Self {
            Self {
                message_id,
                message,
                milestone_index: None,
                inclusion_state: None,
                conflict_reason: None,
            }
        }

        /// Return Message id of the message
        pub fn message_id(&self) -> &MessageId {
            &self.message_id
        }

        /// Return the message
        pub fn message(&self) -> &Message {
            &self.message
        }

        /// Return referenced milestone index
        pub fn milestone_index(&self) -> Option<u32> {
            self.milestone_index
        }

        /// Return inclusion_state
        pub fn inclusion_state(&self) -> Option<&LedgerInclusionState> {
            self.inclusion_state.as_ref()
        }

        /// Return conflict_reason
        pub fn conflict_reason(&self) -> Option<u8> {
            self.conflict_reason
        }

        /// Get the message's nonce
        pub fn nonce(&self) -> u64 {
            self.message.nonce()
        }
    }

    impl Deref for MessageRecord {
        type Target = Message;

        fn deref(&self) -> &Self::Target {
            &self.message
        }
    }

    impl DerefMut for MessageRecord {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.message
        }
    }

    impl PartialOrd for MessageRecord {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for MessageRecord {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.message_id.cmp(&other.message_id)
        }
    }

    impl PartialEq for MessageRecord {
        fn eq(&self, other: &Self) -> bool {
            self.message_id == other.message_id
        }
    }
    impl Eq for MessageRecord {}

    impl From<(Message, MessageMetadataResponse)> for MessageRecord {
        fn from((message, metadata): (Message, MessageMetadataResponse)) -> Self {
            MessageRecord {
                message_id: message.id().0,
                message,
                milestone_index: metadata.referenced_by_milestone_index,
                inclusion_state: metadata.ledger_inclusion_state.map(Into::into),
                conflict_reason: metadata.conflict_reason,
            }
        }
    }
}

#[cfg(feature = "stardust")]
pub mod stardust {
    use std::ops::{Deref, DerefMut};

    use bee_message_stardust::{semantic::ConflictReason, Message, MessageId};
    use bee_rest_api_stardust::types::responses::MessageMetadataResponse;
    use serde::{Deserialize, Serialize};

    use crate::types::LedgerInclusionState;
    /// Chronicle Message record
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct MessageRecord {
        pub message_id: MessageId,
        pub message: Message,
        pub milestone_index: Option<u32>,
        pub inclusion_state: Option<LedgerInclusionState>,
        pub conflict_reason: Option<ConflictReason>,
    }

    impl MessageRecord {
        /// Create new message record
        pub fn new(message_id: MessageId, message: Message) -> Self {
            Self {
                message_id,
                message,
                milestone_index: None,
                inclusion_state: None,
                conflict_reason: None,
            }
        }
        /// Return Message id of the message
        pub fn message_id(&self) -> &MessageId {
            &self.message_id
        }

        /// Return the message
        pub fn message(&self) -> &Message {
            &self.message
        }

        /// Return referenced milestone index
        pub fn milestone_index(&self) -> Option<u32> {
            self.milestone_index
        }

        /// Return inclusion_state
        pub fn inclusion_state(&self) -> Option<&LedgerInclusionState> {
            self.inclusion_state.as_ref()
        }

        /// Return conflict_reason
        pub fn conflict_reason(&self) -> Option<&ConflictReason> {
            self.conflict_reason.as_ref()
        }

        /// Get the message's nonce
        pub fn nonce(&self) -> u64 {
            self.message.nonce()
        }
    }

    impl Deref for MessageRecord {
        type Target = Message;

        fn deref(&self) -> &Self::Target {
            &self.message
        }
    }

    impl DerefMut for MessageRecord {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.message
        }
    }

    impl PartialOrd for MessageRecord {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for MessageRecord {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.message_id.cmp(&other.message_id)
        }
    }

    impl PartialEq for MessageRecord {
        fn eq(&self, other: &Self) -> bool {
            self.message_id == other.message_id
        }
    }
    impl Eq for MessageRecord {}

    impl From<(Message, MessageMetadataResponse)> for MessageRecord {
        fn from((message, metadata): (Message, MessageMetadataResponse)) -> Self {
            MessageRecord {
                message_id: message.id(),
                message,
                milestone_index: metadata.referenced_by_milestone_index,
                inclusion_state: metadata.ledger_inclusion_state.map(Into::into),
                conflict_reason: metadata.conflict_reason.and_then(|c| c.try_into().ok()),
            }
        }
    }
}
