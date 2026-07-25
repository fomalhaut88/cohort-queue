# CohortQueue

A generic Rust queue that combines **FIFO fairness** with **controlled priority-based insertion**.

`CohortQueue` is useful when a new item should be allowed to move ahead of some older items, but should not jump directly to the front of the queue.

Instead of maintaining one flat queue, it groups items into ordered FIFO cohorts. A stronger insertion order allows an item to join an earlier eligible cohort, while preserving the order of all items already inside that cohort.

```text
ordinary FIFO:       A B C D E

strict priority:     E A B C D

CohortQueue:         A C E | B D
                     └─────┘   └─┘
                     cohort 0  cohort 1
```

This provides a practical compromise between:

* arrival time;
* priority, price, stake, or service level;
* predictable FIFO behavior;
* resistance to complete queue bypassing.

## Installation

Using Cargo:

```bash
cargo add cohort-queue
```

Or add the dependency manually:

```toml
[dependencies]
cohort-queue = "0.1"
```

Then import the queue:

```rust
use cohort_queue::CohortQueue;
```

## Quick example

```rust
use cohort_queue::CohortQueue;

fn main() {
    let mut queue = CohortQueue::new();

    // The first ordinary item creates the first cohort.
    queue.push("regular-1", 0);

    // Another order-0 item cannot join that cohort and creates a new one.
    queue.push("regular-2", 0);

    // This item joins the first cohort whose current order is lower than 1.
    queue.push("priority-1", 1);

    // A larger order allows joining the earliest eligible cohort.
    queue.push("priority-2", 3);

    assert_eq!(queue.top_order(), Some(3));

    assert_eq!(queue.pop(), Some("regular-1"));
    assert_eq!(queue.pop(), Some("priority-1"));
    assert_eq!(queue.pop(), Some("priority-2"));
    assert_eq!(queue.pop(), Some("regular-2"));
    assert_eq!(queue.pop(), None);
}
```

The resulting cohorts are:

```text
cohort 0: regular-1, priority-1, priority-2
cohort 1: regular-2
```

The first cohort is drained before the second one, while insertion order remains FIFO inside each cohort.

## How it works

Each internal cohort maintains:

* a FIFO queue of items;
* a current cohort order equal to the order of its most recently accepted item.

When calling:

```rust
queue.push(item, order);
```

the queue finds the earliest cohort whose current order is strictly less than `order`.

The item is appended to that cohort, and the cohort order is replaced with `order`.

If no existing cohort satisfies the condition, a new cohort is created at the back with the supplied order.

Conceptually:

```text
find the first cohort where:

    cohort.order < requested_order
```

A larger `order` therefore gives the item more opportunities to join an earlier cohort. Orders accepted into the same cohort are strictly increasing.

### Example

Suppose the internal cohort orders are:

```text
[5, 4, 4, 2, 1]
```

Insertion with:

```text
order = 4
```

joins the fourth cohort, because it is the first cohort whose current order is strictly lower than `4`:

```text
[5, 4, 4, 2, 1]
          ^
```

After insertion, its order becomes `4`:

```text
[5, 4, 4, 4, 1]
```

The sequence remains non-increasing, which allows the implementation to locate the cohort using binary partitioning.

## Meaning of `order`

The `order` argument is not an absolute queue position.

It is a strictly increasing admission value for a cohort:

* `0` cannot join an existing cohort and therefore creates a new one;
* an item can join only a cohort whose current order is smaller;
* an equal or smaller order cannot join that cohort;
* a larger value can allow an item to join an earlier cohort;
* an item is always appended behind every item already in the selected cohort;
* existing items are never reordered.

Possible real-world interpretations include:

* customer service tier;
* bid or fee level;
* job importance;
* resource contribution;
* reputation score;
* retry urgency;
* transaction priority;
* scheduling weight.

For externally supplied values, it is usually best to normalize them into a small bounded range:

```rust
let order = user_priority.min(MAX_ORDER);
queue.push(job, order);
```

## API

### Create a queue

```rust
let queue = CohortQueue::<String>::new();
```

`CohortQueue<T>` also implements `Default`:

```rust
let queue = CohortQueue::<String>::default();
```

### Insert an item

```rust
queue.push(item, order);
```

Appends the item to the earliest cohort whose current order is strictly lower than `order`, or creates a new cohort at the back.

### Remove the next item

```rust
let item = queue.pop();
```

