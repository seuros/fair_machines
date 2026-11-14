use core::num::NonZeroU32;

use flow_machines::CreditWindow;

use crate::Refusal;

/// The lane a hash maps to: a bucket index, not an issued id.
///
/// [`KeyedFair::admit`] answers the lane it charged so the caller can return
/// the cost with [`KeyedFair::release`] without hashing again. A lane is a
/// pure function of the hash and `LANES`: nothing is counted or issued, the
/// same hash names the same lane in every instance with as many lanes, and
/// [`KeyedFair::lane`], [`KeyedFair::lane_at`] or [`Lane::new`] build one
/// without admitting anything. It therefore carries no scope and proves
/// nothing: holding a lane does not show that cost was admitted to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lane(usize);

impl Lane {
    /// The lane at position `index`, for no instance in particular. Every
    /// position is a lane of some `KeyedFair`; one with `index` lanes or
    /// fewer answers [`Refusal::UnknownLane`] or `None` for it.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Position of the lane, in `0..LANES` of the instance it was built for.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Independent cost budgets for an open set of tenants, hashed onto `LANES`
/// lanes. Tenants sharing a lane share its budget; none can borrow another
/// lane's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyedFair<const LANES: usize> {
    lanes: [CreditWindow; LANES],
}

impl<const LANES: usize> KeyedFair<LANES> {
    const NONEMPTY: () = assert!(LANES > 0, "KeyedFair needs at least one lane");

    /// Start with every lane empty and the same positive `capacity` each.
    /// `LANES` must be positive: zero fails to compile.
    #[must_use]
    pub fn new(capacity: NonZeroU32) -> Self {
        let () = Self::NONEMPTY;
        Self {
            lanes: core::array::from_fn(|_| CreditWindow::new(capacity)),
        }
    }

    /// The lane `hash` maps to: the hash space cut into `LANES` equal,
    /// ordered ranges. A pure function; no instance is involved.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn lane(hash: u64) -> Lane {
        Lane(((hash as u128 * LANES as u128) >> 64) as usize)
    }

    /// The lane at `index`; `None` for an index of `LANES` or more.
    #[must_use]
    pub const fn lane_at(index: usize) -> Option<Lane> {
        if index < LANES {
            Some(Lane(index))
        } else {
            None
        }
    }

    /// Admit `cost` to the lane `hash` maps to and answer that lane.
    ///
    /// # Errors
    ///
    /// [`Refusal::Insufficient`] when `cost` exceeds the lane's available
    /// budget: there is no partial admission and nothing changed.
    pub const fn admit(&mut self, hash: u64, cost: NonZeroU32) -> Result<Lane, Refusal> {
        let lane = Self::lane(hash);
        match self.lanes[lane.0].try_acquire(cost) {
            Ok(()) => Ok(lane),
            Err(refusal) => Err(Refusal::from_flow(refusal)),
        }
    }

    /// Return `cost` to `lane`.
    ///
    /// Lanes count cost, not work, so a repeated `release` cannot be
    /// recognised as a repeat. It is honoured while the lane has that much
    /// cost in flight and refused once it does not.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownLane`] for a lane this instance does not have,
    /// [`Refusal::ExceedsInFlight`] when `cost` exceeds what the lane has in
    /// flight: there is no partial return. Nothing changed.
    pub fn release(&mut self, lane: Lane, cost: NonZeroU32) -> Result<(), Refusal> {
        self.lanes
            .get_mut(lane.0)
            .ok_or(Refusal::UnknownLane)?
            .release(cost)
            .map_err(Refusal::from_flow)
    }

    /// Number of lanes, `LANES`.
    #[must_use]
    pub const fn lane_count(&self) -> usize {
        LANES
    }

    /// Cost admitted to `lane` and not yet released; `None` for a lane this
    /// instance does not have.
    #[must_use]
    pub fn in_flight(&self, lane: Lane) -> Option<u32> {
        self.lanes.get(lane.0).map(CreditWindow::in_flight)
    }

    /// Cost `lane` may still admit; `None` for a lane this instance does not
    /// have.
    #[must_use]
    pub fn available(&self, lane: Lane) -> Option<u32> {
        self.lanes.get(lane.0).map(CreditWindow::available)
    }
}

#[cfg(test)]
mod tests;
