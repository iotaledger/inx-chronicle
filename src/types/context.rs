// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// The equivalent to [`TryFrom`] but with an additional context.
pub trait TryFromWithContext<Ctx, T>: Sized {
    /// The type returned in the event of a conversion error.
    type Error;

    /// Performs the conversion.
    fn try_from_with_context(ctx: &Ctx, value: T) -> Result<Self, Self::Error>;
}

/// The equivalent to [`TryInto`] but with an additional context.
pub trait TryIntoWithContext<Ctx, T>: Sized {
    /// The type returned in the event of a conversion error.
    type Error;

    /// Performs the conversion.
    fn try_into_with_context(self, ctx: &Ctx) -> Result<T, Self::Error>;
}

// TryFromWithContext implies TryIntoWithContext
impl<Ctx, T, U> TryIntoWithContext<Ctx, U> for T
where
    U: TryFromWithContext<Ctx, T>,
{
    type Error = U::Error;

    fn try_into_with_context(self, ctx: &Ctx) -> Result<U, U::Error> {
        U::try_from_with_context(ctx, self)
    }
}
