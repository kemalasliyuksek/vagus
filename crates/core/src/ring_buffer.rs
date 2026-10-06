use std::collections::VecDeque;
use std::num::NonZeroUsize;

/// Fixed-capacity buffer that keeps the most recent items.
///
/// Backs the flight recorder (ADR 0011): once full, every push evicts the
/// oldest item, so memory stays constant no matter how long the daemon runs.
#[derive(Debug, Clone)]
pub struct RingBuffer<T> {
    items: VecDeque<T>,
    capacity: NonZeroUsize,
}

impl<T> RingBuffer<T> {
    /// Creates an empty buffer that holds at most `capacity` items.
    ///
    /// Storage is allocated up front so that pushes never allocate.
    #[must_use]
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            items: VecDeque::with_capacity(capacity.get()),
            capacity,
        }
    }

    /// Appends `item` and returns the evicted oldest item if the buffer was full.
    ///
    /// Returning the evicted item lets the sampler reuse its allocations
    /// (for example a per-sample process list) instead of dropping them.
    pub fn push(&mut self, item: T) -> Option<T> {
        let evicted = if self.items.len() == self.capacity.get() {
            self.items.pop_front()
        } else {
            None
        };
        self.items.push_back(item);
        evicted
    }

    /// Iterates from the oldest to the newest item.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator {
        self.items.iter()
    }

    /// Returns the most recently pushed item.
    #[must_use]
    pub fn latest(&self) -> Option<&T> {
        self.items.back()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[must_use]
    pub fn capacity(&self) -> NonZeroUsize {
        self.capacity
    }
}
