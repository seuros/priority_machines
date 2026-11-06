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
turns. Zero lanes are supported and select nothing.

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

Costs larger than a quantum still make progress. Entire unproductive rounds are
skipped arithmetically: selection uses at most two lane scans, rather than
looping once per unit of a large cost. Credits use `u64` internally for `u32`
quanta/costs. Idle credit is not accumulated.

Fairness is in accounted cost over continuously backlogged lanes, not in job
counts or actual execution time. A bad cost estimate produces a bad resource
share; this core does not measure or preempt work.

## Integration Contract

- Keep lane indices stable for the lifetime of a scheduler.
- Weights, quanta, and lane count are fixed at construction. Construct a fresh
  scheduler when changing them.
- Selection is deterministic for the same initial state and input sequence.
- Snapshot views and dispatch must be coordinated by the caller; the core adds
  no locking.
- No serialization format or Ruby binding is promised by this initial version.
