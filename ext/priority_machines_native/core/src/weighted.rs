use core::num::NonZeroU32;

use crate::{Deactivation, Lane};

/// The head of a ready lane, as [`WeightedRoundRobin::select`] reads it.
///
/// `()` is a ready lane that never says it empties: call
/// [`WeightedRoundRobin::deactivate`] when it drains. An adapter that knows
/// a head is the last of its lane says so with [`Ready::last`], and selecting
/// it gives up the rest of the lane's burst in the same call.
pub trait Ready: Copy {
    /// Whether dispatching this head empties its lane.
    fn last(self) -> bool;
}

impl Ready for () {
    fn last(self) -> bool {
        false
    }
}

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

    /// Consume one turn from the next ready lane: a lane whose entry in
    /// `heads` is `Some`. Answers that lane and its head, so the caller has
    /// nothing to look up.
    ///
    /// A selected head whose [`Ready::last`] is `true` empties its lane: the
    /// rest of its burst is given up before `select` returns, exactly as
    /// [`Self::deactivate`] would, and the cursor moves on.
    ///
    /// A drain: `None` is the honoured answer "no lane is ready", returned
    /// exactly then. It still records what the round observed: the lane being
    /// visited forfeits its unused turns, so its burst starts over at full
    /// weight. The cursor stays where it was, and a second `None` in a row
    /// changes nothing. Runs in O(N) worst-case time; repeated dispatches from
    /// the current lane take O(1).
    pub fn select<H: Ready>(&mut self, heads: &[Option<H>; N]) -> Option<(Lane<N>, H)> {
        for _ in 0..N {
            let lane = Lane(self.cursor);
            let Some(head) = heads.get(lane.get()) else {
                debug_assert!(false, "the cursor is one of the N lanes");
                return None;
            };
            if let Some(head) = *head {
                // `remaining` stays within `1..=weight` while a lane is
                // visited, so this cannot wrap.
                self.remaining -= 1;
                // A spent burst moves on; so does a lane this head empties,
                // which is mid-burst now that it was served a turn.
                if self.remaining == 0 || head.last() {
                    self.advance();
                }
                return Some((lane, head));
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
    /// changes nothing. Never refuses: a [`Lane`] is one of the `N` lanes.
    pub const fn deactivate(&mut self, lane: Lane<N>) -> Deactivation {
        if !self.mid_burst(lane.get()) {
            return Deactivation::Idle;
        }
        self.advance();
        Deactivation::Deactivated
    }

    /// The cursor is on `lane` and this visit has served it a turn.
    /// `remaining` stays within `1..=weight`: a spent burst moves the cursor.
    const fn mid_burst(&self, lane: usize) -> bool {
        if lane != self.cursor {
            return false;
        }
        if let Some(weight) = crate::entry(&self.weights, lane) {
            self.remaining < weight.get()
        } else {
            debug_assert!(false, "a Lane is one of the N lanes");
            false
        }
    }

    /// Move to the next lane and refill its burst. The cursor is one of the
    /// `N` lanes, so the next one always has a weight.
    const fn advance(&mut self) {
        let next = crate::next_lane(self.cursor, N);
        match crate::entry(&self.weights, next) {
            Some(weight) => {
                self.cursor = next;
                self.remaining = weight.get();
            }
            None => debug_assert!(false, "the cursor is one of the N lanes"),
        }
    }
}

#[cfg(test)]
mod tests;
