//! Thin adapters to the tenant fairness core; no scheduling algorithms here.

mod init;
mod keyed;
mod tenants;

use magnus::{
    Error, Integer, RArray, RHash, Ruby, TryConvert, Value,
    scan_args::{get_kwargs, scan_args},
};
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

/// Exactly the two required keywords `keys`, converted in order.
pub(crate) fn kwargs<A: TryConvert, B: TryConvert>(
    args: &[Value],
    keys: &[&str; 2],
) -> Result<(A, B), Error> {
    let args = scan_args::<(), (), (), (), RHash, ()>(args)?;
    Ok(get_kwargs::<_, (A, B), (), ()>(args.keywords, keys, &[])?.required)
}

/// Every element of `values`, converted in order.
pub(crate) fn convert_all<T: TryConvert>(values: RArray) -> Result<Vec<T>, Error> {
    values.into_iter().map(T::try_convert).collect()
}

/// A `u32` Integer.
#[derive(Clone, Copy)]
pub(crate) struct Count(pub(crate) u32);

impl TryConvert for Count {
    fn try_convert(value: Value) -> Result<Self, Error> {
        integer(value)?.to_u32().map(Self)
    }
}

/// A positive `u32` Integer.
#[derive(Clone, Copy)]
pub(crate) struct Positive(pub(crate) NonZeroU32);

impl TryConvert for Positive {
    fn try_convert(value: Value) -> Result<Self, Error> {
        let Count(count) = Count::try_convert(value)?;
        NonZeroU32::new(count)
            .map(Self)
            .ok_or_else(|| arg_error(&Ruby::get_with(value), "expected a positive u32"))
    }
}
