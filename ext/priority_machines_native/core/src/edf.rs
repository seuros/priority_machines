use core::time::Duration;

/// Select the lowest-index eligible item with the earliest deadline.
///
/// Deadlines must share one monotonic origin. `None` marks an ineligible item.
/// Expired items are not automatically dropped. Returns `None` if no item is
/// eligible, including when the input is empty. Runs in O(n) time.
#[must_use]
pub fn earliest_deadline(deadlines: &[Option<Duration>]) -> Option<usize> {
    deadlines
        .iter()
        .enumerate()
        .filter_map(|(index, deadline)| deadline.map(|value| (index, value)))
        .min_by_key(|&(_, deadline)| deadline)
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests;
