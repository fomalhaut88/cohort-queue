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

#![warn(missing_docs)]

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

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.deque.iter_mut()
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

    /// Removes all items and cohorts from the queue.
    ///
    /// After this call, the queue is empty and its length is zero.
    pub fn clear(&mut self) {
        self.sub_queues.clear();
        self.len = 0;
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

    /// Returns a mutable iterator over all items in processing order.
    ///
    /// Items are yielded in FIFO order within each cohort, starting with the
    /// earliest cohort. Mutating items does not affect cohort ordering.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.sub_queues.iter_mut().flat_map(|q| q.iter_mut())
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


impl<'a, T> IntoIterator for &'a CohortQueue<T> {
    type Item = &'a T;
    type IntoIter = Box<dyn Iterator<Item = &'a T> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.iter())
    }
}


impl<'a, T> IntoIterator for &'a mut CohortQueue<T> {
    type Item = &'a mut T;
    type IntoIter = Box<dyn Iterator<Item = &'a mut T> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.iter_mut())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::fmt::Debug;
    use std::ops::RangeInclusive;

    fn assert_internal_invariants<T>(cq: &CohortQueue<T>)
    where
        T: Debug + PartialEq,
    {
        let actual_len: usize = cq
            .sub_queues
            .iter()
            .map(|q| q.deque.len())
            .sum();

        assert_eq!(cq.len, actual_len);
        assert_eq!(cq.is_empty(), cq.len == 0);
        assert_eq!(cq.is_empty(), cq.sub_queues.is_empty());

        // Empty cohorts must be removed immediately.
        assert!(cq.sub_queues.iter().all(|q| q.is_good()));

        // Cohort orders must remain non-increasing.
        let orders = cq
            .sub_queues
            .iter()
            .map(|q| q.order())
            .collect::<Vec<_>>();

        assert!(
            orders.windows(2).all(|pair| pair[0] >= pair[1]),
            "cohort orders are not non-increasing: {orders:?}",
        );

        assert_eq!(cq.top(), cq.iter().next());
        assert_eq!(cq.len(), cq.iter().count());
    }

    #[test]
    fn test_default() {
        let cq = CohortQueue::<i32>::default();

        assert!(cq.is_empty());
        assert_eq!(cq.len(), 0);
        assert_eq!(cq.top(), None);
        assert_eq!(cq.top_order(), None);
        assert_eq!(cq.iter().next(), None);

        assert_internal_invariants(&cq);
    }

    #[test]
    fn test_trivial() {
        let mut cq = CohortQueue::<i32>::new();

        assert_eq!(cq.top(), None);
        assert_eq!(cq.top_order(), None);

        cq.push(32, 0);
        cq.push(35, 0);
        cq.push(38, 0);

        assert_eq!(cq.len(), 3);
        assert_eq!(cq.top(), Some(&32));
        assert_eq!(cq.top_order(), Some(0));

        assert_eq!(
            cq.iter().copied().collect::<Vec<_>>(),
            vec![32, 35, 38],
        );

        assert_internal_invariants(&cq);

        assert_eq!(cq.pop(), Some(32));
        assert_eq!(cq.len(), 2);
        assert_eq!(cq.top(), Some(&35));

        assert_eq!(cq.pop(), Some(35));
        assert_eq!(cq.len(), 1);
        assert_eq!(cq.top(), Some(&38));

        assert_eq!(cq.pop(), Some(38));
        assert_eq!(cq.len(), 0);

        assert_eq!(cq.pop(), None);
        assert_eq!(cq.pop(), None);

        assert!(cq.is_empty());
        assert_eq!(cq.top(), None);
        assert_eq!(cq.top_order(), None);

        assert_internal_invariants(&cq);
    }

    #[test]
    fn test_equal_orders_create_separate_cohorts() {
        let mut cq = CohortQueue::new();

        cq.push(10, 4);
        cq.push(20, 4);
        cq.push(30, 4);

        assert_eq!(cq.sub_queues.len(), 3);

        assert_eq!(
            cq.sub_queues
                .iter()
                .map(|q| q.order())
                .collect::<Vec<_>>(),
            vec![4, 4, 4],
        );

        assert_eq!(
            cq.iter().copied().collect::<Vec<_>>(),
            vec![10, 20, 30],
        );

        assert_internal_invariants(&cq);
    }

    #[test]
    fn test_increase() {
        let mut cq = CohortQueue::<i32>::new();

        assert_eq!(cq.top(), None);

        cq.push(32, 0);
        cq.push(35, 1);
        cq.push(38, 5);

        // Strictly increasing orders join one cohort.
        assert_eq!(cq.sub_queues.len(), 1);

        assert_eq!(cq.len(), 3);
        assert_eq!(cq.top(), Some(&32));
        assert_eq!(cq.top_order(), Some(5));

        assert_eq!(
            cq.iter().copied().collect::<Vec<_>>(),
            vec![32, 35, 38],
        );

        assert_internal_invariants(&cq);

        assert_eq!(cq.pop(), Some(32));
        assert_eq!(cq.len(), 2);

        // top_order belongs to the cohort, not to the front item.
        assert_eq!(cq.top_order(), Some(5));

        assert_eq!(cq.pop(), Some(35));
        assert_eq!(cq.len(), 1);

        assert_eq!(cq.pop(), Some(38));
        assert_eq!(cq.len(), 0);

        assert_eq!(cq.pop(), None);
        assert!(cq.is_empty());

        assert_internal_invariants(&cq);
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
            cq.iter().copied().collect::<Vec<_>>(),
            vec![32, 35, 36, 37, 45, 48, 71, 79, 92],
        );

