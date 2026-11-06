//! Exact const-generic tenant counts erased only for the host binding.

use crate::{Count, positive};
use fair_machines::TenantFair;
use magnus::{
    Error, RArray, RHash, RModule, Ruby, Value, function, method,
    prelude::*,
    scan_args::{get_kwargs, scan_args},
    typed_data::Obj,
};
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

/// Every element of `values` as a positive `u32`, in order.
fn positive_counts(ruby: &Ruby, values: RArray) -> Result<Vec<NonZeroU32>, Error> {
    values
        .into_iter()
        .map(|v| positive(ruby, Count::try_convert(v)?))
        .collect()
}

#[magnus::wrap(class = "FairMachines::TenantFair", free_immediately, size)]
struct Wrapper {
    core: RefCell<Box<dyn Tenants>>,
    count: usize,
}

impl Wrapper {
    fn new(ruby: &Ruby, args: &[Value]) -> Result<Self, Error> {
        let args = scan_args::<(), (), (), (), RHash, ()>(args)?;
        let (capacities, quanta) = get_kwargs::<_, (RArray, RArray), (), ()>(
            args.keywords,
            &["capacities", "quanta"],
            &[],
        )?
        .required;
        if capacities.len() != quanta.len() {
            return Err(Error::new(
                ruby.exception_arg_error(),
                "capacities and quanta lengths must match",
            ));
        }
        let capacities = positive_counts(ruby, capacities)?;
        let quanta = positive_counts(ruby, quanta)?;
        macro_rules! sized {
            ($($n:literal),*) => {
                match capacities.len() {
                    $($n => Box::new(TenantFair::<$n>::new(
                        std::array::from_fn(|i| capacities[i]),
                        std::array::from_fn(|i| quanta[i]))) as Box<dyn Tenants>,)*
                    _ => {
                        let message = "tenant count must be 1..16";
                        return Err(Error::new(ruby.exception_arg_error(), message));
                    }
                }
            };
        }
        let core = sized!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
        Ok(Self {
            core: RefCell::new(core),
            count: capacities.len(),
        })
    }

    fn cost(ruby: &Ruby, args: &[Value]) -> Result<(usize, NonZeroU32), Error> {
        let args = scan_args::<(), (), (), (), RHash, ()>(args)?;
        let (tenant, cost) =
            get_kwargs::<_, (Count, Count), (), ()>(args.keywords, &["tenant", "cost"], &[])?
                .required;
        Ok((tenant.0 as usize, positive(ruby, cost)?))
    }

    fn admit(ruby: &Ruby, rb_self: Obj<Self>, args: &[Value]) -> Result<bool, Error> {
        rb_self.check_frozen()?;
        let (tenant, cost) = Self::cost(ruby, args)?;
        let admitted = rb_self.core.borrow_mut().admit(tenant, cost);
        Ok(admitted)
    }

    fn cancel(ruby: &Ruby, rb_self: Obj<Self>, args: &[Value]) -> Result<bool, Error> {
        rb_self.check_frozen()?;
        let (tenant, cost) = Self::cost(ruby, args)?;
        let canceled = rb_self.core.borrow_mut().cancel(tenant, cost);
        Ok(canceled)
    }

    fn select(ruby: &Ruby, rb_self: Obj<Self>, heads: RArray) -> Result<Option<usize>, Error> {
        rb_self.check_frozen()?;
        if heads.len() != rb_self.count {
            return Err(Error::new(
                ruby.exception_arg_error(),
                "heads length must match tenant count",
            ));
        }
        let heads = heads
            .into_iter()
            .map(|v| {
                if v.is_nil() {
                    Ok(None)
                } else {
                    positive(ruby, Count::try_convert(v)?).map(Some)
                }
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let selected = rb_self.core.borrow_mut().select(&heads);
        Ok(selected)
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
