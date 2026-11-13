# priority-machines

> **AMS Liu**, NCC-4215, *Liu*-class dispatch escort.

Engine-room flight manual: order departures by deadline, weight, or accounted cost.

Allocation-free scheduling policies. No threads, I/O, wall clock, queue storage,
allocator, or dependencies. The crate is always `no_std`.

The caller supplies a view of eligible work; the scheduler returns an index.
A successful selection **consumes a scheduling turn or cost credit**. Serialize
selection with dequeue/admission in your adapter: this is not a reservation or
an execution guarantee. Filter blocked and cancelled work before selecting.

## Earliest Deadline First

```rust
use core::time::Duration;
use priority_machines::earliest_deadline;

let deadlines = [
    Some(Duration::from_millis(30)),
    None,
    Some(Duration::from_millis(10)),
];
assert_eq!(earliest_deadline(&deadlines), Some(2));
```

Supply absolute deadlines as durations from one common monotonic origin.
`None` means ineligible, not "best effort with no deadline". Equal deadlines
choose the lowest index; order the input by arrival if FIFO ties are required.
Overdue deadlines remain eligible: cancellation and deadline enforcement belong
to the caller. This policy makes no preemption or schedulability guarantee.
It holds no state and never refuses.

## Round Robin

```rust
use priority_machines::RoundRobin;

let mut scheduler = RoundRobin::new();
assert_eq!(scheduler.select(&[false, true, true]), Some(1));
assert_eq!(scheduler.select(&[false, true, true]), Some(2));
assert_eq!(scheduler.select(&[true, true]), Some(0));
assert_eq!(scheduler.select(&[]), None);
```

The unweighted case with no fixed lane count. The `ready` slice sets the lanes
on every call and the cursor wraps, so lanes may appear and vanish between
calls. One `usize` of state, readable through `cursor()`. A `None` (nothing
ready) moves nothing.

## Weighted Round Robin

```rust
use core::num::NonZeroU32;
use priority_machines::WeightedRoundRobin;

# fn example() -> Option<()> {
let mut scheduler = WeightedRoundRobin::new([
    NonZeroU32::new(2)?,
    NonZeroU32::MIN,
]);
assert_eq!(
    core::array::from_fn::<_, 6, _>(|_| scheduler.select(&[true, true])),
    [Some(0), Some(0), Some(1), Some(0), Some(0), Some(1)],
);
# Some(())
# }
# assert_eq!(example(), Some(()));
```

This is **burst WRR**, not smooth WRR. Each continuously ready lane gets its
positive weight in consecutive turns per cycle. It shares dispatch counts, not
bytes or CPU time. Unready lanes forfeit unused turns; idle lanes cannot bank
turns. Zero lanes are supported and select nothing. A `None` (nothing ready)
still ends the visit in progress: that lane's burst starts over at full
weight. The cursor does not move, and a second `None` changes nothing.

## Deficit Round Robin

```rust
use core::num::NonZeroU32;
use priority_machines::DeficitRoundRobin;

# fn example() -> Option<()> {
let mut scheduler = DeficitRoundRobin::new([
    NonZeroU32::new(4)?,
    NonZeroU32::new(2)?,
]);
let heads = [NonZeroU32::new(2), NonZeroU32::new(2)];
assert_eq!(
    core::array::from_fn::<_, 6, _>(|_| scheduler.select(&heads)),
    [Some(0), Some(0), Some(1), Some(0), Some(0), Some(1)],
);
# Some(())
# }
# assert_eq!(example(), Some(()));
```

Each lane receives a positive quantum per visit. A head consumes its positive
cost from that lane's deficit. Remaining credit can serve more heads in the same
visit; insufficient credit carries into the next round. Quanta and head costs
must use the same unit, such as bytes or estimated work units.

Pass the current head cost for each lane, or `None` for an empty lane, on every
selection. Update your queues after dispatch. When a lane drains or is removed,
call `deactivate(lane)` **before refilling it** so old credit cannot leak into a
new busy period. Empty lanes observed during selection also lose their credit.
WRR exposes the same lifecycle hook to relinquish unused turns immediately.