        assert_internal_invariants(&cq);

        assert_eq!(cq.pop(), Some(32));
        assert_eq!(cq.pop(), Some(35));

        assert_eq!(cq.len(), 7);
        assert_eq!(cq.top(), Some(&36));
        assert_eq!(cq.top_order(), Some(5));

        cq.push(38, 6);
        cq.push(49, 4);

        assert_eq!(
            cq.iter().copied().collect::<Vec<_>>(),
            vec![36, 37, 38, 45, 48, 49, 71, 79, 92],
        );

        assert_internal_invariants(&cq);

        assert_eq!(cq.pop(), Some(36));
        assert_eq!(cq.pop(), Some(37));
        assert_eq!(cq.pop(), Some(38));
        assert_eq!(cq.pop(), Some(45));

        assert_eq!(cq.len(), 5);
        assert_eq!(cq.top(), Some(&48));
        assert_eq!(cq.top_order(), Some(4));

        assert_eq!(
            cq.iter().copied().collect::<Vec<_>>(),
            vec![48, 49, 71, 79, 92],
        );

        assert_internal_invariants(&cq);
    }

    #[test]
    fn test_iter_mut() {
        let mut cq = CohortQueue::new();

        cq.push(10, 0);
        cq.push(20, 0);
        cq.push(30, 1);

        assert_eq!(
            cq.iter().copied().collect::<Vec<_>>(),
            vec![10, 30, 20],
        );

        for item in cq.iter_mut() {
            *item *= 2;
        }

        assert_eq!(
            cq.iter().copied().collect::<Vec<_>>(),
            vec![20, 60, 40],
        );

        assert_eq!(cq.len(), 3);
        assert_eq!(cq.top_order(), Some(1));

        assert_internal_invariants(&cq);
    }

    #[test]
    fn test_into_iterator_for_shared_reference() {
        #[derive(Debug, PartialEq, Eq)]
        struct NonClone(i32);

        let mut cq = CohortQueue::new();

        cq.push(NonClone(10), 0);
        cq.push(NonClone(20), 0);
        cq.push(NonClone(30), 1);

        let values = (&cq)
            .into_iter()
            .map(|item| item.0)
            .collect::<Vec<_>>();

        assert_eq!(values, vec![10, 30, 20]);

        // Iteration borrows the queue and does not consume it.
        assert_eq!(cq.len(), 3);
        assert_eq!(cq.top(), Some(&NonClone(10)));
    }

    #[test]
    fn test_into_iterator_for_mutable_reference() {
        let mut cq = CohortQueue::new();

        cq.push(10, 0);
        cq.push(20, 0);
        cq.push(30, 1);

        for item in &mut cq {
            *item += 100;
        }

        assert_eq!(
            cq.iter().copied().collect::<Vec<_>>(),
            vec![110, 130, 120],
        );

        assert_eq!(cq.len(), 3);
        assert_eq!(cq.top_order(), Some(1));

        assert_internal_invariants(&cq);
    }

    #[test]
    fn test_clear() {
        let mut cq = CohortQueue::new();

        cq.push(10, 0);
        cq.push(20, 0);
        cq.push(30, 1);

        assert_eq!(cq.len(), 3);
        assert!(!cq.is_empty());

        cq.clear();

        assert_eq!(cq.len(), 0);
        assert!(cq.is_empty());
        assert_eq!(cq.top(), None);
        assert_eq!(cq.top_order(), None);
        assert_eq!(cq.iter().next(), None);
        assert_eq!(cq.pop(), None);

        assert_internal_invariants(&cq);

        // The queue remains reusable.
        cq.push(42, 7);

        assert_eq!(cq.len(), 1);
        assert_eq!(cq.top(), Some(&42));
        assert_eq!(cq.top_order(), Some(7));
        assert_eq!(cq.pop(), Some(42));
        assert!(cq.is_empty());
    }

    #[test]
    fn test_iterator_matches_pop_order() {
        let input = [
            (10, 0),
            (20, 0),
            (30, 2),
            (40, 1),
            (50, 5),
            (60, 0),
            (70, 3),
        ];

        let mut iter_queue = CohortQueue::new();
        let mut pop_queue = CohortQueue::new();

        for (item, order) in input {
            iter_queue.push(item, order);
            pop_queue.push(item, order);
        }

        let iterated = iter_queue.iter().copied().collect::<Vec<_>>();

        let mut popped = Vec::new();

        while let Some(item) = pop_queue.pop() {
            popped.push(item);
        }

        assert_eq!(iterated, popped);
        assert_eq!(iter_queue.len(), iterated.len());
        assert!(pop_queue.is_empty());
    }

    #[test]
    fn test_maximum_order() {
        let mut cq = CohortQueue::new();

        cq.push(10, 0);
        cq.push(20, usize::MAX);
        cq.push(30, usize::MAX);

        // Equal usize::MAX cannot join the first cohort.
        assert_eq!(cq.sub_queues.len(), 2);

        assert_eq!(
            cq.iter().copied().collect::<Vec<_>>(),
            vec![10, 20, 30],
        );

        assert_eq!(cq.top_order(), Some(usize::MAX));

        assert_internal_invariants(&cq);
    }

    struct ReferenceSubQueue<T> {
        deque: VecDeque<T>,
        order: usize,
    }

    struct ReferenceQueue<T> {
        sub_queues: VecDeque<ReferenceSubQueue<T>>,
        len: usize,
    }

    impl<T> ReferenceQueue<T> {
        fn new() -> Self {
            Self {
                sub_queues: VecDeque::new(),
                len: 0,
            }
        }

        fn is_empty(&self) -> bool {
            self.len == 0
        }

        fn len(&self) -> usize {
            self.len
        }

        fn push(&mut self, item: T, order: usize) {
            if let Some(sub_queue) = self
                .sub_queues
                .iter_mut()
                .find(|q| q.order < order)
            {
                sub_queue.deque.push_back(item);
                sub_queue.order = order;
            } else {
                self.sub_queues.push_back(ReferenceSubQueue {
                    deque: VecDeque::from([item]),
                    order,
                });
            }

            self.len += 1;
        }

        fn pop(&mut self) -> Option<T> {
            let sub_queue = self.sub_queues.front_mut()?;
            let item = sub_queue.deque.pop_front();

            if sub_queue.deque.is_empty() {
                self.sub_queues.pop_front();
            }

            if item.is_some() {
                self.len -= 1;
            }

            item
        }

        fn top(&self) -> Option<&T> {
            self.sub_queues
                .front()
                .and_then(|q| q.deque.front())
        }

        fn top_order(&self) -> Option<usize> {
            self.sub_queues.front().map(|q| q.order)
        }

        fn iter(&self) -> impl Iterator<Item = &T> {
            self.sub_queues
                .iter()
                .flat_map(|q| q.deque.iter())
        }
    }

    fn visit_order_sequences<F>(
        sequence: &mut Vec<usize>,
        max_len: usize,
        values: RangeInclusive<usize>,
        callback: &mut F,
    )
    where
        F: FnMut(&[usize]),
    {
        callback(sequence);

        if sequence.len() == max_len {
            return;
        }

        for order in values.clone() {
            sequence.push(order);
            visit_order_sequences(sequence, max_len, values.clone(), callback);
            sequence.pop();
        }
    }

    #[test]
    fn test_exhaustive_against_reference_implementation() {
        let mut sequence = Vec::new();

        visit_order_sequences(
            &mut sequence,
            7,
            0..=3,
            &mut |orders| {
                let mut cq = CohortQueue::new();
                let mut reference = ReferenceQueue::new();

                for (item, &order) in orders.iter().enumerate() {
                    cq.push(item, order);
                    reference.push(item, order);

                    assert_eq!(cq.len(), reference.len(), "orders: {orders:?}");
                    assert_eq!(
                        cq.is_empty(),
                        reference.is_empty(),
                        "orders: {orders:?}",
                    );
                    assert_eq!(cq.top(), reference.top(), "orders: {orders:?}");
                    assert_eq!(
                        cq.top_order(),
                        reference.top_order(),
                        "orders: {orders:?}",
                    );

                    assert_eq!(
                        cq.iter().copied().collect::<Vec<_>>(),
                        reference.iter().copied().collect::<Vec<_>>(),
                        "orders: {orders:?}",
                    );

                    assert_internal_invariants(&cq);
                }

                while !reference.is_empty() {
                    assert_eq!(
                        cq.pop(),
                        reference.pop(),
                        "orders: {orders:?}",
                    );

                    assert_eq!(cq.len(), reference.len(), "orders: {orders:?}");
                    assert_eq!(
                        cq.is_empty(),
                        reference.is_empty(),
                        "orders: {orders:?}",
                    );
                    assert_eq!(cq.top(), reference.top(), "orders: {orders:?}");
                    assert_eq!(
                        cq.top_order(),
                        reference.top_order(),
                        "orders: {orders:?}",
                    );

                    assert_internal_invariants(&cq);
                }

                assert_eq!(cq.pop(), None, "orders: {orders:?}");
                assert!(cq.is_empty(), "orders: {orders:?}");
                assert_eq!(cq.len(), 0, "orders: {orders:?}");
            },
        );
    }
}
