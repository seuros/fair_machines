//! Helpers shared by the crate's test modules.

use core::{fmt::Debug, num::NonZeroU32};

use matryona_machines::testing::XorShift64;

use crate::Refusal;

/// A positive test amount; zero is a broken test assumption.
pub fn nz(value: u32) -> NonZeroU32 {
    let Some(value) = NonZeroU32::new(value) else {
        panic!("test amounts are positive");
    };
    value
}

/// The next draw of `rng`, in `0..bound`, as the `u32` the cores count in.
pub fn below(rng: &mut XorShift64, bound: u32) -> u32 {
    let Ok(draw) = u32::try_from(rng.below(u64::from(bound))) else {
        panic!("a draw below a u32 bound fits a u32");
    };
    draw
}

/// The next draw of `rng` as an index, in `0..bound`.
pub fn index(rng: &mut XorShift64, bound: u32) -> usize {
    let Ok(index) = usize::try_from(below(rng, bound)) else {
        panic!("a u32 index fits a usize");
    };
    index
}

/// How often a randomized run saw each refusal. Every refusal must leave the
/// value exactly as it was.
#[derive(Debug, Default)]
pub struct Refusals {
    pub unknown_tenant: u32,
    pub unknown_lane: u32,
    pub insufficient: u32,
    pub exceeds_in_flight: u32,
}

impl Refusals {
    pub fn record<T: PartialEq + Debug>(
        &mut self,
        refused: Option<Refusal>,
        after: &T,
        before: &T,
    ) {
        let Some(refusal) = refused else {
            return;
        };
        assert_eq!(after, before, "{refusal:?} changed the value");
        *match refusal {
            Refusal::UnknownTenant => &mut self.unknown_tenant,
            Refusal::UnknownLane => &mut self.unknown_lane,
            Refusal::Insufficient => &mut self.insufficient,
            Refusal::ExceedsInFlight => &mut self.exceeds_in_flight,
        } += 1;
    }
}
