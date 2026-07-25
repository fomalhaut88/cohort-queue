//! A generic Rust queue that combines **FIFO fairness** with
//! **controlled priority-based insertion**.
//!
//! `CohortQueue` is useful when a new item should be allowed to move ahead
//! of some older items, but should not jump directly to the front of the
//! queue. It achieves this by grouping items into ordered FIFO cohorts.
//! A higher insertion order allows an item to join an earlier eligible cohort,
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
//! // This item joins the first cohort whose current order is lower than 1.
//! queue.push("priority-1", 1);
//!
//! assert_eq!(queue.pop(), Some("regular-1"));
//! assert_eq!(queue.pop(), Some("priority-1"));
//! assert_eq!(queue.pop(), Some("regular-2"));
//! ```
//!
//! # Complexity
//!
//! | Operation   |  Complexity |
//! | ----------- | ----------: |
//! | `new`       |      O(1)   |
//! | `len`       |      O(1)   |
//! | `is_empty`  |      O(1)   |
//! | `top`       |      O(1)   |
//! | `top_order` |      O(1)   |
//! | `pop`       |      O(1)   |
//! | `push`      |  O(log c)   |
//!
//! where `c` is the number of active cohorts.

use std::collections::VecDeque;


#[derive(Debug)]
struct SubQueue<T> {
    deque: VecDeque<T>,
    order: Option<usize>,
}


impl<T> SubQueue<T> {
    fn new() -> Self {
        Self {
            deque: VecDeque::new(),
            order: None,
        }
    }

    fn is_good(&self) -> bool {
        !self.deque.is_empty()
    }

    fn front(&self) -> &T {
        self.deque.front().expect("broken data")
    }

    fn order(&self) -> usize {
        self.order.expect("broken data")
    }

    fn push(&mut self, item: T, order: usize) -> bool {
        let success = self.order.map(|o| o < order).unwrap_or(true);
        if success {
            self.deque.push_back(item);
            self.order = Some(order);
        }
        success
    }

    fn pop(&mut self) -> Option<T> {
        self.deque.pop_front()
    }

    fn iter(&self) -> impl Iterator<Item = &T> {
        self.deque.iter()
    }
}


/// A queue that combines FIFO fairness with controlled
/// priority-based insertion.
///
/// Items are grouped into ordered cohorts. Each cohort stores the order of
/// its most recently accepted item. A new item joins the earliest cohort whose
/// current order is strictly lower than the item's order, and is appended
/// behind all existing members of that cohort.
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
    /// An item joins the first cohort whose current order is strictly less
    /// than `order`. The selected cohort's order is then replaced with
    /// `order`. If no cohort is eligible, a new cohort is created at the back.
    pub fn push(&mut self, item: T, order: usize) {
        if let Some(sub_queue) = self.find_sub_queue(order) {
            sub_queue.push(item, order);
        } else {
            let mut sub_queue = SubQueue::new();
            sub_queue.push(item, order);
            self.sub_queues.push_back(sub_queue);
        }
        self.len += 1;
    }

    /// Removes and returns the front item from the earliest non-empty cohort.
    ///
    /// When a cohort becomes empty, it is removed automatically.
    pub fn pop(&mut self) -> Option<T> {
        if let Some(sub_queue) = self.sub_queues.front_mut() {
            let item = sub_queue.pop();
            if !sub_queue.is_good() {
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
        self.sub_queues.front().map(|q| q.front())
    }

    /// Returns the current order of the earliest cohort.
    ///
    /// This is cohort metadata: the order of the most recently accepted item
    /// in that cohort. It is not necessarily the order originally supplied
    /// for the item returned by [`top`](Self::top).
    pub fn top_order(&self) -> Option<usize> {
        self.sub_queues.front().map(|q| q.order())
    }

    /// Returns an iterator over all items in processing order.
    ///
    /// Items are yielded in FIFO order within each cohort, starting with the
    /// earliest cohort.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.sub_queues.iter().flat_map(|q| q.iter())
    }

    fn find_sub_queue(&mut self, order: usize) -> Option<&mut SubQueue<T>> {
        let ix = self.sub_queues.partition_point(|q| q.order() >= order);
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

        cq.push(32, 0);
        cq.push(35, 0);
        cq.push(38, 0);

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

        cq.push(32, 0);
        cq.push(35, 1);
        cq.push(38, 5);

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
        assert_eq!(cq.top_order(), None);

        cq.push(32, 0);
        cq.push(45, 0);
        cq.push(71, 0);
        cq.push(35, 1);
        cq.push(48, 1);
        cq.push(36, 2);
        cq.push(79, 1);
        cq.push(92, 0);
        cq.push(37, 5);

        assert_eq!(cq.len(), 9);

        assert_eq!(
            cq.iter().cloned().collect::<Vec<_>>(),
            vec![32, 35, 36, 37, 45, 48, 71, 79, 92],
        );

        assert_eq!(cq.pop(), Some(32));
        assert_eq!(cq.pop(), Some(35));

        assert_eq!(cq.len(), 7);

        assert_eq!(cq.top_order(), Some(5));

        cq.push(38, 6);
        cq.push(49, 4);

        assert_eq!(cq.pop(), Some(36));
        assert_eq!(cq.pop(), Some(37));
        assert_eq!(cq.pop(), Some(38));

        assert_eq!(cq.pop(), Some(45));

        assert_eq!(cq.len(), 5);
        assert_eq!(cq.top(), Some(&48));
        assert_eq!(cq.top_order(), Some(4));

        assert_eq!(
            cq.iter().cloned().collect::<Vec<_>>(),
            vec![48, 49, 71, 79, 92],
        );
    }
}
