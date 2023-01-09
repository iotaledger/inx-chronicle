//! Module containing the tangle models.

mod milestone;
mod protocol;

pub use self::{
    milestone::MilestoneIndex,
    protocol::{ProtocolParameters, RentStructure},
};
