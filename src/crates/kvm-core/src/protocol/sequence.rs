//! Thread-safe sequence counter for protocol message numbering.

use std::sync::atomic::{AtomicU64, Ordering};

/// A thread-safe, monotonically increasing counter for protocol sequence numbers.
///
/// Sequence numbers start at 0 and increment by 1 with each call to [`next`].
/// The counter wraps around at `u64::MAX` back to 0 without panicking.
///
/// # Examples
///
/// ```rust
/// use kvm_core::protocol::SequenceCounter;
///
/// let counter = SequenceCounter::new();
/// assert_eq!(counter.next(), 0);
/// assert_eq!(counter.next(), 1);
/// ```
pub struct SequenceCounter {
    inner: AtomicU64,
}

impl SequenceCounter {
    /// Creates a new counter starting at 0.
    pub fn new() -> Self {
        Self {
            inner: AtomicU64::new(0),
        }
    }

    /// Returns the next sequence number and atomically increments the counter.
    ///
    /// Wraps around from `u64::MAX` to 0 on overflow without panicking.
    pub fn next(&self) -> u64 {
        // fetch_add wraps on overflow due to two's-complement arithmetic.
        self.inner.fetch_add(1, Ordering::Relaxed)
    }

    /// Returns the current value without incrementing.
    pub fn current(&self) -> u64 {
        self.inner.load(Ordering::Relaxed)
    }
}

impl Default for SequenceCounter {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: SequenceCounter uses an AtomicU64 internally and is safe to share
// across threads.
unsafe impl Send for SequenceCounter {}
unsafe impl Sync for SequenceCounter {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_sequence_counter_starts_at_zero() {
        // Arrange
        let counter = SequenceCounter::new();

        // Act
        let first = counter.next();

        // Assert
        assert_eq!(first, 0);
    }

    #[test]
    fn test_sequence_counter_increments_monotonically() {
        // Arrange
        let counter = SequenceCounter::new();

        // Act
        let values: Vec<u64> = (0..100).map(|_| counter.next()).collect();

        // Assert – values must be strictly monotonically increasing
        for window in values.windows(2) {
            assert!(window[1] > window[0], "values must be monotonically increasing");
        }
    }

    #[test]
    fn test_sequence_counter_wraps_at_u64_max() {
        // Arrange – start the counter one step before overflow
        let counter = SequenceCounter {
            inner: AtomicU64::new(u64::MAX),
        };

        // Act
        let before_wrap = counter.next();
        let after_wrap = counter.next();

        // Assert
        assert_eq!(before_wrap, u64::MAX);
        assert_eq!(after_wrap, 0, "counter must wrap to 0 after u64::MAX");
    }

    #[test]
    fn test_sequence_counter_is_thread_safe() {
        // Arrange
        let counter = Arc::new(SequenceCounter::new());
        let thread_count = 8;
        let increments_per_thread = 1000;

        // Act – increment from many threads simultaneously
        let handles: Vec<_> = (0..thread_count)
            .map(|_| {
                let c = Arc::clone(&counter);
                thread::spawn(move || {
                    (0..increments_per_thread)
                        .map(|_| c.next())
                        .collect::<Vec<_>>()
                })
            })
            .collect();

        let mut all_values: Vec<u64> = handles
            .into_iter()
            .flat_map(|h| h.join().expect("thread panicked"))
            .collect();

        // Assert – all values are unique (no two threads got the same sequence number)
        all_values.sort_unstable();
        all_values.dedup();
        assert_eq!(
            all_values.len(),
            thread_count * increments_per_thread,
            "every sequence number must be unique across threads"
        );
    }

    #[test]
    fn test_current_does_not_increment() {
        // Arrange
        let counter = SequenceCounter::new();
        counter.next(); // advance to 1

        // Act
        let current = counter.current();
        let next = counter.next();

        // Assert
        assert_eq!(current, 1, "current() should return 1 without advancing");
        assert_eq!(next, 1, "next() should return 1 (the value before this increment)");
    }

    #[test]
    fn test_default_creates_counter_at_zero() {
        // Arrange / Act
        let counter = SequenceCounter::default();

        // Assert
        assert_eq!(counter.next(), 0);
    }
}
