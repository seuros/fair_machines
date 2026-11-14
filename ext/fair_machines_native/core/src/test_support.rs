//! Helpers shared by the crate's test modules.

use core::{fmt::Debug, num::NonZeroU32};

use crate::Refusal;

/// A positive test amount; zero is a broken test assumption.
pub fn nz(value: u32) -> NonZeroU32 {
    let Some(value) = NonZeroU32::new(value) else {
        panic!("test amounts are positive");
    };
    value
}

/// xorshift64: a fixed seed keeps a run reproducible.
pub struct Rng(pub u64);

impl Rng {
    pub const fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// The next draw, in `0..bound`.
    pub fn below(&mut self, bound: u32) -> u32 {
        let Ok(draw) = u32::try_from(self.next() % u64::from(bound)) else {
            panic!("a draw below a u32 bound fits a u32");
        };
        draw
    }

    /// The next draw as an index, in `0..bound`.
    pub fn index(&mut self, bound: u32) -> usize {
        let Ok(index) = usize::try_from(self.below(bound)) else {
            panic!("a u32 index fits a usize");
        };
        index
    }
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
