//! `PriorityMachines::RoundRobin`: the lane count is each call's array length.

use std::cell::RefCell;

use magnus::{Error, RModule, Ruby, Value, function, method, prelude::*, typed_data::Obj};
use matryona_machines::ruby::boolean;
use priority_machines::RoundRobin;

use crate::array;

#[magnus::wrap(class = "PriorityMachines::RoundRobin", free_immediately, size)]
struct Wrapper(RefCell<RoundRobin>);

impl Wrapper {
    const fn new() -> Self {
        Self(RefCell::new(RoundRobin::new()))
    }

    fn select(ruby: &Ruby, rb_self: Obj<Self>, flags: Value) -> Result<Option<usize>, Error> {
        rb_self.check_frozen()?;
        // Convert the complete call before entering the core.
        let flags = array(ruby, flags, "ready")?
            .into_iter()
            // Real booleans: no truthiness, so `nil` is not "empty".
            .map(|value| boolean(value, "ready"))
            .collect::<Result<Vec<_>, _>>()?;
        let selected = rb_self.0.borrow_mut().select(&flags);
        Ok(selected)
    }

    fn cursor(&self) -> usize {
        self.0.borrow().cursor()
    }
}

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    let class = module.define_class("RoundRobin", ruby.class_object())?;
    class.undef_default_alloc_func();
    class.define_singleton_method("new", function!(Wrapper::new, 0))?;
    class.define_method("select", method!(Wrapper::select, 1))?;
    class.define_method("cursor", method!(Wrapper::cursor, 0))
}
