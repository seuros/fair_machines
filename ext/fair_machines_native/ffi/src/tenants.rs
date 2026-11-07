//! Exact const-generic tenant counts erased only for the host binding.

use crate::{Count, Positive, arg_error, convert_all, kwargs};
use fair_machines::TenantFair;
use magnus::{Error, RArray, RModule, Ruby, Value, function, method, prelude::*, typed_data::Obj};
use std::{cell::RefCell, num::NonZeroU32};

trait Tenants: Send {
    fn admit(&mut self, tenant: usize, cost: NonZeroU32) -> bool;
    fn cancel(&mut self, tenant: usize, cost: NonZeroU32) -> bool;
    fn select(&mut self, heads: &[Option<NonZeroU32>]) -> Option<usize>;
}

impl<const N: usize> Tenants for TenantFair<N> {
    fn admit(&mut self, tenant: usize, cost: NonZeroU32) -> bool {
        self.admit(tenant, cost)
    }
    fn cancel(&mut self, tenant: usize, cost: NonZeroU32) -> bool {
        self.cancel(tenant, cost)
    }
    fn select(&mut self, heads: &[Option<NonZeroU32>]) -> Option<usize> {
        self.select(&std::array::from_fn(|index| heads[index]))
    }
}

#[magnus::wrap(class = "FairMachines::TenantFair", free_immediately, size)]
struct Wrapper {
    core: RefCell<Box<dyn Tenants>>,
    count: usize,
}

impl Wrapper {
    fn new(ruby: &Ruby, args: &[Value]) -> Result<Self, Error> {
        let (capacities, quanta): (RArray, RArray) = kwargs(args, &["capacities", "quanta"])?;
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
        Ok(Self {
            core: RefCell::new(core),
            count: capacities.len(),
        })
    }

    /// Parse `tenant:`/`cost:` and apply `op` to the unfrozen core.
    fn with_cost(
        rb_self: Obj<Self>,
        args: &[Value],
        op: fn(&mut dyn Tenants, usize, NonZeroU32) -> bool,
    ) -> Result<bool, Error> {
        rb_self.check_frozen()?;
        let (Count(tenant), Positive(cost)) = kwargs(args, &["tenant", "cost"])?;
        Ok(op(&mut **rb_self.core.borrow_mut(), tenant as usize, cost))
    }

    fn admit(rb_self: Obj<Self>, args: &[Value]) -> Result<bool, Error> {
        Self::with_cost(rb_self, args, |core, tenant, cost| core.admit(tenant, cost))
    }

    fn cancel(rb_self: Obj<Self>, args: &[Value]) -> Result<bool, Error> {
        Self::with_cost(rb_self, args, |core, tenant, cost| {
            core.cancel(tenant, cost)
        })
    }

    fn select(ruby: &Ruby, rb_self: Obj<Self>, heads: RArray) -> Result<Option<usize>, Error> {
        rb_self.check_frozen()?;
        if heads.len() != rb_self.count {
            return Err(arg_error(ruby, "heads length must match tenant count"));
        }
        let heads: Vec<_> = convert_all::<Option<Positive>>(heads)?
            .into_iter()
            .map(|head| head.map(|Positive(cost)| cost))
            .collect();
        Ok(rb_self.core.borrow_mut().select(&heads))
    }

    const fn tenant_count(&self) -> usize {
        self.count
    }
}

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    let class = module.define_class("TenantFair", ruby.class_object())?;
    class.define_singleton_method("new", function!(Wrapper::new, -1))?;
    class.define_method("admit", method!(Wrapper::admit, -1))?;
    class.define_method("cancel", method!(Wrapper::cancel, -1))?;
    class.define_method("select", method!(Wrapper::select, 1))?;
    class.define_method("tenant_count", method!(Wrapper::tenant_count, 0))
}
