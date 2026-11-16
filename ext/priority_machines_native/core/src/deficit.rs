use core::num::NonZeroU32;

use crate::{Deactivation, Refusal};

/// The head of a lane, as [`DeficitRoundRobin::select`] reads it.
///
/// A bare `NonZeroU32` is a head that costs that much and never says its
/// lane empties: call [`DeficitRoundRobin::deactivate`] when the lane drains.
/// An adapter that knows a head is the last of its lane says so with
/// [`Head::last`], and selecting it deactivates the lane in the same call,
/// with no index to pass back and no second call that could be refused.
pub trait Head: Copy {
    /// What dispatching this head charges against its lane's credit.
    fn cost(self) -> NonZeroU32;

    /// Whether dispatching this head empties its lane.
    fn last(self) -> bool;
}

impl Head for NonZeroU32 {
    fn cost(self) -> NonZeroU32 {
        self
    }

    fn last(self) -> bool {
        false
    }
}

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
    ///
    /// A selected head whose [`Head::last`] is `true` empties its lane: the
    /// lane is deactivated before `select` returns, exactly as
    /// [`Self::deactivate`] would, so its leftover credit is gone and the
    /// cursor moves on.
    pub fn select<H: Head>(&mut self, heads: &[Option<H>; N]) -> Option<usize> {
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
        // The failed round left every active lane with `deficit < cost`, so
        // the subtraction cannot wrap and `rounds >= 1`. `skipped` is below
        // every active lane's own round count, so `deficit + skipped *
        // quantum` stays below that lane's `cost <= u32::MAX`: no overflow.
        let rounds = heads
            .iter()
            .zip(&self.deficits)
            .zip(&self.quanta)
            .filter_map(|((head, deficit), quantum)| {
                head.map(|head| {
                    (u64::from(head.cost().get()) - deficit).div_ceil(u64::from(quantum.get()))
                })
            })
            .min()?;
        let skipped = rounds - 1;
        for ((deficit, head), quantum) in self.deficits.iter_mut().zip(heads).zip(&self.quanta) {
            if head.is_some() {
                *deficit += skipped * u64::from(quantum.get());
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
        let mid_visit = lane == self.cursor && !self.needs_quantum;
        // The lookup is the bounds check: an index of `N` or more has no deficit.
        let Some(deficit) = crate::entry_mut(&mut self.deficits, lane) else {
            return Err(Refusal::UnknownLane);
        };
        if *deficit == 0 && !mid_visit {
            return Ok(Deactivation::Idle);
        }
        *deficit = 0;
        if mid_visit {
            self.advance();
        }
        Ok(Deactivation::Deactivated)
    }

    fn select_round<H: Head>(&mut self, heads: &[Option<H>; N]) -> Option<usize> {
        for _ in 0..N {
            let lane = self.cursor;
            let (Some(head), Some(deficit), Some(quantum)) = (
                heads.get(lane),
                self.deficits.get_mut(lane),
                self.quanta.get(lane),
            ) else {
                debug_assert!(false, "the cursor is one of the N lanes");
                return None;
            };
            if let Some(head) = *head {
                let cost = head.cost();
                if self.needs_quantum {
                    // An unsuccessful visit leaves deficit < cost <= u32::MAX.
                    // Adding a u32 quantum therefore cannot overflow u64.
                    *deficit += u64::from(quantum.get());
                    self.needs_quantum = false;
                }
                if *deficit >= u64::from(cost.get()) {
                    *deficit -= u64::from(cost.get());
                    if head.last() {
                        // The lane is mid-visit: it was just paid and charged.
                        *deficit = 0;
                        self.advance();
                    }
                    return Some(lane);
                }
            }
            self.advance();
        }
        None
    }

    const fn advance(&mut self) {
        self.cursor = crate::next_lane(self.cursor, N);
        self.needs_quantum = true;
    }
}

#[cfg(test)]
mod tests;
