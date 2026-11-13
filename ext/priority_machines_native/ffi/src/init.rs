use magnus::{Error, Ruby, function, prelude::*};

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("PriorityMachines")?;
    module.define_singleton_method("earliest_deadline", function!(crate::earliest_deadline, 1))?;
    crate::round_robin::define(ruby, module)?;
    crate::scheduler::define(ruby, module)
}
