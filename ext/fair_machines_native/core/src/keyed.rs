use core::num::NonZeroU32;

use flow_machines::CreditWindow;

/// Lane a hashed key was admitted to; return its cost with [`KeyedFair::release`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lane(usize);

impl Lane {
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Independent cost budgets for an open set of tenants, hashed onto `LANES`
/// lanes. Tenants sharing a lane share its budget; none can borrow another
/// lane's.
#[derive(Debug)]
pub struct KeyedFair<const LANES: usize> {
    lanes: [CreditWindow; LANES],
}

impl<const LANES: usize> KeyedFair<LANES> {
    const NONEMPTY: () = assert!(LANES > 0, "KeyedFair needs at least one lane");

    #[must_use]
    pub fn new(capacity: NonZeroU32) -> Self {
        let () = Self::NONEMPTY;
        Self {
            lanes: core::array::from_fn(|_| CreditWindow::new(capacity)),
        }
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn lane(hash: u64) -> Lane {
        Lane(((hash as u128 * LANES as u128) >> 64) as usize)
    }

    #[must_use]
    pub const fn lane_at(index: usize) -> Option<Lane> {
        if index < LANES {
            Some(Lane(index))
        } else {
            None
        }
    }

    #[must_use]
    pub fn admit(&mut self, hash: u64, cost: NonZeroU32) -> Option<Lane> {
        let lane = Self::lane(hash);
        self.lanes[lane.0].try_acquire(cost).then_some(lane)
    }

    /// Over-release leaves state unchanged and returns `false`.
    #[must_use]
    pub fn release(&mut self, lane: Lane, cost: NonZeroU32) -> bool {
        self.lanes
            .get_mut(lane.0)
            .is_some_and(|budget| budget.release(cost))
    }

    #[must_use]
    pub fn in_flight(&self, lane: Lane) -> u32 {
        self.lanes.get(lane.0).map_or(0, CreditWindow::in_flight)
    }
}

#[cfg(test)]
mod tests;
