//! Exact const-generic lane counts erased only for the host binding.

use crate::{done, range_error, refusal};
use fair_machines::{KeyedFair, Lane, Refusal};
use magnus::{Error, RModule, Ruby, Value, function, method, prelude::*, typed_data::Obj};
use matryona_machines::ruby::{Class, Registry, exact_fields, integer, integer_in, keep, positive};
use std::{any::Any, cell::RefCell, num::NonZeroU32};

/// The `Data` class the binding builds and reads, resolved once at init.
struct Classes {
    lane: Class,
}

static CLASSES: Registry<Classes> = Registry::new("fair_machines");

/// Largest `LANES` compiled into the binding: `KeyedFair<1>` to
/// `KeyedFair<256>`. The core takes any `LANES > 0`.
const MAX_LANES: usize = 256;

/// One lane count: the associated functions, and a constructor.
#[derive(Clone, Copy)]
struct Shape {
    lane: fn(u64) -> usize,
    lane_at: fn(usize) -> Option<usize>,
    build: fn(NonZeroU32) -> Box<dyn Lanes>,
}

trait Lanes: Send {
    fn copy(&self) -> Box<dyn Lanes>;
    fn same(&self, other: &dyn Lanes) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn admit(&mut self, hash: u64, cost: NonZeroU32) -> Result<Lane, Refusal>;
    fn release(&mut self, lane: Lane, cost: NonZeroU32) -> Result<(), Refusal>;
    fn lane_count(&self) -> usize;
    fn in_flight(&self, lane: Lane) -> Option<u32>;
    fn available(&self, lane: Lane) -> Option<u32>;
}

/// What clone-then-keep copies and compares: the erased core, by value.
impl Clone for Box<dyn Lanes> {
    fn clone(&self) -> Self {
        self.copy()
    }
}

impl PartialEq for Box<dyn Lanes> {
    fn eq(&self, other: &Self) -> bool {
        self.same(&**other)
    }
}

impl<const N: usize> Lanes for KeyedFair<N> {
    fn copy(&self) -> Box<dyn Lanes> {
        Box::new(self.clone())
    }
    fn same(&self, other: &dyn Lanes) -> bool {
        other.as_any().downcast_ref::<Self>() == Some(self)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn admit(&mut self, hash: u64, cost: NonZeroU32) -> Result<Lane, Refusal> {
        self.admit(hash, cost)
    }
    fn release(&mut self, lane: Lane, cost: NonZeroU32) -> Result<(), Refusal> {
        self.release(lane, cost)
    }
    fn lane_count(&self) -> usize {
        self.lane_count()
    }
    fn in_flight(&self, lane: Lane) -> Option<u32> {
        self.in_flight(lane)
    }
    fn available(&self, lane: Lane) -> Option<u32> {
        self.available(lane)
    }
}

/// The shape of `KeyedFair<lanes>`, or `RangeError` outside `1..=256`.
fn shape(ruby: &Ruby, lanes: Value) -> Result<Shape, Error> {
    let lanes = integer_in(lanes, "lanes", 1..=MAX_LANES)?;
    macro_rules! sized {
        ($($n:literal),*) => {
            match lanes {
                $($n => Shape {
                    lane: |hash| KeyedFair::<$n>::lane(hash).index(),
                    lane_at: |index| KeyedFair::<$n>::lane_at(index).map(Lane::index),
                    build: |capacity| Box::new(KeyedFair::<$n>::new(capacity)),
                },)*
                _ => return Err(range_error(ruby, format!("lanes must be in 1..={MAX_LANES}"))),
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

/// A `FairMachines::Lane` for `index`, through `Lane.new`.
fn lane_value(ruby: &Ruby, index: usize) -> Result<Value, Error> {
    let class = CLASSES.get(ruby)?.lane.get(ruby);
    class.funcall("new", (index,))
}

/// The core `Lane` of a `FairMachines::Lane`: `TypeError` for anything but
/// an instance of exactly that class, and its member re-validated as
/// `Lane::new` holds it, so a forged value never reaches the core.
fn core_lane(ruby: &Ruby, lane: Value) -> Result<Lane, Error> {
    let class = CLASSES.get(ruby)?.lane.get(ruby);
    let [index] = exact_fields::<1>(ruby, lane, class, "lane must be a FairMachines::Lane")?;
    integer(index, "lane index").map(Lane::new)
}

/// `FairMachines.__lane`, the private hook `Lane.new` validates through:
/// the index as `Lane::new` holds it.
fn validated(position: Value) -> Result<usize, Error> {
    integer(position, "index").map(|position| Lane::new(position).index())
}

#[magnus::wrap(class = "FairMachines::KeyedFair", free_immediately, size)]
struct Wrapper(RefCell<Box<dyn Lanes>>);

impl Wrapper {
    fn new(ruby: &Ruby, lanes: Value, capacity: Value) -> Result<Self, Error> {
        let build = shape(ruby, lanes)?.build;
        Ok(Self(RefCell::new(build(positive(capacity, "capacity")?))))
    }

    /// `KeyedFair::<LANES>::lane(hash)`: the lane count comes first.
    fn lane(ruby: &Ruby, lanes: Value, hash: Value) -> Result<Value, Error> {
        let lane = shape(ruby, lanes)?.lane;
        lane_value(ruby, lane(integer(hash, "hash")?))
    }

    /// `KeyedFair::<LANES>::lane_at(index)`: the lane count comes first.
    fn lane_at(ruby: &Ruby, lanes: Value, position: Value) -> Result<Option<Value>, Error> {
        let lane_at = shape(ruby, lanes)?.lane_at;
        lane_at(integer(position, "index")?)
            .map(|position| lane_value(ruby, position))
            .transpose()
    }

    /// Clone-then-keep: the core admits on a copy, the Lane is built, and
    /// the copy is stored only if the receiver is still unfrozen and
    /// unchanged; otherwise nothing is admitted. A frozen receiver raises
    /// before the arguments are read.
    fn admit(ruby: &Ruby, this: Obj<Self>, hash: Value, cost: Value) -> Result<Value, Error> {
        this.check_frozen()?;
        let (hash, cost) = (integer(hash, "hash")?, positive(cost, "cost")?);
        keep(
            ruby,
            this,
            &this.0,
            |core| core.admit(hash, cost),
            |admitted| match admitted {
                Ok(lane) => lane_value(ruby, lane.index()),
                Err(refused) => Ok(refusal(ruby, refused)),
            },
        )
    }

    fn release(ruby: &Ruby, this: Obj<Self>, lane: Value, cost: Value) -> Result<Value, Error> {
        this.check_frozen()?;
        let lane = core_lane(ruby, lane)?;
        let cost = positive(cost, "cost")?;
        let answer = this.0.borrow_mut().release(lane, cost);
        Ok(done(ruby, answer, "released"))
    }

    fn lane_count(&self) -> usize {
        self.0.borrow().lane_count()
    }

    fn in_flight(ruby: &Ruby, this: &Self, lane: Value) -> Result<Option<u32>, Error> {
        let lane = core_lane(ruby, lane)?;
        Ok(this.0.borrow().in_flight(lane))
    }

    fn available(ruby: &Ruby, this: &Self, lane: Value) -> Result<Option<u32>, Error> {
        let lane = core_lane(ruby, lane)?;
        Ok(this.0.borrow().available(lane))
    }
}

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    let lane = Class::resolve(ruby, module, "Lane")?;
    CLASSES.set(ruby, Classes { lane })?;
    module.define_singleton_method("__lane", function!(validated, 1))?;
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
