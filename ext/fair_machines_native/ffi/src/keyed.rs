use crate::{Count, Positive, arg_error, integer, kwargs};
use fair_machines::{KeyedFair, Lane};
use magnus::{
    Error, RModule, Ruby, TryConvert, Value, function, method, prelude::*, typed_data::Obj,
};
use std::{cell::RefCell, num::NonZeroU32};

trait Lanes: Send {
    fn admit(&mut self, hash: u64, cost: NonZeroU32) -> Option<usize>;
    fn release(&mut self, lane: usize, cost: NonZeroU32) -> bool;
    fn in_flight(&self, lane: usize) -> u32;
    fn lane(&self, hash: u64) -> usize;
}

impl<const N: usize> Lanes for KeyedFair<N> {
    fn admit(&mut self, hash: u64, cost: NonZeroU32) -> Option<usize> {
        self.admit(hash, cost).map(Lane::index)
    }
    fn release(&mut self, lane: usize, cost: NonZeroU32) -> bool {
        Self::lane_at(lane).is_some_and(|lane| self.release(lane, cost))
    }
    fn in_flight(&self, lane: usize) -> u32 {
        Self::lane_at(lane).map_or(0, |lane| self.in_flight(lane))
    }
    fn lane(&self, hash: u64) -> usize {
        Self::lane(hash).index()
    }
}

/// Any Integer, reduced to its low 64 bits, so `key.hash` works as is.
#[derive(Clone, Copy)]
struct Hash(u64);

impl TryConvert for Hash {
    #[allow(clippy::cast_sign_loss)]
    fn try_convert(value: Value) -> Result<Self, Error> {
        let int = integer(value)?;
        int.to_i64()
            .map(|v| v as u64)
            .or_else(|_| int.to_u64())
            .map(Self)
    }
}

#[magnus::wrap(class = "FairMachines::KeyedFair", free_immediately, size)]
struct Wrapper {
    core: RefCell<Box<dyn Lanes>>,
    lanes: usize,
}

impl Wrapper {
    fn new(ruby: &Ruby, args: &[Value]) -> Result<Self, Error> {
        let (Count(lanes), Positive(capacity)) = kwargs(args, &["lanes", "capacity"])?;
        macro_rules! sized {
            ($($n:literal),*) => {
                match lanes {
                    $($n => Box::new(KeyedFair::<$n>::new(capacity)) as Box<dyn Lanes>,)*
                    _ => {
                        let message = "lanes must be 1, 2, 4, 8, 16, 32, 64, 128 or 256";
                        return Err(arg_error(ruby, message));
                    }
                }
            };
        }
        let core = sized!(1, 2, 4, 8, 16, 32, 64, 128, 256);
        Ok(Self {
            core: RefCell::new(core),
            lanes: lanes as usize,
        })
    }

    fn admit(rb_self: Obj<Self>, args: &[Value]) -> Result<Option<usize>, Error> {
        rb_self.check_frozen()?;
        let (Hash(hash), Positive(cost)) = kwargs(args, &["hash", "cost"])?;
        Ok(rb_self.core.borrow_mut().admit(hash, cost))
    }

    fn release(rb_self: Obj<Self>, args: &[Value]) -> Result<bool, Error> {
        rb_self.check_frozen()?;
        let (Count(lane), Positive(cost)) = kwargs(args, &["lane", "cost"])?;
        Ok(rb_self.core.borrow_mut().release(lane as usize, cost))
    }

    fn in_flight(&self, lane: Count) -> u32 {
        self.core.borrow().in_flight(lane.0 as usize)
    }

    fn lane(&self, hash: Hash) -> usize {
        self.core.borrow().lane(hash.0)
    }

    const fn lane_count(&self) -> usize {
        self.lanes
    }
}

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    let class = module.define_class("KeyedFair", ruby.class_object())?;
    class.define_singleton_method("new", function!(Wrapper::new, -1))?;
    class.define_method("admit", method!(Wrapper::admit, -1))?;
    class.define_method("release", method!(Wrapper::release, -1))?;
    class.define_method("in_flight", method!(Wrapper::in_flight, 1))?;
    class.define_method("lane", method!(Wrapper::lane, 1))?;
    class.define_method("lane_count", method!(Wrapper::lane_count, 0))
}
