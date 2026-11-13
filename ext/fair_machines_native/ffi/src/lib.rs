//! Thin adapters to the tenant fairness core; no scheduling algorithms here.

mod init;
mod keyed;
mod tenants;

use fair_machines::Refusal;
use magnus::{Error, Integer, RArray, Ruby, TryConvert, Value, prelude::*};
use std::num::NonZeroU32;

pub(crate) fn arg_error(ruby: &Ruby, message: &'static str) -> Error {
    Error::new(ruby.exception_arg_error(), message)
}

/// `value` as an Integer; Floats and numeric Strings raise `TypeError`.
pub(crate) fn integer(value: Value) -> Result<Integer, Error> {
    Integer::from_value(value).ok_or_else(|| {
        Error::new(
            Ruby::get_with(value).exception_type_error(),
            "expected an Integer",
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

/// Every element of `values`, converted in order.
pub(crate) fn convert_all<T: TryConvert>(values: RArray) -> Result<Vec<T>, Error> {
    values.into_iter().map(T::try_convert).collect()
}

/// A tenant or lane position: an Integer in `0..2**64`. One the platform
/// cannot index names nothing, like any other index past the end.
#[derive(Clone, Copy)]
pub(crate) struct Index(pub(crate) usize);

impl TryConvert for Index {
    fn try_convert(value: Value) -> Result<Self, Error> {
        let index = integer(value)?.to_u64()?;
        Ok(Self(usize::try_from(index).unwrap_or(usize::MAX)))
    }
}

/// A capacity, quantum or cost: an Integer in `1..2**32`.
#[derive(Clone, Copy)]
pub(crate) struct Positive(pub(crate) NonZeroU32);

impl TryConvert for Positive {
    fn try_convert(value: Value) -> Result<Self, Error> {
        NonZeroU32::new(integer(value)?.to_u32()?)
            .map(Self)
            .ok_or_else(|| {
                Error::new(
                    Ruby::get_with(value).exception_range_error(),
                    "expected a positive Integer",
                )
            })
    }
}
