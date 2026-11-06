/// Plain round robin over however many lanes the caller has right now.
///
/// The lane count is the length of the `ready` slice on each call, so lanes may
/// be added or removed between calls; the cursor wraps. All lanes weigh the
/// same. The caller owns queue storage and execution.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoundRobin {
    cursor: usize,
}

impl RoundRobin {
    /// Construct a scheduler initially visiting lane zero.
    #[must_use]
    pub const fn new() -> Self {
        Self { cursor: 0 }
    }

    /// The next ready lane at or after the cursor; the cursor moves past it.
    ///
    /// Returns `None` exactly when no lane is ready. O(N) worst case.
    pub const fn select(&mut self, ready: &[bool]) -> Option<usize> {
        let lanes = ready.len();
        if lanes == 0 {
            return None;
        }
        let mut lane = self.cursor % lanes;
        let mut visited = 0;
        while visited < lanes {
            if ready[lane] {
                self.cursor = if lane + 1 == lanes { 0 } else { lane + 1 };
                return Some(lane);
            }
            lane = if lane + 1 == lanes { 0 } else { lane + 1 };
            visited += 1;
        }
        None
    }

    /// Where the next search starts.
    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }
}

#[cfg(test)]
mod tests;
