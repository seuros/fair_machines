//! Thin adapters to the tenant fairness core; no scheduling algorithms here.

mod init;
mod keyed;
mod tenants;

use fair_machines::Refusal;
use magnus::{Error, RArray, Ruby, Value, prelude::*};
use matryona_machines::ruby::{Symbols, positive, symbol};
use std::num::NonZeroU32;

matryona_machines::symbols! {
    /// The core's refusals as Symbols: the snake case of each variant.
    pub(crate) Refusals for Refusal as "a refusal" {
        UnknownTenant => "unknown_tenant",
        UnknownLane => "unknown_lane",
        Insufficient => "insufficient",
        ExceedsInFlight => "exceeds_in_flight",
    }
}

/// `RangeError`: the right type, a value the core cannot be built with.
pub(crate) fn range_error(ruby: &Ruby, message: impl Into<String>) -> Error {
    Error::new(ruby.exception_range_error(), message.into())
}

/// Ruby symbol of a refusal: the snake case of the core variant.
pub(crate) fn refusal(ruby: &Ruby, refused: Refusal) -> Value {
    Refusals::symbol(ruby, refused).as_value()
}

/// The symbol `honoured` for `Ok(())`, the refusal's symbol otherwise.
pub(crate) fn done(ruby: &Ruby, answer: Result<(), Refusal>, honoured: &str) -> Value {
    match answer {
        Ok(()) => symbol(ruby, honoured).as_value(),
        Err(refused) => refusal(ruby, refused),
    }
}

/// Every element of `values`, each a positive `u32`.
pub(crate) fn positives(values: RArray, name: &str) -> Result<Vec<NonZeroU32>, Error> {
    values
        .into_iter()
        .map(|value| positive(value, name))
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
                positive(value, "heads").map(Some)
            }
        })
        .collect()
}
