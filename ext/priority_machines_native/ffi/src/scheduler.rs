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

/// The largest `N` this binding compiles the core's schedulers for. The core
/// itself has no ceiling: `N` is its const generic.
const MAX_LANES: usize = 16;

/// Which core scheduler a wrapper holds.
#[derive(Clone, Copy)]
enum Kind {
    Weighted,
    Deficit,
}

fn make(ruby: &Ruby, values: RArray, kind: Kind) -> Result<State, Error> {
    let name = match kind {
        Kind::Weighted => "weights",
        Kind::Deficit => "quanta",
    };
    let values = values
        .into_iter()
        .map(|value| positive(ruby, value, name))
        .collect::<Result<Vec<_>, _>>()?;
    macro_rules! sized {
        ($($n:literal),*) => {
            match values.len() {
                $($n => {
                    let array = std::array::from_fn::<_, $n, _>(|index| values[index]);
                    let policy: Box<dyn Policy> = match kind {
                        Kind::Weighted => Box::new(WeightedRoundRobin::new(array)),
                        Kind::Deficit => Box::new(DeficitRoundRobin::new(array)),
                    };
                    policy
                },)*
                _ => return Err(Error::new(
                    ruby.exception_range_error(),
                    format!("{name} holds at most {MAX_LANES} lanes"),
                )),
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
    kind: Kind,
) -> Result<Vec<Option<NonZeroU32>>, Error> {
    let name = match kind {
        Kind::Weighted => "ready",
        Kind::Deficit => "heads",
    };
    if values.len() != state.lanes {
        return Err(Error::new(
            ruby.exception_range_error(),
            format!(
                "{name} must hold one entry per lane: {} given, {} lanes",
                values.len(),
                state.lanes
            ),
        ));
    }
    values
        .into_iter()
        .map(|value| match kind {
            Kind::Weighted => ready(ruby, value).map(|flag| flag.then_some(NonZeroU32::MIN)),
            Kind::Deficit if value.is_nil() => Ok(None),
            Kind::Deficit => positive(ruby, value, name).map(Some),
        })
        .collect()
}

#[magnus::wrap(class = "PriorityMachines::WeightedRoundRobin", free_immediately, size)]
struct Weighted(State);

#[magnus::wrap(class = "PriorityMachines::DeficitRoundRobin", free_immediately, size)]
struct Deficit(State);

macro_rules! wrapper {
    ($type:ident, $kind:expr) => {
        impl $type {
            fn new(ruby: &Ruby, values: RArray) -> Result<Self, Error> {
                make(ruby, values, $kind).map(Self)
            }
            fn select(
                ruby: &Ruby,
                rb_self: Obj<Self>,
                values: RArray,
            ) -> Result<Option<usize>, Error> {
                rb_self.check_frozen()?;
                let values = inputs(ruby, &rb_self.0, values, $kind)?;
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

wrapper!(Weighted, Kind::Weighted);
wrapper!(Deficit, Kind::Deficit);

pub fn define(ruby: &Ruby, module: RModule) -> Result<(), Error> {
    macro_rules! register {
        ($type:ident, $name:literal) => {{
            let class = module.define_class($name, ruby.class_object())?;
            class.undef_default_alloc_func();
            class.define_singleton_method("new", function!($type::new, 1))?;
            class.define_method("select", method!($type::select, 1))?;
            class.define_method("deactivate", method!($type::deactivate, 1))?;
        }};
    }
    register!(Weighted, "WeightedRoundRobin");
    register!(Deficit, "DeficitRoundRobin");
    module.const_set("MAX_LANES", MAX_LANES)?;
    Ok(())
}
