#![no_std]
#![doc = include_str!("../README.md")]
use core::num::NonZeroU32;
use flow_machines::CreditWindow;
use priority_machines::DeficitRoundRobin;

/// Fixed tenants with independent cost budgets and DRR scheduling.
#[derive(Debug)]
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
    #[must_use]
    pub fn admit(&mut self, tenant: usize, cost: NonZeroU32) -> bool {
        self.budgets
            .get_mut(tenant)
            .is_some_and(|budget| budget.try_acquire(cost))
    }

    /// Dispatch one admitted head and return its queued-cost credit.
    ///
    /// # Panics
    /// Panics if an internal scheduling/accounting invariant is violated.
    pub fn select(&mut self, heads: &[Option<NonZeroU32>; N]) -> Option<usize> {
        let eligible = core::array::from_fn(|i| {
            heads[i].filter(|cost| cost.get() <= self.budgets[i].in_flight())
        });
        let tenant = self.scheduler.select(&eligible)?;
        if let Some(cost) = eligible[tenant] {
            assert!(self.budgets[tenant].release(cost));
            if self.budgets[tenant].in_flight() == 0 {
                assert!(self.scheduler.deactivate(tenant));
            }
        }
        Some(tenant)
    }

    /// Cancel queued cost; invalid/over-release requests leave state unchanged.
    ///
    /// # Panics
    /// Panics if an internal lane-index invariant is violated.
    #[must_use]
    pub fn cancel(&mut self, tenant: usize, cost: NonZeroU32) -> bool {
        let Some(budget) = self.budgets.get_mut(tenant) else {
            return false;
        };
        if !budget.release(cost) {
            return false;
        }
        if budget.in_flight() == 0 {
            assert!(self.scheduler.deactivate(tenant));
        }
        true
    }
}

#[cfg(test)]
mod tests;
