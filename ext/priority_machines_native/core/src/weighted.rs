use core::num::NonZeroU32;

/// Burst weighted round robin over a fixed set of caller-owned lanes.
///
/// Weights count dispatches, not work cost. Empty lanes forfeit their remaining
/// turns. The caller owns queue storage and execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightedRoundRobin<const N: usize> {
    weights: [NonZeroU32; N],
    cursor: usize,
    remaining: u32,
}

impl<const N: usize> WeightedRoundRobin<N> {
    /// Construct a scheduler with positive weights, initially visiting lane zero.
    #[must_use]
    pub const fn new(weights: [NonZeroU32; N]) -> Self {
        Self {
            remaining: if N == 0 { 0 } else { weights[0].get() },
            weights,
            cursor: 0,
        }
    }

    /// Consume one turn from the next ready lane.
    ///
    /// Returns `None` exactly when no lane is ready. Runs in O(N) worst-case
    /// time; repeated dispatches from the current lane take O(1).
    pub fn select(&mut self, ready: &[bool; N]) -> Option<usize> {
        for _ in 0..N {
            let lane = self.cursor;
            if ready[lane] {
                self.remaining -= 1;
                if self.remaining == 0 {
                    self.advance();
                }
                return Some(lane);
            }
            self.advance();
        }
        None
    }

    /// Relinquish unused turns if this lane is currently being visited.
    ///
    /// Call when a queue drains or is removed, before reusing that lane.
    /// Returns `false` for an out-of-range index, without changing state.
    pub const fn deactivate(&mut self, lane: usize) -> bool {
        if lane >= N {
            return false;
        }
        if lane == self.cursor {
            self.advance();
        }
        true
    }

    const fn advance(&mut self) {
        self.cursor = if self.cursor + 1 == N {
            0
        } else {
            self.cursor + 1
        };
        self.remaining = self.weights[self.cursor].get();
    }
}

#[cfg(test)]
mod tests;
