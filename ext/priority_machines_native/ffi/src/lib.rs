//! Thin Ruby adapters; scheduling decisions belong exclusively to the core.

mod init;
mod round_robin;
mod scheduler;

use std::time::Duration;

use magnus::{Error, RArray, Ruby, Value};
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

matryona_machines::symbols! {
    /// One symbol per core variant, the snake case of its name.
    pub(crate) Refusals for Refusal as "a refusal" {
        UnknownLane => "unknown_lane",
    }
}

matryona_machines::symbols! {
    /// An honoured deactivate.
    pub(crate) Deactivations for Deactivation as "a deactivation" {
        Deactivated => "deactivated",
        Idle => "idle",
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
