//! Thin Ruby adapters; scheduling decisions belong exclusively to the core.

mod init;
mod scheduler;

use std::{num::NonZeroU32, time::Duration};

use magnus::{Error, Float, Integer, RArray, Ruby, TryConvert, Value, prelude::*};

#[derive(Clone, Copy)]
pub(crate) struct Count(pub(crate) u32);

impl TryConvert for Count {
    fn try_convert(value: Value) -> Result<Self, Error> {
        let ruby = Ruby::get_with(value);
        if Integer::from_value(value).is_none() {
            return Err(Error::new(
                ruby.exception_type_error(),
                "expected an Integer",
            ));
        }
        u32::try_convert(value).map(Self)
    }
}

pub(crate) fn positive(ruby: &Ruby, value: Value) -> Result<NonZeroU32, Error> {
    NonZeroU32::new(Count::try_convert(value)?.0)
        .ok_or_else(|| Error::new(ruby.exception_arg_error(), "expected a positive u32"))
}

fn duration(ruby: &Ruby, value: Value) -> Result<Duration, Error> {
    if Integer::from_value(value).is_some() {
        return u64::try_convert(value).map(Duration::from_secs);
    }
    if Float::from_value(value).is_none() {
        return Err(Error::new(
            ruby.exception_type_error(),
            "seconds must be Integer or Float",
        ));
    }
    Duration::try_from_secs_f64(f64::try_convert(value)?).map_err(|_| {
        Error::new(
            ruby.exception_arg_error(),
            "seconds must be finite, nonnegative and representable",
        )
    })
}

pub(crate) fn earliest_deadline(ruby: &Ruby, deadlines: RArray) -> Result<Option<usize>, Error> {
    // Convert the complete call before entering the core.
    let deadlines = deadlines
        .into_iter()
        .map(|value| {
            if value.is_nil() {
                Ok(None)
            } else {
                duration(ruby, value).map(Some)
            }
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(priority_machines::earliest_deadline(&deadlines))
}
