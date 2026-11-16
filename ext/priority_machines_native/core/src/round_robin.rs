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
    /// A drain: `None` is the honoured answer "no lane is ready", returned
    /// exactly then (an empty slice included), and it changes nothing. O(N)
    /// worst case.
    pub const fn select(&mut self, ready: &[bool]) -> Option<usize> {
        let lanes = ready.len();
        if lanes == 0 {
            return None;
        }
        // Search `start..lanes`, then wrap to `0..start`.
        let start = self.cursor % lanes;
        let Some((before, after)) = ready.split_at_checked(start) else {
            debug_assert!(false, "start is below lanes");
            return None;
        };
        let mut lane = start;
        let mut rest = after;
        // `lane` counts the entries consumed, so it stays below `lanes`.
        while let [here, tail @ ..] = rest {
            if *here {
                self.cursor = crate::next_lane(lane, lanes);
                return Some(lane);
            }
            lane += 1;
            rest = tail;
        }
        lane = 0;
        rest = before;
        while let [here, tail @ ..] = rest {
            if *here {
                self.cursor = crate::next_lane(lane, lanes);
                return Some(lane);
            }
            lane += 1;
            rest = tail;
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
