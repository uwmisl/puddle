// heavily inspired by petgraph's implementation
// https://github.com/bluss/petgraph/blob/master/src/scored.rs
// but we use timestamps to break ties
use std::cmp::Ordering;
use std::collections::BinaryHeap;

pub struct MinHeap<K: Ord, T: Eq> {
    heap: BinaryHeap<MinHeapElem<K, T>>,
    timestamp: u32,
}

impl<K: Ord, T: Eq> MinHeap<K, T> {
    pub fn new() -> MinHeap<K, T> {
        MinHeap {
            heap: BinaryHeap::new(),
            timestamp: 0,
        }
    }

    pub fn push(&mut self, cost: K, elem: T) {
        let x = MinHeapElem {
            cost: cost,
            timestamp: self.timestamp,
            elem: elem,
        };
        self.timestamp += 1;
        self.heap.push(x)
    }

    pub fn pop(&mut self) -> Option<(K, T)> {
        if let Some(heap_elem) = self.heap.pop() {
            Some((heap_elem.cost, heap_elem.elem))
        } else {
            None
        }
    }
}

/// `MinHeapElem<K, T>` holds a score `K` and a scored object `T` in
/// a pair for use with a `BinaryHeap`.
///
/// `MinHeapElem` compares in reverse order by the score, so that we can
/// use `BinaryHeap` as a min-heap to extract the score-value pair with the
/// least score.
///
/// **Note:** `MinHeapElem` implements a total order (`Ord`), so that it is
/// possible to use float types as scores.
#[derive(PartialEq, Eq, Debug)]
struct MinHeapElem<K: Ord, T: Eq> {
    cost: K,
    timestamp: u32,
    elem: T,
}

impl<K: Ord, T: Eq> Ord for MinHeapElem<K, T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        // NOTE the order is flipped here
        match other.cost.cmp(&self.cost) {
            Ordering::Equal => {
                // use timestamp to resolve cost ties to ensure a LIFO behavior of the Heap
                let ord = other.timestamp.cmp(&self.timestamp);
                assert_ne!(ord, Ordering::Equal);
                ord
            },
            ord => ord
        }
    }
}

impl<K: Ord, T: Eq> PartialOrd for MinHeapElem<K, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
