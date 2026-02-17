//! Thread-safe sequence counter for protocol message numbering.
//!
//! # What is a sequence number? (for beginners)
//!
//! Every message sent over the wire carries a monotonically increasing integer
//! called a *sequence number*.  Sequence numbers are used to:
//!
//! - **Detect missing messages** – if you receive message 1, 2, and 4 but not 3,
//!   you know message 3 was dropped in transit.
//! - **Detect duplicates** – if you receive message 5 twice, you can discard the
//!   second copy.
//! - **Measure latency** – by comparing the sequence number in a Ping reply to
//!   what you sent, you can confirm the reply matches your request.
//!
//! # Thread safety
//!
//! The counter uses `AtomicU64` internally.  An atomic operation is a CPU
//! instruction that reads, modifies, and writes a value *as a single
//! indivisible step*, so two threads can both call `next()` simultaneously
//! without corrupting the counter or producing the same value twice.
//!
//! Contrast this with a non-atomic counter protected by a `Mutex`: a Mutex
//! works, but it requires the thread to briefly block while waiting to acquire
//! the lock.  Atomics are lock-free and typically faster for this simple
//! increment-and-read pattern.

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
    /// The underlying atomic integer.
    ///
    /// `AtomicU64` lives on the heap (inside this struct) and can be accessed
    /// by multiple threads simultaneously without a lock.
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
    /// The first call returns 0, the second returns 1, and so on.
    ///
    /// Wraps around from `u64::MAX` to 0 on overflow without panicking.
    ///
    /// # Atomic ordering
    ///
    /// `Ordering::Relaxed` is sufficient here because sequence numbers are
    /// only used for message ordering, not for memory synchronisation between
    /// threads.  A higher ordering (like `SeqCst`) would add unnecessary
    /// memory barriers.
    pub fn next(&self) -> u64 {
        // `fetch_add` atomically adds 1 and returns the *old* value (the value
        // before the addition).  Rust's two's-complement arithmetic means that
        // adding 1 to u64::MAX naturally wraps to 0.
        self.inner.fetch_add(1, Ordering::Relaxed)
    }

    /// Returns the current value without incrementing.
    ///
    /// Useful for logging and diagnostics.  Note that by the time the caller
    /// uses the returned value, another thread may have already incremented
    /// the counter further.
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
//
// Rust requires explicit `unsafe impl Send` and `Sync` for types that contain
// raw pointers or other non-`Send`/non-`Sync` types.  `AtomicU64` is already
// `Send + Sync`, so these impls are technically not required — Rust would
// derive them automatically.  They are stated explicitly here for clarity.
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
            assert!(
                window[1] > window[0],
                "values must be monotonically increasing"
            );
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
        assert_eq!(
            next, 1,
            "next() should return 1 (the value before this increment)"
        );
    }

    #[test]
    fn test_default_creates_counter_at_zero() {
        // Arrange / Act
        let counter = SequenceCounter::default();

        // Assert
        assert_eq!(counter.next(), 0);
    }
}
