//! A generic Rust queue that combines **FIFO fairness** with
//! **controlled priority-based insertion**.
//!
//! `CohortQueue` is useful when a new item should be allowed to move ahead
//! of some older items, but should not jump directly to the front of the
//! queue. It achieves this by grouping items into ordered FIFO cohorts.
//! A higher insertion order allows an item to join an earlier cohort,
//! while preserving the order of all items already inside that cohort.
//!
//! # Example
//!
//! ```rust
//! use cohort_queue::CohortQueue;
//!
//! let mut queue = CohortQueue::new();
//!
//! // First ordinary item creates the first cohort.
//! queue.push("regular-1", 0);
//!
//! // Another order-0 item cannot join that cohort and creates a new one.
//! queue.push("regular-2", 0);
//!
//! // This item may join a cohort whose historical width is at most 1.
//! queue.push("priority-1", 1);
//!
//! assert_eq!(queue.pop(), Some("regular-1"));
//! assert_eq!(queue.pop(), Some("priority-1"));
//! assert_eq!(queue.pop(), Some("regular-2"));
//! ```
//!
//! # Complexity
//!
//! | Operation  |  Complexity |
//! | ---------- | ----------: |
//! | `new`      |      O(1)   |
//! | `len`      |      O(1)   |
//! | `is_empty` |      O(1)   |
//! | `top`      |      O(1)   |
//! | `pop`      |      O(1)   |
//! | `push`     |  O(log c)   |
//!
//! where `c` is the number of active cohorts.

use std::collections::VecDeque;


#[derive(Debug)]
struct SubQueue<T> {
    deque: VecDeque<T>,
    order: usize,
}


impl<T> SubQueue<T> {
    fn new() -> Self {
        Self {
            deque: VecDeque::new(),
            order: 0,
        }
    }

    fn push(&mut self, item: T) {
        self.deque.push_back(item);
        self.order += 1;
    }

    fn pop(&mut self) -> Option<T> {
        self.deque.pop_front()
    }
}


/// A queue that combines FIFO fairness with controlled
/// priority-based insertion.
///
/// Items are grouped into ordered cohorts. A higher insertion order allows
/// an item to join an earlier cohort, but it always remains behind
/// existing members of that cohort.
#[derive(Debug)]
pub struct CohortQueue<T> {
    sub_queues: VecDeque<SubQueue<T>>,
    len: usize,
}


impl<T> CohortQueue<T> {
    /// Creates a new, empty `CohortQueue`.
    pub fn new() -> Self {
        Self {
            sub_queues: VecDeque::new(),
            len: 0,
        }
    }

    /// Returns true if the queue contains no items.
    pub fn is_empty(&self) -> bool {
        self.sub_queues.is_empty()
    }

    /// Returns the total number of items currently in the queue across all
    /// cohorts.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Appends an item to the earliest eligible cohort or creates a new one.
    ///
    /// An item joins the first cohort whose historical insertion count is
    /// less than or equal to `order`. Returns the selected cohort's
    /// historical insertion count before the new item was added.
    pub fn push(&mut self, item: T, order: usize) -> usize {
        if let Some(sub_queue) = self.find_sub_queue(order) {
            let order_inserted = sub_queue.order;
            sub_queue.push(item);
            self.len += 1;
            order_inserted
        } else {
            let mut sub_queue = SubQueue::new();
            sub_queue.push(item);
            self.sub_queues.push_back(sub_queue);
            self.len += 1;
            0
        }
    }

    /// Removes and returns the front item from the earliest non-empty cohort.
    ///
    /// When a cohort becomes empty, it is removed automatically.
    pub fn pop(&mut self) -> Option<T> {
        if let Some(sub_queue) = self.sub_queues.front_mut() {
            let item = sub_queue.pop();
            if sub_queue.deque.is_empty() {
                self.sub_queues.pop_front();
            }
            self.len -= 1;
            item
        } else {
            None
        }
    }

    /// Returns a reference to the next item without removing it.
    pub fn top(&self) -> Option<&T> {
        self.sub_queues.front().and_then(|q| q.deque.front())
    }

    fn find_sub_queue(&mut self, order: usize) -> Option<&mut SubQueue<T>> {
        let ix = self.sub_queues.partition_point(|q| q.order > order);
        self.sub_queues.get_mut(ix)
    }
}


impl<T> Default for CohortQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trivial() {
        let mut cq = CohortQueue::<i32>::new();

        assert_eq!(cq.top(), None);

        assert_eq!(cq.push(32, 0), 0);
        assert_eq!(cq.push(35, 0), 0);
        assert_eq!(cq.push(38, 0), 0);

        assert_eq!(cq.len(), 3);
        assert_eq!(cq.top(), Some(&32));

        assert_eq!(cq.pop(), Some(32));
        assert_eq!(cq.len(), 2);

        assert_eq!(cq.pop(), Some(35));
        assert_eq!(cq.len(), 1);

        assert_eq!(cq.pop(), Some(38));
        assert_eq!(cq.len(), 0);

        assert_eq!(cq.pop(), None);

        assert!(cq.is_empty());
    }

    #[test]
    fn test_increase() {
        let mut cq = CohortQueue::<i32>::new();

        assert_eq!(cq.top(), None);

        assert_eq!(cq.push(32, 0), 0);
        assert_eq!(cq.push(35, 1), 1);
        assert_eq!(cq.push(38, 5), 2);

        assert_eq!(cq.len(), 3);
        assert_eq!(cq.top(), Some(&32));

        assert_eq!(cq.pop(), Some(32));
        assert_eq!(cq.len(), 2);

        assert_eq!(cq.pop(), Some(35));
        assert_eq!(cq.len(), 1);

        assert_eq!(cq.pop(), Some(38));
        assert_eq!(cq.len(), 0);

        assert_eq!(cq.pop(), None);

        assert!(cq.is_empty());
    }

    #[test]
    fn test_full() {
        let mut cq = CohortQueue::<i32>::new();

        assert_eq!(cq.top(), None);

        assert_eq!(cq.push(32, 0), 0);
        assert_eq!(cq.push(45, 0), 0);
        assert_eq!(cq.push(71, 0), 0);
        assert_eq!(cq.push(35, 1), 1);
        assert_eq!(cq.push(48, 1), 1);
        assert_eq!(cq.push(36, 2), 2);
        assert_eq!(cq.push(79, 1), 1);
        assert_eq!(cq.push(92, 0), 0);
        assert_eq!(cq.push(37, 5), 3);

        assert_eq!(cq.len(), 9);

        assert_eq!(cq.pop(), Some(32));
        assert_eq!(cq.pop(), Some(35));

        assert_eq!(cq.len(), 7);

        assert_eq!(cq.push(38, 4), 4);
        assert_eq!(cq.push(49, 4), 2);

        assert_eq!(cq.pop(), Some(36));
        assert_eq!(cq.pop(), Some(37));
        assert_eq!(cq.pop(), Some(38));

        assert_eq!(cq.pop(), Some(45));

        assert_eq!(cq.len(), 5);
        assert_eq!(cq.top(), Some(&48));
    }
}
