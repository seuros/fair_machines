#![no_std]
#![doc = include_str!("../README.md")]
use core::num::NonZeroU32;
use flow_machines::CreditWindow;
use priority_machines::{Deactivation, DeficitRoundRobin};

mod keyed;
mod refusal;
#[cfg(test)]
mod test_support;

pub use keyed::{KeyedFair, Lane};
pub use refusal::Refusal;

/// Fixed tenants with independent cost budgets and DRR scheduling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantFair<const N: usize> {
    budgets: [CreditWindow; N],
    scheduler: DeficitRoundRobin<N>,
}

impl<const N: usize> TenantFair<N> {
    /// Configure tenant cost capacities and positive DRR quanta.
    #[must_use]
    pub fn new(capacities: [NonZeroU32; N], quanta: [NonZeroU32; N]) -> Self {
        Self {
            budgets: core::array::from_fn(|i| CreditWindow::new(capacities[i])),
            scheduler: DeficitRoundRobin::new(quanta),
        }
    }

    /// Admit cost without borrowing another tenant's capacity.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownTenant`] for an index of `N` or more,
    /// [`Refusal::Insufficient`] when `cost` exceeds the tenant's available
    /// budget: there is no partial admission. Nothing changed.
    pub fn admit(&mut self, tenant: usize, cost: NonZeroU32) -> Result<(), Refusal> {
        self.budgets
            .get_mut(tenant)
            .ok_or(Refusal::UnknownTenant)?
            .try_acquire(cost)
            .map_err(Refusal::from_flow)
    }

    /// Dispatch one admitted head and return its cost to the tenant's budget.
    ///
    /// `heads[i]` is the cost at the head of tenant `i`'s queue, `None` when
    /// it has nothing waiting. A drain: `Ok(None)` is the honoured answer
    /// "every head is `None`", returned exactly then. Like every selection it
    /// records what the round observed: a tenant presented as `None` loses
    /// the credit it had saved.
    ///
    /// # Errors
    ///
    /// [`Refusal::ExceedsInFlight`] when any head costs more than its tenant
    /// has admitted. Such a head was never admitted, so it cannot be
    /// dispatched, and the call is refused whole rather than quietly treating
    /// that tenant as empty. Nothing changed: no credit was paid, no head
    /// charged, no budget touched.
    pub fn select(&mut self, heads: &[Option<NonZeroU32>; N]) -> Result<Option<usize>, Refusal> {
        let admitted = |(budget, head): (&CreditWindow, &Option<NonZeroU32>)| {
            head.is_none_or(|cost| cost.get() <= budget.in_flight())
        };
        if !self.budgets.iter().zip(heads).all(admitted) {
            return Err(Refusal::ExceedsInFlight);
        }
        // The round is decided on a copy and committed last, so no path
        // out of here leaves half a round behind.
        let mut scheduler = self.scheduler.clone();
        let Some(tenant) = scheduler.select(heads) else {
            self.scheduler = scheduler;
            return Ok(None);
        };
        let Some(cost) = heads.get(tenant).copied().flatten() else {
            debug_assert!(false, "DRR selects only a lane presented with a head");
            return Err(Refusal::ExceedsInFlight);
        };
        let Ok(budget) = Self::returned(&self.budgets, &mut scheduler, tenant, cost) else {
            debug_assert!(false, "every head was checked against its tenant's budget");
            return Err(Refusal::ExceedsInFlight);
        };
        self.budgets[tenant] = budget;
        self.scheduler = scheduler;
        Ok(Some(tenant))
    }

    /// Cancel queued cost: work that left the queue without being selected.
    ///
    /// Budgets count cost, not jobs, so a repeated `cancel` cannot be
    /// recognised as a repeat. It is honoured while the tenant has that much
    /// cost admitted and refused once it does not.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownTenant`] for an index of `N` or more,
    /// [`Refusal::ExceedsInFlight`] when `cost` exceeds what the tenant has
    /// admitted: there is no partial return. Nothing changed.
    pub fn cancel(&mut self, tenant: usize, cost: NonZeroU32) -> Result<(), Refusal> {
        let budget = Self::returned(&self.budgets, &mut self.scheduler, tenant, cost)?;
        self.budgets[tenant] = budget;
        Ok(())
    }

    /// Number of tenants, `N`.
    #[must_use]
    pub const fn tenant_count(&self) -> usize {
        N
    }

    /// Cost the tenant has admitted and not yet dispatched or cancelled;
    /// `None` for an index of `N` or more.
    #[must_use]
    pub fn in_flight(&self, tenant: usize) -> Option<u32> {
        self.budgets.get(tenant).map(CreditWindow::in_flight)
    }

    /// Cost the tenant may still admit; `None` for an index of `N` or more.
    #[must_use]
    pub fn available(&self, tenant: usize) -> Option<u32> {
        self.budgets.get(tenant).map(CreditWindow::available)
    }

    /// The tenant's budget with `cost` returned; a drained tenant leaves the
    /// rotation of `scheduler`. The budget is answered, not stored: the
    /// scheduler's answer comes last, so a refusal has changed nothing.
    fn returned(
        budgets: &[CreditWindow; N],
        scheduler: &mut DeficitRoundRobin<N>,
        tenant: usize,
        cost: NonZeroU32,
    ) -> Result<CreditWindow, Refusal> {
        let mut budget = budgets.get(tenant).ok_or(Refusal::UnknownTenant)?.clone();
        budget.release(cost).map_err(Refusal::from_flow)?;
        if budget.in_flight() > 0 {
            return Ok(budget);
        }
        match scheduler.deactivate(tenant) {
            Ok(Deactivation::Deactivated | Deactivation::Idle) => Ok(budget),
            Err(refusal) => {
                debug_assert!(false, "the scheduler has the same N lanes as the budgets");
                Err(Refusal::from_priority(refusal))
            }
        }
    }
}

#[cfg(test)]
mod tests;
