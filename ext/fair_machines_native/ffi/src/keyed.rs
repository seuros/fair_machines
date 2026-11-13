//! Exact const-generic lane counts erased only for the host binding.

use crate::{Index, Positive, arg_error, done, integer, refusal};
use fair_machines::{KeyedFair, Lane, Refusal};
use magnus::{
    Error, RClass, RModule, Ruby, TryConvert, Value, function, method, prelude::*, typed_data::Obj,
};
use std::{cell::RefCell, num::NonZeroU32};

/// Largest lane count the binding instantiates; the core takes any `LANES > 0`.
const MAX_LANES: usize = 256;

/// One lane count: the associated functions, and a constructor.
#[derive(Clone, Copy)]
struct Shape {
    lane: fn(u64) -> usize,
    lane_at: fn(usize) -> Option<usize>,
    build: fn(NonZeroU32) -> Box<dyn Lanes>,
}

trait Lanes: Send {
    fn admit(&mut self, hash: u64, cost: NonZeroU32) -> Result<usize, Refusal>;
    fn release(&mut self, lane: usize, cost: NonZeroU32) -> Result<(), Refusal>;
    fn lane_count(&self) -> usize;
    fn in_flight(&self, lane: usize) -> Option<u32>;
    fn available(&self, lane: usize) -> Option<u32>;
}

impl<const N: usize> Lanes for KeyedFair<N> {
    fn admit(&mut self, hash: u64, cost: NonZeroU32) -> Result<usize, Refusal> {
        self.admit(hash, cost).map(Lane::index)
    }
    /// An index this lane count cannot build a `Lane` for is the core's
    /// `UnknownLane`: a lane this instance does not have.
    fn release(&mut self, lane: usize, cost: NonZeroU32) -> Result<(), Refusal> {
        let lane = Self::lane_at(lane).ok_or(Refusal::UnknownLane)?;
        self.release(lane, cost)
    }
    fn lane_count(&self) -> usize {
        self.lane_count()
    }
    fn in_flight(&self, lane: usize) -> Option<u32> {
        self.in_flight(Self::lane_at(lane)?)
    }
    fn available(&self, lane: usize) -> Option<u32> {
        self.available(Self::lane_at(lane)?)
    }
}

/// The shape of `KeyedFair<lanes>`, or `ArgumentError` outside `1..=256`.
fn shape(ruby: &Ruby, lanes: Index) -> Result<Shape, Error> {
    macro_rules! sized {
        ($($n:literal),*) => {
            match lanes.0 {
                $($n => Shape {
                    lane: |hash| KeyedFair::<$n>::lane(hash).index(),
                    lane_at: |index| KeyedFair::<$n>::lane_at(index).map(Lane::index),
                    build: |capacity| Box::new(KeyedFair::<$n>::new(capacity)),
                },)*
                _ => return Err(arg_error(ruby, "lanes must be 1..256")),
            }
        };
    }
    #[rustfmt::skip]
    let shape = sized!(
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46,
            47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68,
            69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90,
            91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109,
            110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126,
            127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143,
            144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160,
            161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177,
            178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194,
            195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211,
            212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225, 226, 227, 228,
            229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245,
            246, 247, 248, 249, 250, 251, 252, 253, 254, 255, 256
    );
    Ok(shape)
}

/// A hash: an Integer in `0..2**64`.
#[derive(Clone, Copy)]
struct Hash(u64);

impl TryConvert for Hash {
    fn try_convert(value: Value) -> Result<Self, Error> {
        integer(value)?.to_u64().map(Self)
    }
}

fn lane_class(ruby: &Ruby) -> Result<RClass, Error> {
    ruby.define_module("FairMachines")?.const_get("Lane")
}

/// A `FairMachines::Lane` for `index`.
fn lane_value(ruby: &Ruby, index: usize) -> Result<Value, Error> {
    lane_class(ruby)?.funcall("new", (index,))
}

/// The index of a `FairMachines::Lane`; `TypeError` for anything else.
fn lane_index(ruby: &Ruby, lane: Value) -> Result<usize, Error> {
    if !lane.is_kind_of(lane_class(ruby)?) {
        return Err(Error::new(
            ruby.exception_type_error(),
            "expected a FairMachines::Lane",
        ));
    }
    lane.funcall::<_, _, Index>("index", ())
        .map(|index| index.0)
}

#[magnus::wrap(class = "FairMachines::KeyedFair", free_immediately, size)]
struct Wrapper(RefCell<Box<dyn Lanes>>);

impl Wrapper {
    fn new(ruby: &Ruby, lanes: Index, capacity: Positive) -> Result<Self, Error> {
        let build = shape(ruby, lanes)?.build;
        Ok(Self(RefCell::new(build(capacity.0))))
    }

    /// `KeyedFair::<LANES>::lane(hash)`: the lane count comes first.
    fn lane(ruby: &Ruby, lanes: Index, hash: Hash) -> Result<Value, Error> {
        let lane = shape(ruby, lanes)?.lane;
        lane_value(ruby, lane(hash.0))
    }

    /// `KeyedFair::<LANES>::lane_at(index)`: the lane count comes first.
    fn lane_at(ruby: &Ruby, lanes: Index, index: Index) -> Result<Option<Value>, Error> {
        let lane_at = shape(ruby, lanes)?.lane_at;
        lane_at(index.0)
            .map(|index| lane_value(ruby, index))
            .transpose()
    }

    fn admit(ruby: &Ruby, this: Obj<Self>, hash: Hash, cost: Positive) -> Result<Value, Error> {
        this.check_frozen()?;
        // Resolve the class before admitting, so a failure admits nothing.
        lane_class(ruby)?;
        let answer = this.0.borrow_mut().admit(hash.0, cost.0);
        match answer {
            Ok(index) => lane_value(ruby, index),
            Err(refused) => Ok(refusal(ruby, refused)),
        }
    }

    fn release(ruby: &Ruby, this: Obj<Self>, lane: Value, cost: Positive) -> Result<Value, Error> {
        this.check_frozen()?;
        let index = lane_index(ruby, lane)?;
        let answer = this.0.borrow_mut().release(index, cost.0);
        Ok(done(ruby, answer, "released"))
    }

    fn lane_count(&self) -> usize {
        self.0.borrow().lane_count()
    }

    fn in_flight(ruby: &Ruby, this: &Self, lane: Value) -> Result<Option<u32>, Error> {
        let index = lane_index(ruby, lane)?;
        Ok(this.0.borrow().in_flight(index))
    }

    fn available(ruby: &Ruby, this: &Self, lane: Value) -> Result<Option<u32>, Error> {
        let index = lane_index(ruby, lane)?;
        Ok(this.0.borrow().available(index))
    }
}

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    let class = module.define_class("KeyedFair", ruby.class_object())?;
    class.undef_default_alloc_func();
    class.const_set("MAX_LANES", MAX_LANES)?;
    class.define_singleton_method("new", function!(Wrapper::new, 2))?;
    class.define_singleton_method("lane", function!(Wrapper::lane, 2))?;
    class.define_singleton_method("lane_at", function!(Wrapper::lane_at, 2))?;
    class.define_method("admit", method!(Wrapper::admit, 2))?;
    class.define_method("release", method!(Wrapper::release, 2))?;
    class.define_method("lane_count", method!(Wrapper::lane_count, 0))?;
    class.define_method("in_flight", method!(Wrapper::in_flight, 1))?;
    class.define_method("available", method!(Wrapper::available, 1))
}
