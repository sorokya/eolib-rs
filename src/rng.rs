//! A pseudo-random number generator that faithfully emulates the one used by the
//! original 32-bit Borland EO game client.
//!
//! This is a dual-state linear congruential generator with two 32-bit state words
//! (`seed` and `high`) that together give it an effective period far longer than a
//! classic single-register LCG.
//!
//! All arithmetic intentionally wraps on overflow (matching the original client's
//! use of unsigned 32-bit integer overflow), so [`wrapping_add`](u32::wrapping_add)
//! /[`wrapping_mul`](u32::wrapping_mul) operations are used throughout instead of
//! the checked/panicking defaults.
//!
//! # Two APIs
//!
//! - [`Rng`] is a plain, owned struct holding its own state. Create one with
//!   [`Rng::new`], keep it wherever makes sense for your program (a connection
//!   struct, a task, a session), and call [`Rng::rand`] /
//!   [`Rng::rand_range`] on it directly.
//! - [`srand`] / [`rand`] / [`rand_range`] are free functions backed by a hidden,
//!   *thread-local* [`Rng`], provided for parity with the original C API
//!   (`eo_srand` / `eo_rand` / `eo_rand_range`), which also keeps its state in
//!   global variables.
//!
//! New code should generally prefer owning an [`Rng`] directly — see
//! [Thread safety](#thread-safety) below for why.
//!
//! # Thread safety
//!
//! [`Rng`] holds no interior mutability and no global state: it is a plain
//! `Copy`-able pair of `u32`s, so it is trivially `Send` and can move between
//! threads, be cloned, or be stored per-connection/per-task with no special
//! handling. Each instance is completely independent of every other.
//!
//! The free functions ([`srand`]/[`rand`]/[`rand_range`]) instead share one
//! [`Rng`] *per OS thread* behind a [`std::thread_local`]. That works fine for
//! simple, synchronous programs, but has sharp edges in multi-threaded and
//! async applications:
//!
//! - Calling [`srand`] on one thread has no effect on any other thread's state.
//!   There is no single global sequence to seed "once at startup" the way the
//!   original C globals allowed.
//! - On an async runtime with a work-stealing scheduler (e.g. Tokio's
//!   multi-threaded runtime), a task is **not** pinned to one OS thread — it can
//!   resume on a different worker thread after an `.await`. Since the scheduler
//!   only reschedules tasks at `.await` points (it does not preempt mid-poll), a
//!   call to [`srand`] followed immediately by [`rand`] calls with **no**
//!   `.await` in between is safe and deterministic. But splitting a sequence
//!   across an `.await` is not:
//!
//!   ```rust,ignore
//!   rng::srand(seed);
//!   some_async_io().await; // task may resume on a different worker thread here
//!   let a = rng::rand();   // may now be reading a completely different thread's state
//!   ```
//!
//! **Recommendation for multi-threaded / async applications:** give each task,
//! connection, or session its own owned [`Rng`] (e.g. as a struct field) instead
//! of relying on the thread-local free functions. An owned `Rng` behaves
//! correctly no matter how the runtime schedules or migrates the surrounding
//! task, since its state travels with it rather than living on whichever thread
//! happens to be current. Reach for the free functions only in single-threaded
//! contexts, or for one-off, non-yielding synchronous sequences where sharing
//! thread-local state is not a concern.

use std::{cell::RefCell, ops::Range};

/// An instance of the dual-state LCG used by the original EO game client.
///
/// Unlike the free functions in this module, an `Rng` owns its state directly
/// rather than storing it in a thread-local, so it can be freely moved, cloned,
/// or stored per-task/connection. See the [module-level docs](self#thread-safety)
/// for guidance on choosing between the two.
///
/// # Examples
///
/// ```
/// use eolib::rng::Rng;
///
/// let mut rng = Rng::new(1);
/// assert_eq!(rng.rand(), 1367424485);
/// assert_eq!(rng.rand(), 18557838);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rng {
    seed: u32,
    high: u32,
}

impl Rng {
    /// Creates a new [`Rng`] seeded with a 32-bit unsigned integer.
    ///
    /// The same seed will always produce the same sequence of values from
    /// [`Rng::rand`]. The seed is scrambled internally before being used as the
    /// generator's initial state.
    pub fn new(seed: u32) -> Self {
        let mut rng = Self { seed: 0, high: 0 };
        rng.seed(seed);
        rng
    }

    /// Re-seeds this [`Rng`] with a new 32-bit unsigned integer, discarding its
    /// current state.
    ///
    /// # Examples
    ///
    /// ```
    /// use eolib::rng::Rng;
    ///
    /// let mut rng = Rng::new(1);
    /// let a = rng.rand();
    ///
    /// rng.seed(1);
    /// let b = rng.rand();
    ///
    /// assert_eq!(a, b);
    /// ```
    pub fn seed(&mut self, new_seed: u32) {
        self.seed = 0x1586B75u32.wrapping_mul(new_seed).wrapping_add(1);
        self.high = 0;
    }