`select` answers `None` exactly when every lane is empty. That is an honoured
"nothing to serve", and it records what the round observed like any other
selection: every lane is empty, so every deficit is zero, and the visit in
progress ends, so the lane under the cursor is paid a fresh quantum when it is
next visited. The cursor does not move, and a second `None` changes nothing.

Costs larger than a quantum still make progress. Entire unproductive rounds are
skipped arithmetically: selection uses at most two lane scans, rather than
looping once per unit of a large cost. Credits use `u64` internally for `u32`
quanta/costs. Idle credit is not accumulated.

Fairness is in accounted cost over continuously backlogged lanes, not in job
counts or actual execution time. A bad cost estimate produces a bad resource
share; this core does not measure or preempt work.

## Deactivation

```rust
use core::num::NonZeroU32;
use priority_machines::{Deactivation, Refusal, WeightedRoundRobin};

# fn example() -> Option<()> {
let mut scheduler = WeightedRoundRobin::new([NonZeroU32::new(3)?, NonZeroU32::MIN]);
assert_eq!(scheduler.select(&[true, true]), Some(0));
assert_eq!(scheduler.deactivate(0), Ok(Deactivation::Deactivated));
assert_eq!(scheduler.deactivate(0), Ok(Deactivation::Idle));
assert_eq!(scheduler.deactivate(2), Err(Refusal::UnknownLane));
assert_eq!(scheduler.select(&[true, true]), Some(1));
# Some(())
# }
# assert_eq!(example(), Some(()));
```

## Contract

- **Refusals.** `Refusal` is the crate's only refusal type and has one
  variant: `UnknownLane`, answered by `WeightedRoundRobin::deactivate` and
  `DeficitRoundRobin::deactivate` for an index of `N` or more. A refused call
  changes nothing: the scheduler equals a clone taken before the call.
- **Mid-visit.** A lane holds something only in two ways. It is *mid-visit*
  when the cursor is on it and this visit has already given it something:
  WRR has served it at least one turn of the burst, DRR has paid it its
  quantum. A lane merely under the cursor, unserved and unpaid, is not being
  visited. In DRR a lane may also hold credit carried from earlier rounds.
- **Honoured outcomes.** `deactivate` answers `Ok(Deactivation)`.
  `Deactivated`: the lane gave up what it held. WRR: it was mid-visit, so its
  unused turns are gone and the cursor moved on. DRR: its credit is zero, and
  if it was mid-visit the visit ended and the cursor moved on; a lane that
  only held credit keeps its place.
- **Repeats.** `Deactivation::Idle` is the honoured repeat: the lane held
  nothing, and nothing changed. It is answered exactly when the call changes
  nothing, for any `N`: the lane has no credit and is not mid-visit. So
  deactivating a lane that was never served or paid costs it neither its
  place in the round nor its next burst or quantum.
- **Drains.** Every `select` and `earliest_deadline` answers `Option<usize>`;
  `None` is "nothing to serve", never a refusal. What a `None` records is
  stated per scheduler above: `RoundRobin` nothing, WRR a reset burst, DRR
  cleared credit and an ended visit.
- **Scopes and restarts.** Lanes are indices the caller chose. The crate
  issues no ids, so there is no scope and nothing to resume: after a restart,
  construct a fresh scheduler.
- **Time.** Only `earliest_deadline` takes time: instants as
  `core::time::Duration` since one monotonic epoch the caller picks. No
  scheduler reads or stores a clock. Weights, quanta and costs are counts.
- **Exhaustive.** `Refusal` and `Deactivation` are exhaustive: a new variant
  is a breaking change every `match` must handle.

## Integration Contract

- Keep lane indices stable for the lifetime of a scheduler.
- Weights, quanta, and lane count are fixed at construction. Construct a fresh
  scheduler when changing them.
- Selection is deterministic for the same initial state and input sequence.
- Snapshot views and dispatch must be coordinated by the caller; the core adds
  no locking.
- No serialization format is promised.