Removes the front item from the earliest cohort.

When that cohort becomes empty, it is removed automatically.

### Inspect the next item

```rust
let item = queue.top();
```

Returns a reference to the next item without removing it.

### Inspect the earliest cohort order

```rust
let order = queue.top_order();
```

Returns the current order of the earliest cohort. This is the order of the most recently accepted item in that cohort, not necessarily the order originally supplied for the item returned by `top`.

### Get the number of items

```rust
let count = queue.len();
```

### Check whether the queue is empty

```rust
if queue.is_empty() {
    println!("No pending items");
}
```

### Iterate over items

```rust
for item in queue.iter() {
    println!("{item:?}");
}
```

Items are yielded by shared reference in processing order: FIFO within each cohort, starting with the earliest cohort.

`CohortQueue` also implements `IntoIterator` for shared references, so the equivalent shorthand is available:

```rust
for item in &queue {
    println!("{item:?}");
}
```

Neither form consumes the queue or clones its items.

### Mutate items

```rust
for item in queue.iter_mut() {
    // Modify item.
}
```

Mutable iteration follows the same processing order. It is also available through a mutable reference:

```rust
for item in &mut queue {
    // Modify item.
}
```

Mutating stored items does not affect cohort ordering because cohort orders are maintained separately from item values.

### Clear the queue

```rust
queue.clear();
```

Removes all items and cohorts. After the call, the queue is empty and `len()` returns `0`. The queue can be reused normally.

## Properties

### FIFO inside every cohort

Items already placed in the same cohort are always processed in insertion order.

A newly inserted high-order item cannot move ahead of existing members of the cohort it joins.

### Stable existing order

Insertion never rearranges existing items.

Only the destination cohort of the new item is selected, and that cohort's current order is updated.

### Consistent iteration order

`iter`, `iter_mut`, and iteration over references traverse items in the same order in which repeated calls to `pop` would remove them.

### Controlled overtaking

A larger order can move an item ahead of items in later cohorts, but cannot place it before items already waiting in the selected cohort.

This differs from a conventional priority queue, where a sufficiently high-priority item may immediately become the next item.

### Starvation resistance with bounded orders

When insertion orders are bounded by a finite maximum, each cohort can accept only finitely many additional items because every accepted order must be strictly larger than its current order.

Once a cohort reaches the maximum order, no new items can join it, so it can be drained completely.

Therefore, assuming calls to `pop` continue, bounded insertion orders prevent an earlier cohort from accepting new items indefinitely.

### Generic storage

The queue can store any Rust type:

```rust
CohortQueue<String>
CohortQueue<Job>
CohortQueue<Arc<Request>>
CohortQueue<Box<dyn Task>>
```

### Standard-library implementation

The implementation uses `std::collections::VecDeque` and does not require additional runtime dependencies.

## Complexity

Let:

* (n) be the total number of items;
* (c) be the number of active cohorts.

| Operation        |  Complexity |
| ---------------- | ----------: |
| `new`            |      (O(1)) |
| `len`            |      (O(1)) |
| `is_empty`       |      (O(1)) |
| `top`            |      (O(1)) |
| `top_order`      |      (O(1)) |
| `pop`            |      (O(1)) |
| `push`           | (O(\log c)) |
| create iterator  |      (O(1)) |
| full iteration   |    (O(n+c)) |
| `clear`          |    (O(n+c)) |
| memory           |  (O(n + c)) |

`push` uses `VecDeque::partition_point` over the ordered cohort metadata.

Creating an iterator is constant-time. Traversing all items with `iter`, `iter_mut`, `&queue`, or `&mut queue` visits every item and cohort. `clear` destroys all stored items and cohort metadata.

## Practical use cases

### Paid service queues

Customers paying a larger fee can enter an earlier service cohort without bypassing every customer who arrived before them.

```rust
queue.push(request, service_tier);
```

### Background job scheduling

Important jobs can be moved into earlier batches while preserving FIFO order within each batch.

```rust
queue.push(job, job.priority());
```

### Transaction processing

A transaction with a larger fee can receive better placement without implementing strict fee-only ordering.

```rust
queue.push(transaction, normalized_fee);
```

### API rate-limit recovery

Requests with greater urgency can join an earlier retry cohort while older requests in that cohort remain protected.

### Marketplace allocation

Participants can receive better placement based on a bid, stake, or reputation level without allowing unrestricted queue jumping.