    /// Generates a pseudo-random 31-bit unsigned integer in the range
    /// `[0, 2147483647]`.
    ///
    /// The sequence of numbers generated depends on the seed this [`Rng`] was
    /// created or last seeded with.
    pub fn rand(&mut self) -> u32 {
        let tmp_state_product = if self.high != 0 {
            self.high.wrapping_mul(20021)
        } else {
            0
        };
        let tmp_seed_product = self.seed.wrapping_mul(346);

        let full_product: u64 = (self.seed as u64) * 20021;

        let seed_lo = (full_product as u32).wrapping_add(1);
        let seed_hi = (full_product >> 32) as u32;
        let seed_hi = seed_hi
            .wrapping_add(tmp_state_product)
            .wrapping_add(tmp_seed_product);

        self.seed = seed_lo;
        self.high = seed_hi;

        seed_hi & 0x7FFFFFFF
    }

    /// Generates a pseudo-random unsigned integer within the inclusive range
    /// `[min, max]`.
    ///
    /// If `min` is greater than `max`, `min` is returned unchanged.
    ///
    /// The sequence of numbers generated depends on the seed this [`Rng`] was
    /// created or last seeded with.
    pub fn rand_range(&mut self, range: Range<u32>) -> u32 {
        if range.start > range.end {
            return range.start;
        }

        let max = range.end - range.start + 1;
        range.start + (self.rand() % max)
    }
}

/// Derives a seed from the current time and thread, for the crate's own
/// internal, auto-seeded generator below. Not exposed publicly: it is not a
/// source of cryptographic-quality randomness, just enough to avoid every
/// thread (or every process started at nearly the same time) producing an
/// identical sequence.
fn entropy_seed() -> u32 {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
        time::{SystemTime, UNIX_EPOCH},
    };

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    let mut hasher = DefaultHasher::new();
    nanos.hash(&mut hasher);
    std::thread::current().id().hash(&mut hasher);
    hasher.finish() as u32
}

thread_local! {
    #[allow(clippy::missing_const_for_thread_local)]
    static INTERNAL_STATE: RefCell<Rng> = RefCell::new(Rng::new(entropy_seed()));
}

/// Generates a value in the inclusive range `[range.start, range.end]` using a
/// library-internal, auto-seeded per-thread [`Rng`].
///
/// This powers the crate's own randomness needs (e.g. swap multiples, packet
/// sequence start values) and is deliberately kept separate from the
/// public-facing thread-local behind [`srand`]/[`rand`]/[`rand_range`], so that
/// a consumer using those functions for their own deterministic sequences is
/// never affected by the crate's internal random number consumption.
pub(crate) fn internal_rand_range(range: Range<u32>) -> u32 {
    INTERNAL_STATE.with_borrow_mut(|rng| rng.rand_range(range))
}

thread_local!(static STATE: RefCell<Rng> = const { RefCell::new(Rng { seed: 0, high: 0 }) });

/// Seeds the current thread's random number generator with a 32-bit unsigned
/// integer.
///
/// The same seed will always produce the same sequence of values from [`rand`]
/// *on the same thread*. See [Thread safety](self#thread-safety) for why this
/// matters and when to prefer an owned [`Rng`] instead.
///
/// # Examples
///
/// ```
/// use eolib::rng::{srand, rand};
///
/// srand(1);
/// let a = rand();
///
/// srand(1);
/// let b = rand();
///
/// assert_eq!(a, b);
/// ```
pub fn srand(new_seed: u32) {
    STATE.with_borrow_mut(|rng| rng.seed(new_seed));
}

/// Generates a pseudo-random 31-bit unsigned integer in the range
/// `[0, 2147483647]`, using the current thread's generator.
///
/// The sequence of numbers generated depends on the seed set by [`srand`] *on
/// the same thread*. See [Thread safety](self#thread-safety) for why this
/// matters and when to prefer an owned [`Rng`] instead.
///
/// # Examples
///
/// ```
/// use eolib::rng::{srand, rand};
///
/// srand(1);
/// assert_eq!(rand(), 1367424485);
/// assert_eq!(rand(), 18557838);
/// ```
pub fn rand() -> u32 {
    STATE.with_borrow_mut(|rng| rng.rand())
}

