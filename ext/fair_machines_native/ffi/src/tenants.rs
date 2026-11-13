//! Exact const-generic tenant counts erased only for the host binding.

use crate::{Index, Positive, arg_error, convert_all, done, refusal};
use fair_machines::{Refusal, TenantFair};
use magnus::{Error, RArray, RModule, Ruby, Value, function, method, prelude::*, typed_data::Obj};
use std::{cell::RefCell, num::NonZeroU32};

/// Largest tenant count the binding instantiates; the core takes any `N`.
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

#[magnus::wrap(class = "FairMachines::TenantFair", free_immediately, size)]
struct Wrapper(RefCell<Box<dyn Tenants>>);

impl Wrapper {
    fn new(ruby: &Ruby, capacities: RArray, quanta: RArray) -> Result<Self, Error> {
        if capacities.len() != quanta.len() {
            return Err(arg_error(ruby, "capacities and quanta lengths must match"));
        }
        let capacities = convert_all::<Positive>(capacities)?;
        let quanta = convert_all::<Positive>(quanta)?;
        macro_rules! sized {
            ($($n:literal),*) => {
                match capacities.len() {
                    $($n => Box::new(TenantFair::<$n>::new(
                        std::array::from_fn(|i| capacities[i].0),
                        std::array::from_fn(|i| quanta[i].0))) as Box<dyn Tenants>,)*
                    _ => return Err(arg_error(ruby, "tenant count must be 1..16")),
                }
            };
        }
        let core = sized!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
        Ok(Self(RefCell::new(core)))
    }

    fn admit(ruby: &Ruby, this: Obj<Self>, tenant: Index, cost: Positive) -> Result<Value, Error> {
        this.check_frozen()?;
        let answer = this.0.borrow_mut().admit(tenant.0, cost.0);
        Ok(done(ruby, answer, "admitted"))
    }

    fn cancel(ruby: &Ruby, this: Obj<Self>, tenant: Index, cost: Positive) -> Result<Value, Error> {
        this.check_frozen()?;
        let answer = this.0.borrow_mut().cancel(tenant.0, cost.0);
        Ok(done(ruby, answer, "cancelled"))
    }

    fn select(ruby: &Ruby, this: Obj<Self>, heads: RArray) -> Result<Option<Value>, Error> {
        this.check_frozen()?;
        if heads.len() != this.0.borrow().tenant_count() {
            return Err(arg_error(ruby, "heads length must match tenant count"));
        }
        let heads: Vec<_> = convert_all::<Option<Positive>>(heads)?
            .into_iter()
            .map(|head| head.map(|Positive(cost)| cost))
            .collect();
        let answer = this.0.borrow_mut().select(&heads);
        Ok(match answer {
            Ok(tenant) => tenant.map(|tenant| ruby.integer_from_u64(tenant as u64).as_value()),
            Err(refused) => Some(refusal(ruby, refused)),
        })
    }

    fn tenant_count(&self) -> usize {
        self.0.borrow().tenant_count()
    }

    fn in_flight(&self, tenant: Index) -> Option<u32> {
        self.0.borrow().in_flight(tenant.0)
    }

    fn available(&self, tenant: Index) -> Option<u32> {
        self.0.borrow().available(tenant.0)
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
