//! Thin Ruby adapters; scheduling decisions belong exclusively to the core.

mod init;
mod round_robin;
mod scheduler;

use std::{num::NonZeroU32, time::Duration};

use magnus::{
    Error, Integer, RArray, Ruby, TryConvert, Value,
    prelude::*,
    value::{Qfalse, Qtrue},
};
use priority_machines::{Deactivation, Refusal};

fn integer(value: Value, message: &'static str) -> Result<(), Error> {
    if Integer::from_value(value).is_some() {
        return Ok(());
    }
    Err(Error::new(
        Ruby::get_with(value).exception_type_error(),
        message,
    ))
}

/// The core's `NonZeroU32`: an Integer in `1..=u32::MAX`, zero out of range.
pub(crate) fn positive(ruby: &Ruby, value: Value) -> Result<NonZeroU32, Error> {
    integer(value, "expected an Integer")?;
    NonZeroU32::new(u32::try_convert(value)?)
        .ok_or_else(|| Error::new(ruby.exception_range_error(), "expected a positive Integer"))
}

/// A lane index: any Integer a `usize` holds. The core answers for the rest.
#[derive(Clone, Copy)]
pub(crate) struct Lane(pub(crate) usize);

impl TryConvert for Lane {
    fn try_convert(value: Value) -> Result<Self, Error> {
        integer(value, "lane must be an Integer")?;
        usize::try_convert(value).map(Self)
    }
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

/// A monotonic instant: Integer nanoseconds, `0..=u64::MAX`.
fn instant(value: Value) -> Result<Duration, Error> {
    integer(value, "deadline must be Integer nanoseconds or nil")?;
    u64::try_convert(value).map(Duration::from_nanos)
}

pub(crate) fn earliest_deadline(deadlines: RArray) -> Result<Option<usize>, Error> {
    // Convert the complete call before entering the core.
    let deadlines = deadlines
        .into_iter()
        .map(|value| {
            if value.is_nil() {
                Ok(None)
            } else {
                instant(value).map(Some)
            }
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(priority_machines::earliest_deadline(&deadlines))
}
