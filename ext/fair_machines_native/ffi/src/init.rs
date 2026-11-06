use magnus::{Error, Ruby};

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    crate::tenants::define(ruby, ruby.define_module("FairMachines")?)
}