/// Generates a pseudo-random unsigned integer within the inclusive range
/// `[min, max]`, using the current thread's generator.
///
/// If `min` is greater than `max`, `min` is returned unchanged.
///
/// The sequence of numbers generated depends on the seed set by [`srand`] *on
/// the same thread*. See [Thread safety](self#thread-safety) for why this
/// matters and when to prefer an owned [`Rng`] instead.
///
/// # Examples
///
/// ```
/// use eolib::rng::{srand, rand_range};
///
/// srand(99);
/// assert_eq!(rand_range(7..7), 7);
/// ```
pub fn rand_range(range: Range<u32>) -> u32 {
    STATE.with_borrow_mut(|rng| rng.rand_range(range))
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use super::{Rng, rand, rand_range, srand};

    #[test]
    fn rand_is_deterministic_for_same_seed() {
        srand(12345);
        let a: Vec<u32> = (0..3).map(|_| rand()).collect();

        srand(12345);
        let b: Vec<u32> = (0..3).map(|_| rand()).collect();

        assert_eq!(a, b);
    }

    #[test]
    fn rand_matches_known_sequence() {
        // Values captured from the reference eolib-c implementation.
        srand(1);
        assert_eq!(rand(), 1367424485);
        assert_eq!(rand(), 18557838);
        assert_eq!(rand(), 438012954);
        assert_eq!(rand(), 1351954191);
        assert_eq!(rand(), 2046927527);

        srand(12345);
        assert_eq!(rand(), 1628244379);
        assert_eq!(rand(), 205855945);
        assert_eq!(rand(), 1598090830);
        assert_eq!(rand(), 423792828);
        assert_eq!(rand(), 1122741425);

        srand(0);
        assert_eq!(rand(), 346);
        assert_eq!(rand(), 13854878);
        assert_eq!(rand(), 1621890440);
        assert_eq!(rand(), 816282396);
        assert_eq!(rand(), 1778554774);

        // A seed chosen to exercise wraparound in the seed-scrambling multiply.
        srand(u32::MAX);
        assert_eq!(rand(), 780079875);
        assert_eq!(rand(), 409992358);
        assert_eq!(rand(), 738360963);
        assert_eq!(rand(), 1473119578);
        assert_eq!(rand(), 1009212074);
    }

    #[test]
    fn rand_never_sets_the_high_bit() {
        srand(7);
        for _ in 0..10_000 {
            assert_eq!(rand() & 0x80000000, 0);
        }
    }

    #[test]
    fn rand_range_stays_within_bounds() {
        srand(42);
        for _ in 0..1000 {
            let v = rand_range(10..20);
            assert!((10..=20).contains(&v));
        }
    }

    #[test]
    fn rand_range_with_equal_bounds_always_returns_that_value() {
        srand(99);
        for _ in 0..10 {
            assert_eq!(rand_range(7..7), 7);
        }
    }

    #[test]
    fn rand_range_with_inverted_bounds_returns_min_unchanged() {
        srand(0);
        // Constructed rather than written as a `20..10` literal to avoid
        // clippy's reversed_empty_ranges lint; this exercises the same
        // min > max guard as the C `eo_rand_range` implementation.
        let inverted = Range { start: 20, end: 10 };
        assert_eq!(rand_range(inverted), 20);
    }

    #[test]
    fn different_threads_have_independent_state() {
        srand(555);
        let main_thread_first = rand();

        let handle = std::thread::spawn(|| {
            srand(555);
            rand()
        });
        let other_thread_first = handle.join().unwrap();

        assert_eq!(main_thread_first, other_thread_first);
    }

    #[test]
    fn rng_struct_matches_known_sequence() {
        // Same reference vectors as the free-function tests: the struct and the
        // thread-local free functions share the exact same algorithm.
        let mut rng = Rng::new(1);
        assert_eq!(rng.rand(), 1367424485);
        assert_eq!(rng.rand(), 18557838);
        assert_eq!(rng.rand(), 438012954);
    }

    #[test]
    fn rng_struct_instances_are_independent() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);

        assert_eq!(a.rand(), b.rand());

        // Advancing one instance must not affect the other.
        a.rand();
        assert_ne!(a.rand(), b.rand());
    }

    #[test]
    fn rng_struct_reseed_resets_sequence() {
        let mut rng = Rng::new(1);
        let a = rng.rand();

        rng.seed(1);
        let b = rng.rand();

        assert_eq!(a, b);
    }

    #[test]
    fn rng_struct_is_send_and_can_move_across_threads() {
        let mut rng = Rng::new(555);
        let first = rng.rand();

        let handle = std::thread::spawn(move || rng.rand());
        let second_on_other_thread = handle.join().unwrap();

        // Continuing the same owned Rng on a different thread produces the same
        // result as continuing it on the original thread would have.
        let mut reference = Rng::new(555);
        reference.rand();
        let expected = reference.rand();

        assert_ne!(first, second_on_other_thread);
        assert_eq!(second_on_other_thread, expected);
    }
}
