//! Exact const-generic lane counts, erased only at the Ruby boundary.

use std::{cell::RefCell, num::NonZeroU32};

use magnus::{
    Error, RArray, RHash, RModule, Ruby, Value, function, method,
    prelude::*,
    scan_args::{get_kwargs, scan_args},
    typed_data::Obj,
};
use priority_machines::{DeficitRoundRobin, WeightedRoundRobin};

use crate::{Count, positive};

// This trait erases const-generic sizes, never adds inactive/phantom lanes.
trait Policy: Send {
    fn select(&mut self, heads: &[Option<NonZeroU32>]) -> Option<usize>;
    fn deactivate(&mut self, lane: usize) -> bool;
}

impl<const N: usize> Policy for WeightedRoundRobin<N> {
    fn select(&mut self, heads: &[Option<NonZeroU32>]) -> Option<usize> {
        self.select(&std::array::from_fn(|index| heads[index].is_some()))
    }
    fn deactivate(&mut self, lane: usize) -> bool {
        self.deactivate(lane)
    }
}

impl<const N: usize> Policy for DeficitRoundRobin<N> {
    fn select(&mut self, heads: &[Option<NonZeroU32>]) -> Option<usize> {
        self.select(&std::array::from_fn(|index| heads[index]))
    }
    fn deactivate(&mut self, lane: usize) -> bool {
        self.deactivate(lane)
    }
}

struct State {
    policy: RefCell<Box<dyn Policy>>,
    lanes: usize,
}

fn make(ruby: &Ruby, values: RArray, deficit: bool) -> Result<State, Error> {
    let values = values
        .into_iter()
        .map(|value| positive(ruby, value))
        .collect::<Result<Vec<_>, _>>()?;
    macro_rules! sized {
        ($($n:literal),*) => {
            match values.len() {
                $($n => {
                    let array = std::array::from_fn::<_, $n, _>(|index| values[index]);
                    if deficit { Box::new(DeficitRoundRobin::new(array)) as Box<dyn Policy> }
                    else { Box::new(WeightedRoundRobin::new(array)) as Box<dyn Policy> }
                },)*
                _ => return Err(Error::new(ruby.exception_arg_error(), "lane count must be 1..16")),
            }
        };
    }
    let policy = sized!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
    Ok(State {
        policy: RefCell::new(policy),
        lanes: values.len(),
    })
}

fn inputs(
    ruby: &Ruby,
    state: &State,
    values: RArray,
    deficit: bool,
) -> Result<Vec<Option<NonZeroU32>>, Error> {
    if values.len() != state.lanes {
        return Err(Error::new(
            ruby.exception_arg_error(),
            "input length must match lane count",
        ));
    }
    values
        .into_iter()
        .map(|value| {
            if deficit {
                if value.is_nil() {
                    Ok(None)
                } else {
                    positive(ruby, value).map(Some)
                }
            } else if magnus::value::Qtrue::from_value(value).is_some() {
                Ok(Some(NonZeroU32::MIN))
            } else if magnus::value::Qfalse::from_value(value).is_some() {
                Ok(None)
            } else {
                Err(Error::new(
                    ruby.exception_type_error(),
                    "ready entries must be true or false",
                ))
            }
        })
        .collect()
}

#[magnus::wrap(class = "PriorityMachines::WeightedRoundRobin", free_immediately, size)]
struct Weighted(State);

#[magnus::wrap(class = "PriorityMachines::DeficitRoundRobin", free_immediately, size)]
struct Deficit(State);

macro_rules! wrapper {
    ($type:ident, $class:literal, $keyword:literal, $deficit:literal) => {
        impl $type {
            fn new(ruby: &Ruby, args: &[Value]) -> Result<Self, Error> {
                let args = scan_args::<(), (), (), (), RHash, ()>(args)?;
                let (values,) =
                    get_kwargs::<_, (RArray,), (), ()>(args.keywords, &[$keyword], &[])?.required;
                make(ruby, values, $deficit).map(Self)
            }
            fn select(
                ruby: &Ruby,
                rb_self: Obj<Self>,
                values: RArray,
            ) -> Result<Option<usize>, Error> {
                rb_self.check_frozen()?;
                let values = inputs(ruby, &rb_self.0, values, $deficit)?;
                let selected = rb_self.0.policy.borrow_mut().select(&values);
                Ok(selected)
            }
            fn deactivate(rb_self: Obj<Self>, lane: Count) -> Result<bool, Error> {
                rb_self.check_frozen()?;
                let changed = rb_self.0.policy.borrow_mut().deactivate(lane.0 as usize);
                Ok(changed)
            }
            const fn lane_count(&self) -> usize {
                self.0.lanes
            }
        }
    };
}

wrapper!(Weighted, "WeightedRoundRobin", "weights", false);
wrapper!(Deficit, "DeficitRoundRobin", "quanta", true);

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    macro_rules! register {
        ($type:ident, $name:literal) => {{
            let class = module.define_class($name, ruby.class_object())?;
            class.define_singleton_method("new", function!($type::new, -1))?;
            class.define_method("select", method!($type::select, 1))?;
            class.define_method("deactivate", method!($type::deactivate, 1))?;
            class.define_method("lane_count", method!($type::lane_count, 0))?;
        }};
    }
    register!(Weighted, "WeightedRoundRobin");
    register!(Deficit, "DeficitRoundRobin");
    Ok(())
}
