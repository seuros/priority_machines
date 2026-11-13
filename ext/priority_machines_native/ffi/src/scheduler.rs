//! Exact const-generic lane counts, erased only at the Ruby boundary.

use std::{cell::RefCell, num::NonZeroU32};

use magnus::{Error, RArray, RModule, Ruby, Symbol, function, method, prelude::*, typed_data::Obj};
use priority_machines::{Deactivation, DeficitRoundRobin, Refusal, WeightedRoundRobin};

use crate::{Lane, deactivation, positive, ready, refusal};

// This trait erases const-generic sizes, never adds inactive/phantom lanes.
trait Policy: Send {
    fn select(&mut self, heads: &[Option<NonZeroU32>]) -> Option<usize>;
    fn deactivate(&mut self, lane: usize) -> Result<Deactivation, Refusal>;
}

impl<const N: usize> Policy for WeightedRoundRobin<N> {
    fn select(&mut self, heads: &[Option<NonZeroU32>]) -> Option<usize> {
        self.select(&std::array::from_fn(|index| heads[index].is_some()))
    }
    fn deactivate(&mut self, lane: usize) -> Result<Deactivation, Refusal> {
        self.deactivate(lane)
    }
}

impl<const N: usize> Policy for DeficitRoundRobin<N> {
    fn select(&mut self, heads: &[Option<NonZeroU32>]) -> Option<usize> {
        self.select(&std::array::from_fn(|index| heads[index]))
    }
    fn deactivate(&mut self, lane: usize) -> Result<Deactivation, Refusal> {
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
                _ => return Err(Error::new(ruby.exception_arg_error(), "lane count must be 0..16")),
            }
        };
    }
    let policy = sized!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
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
            if !deficit {
                ready(ruby, value).map(|flag| flag.then_some(NonZeroU32::MIN))
            } else if value.is_nil() {
                Ok(None)
            } else {
                positive(ruby, value).map(Some)
            }
        })
        .collect()
}

#[magnus::wrap(class = "PriorityMachines::WeightedRoundRobin", free_immediately, size)]
struct Weighted(State);

#[magnus::wrap(class = "PriorityMachines::DeficitRoundRobin", free_immediately, size)]
struct Deficit(State);

macro_rules! wrapper {
    ($type:ident, $deficit:literal) => {
        impl $type {
            fn new(ruby: &Ruby, values: RArray) -> Result<Self, Error> {
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
            fn deactivate(ruby: &Ruby, rb_self: Obj<Self>, lane: Lane) -> Result<Symbol, Error> {
                rb_self.check_frozen()?;
                let outcome = rb_self.0.policy.borrow_mut().deactivate(lane.0);
                Ok(ruby.to_symbol(outcome.map_or_else(refusal, deactivation)))
            }
        }
    };
}

wrapper!(Weighted, false);
wrapper!(Deficit, true);

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    macro_rules! register {
        ($type:ident, $name:literal) => {{
            let class = module.define_class($name, ruby.class_object())?;
            class.define_singleton_method("new", function!($type::new, 1))?;
            class.define_method("select", method!($type::select, 1))?;
            class.define_method("deactivate", method!($type::deactivate, 1))?;
        }};
    }
    register!(Weighted, "WeightedRoundRobin");
    register!(Deficit, "DeficitRoundRobin");
    Ok(())
}
