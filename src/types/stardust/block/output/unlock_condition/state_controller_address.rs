use std::borrow::Borrow;

use iota_types::block::output::unlock_condition as iota;
use serde::{Deserialize, Serialize};

use crate::types::stardust::block::Address;

/// Defines the State Controller Address that owns this output, that is, it can unlock it with the proper Unlock in a
/// transaction that state transitions the alias output.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateControllerAddressUnlockCondition {
    /// The associated address of this [`StateControllerAddressUnlockCondition`].
    pub address: Address,
}

impl<T: Borrow<iota::StateControllerAddressUnlockCondition>> From<T> for StateControllerAddressUnlockCondition {
    fn from(value: T) -> Self {
        Self {
            address: value.borrow().address().into(),
        }
    }
}

impl From<StateControllerAddressUnlockCondition> for iota::StateControllerAddressUnlockCondition {
    fn from(value: StateControllerAddressUnlockCondition) -> Self {
        Self::new(value.address.into())
    }
}

impl From<StateControllerAddressUnlockCondition> for iota::dto::StateControllerAddressUnlockConditionDto {
    fn from(value: StateControllerAddressUnlockCondition) -> Self {
        Self {
            kind: iota::StateControllerAddressUnlockCondition::KIND,
            address: value.address.into(),
        }
    }
}

#[cfg(feature = "rand")]
mod rand {
    use super::*;

    impl StateControllerAddressUnlockCondition {
        /// Generates a random [`StateControllerAddressUnlockCondition`].
        pub fn rand() -> Self {
            Self {
                address: Address::rand_ed25519(),
            }
        }
    }
}
