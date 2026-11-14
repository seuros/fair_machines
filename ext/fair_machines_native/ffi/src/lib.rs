//! Thin adapters to the tenant fairness core; no scheduling algorithms here.

mod init;
mod keyed;
mod sealed;
mod tenants;

use fair_machines::Refusal;
use magnus::{Error, Integer, RArray, Ruby, TryConvert, Value, prelude::*};
use std::{fmt::Display, num::NonZeroU32};

/// `RangeError`: the right type, a value the core cannot be built with.
pub(crate) fn range_error(ruby: &Ruby, message: impl Into<String>) -> Error {
    Error::new(ruby.exception_range_error(), message.into())
}

/// `value` as an Integer; Floats and numeric Strings raise `TypeError`.
/// `name` is the argument and `wanted` what it takes, so the message says
/// which one was wrong.
fn integer(value: Value, name: &str, wanted: &str) -> Result<Integer, Error> {
    Integer::from_value(value).ok_or_else(|| {
        Error::new(
            Ruby::get_with(value).exception_type_error(),
            format!("{name} must be {wanted}, not {}", value.class()),
        )
    })
}

/// Ruby symbol of a refusal: the snake case of the core variant.
pub(crate) fn refusal(ruby: &Ruby, refused: Refusal) -> Value {
    ruby.to_symbol(match refused {
        Refusal::UnknownTenant => "unknown_tenant",
        Refusal::UnknownLane => "unknown_lane",
        Refusal::Insufficient => "insufficient",
        Refusal::ExceedsInFlight => "exceeds_in_flight",
    })
    .as_value()
}

/// The symbol `honoured` for `Ok(())`, the refusal's symbol otherwise.
pub(crate) fn done(ruby: &Ruby, answer: Result<(), Refusal>, honoured: &str) -> Value {
    match answer {
        Ok(()) => ruby.to_symbol(honoured).as_value(),
        Err(refused) => refusal(ruby, refused),
    }
}

/// An Integer converted to `T`, `RangeError` naming the argument and the
/// range `0..=max` when it does not fit.
fn unsigned<T: TryConvert>(value: Value, name: &str, max: impl Display) -> Result<T, Error> {
    integer(value, name, "an Integer")?;
    T::try_convert(value).map_err(|_| {
        range_error(
            &Ruby::get_with(value),
            format!("{name} must be in 0..={max}"),
        )
    })
}

/// A tenant or lane position, or a lane count: any Integer a `usize` holds.
/// The core answers for positions past the end; one the platform cannot
/// hold at all is `RangeError`.
pub(crate) fn index(value: Value, name: &str) -> Result<usize, Error> {
    unsigned(value, name, usize::MAX)
}

/// A hash: an Integer in `0..=u64::MAX`.
pub(crate) fn hash(value: Value) -> Result<u64, Error> {
    unsigned(value, "hash", u64::MAX)
}

/// A capacity, quantum or cost: the core's `NonZeroU32`, zero out of range.
fn nonzero(value: Value, name: &str, wanted: &str) -> Result<NonZeroU32, Error> {
    integer(value, name, wanted)?;
    u32::try_convert(value)
        .ok()
        .and_then(NonZeroU32::new)
        .ok_or_else(|| {
            range_error(
                &Ruby::get_with(value),
                format!("{name} must be in 1..={}", u32::MAX),
            )
        })
}

/// One positive argument.
pub(crate) fn positive(value: Value, name: &str) -> Result<NonZeroU32, Error> {
    nonzero(value, name, "an Integer")
}

/// Every element of `values`, each positive.
pub(crate) fn positives(values: RArray, name: &str) -> Result<Vec<NonZeroU32>, Error> {
    values
        .into_iter()
        .map(|value| nonzero(value, name, "Integers"))
        .collect()
}

/// Head costs: each positive, or `nil` for a tenant with nothing waiting.
pub(crate) fn heads(values: RArray) -> Result<Vec<Option<NonZeroU32>>, Error> {
    values
        .into_iter()
        .map(|value| {
            if value.is_nil() {
                Ok(None)
            } else {
                nonzero(value, "heads", "Integers or nil").map(Some)
            }
        })
        .collect()
}
