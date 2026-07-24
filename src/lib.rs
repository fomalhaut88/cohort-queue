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


#[derive(Debug)]
pub struct CohortQueue<T> {
    sub_queues: VecDeque<SubQueue<T>>,
    len: usize,
}


impl<T> CohortQueue<T> {
    pub fn new() -> Self {
        Self {
            sub_queues: VecDeque::new(),
            len: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.sub_queues.is_empty()
    }

    pub fn len(&self) -> usize {
        self.len
    }

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
