use magnus::{Error, Ruby};

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("FairMachines")?;
    crate::tenants::define(ruby, module)?;
    crate::keyed::define(ruby, module)
}
