//! Exact const-generic tenant counts erased only for the host binding.

use crate::{done, positives, range_error, refusal};
use fair_machines::{Refusal, TenantFair};
use magnus::{Error, RModule, Ruby, Value, function, method, prelude::*, typed_data::Obj};
use matryona_machines::ruby::{exact_array, integer, positive};
use std::{cell::RefCell, num::NonZeroU32};

/// Largest `N` compiled into the binding: `TenantFair<1>` to `TenantFair<16>`.
/// The core takes any `N`; Ruby picks one of these by the arrays' length.
const MAX_TENANTS: usize = 16;

trait Tenants: Send {
    fn admit(&mut self, tenant: usize, cost: NonZeroU32) -> Result<(), Refusal>;
    fn cancel(&mut self, tenant: usize, cost: NonZeroU32) -> Result<(), Refusal>;
    fn select(&mut self, heads: &[Option<NonZeroU32>]) -> Result<Option<usize>, Refusal>;
    fn tenant_count(&self) -> usize;
    fn in_flight(&self, tenant: usize) -> Option<u32>;
    fn available(&self, tenant: usize) -> Option<u32>;
}

impl<const N: usize> Tenants for TenantFair<N> {
    fn admit(&mut self, tenant: usize, cost: NonZeroU32) -> Result<(), Refusal> {
        self.admit(tenant, cost)
    }
    fn cancel(&mut self, tenant: usize, cost: NonZeroU32) -> Result<(), Refusal> {
        self.cancel(tenant, cost)
    }
    fn select(&mut self, heads: &[Option<NonZeroU32>]) -> Result<Option<usize>, Refusal> {
        self.select(&std::array::from_fn(|index| heads[index]))
    }
    fn tenant_count(&self) -> usize {
        self.tenant_count()
    }
    fn in_flight(&self, tenant: usize) -> Option<u32> {
        self.in_flight(tenant)
    }
    fn available(&self, tenant: usize) -> Option<u32> {
        self.available(tenant)
    }
}

/// A core call that admits or returns one tenant's cost.
type BudgetCall = fn(&mut Box<dyn Tenants>, usize, NonZeroU32) -> Result<(), Refusal>;

#[magnus::wrap(class = "FairMachines::TenantFair", free_immediately, size)]
struct Wrapper(RefCell<Box<dyn Tenants>>);

impl Wrapper {
    fn new(ruby: &Ruby, capacities: Value, quanta: Value) -> Result<Self, Error> {
        let capacities = exact_array(ruby, capacities, "capacities must be an Array")?;
        let quanta = exact_array(ruby, quanta, "quanta must be an Array")?;
        if capacities.len() != quanta.len() {
            return Err(range_error(
                ruby,
                "capacities and quanta lengths must match",
            ));
        }
        let capacities = positives(capacities, "capacities")?;
        let quanta = positives(quanta, "quanta")?;
        macro_rules! sized {
            ($($n:literal),*) => {
                match capacities.len() {
                    $($n => Box::new(TenantFair::<$n>::new(
                        std::array::from_fn(|i| capacities[i]),
                        std::array::from_fn(|i| quanta[i]))) as Box<dyn Tenants>,)*
                    _ => return Err(range_error(
                        ruby,
                        format!("capacities must hold 1..={MAX_TENANTS} tenants"),
                    )),
                }
            };
        }
        let core = sized!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
        Ok(Self(RefCell::new(core)))
    }

    fn admit(ruby: &Ruby, this: Obj<Self>, tenant: Value, cost: Value) -> Result<Value, Error> {
        Self::apply(
            ruby,
            this,
            (tenant, cost),
            |core, tenant, cost| core.admit(tenant, cost),
            "admitted",
        )
    }

    fn cancel(ruby: &Ruby, this: Obj<Self>, tenant: Value, cost: Value) -> Result<Value, Error> {
        Self::apply(
            ruby,
            this,
            (tenant, cost),
            |core, tenant, cost| core.cancel(tenant, cost),
            "cancelled",
        )
    }

    /// Check and convert everything that can raise, then call the core once.
    fn apply(
        ruby: &Ruby,
        this: Obj<Self>,
        (tenant, cost): (Value, Value),
        operation: BudgetCall,
        honoured: &str,
    ) -> Result<Value, Error> {
        this.check_frozen()?;
        let tenant = integer(tenant, "tenant")?;
        let cost = positive(cost, "cost")?;
        let answer = operation(&mut this.0.borrow_mut(), tenant, cost);
        Ok(done(ruby, answer, honoured))
    }

    fn select(ruby: &Ruby, this: Obj<Self>, heads: Value) -> Result<Option<Value>, Error> {
        this.check_frozen()?;
        let heads = exact_array(ruby, heads, "heads must be an Array")?;
        let tenants = this.0.borrow().tenant_count();
        if heads.len() != tenants {
            return Err(range_error(
                ruby,
                format!(
                    "heads must hold one entry per tenant: {} given, {tenants} tenants",
                    heads.len()
                ),
            ));
        }
        let heads = crate::heads(heads)?;
        let answer = this.0.borrow_mut().select(&heads);
        Ok(match answer {
            Ok(tenant) => tenant.map(|tenant| ruby.integer_from_u64(tenant as u64).as_value()),
            Err(refused) => Some(refusal(ruby, refused)),
        })
    }

    fn tenant_count(&self) -> usize {
        self.0.borrow().tenant_count()
    }

    fn in_flight(&self, tenant: Value) -> Result<Option<u32>, Error> {
        Ok(self.0.borrow().in_flight(integer(tenant, "tenant")?))
    }

    fn available(&self, tenant: Value) -> Result<Option<u32>, Error> {
        Ok(self.0.borrow().available(integer(tenant, "tenant")?))
    }
}

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    let class = module.define_class("TenantFair", ruby.class_object())?;
    class.undef_default_alloc_func();
    class.const_set("MAX_TENANTS", MAX_TENANTS)?;
    class.define_singleton_method("new", function!(Wrapper::new, 2))?;
    class.define_method("admit", method!(Wrapper::admit, 2))?;
    class.define_method("cancel", method!(Wrapper::cancel, 2))?;
    class.define_method("select", method!(Wrapper::select, 1))?;
    class.define_method("tenant_count", method!(Wrapper::tenant_count, 0))?;
    class.define_method("in_flight", method!(Wrapper::in_flight, 1))?;
    class.define_method("available", method!(Wrapper::available, 1))
}
