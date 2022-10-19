// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! This module provides conversion methods between types while respecting the context that is the current
//! [`ProtocolParameters`](iota_types::block::protocol::ProtocolParameters).

/// The equivalent to [`TryFrom`] but with an additional context.
pub trait TryFromWithContext<T>: Sized {
    /// The type returned in the event of a conversion error.
    type Error;

    /// Performs the conversion.
    fn try_from_with_context(
        ctx: &iota_types::block::protocol::ProtocolParameters,
        value: T,
    ) -> Result<Self, Self::Error>;
}

/// The equivalent to [`TryInto`] but with an additional context.
pub trait TryIntoWithContext<T>: Sized {
    /// The type returned in the event of a conversion error.
    type Error;

    /// Performs the conversion.
    fn try_into_with_context(self, ctx: &iota_types::block::protocol::ProtocolParameters) -> Result<T, Self::Error>;
}

// TryFromWithContext implies TryIntoWithContext
impl<T, U> TryIntoWithContext<U> for T
where
    U: TryFromWithContext<T>,
{
    type Error = U::Error;

    fn try_into_with_context(self, ctx: &iota_types::block::protocol::ProtocolParameters) -> Result<U, U::Error> {
        U::try_from_with_context(ctx, self)
    }
}
