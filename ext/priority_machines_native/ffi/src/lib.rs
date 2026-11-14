//! Thin Ruby adapters; scheduling decisions belong exclusively to the core.

mod init;
mod round_robin;
mod scheduler;

use std::time::Duration;

use magnus::{
    Error, RArray, Ruby, Value,
    value::{Qfalse, Qtrue},
};
use matryona_machines::ruby::{exact_array, integer, optional};
use priority_machines::{Deactivation, Refusal};

/// `value` as exactly an `Array` (no subclass), `TypeError` naming the
/// argument otherwise.
pub(crate) fn array(ruby: &Ruby, value: Value, name: &'static str) -> Result<RArray, Error> {
    exact_array(ruby, value, format!("{name} must be an Array"))
}

/// A lane index: any Integer a `usize` holds. The core answers for the rest.
pub(crate) fn lane(value: Value) -> Result<usize, Error> {
    integer(value, "lane")
}

/// Ready flags are real booleans: no truthiness, so `nil` is not "empty".
pub(crate) fn ready(ruby: &Ruby, value: Value) -> Result<bool, Error> {
    if Qtrue::from_value(value).is_some() {
        Ok(true)
    } else if Qfalse::from_value(value).is_some() {
        Ok(false)
    } else {
        Err(Error::new(
            ruby.exception_type_error(),
            "ready entries must be true or false",
        ))
    }
}

/// One symbol per core variant, the snake case of its name.
pub(crate) const fn refusal(refusal: Refusal) -> &'static str {
    match refusal {
        Refusal::UnknownLane => "unknown_lane",
    }
}

pub(crate) const fn deactivation(deactivation: Deactivation) -> &'static str {
    match deactivation {
        Deactivation::Deactivated => "deactivated",
        Deactivation::Idle => "idle",
    }
}

pub(crate) fn earliest_deadline(ruby: &Ruby, deadlines: Value) -> Result<Option<usize>, Error> {
    // Convert the complete call before entering the core.
    let deadlines = array(ruby, deadlines, "deadlines")?
        .into_iter()
        .map(|value| Ok(optional::<u64>(value, "deadlines")?.map(Duration::from_nanos)))
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(priority_machines::earliest_deadline(&deadlines))
}