### Multiplayer or matchmaking systems

Players with tickets, priority passes, or longer accumulated waiting scores can enter earlier cohorts while retaining deterministic ordering.

## When to use CohortQueue

`CohortQueue` is a good fit when:

* strict FIFO is too inflexible;
* strict priority is too aggressive;
* existing items must never be reordered;
* priority should improve placement but not guarantee immediate service;
* deterministic behavior is important;
* priority values can be mapped to bounded non-negative integers.

## When not to use it

A conventional data structure may be more appropriate when you need:

* the highest-priority item to always be processed next;
* arbitrary removal of queued items;
* priority updates after insertion;
* deadline scheduling;
* weighted round-robin service;
* exact proportional resource allocation;
* ordering by a total comparator;
* globally accurate queue-position estimates.

For strict priority ordering, consider `BinaryHeap`.

For weighted fair scheduling, consider weighted round-robin, deficit round-robin, or weighted fair queueing.

## Recommendations

### Keep order values bounded

Avoid accepting arbitrary unbounded values directly from users:

```rust
const MAX_ORDER: usize = 100;

let order = external_order.min(MAX_ORDER);
queue.push(item, order);
```

Bounded values provide more predictable behavior and prevent a continuous stream of increasing orders from extending an early cohort indefinitely.

### Normalize large business values

Do not use raw prices, balances, or timestamps unless their scale is intentionally meaningful.

Instead, map them to service levels:

```rust
fn fee_to_order(fee: u64) -> usize {
    match fee {
        0..=99 => 0,
        100..=499 => 1,
        500..=999 => 2,
        1000..=4999 => 3,
        _ => 4,
    }
}
```

### Document the meaning of order

Application code should clearly define what one order level represents.

For example:

```text
0 = standard
1 = bronze
2 = silver
3 = gold
4 = critical
```

### Use synchronization for shared access

`CohortQueue` does not provide internal locking.

For concurrent shared mutation, wrap it in an appropriate synchronization primitive:

```rust
use std::sync::{Arc, Mutex};

let queue = Arc::new(Mutex::new(CohortQueue::<String>::new()));
```

In asynchronous applications, use a runtime-compatible mutex when the lock may be held across an `.await`.

## Comparison with other queues

| Queue type                      |    FIFO protection | Priority support | Immediate queue jumping |
| ------------------------------- | -----------------: | ---------------: | ----------------------: |
| FIFO queue                      |               Full |               No |                      No |
| Binary heap                     |                 No |             Full |                     Yes |
| Multiple strict-priority queues |  Within each level |             Full |             Usually yes |
| Weighted round-robin            |  Within each class |         Weighted |                      No |
| `CohortQueue`                   | Within each cohort |       Controlled |                      No |

`CohortQueue` is intended for systems where priority should influence placement without completely replacing arrival order.

## Full example

```rust
use cohort_queue::CohortQueue;

#[derive(Debug, PartialEq, Eq)]
struct Request {
    id: u64,
    customer: &'static str,
}

fn main() {
    let mut queue = CohortQueue::new();

    queue.push(
        Request {
            id: 1,
            customer: "Alice",
        },
        0,
    );

    queue.push(
        Request {
            id: 2,
            customer: "Bob",
        },
        0,
    );

    queue.push(
        Request {
            id: 3,
            customer: "Carol",
        },
        1,
    );

    queue.push(
        Request {
            id: 4,
            customer: "Dave",
        },
        5,
    );

    while let Some(request) = queue.pop() {
        println!(
            "Processing request {} from {}",
            request.id,
            request.customer
        );
    }
}
```

Output:

```text
Processing request 1 from Alice
Processing request 3 from Carol
Processing request 4 from Dave
Processing request 2 from Bob
```

Alice remains first because she was already in the selected cohort.

Carol and Dave receive improved placement, but they are appended behind Alice rather than moving ahead of her.

Bob remains in the following cohort.

## Conceptual model

The queue can be viewed as a sequence of FIFO batches:

```text
C₀ | C₁ | C₂ | C₃ | ...
```

A new item attempts to join the earliest cohort whose current order is strictly lower than its requested order.

The complete processing order is:

```text
FIFO(C₀) → FIFO(C₁) → FIFO(C₂) → FIFO(C₃) → ...
```

This creates a staircase-shaped allocation of items across cohorts and provides a simple form of priority-aware batching.

## License

```text
MIT
```
