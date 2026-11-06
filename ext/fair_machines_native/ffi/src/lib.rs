//! Thin adapters to the tenant fairness core; no scheduling algorithms here.

mod init;
mod tenants;

use magnus::{Error, Integer, Ruby, TryConvert, Value};
use std::num::NonZeroU32;

#[derive(Clone, Copy)]
pub(crate) struct Count(pub(crate) u32);

impl TryConvert for Count {
    fn try_convert(value: Value) -> Result<Self, Error> {
        if Integer::from_value(value).is_none() {
            return Err(Error::new(
                Ruby::get_with(value).exception_type_error(),
                "expected an Integer",
            ));
        }
        u32::try_convert(value).map(Self)
    }
}

pub(crate) fn positive(ruby: &Ruby, count: Count) -> Result<NonZeroU32, Error> {
    NonZeroU32::new(count.0)
        .ok_or_else(|| Error::new(ruby.exception_arg_error(), "expected a positive u32"))
}
