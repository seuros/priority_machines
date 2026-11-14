use core::num::NonZeroU32;

use crate::{Deactivation, Refusal};

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
    /// A drain: `None` is the honoured answer "no lane is ready", returned
    /// exactly then. It still records what the round observed: the lane being
    /// visited forfeits its unused turns, so its burst starts over at full
    /// weight. The cursor stays where it was, and a second `None` in a row
    /// changes nothing. Runs in O(N) worst-case time; repeated dispatches from
    /// the current lane take O(1).
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

    /// Relinquish the unused turns of the burst this lane is in the middle of.
    ///
    /// Call when a queue drains or is removed, before reusing that lane.
    ///
    /// A lane is mid-burst when the cursor is on it and this visit has already
    /// served it at least one turn. Only then does it hold anything: its
    /// unused turns. [`Deactivation::Deactivated`]: the lane was mid-burst;
    /// those turns are gone and the cursor moved on. [`Deactivation::Idle`]:
    /// it was not (another lane is under the cursor, or this one is and its
    /// burst is untouched), and nothing changed, so a lane that was never
    /// served keeps its place. `Idle` is answered exactly when the call
    /// changes nothing.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownLane`] for an index of `N` or more; nothing changes.
    pub const fn deactivate(&mut self, lane: usize) -> Result<Deactivation, Refusal> {
        if lane >= N {
            return Err(Refusal::UnknownLane);
        }
        if !self.mid_burst(lane) {
            return Ok(Deactivation::Idle);
        }
        self.advance();
        Ok(Deactivation::Deactivated)
    }

    /// The cursor is on `lane` and this visit has served it a turn.
    /// `remaining` stays within `1..=weight`: a spent burst moves the cursor.
    const fn mid_burst(&self, lane: usize) -> bool {
        lane == self.cursor && self.remaining < self.weights[lane].get()
    }

    const fn advance(&mut self) {
        self.cursor = crate::next_lane(self.cursor, N);
        self.remaining = self.weights[self.cursor].get();
    }
}

#[cfg(test)]
mod tests;
