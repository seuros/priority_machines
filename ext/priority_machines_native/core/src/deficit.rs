use core::num::NonZeroU32;

use crate::{Deactivation, Refusal};

/// Deficit round robin over fixed, caller-owned lanes with explicit head costs.
///
/// Positive quanta and costs use the same unit. Empty lanes lose their credit.
/// Each successful selection charges one head; update the queue view before
/// selecting again. This policy measures accounted work, not actual CPU time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeficitRoundRobin<const N: usize> {
    quanta: [NonZeroU32; N],
    deficits: [u64; N],
    cursor: usize,
    needs_quantum: bool,
}

impl<const N: usize> DeficitRoundRobin<N> {
    /// Construct a scheduler with positive quanta and no accumulated credit.
    #[must_use]
    pub const fn new(quanta: [NonZeroU32; N]) -> Self {
        Self {
            quanta,
            deficits: [0; N],
            cursor: 0,
            needs_quantum: true,
        }
    }

    /// Charge and select the next affordable head.
    ///
    /// `None` marks an empty lane. Expensive heads accumulate enough credit
    /// via skipped rounds in O(N) time, using at most two selection scans.
    ///
    /// A drain: `None` is the honoured answer "every lane is empty", returned
    /// exactly then. It still records what the round observed, as every
    /// selection does: each empty lane loses its credit, so every deficit is
    /// zero, and the visit in progress ends, so the lane under the cursor is
    /// paid a fresh quantum when it is next visited. The cursor stays where
    /// it was, and a second `None` in a row changes nothing.
    pub fn select(&mut self, heads: &[Option<NonZeroU32>; N]) -> Option<usize> {
        // Observe every empty lane, even if selection stops before visiting it.
        for (deficit, head) in self.deficits.iter_mut().zip(heads) {
            if head.is_none() {
                *deficit = 0;
            }
        }
        if let Some(lane) = self.select_round(heads) {
            return Some(lane);
        }

        // Every active head is now unaffordable and every lane has been visited.
        // Skip only whole rounds in which no head could have been dispatched.
        let rounds = heads
            .iter()
            .enumerate()
            .filter_map(|(lane, head)| {
                head.map(|cost| {
                    (u64::from(cost.get()) - self.deficits[lane])
                        .div_ceil(u64::from(self.quanta[lane].get()))
                })
            })
            .min()?;
        let skipped = rounds - 1;
        for (lane, head) in heads.iter().enumerate() {
            if head.is_some() {
                self.deficits[lane] += skipped * u64::from(self.quanta[lane].get());
            }
        }
        self.select_round(heads)
    }

    /// Discard a lane's credit and end the visit it is in the middle of.
    ///
    /// Call when a queue drains or is removed, before refilling that lane.
    ///
    /// A lane is mid-visit when the cursor is on it and this visit has already
    /// paid it its quantum. A lane merely under the cursor, not yet paid, is
    /// not being visited. [`Deactivation::Deactivated`]: the lane held credit,
    /// was mid-visit, or both; the credit is gone, and a visit in progress
    /// ended, moving the cursor on. A lane that only held credit keeps its
    /// place. [`Deactivation::Idle`]: it held no credit and was not mid-visit,
    /// and nothing changed. `Idle` is answered exactly when the call changes
    /// nothing.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownLane`] for an index of `N` or more; nothing changes.
    pub const fn deactivate(&mut self, lane: usize) -> Result<Deactivation, Refusal> {
        if lane >= N {
            return Err(Refusal::UnknownLane);
        }
        let mid_visit = lane == self.cursor && !self.needs_quantum;
        if self.deficits[lane] == 0 && !mid_visit {
            return Ok(Deactivation::Idle);
        }
        self.deficits[lane] = 0;
        if mid_visit {
            self.advance();
        }
        Ok(Deactivation::Deactivated)
    }

    fn select_round(&mut self, heads: &[Option<NonZeroU32>; N]) -> Option<usize> {
        for _ in 0..N {
            let lane = self.cursor;
            if let Some(cost) = heads[lane] {
                if self.needs_quantum {
                    // An unsuccessful visit leaves deficit < cost <= u32::MAX.
                    // Adding a u32 quantum therefore cannot overflow u64.
                    self.deficits[lane] += u64::from(self.quanta[lane].get());
                    self.needs_quantum = false;
                }
                if self.deficits[lane] >= u64::from(cost.get()) {
                    self.deficits[lane] -= u64::from(cost.get());
                    return Some(lane);
                }
            }
            self.advance();
        }
        None
    }

    const fn advance(&mut self) {
        self.cursor = if self.cursor + 1 == N {
            0
        } else {
            self.cursor + 1
        };
        self.needs_quantum = true;
    }
}

#[cfg(test)]
mod tests;
