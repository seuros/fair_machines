#![no_std]
#![doc = include_str!("../README.md")]
use core::num::NonZeroU32;
use flow_machines::CreditWindow;
use priority_machines::{Deactivation, DeficitRoundRobin};

mod keyed;
mod refusal;

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
        let Some(tenant) = self.scheduler.select(heads) else {
            return Ok(None);
        };
        // The scheduler only selects a lane with a head, and every head was
        // checked against its budget above, so this return is never refused.
        if let Some(cost) = heads[tenant] {
            self.release(tenant, cost)?;
        }
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
        self.release(tenant, cost)
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

    /// Return queued cost; a drained tenant leaves the DRR rotation.
    fn release(&mut self, tenant: usize, cost: NonZeroU32) -> Result<(), Refusal> {
        let budget = self.budgets.get_mut(tenant).ok_or(Refusal::UnknownTenant)?;
        budget.release(cost).map_err(Refusal::from_flow)?;
        if budget.in_flight() > 0 {
            return Ok(());
        }
        // `tenant` indexed `budgets`, and the scheduler has the same `N`
        // lanes, so this is never refused.
        match self.scheduler.deactivate(tenant) {
            Ok(Deactivation::Deactivated | Deactivation::Idle) => Ok(()),
            Err(refusal) => Err(Refusal::from_priority(refusal)),
        }
    }
}

#[cfg(test)]
mod tests;
