//! The `Data` classes the binding builds and reads, resolved once at init.
//!
//! Sealing in Ruby stops accidents, not reflection: a subclass constant is
//! bound before `inherited` raises, `Data#initialize` can be bound directly
//! and `Marshal.load` skips `initialize`. So the binding is the authority: a
//! value it reads must be an instance of exactly the sealed class, its members
//! are read natively (no method is dispatched) and the caller re-validates
//! them through the core constructor.

use std::sync::OnceLock;

use magnus::{
    Error, RClass, RModule, RStruct, Ruby, TryConvert, Value, prelude::*, rb_sys::AsRawValue,
    value::Opaque,
};

/// One sealed class of the gem, looked up by name once.
pub struct Sealed {
    name: &'static str,
    class: OnceLock<Opaque<RClass>>,
}

impl Sealed {
    /// The class `name` under the gem module, still to be resolved.
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            class: OnceLock::new(),
        }
    }

    /// Look the class up at init and keep it from the garbage collector.
    pub fn resolve(&self, ruby: &Ruby, module: RModule) -> Result<(), Error> {
        let class: RClass = module.const_get(self.name)?;
        ruby.gc_register_mark_object(class);
        self.class.get_or_init(|| class.into());
        Ok(())
    }

    /// The class resolved at init.
    pub fn class(&self, ruby: &Ruby) -> Result<RClass, Error> {
        self.class
            .get()
            .map(|class| ruby.get_inner(*class))
            .ok_or_else(|| {
                Error::new(
                    ruby.exception_runtime_error(),
                    format!("{} was not resolved at init", self.name),
                )
            })
    }

    /// A new value, through the class's own `new` (so through its check).
    pub fn build<A: magnus::ArgList>(&self, ruby: &Ruby, args: A) -> Result<Value, Error> {
        self.class(ruby)?.funcall("new", args)
    }

    /// `value` when it is an instance of exactly this class (`instance_of?`,
    /// not `is_a?`); `TypeError` naming `argument` otherwise.
    pub fn read(&self, ruby: &Ruby, value: Value, argument: &str) -> Result<Members, Error> {
        let class = self.class(ruby)?;
        RStruct::from_value(value)
            .filter(|members| instance_of(*members, class))
            .map(Members)
            .ok_or_else(|| {
                Error::new(
                    ruby.exception_type_error(),
                    format!("{argument} must be a {class}"),
                )
            })
    }
}

/// The members of a checked value, read without dispatching any method.
#[derive(Clone, Copy)]
pub struct Members(RStruct);

impl Members {
    /// The member `name`, converted to `T`.
    pub fn get<T: TryConvert>(self, name: &str) -> Result<T, Error> {
        self.0.getmember(name)
    }
}

/// Pointer equality on the classes, so no `==` the caller could redefine runs.
/// A singleton class on the instance itself does not change what it is.
fn instance_of(value: RStruct, class: RClass) -> bool {
    let actual = value.class();
    if actual.as_raw() == class.as_raw() {
        return true;
    }
    let singleton = value
        .singleton_class()
        .is_ok_and(|own| own.as_raw() == actual.as_raw());
    singleton
        && actual
            .superclass()
            .is_ok_and(|parent| parent.as_raw() == class.as_raw())
}
