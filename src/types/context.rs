// Copyright 2022 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

trait Context: TryInto<bee_block_stardust::protocol::ProtocolParameters> {
    fn token_supply(&self) -> u64;
    fn min_pow_score(&self) -> u64;
}

/// The equivalent to [`TryFrom`] but with an additional context.
pub trait TryFromWithContext<C: Context, T>: Sized {
    /// The type returned in the event of a conversion error.
    type Error;

    /// Performs the conversion.
    fn try_from_with_context(ctx: &C, value: T) -> Result<Self, Self::Error>;
}

/// The equivalent to [`TryInto`] but with an additional context.
pub trait TryIntoWithContext<Ctx, T>: Sized {
    /// The type returned in the event of a conversion error.
    type Error;

    /// Performs the conversion.
    fn try_into_with_context(self, ctx: &Ctx) -> Result<T, Self::Error>;
}

// TryFromWithContext implies TryIntoWithContext
impl<C: Context, T, U> TryIntoWithContext<C, U> for T
where
    U: TryFromWithContext<C, T>,
{
    type Error = U::Error;

    fn try_into_with_context(self, ctx: &C) -> Result<U, U::Error> {
        U::try_from_with_context(ctx, self)
    }
}
