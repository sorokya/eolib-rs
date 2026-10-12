//! A pseudo-random number generator that faithfully emulates the one used by the
//! original 32-bit Borland EO game client.
//!
//! The client links the Borland C++ 5.5 C runtime's `rand.c`
//! (`ref/Borland5/Source/RTL/source/math/rand.c` in the Endless Online
//! decompilation), which supplies three entry points:
//!
//! - `srand(seed)` — stores the seed and primes it with one `rand()`;
//! - `rand()` — a 32-bit multiplicative congruential generator over `Seed.lo`
//!   whose result is bits 16..30 (0..2¹⁵-1). The client never calls this, but
//!   `srand` does, so it moves the state;
//! - `_lrand()` — the dual-state generator below, returning 0..2³¹-1. The
//!   client's `random(n)` macro is `_lrand() % n`.
//!
//! [`Rng::rand`] is `_lrand`: the name predates this documentation, and the
//! thread-local C-shaped API ([`srand`]/[`rand`]) mirrors it. `_lrand` is
//! **not** a textbook 64-bit `Seed = 0x015A4E35 * Seed + 1`; it is the inline
//! assembly in `rand.c`, which multiplies the two state words separately and
//! discards the parts the assembly drops. [`Rng::rand`] reproduces that
//! instruction for instruction, because the sequence is observable: the client
//! seeds with `srand(time(0))` and reads `random(n)` for things like the
//! softpad's anti-keylogger jump and the weapon sounds.
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

/// An instance of the dual-state generator the original EO game client uses
/// (`_lrand`, `src/rng.rs`'s module docs).
///
/// The fields are `rand.c`'s `Seed.lo` and `Seed.hi`.
///
/// Unlike the free functions in this module, an `Rng` owns its state directly
/// rather than storing it in a thread-local, so it can be freely moved, cloned,
/// or stored per-task/connection. See the [module-level docs](self#thread-safety)
/// for guidance on choosing between the two.
///
/// # Examples
///
/// The vectors below are the runtime's own for `srand(1)`, verified against the
/// `_lrand` assembly in the reference client's image:
///
/// ```
/// use eolib::rng::Rng;
///
/// let mut rng = Rng::new(1);
/// assert_eq!(rng.rand(), 1410184549);
/// assert_eq!(rng.rand(), 672584604);
/// assert_eq!(rng.rand(), 1915668986);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rng {
    /// `Seed.lo`.
    seed: u32,
    /// `Seed.hi`.
    high: u32,
}

/// `MULTIPLIER` (`rand.c`), and its two 16-bit halves, which `_lrand`'s
/// assembly multiplies by separately.
const MULTIPLIER: u32 = 0x015a_4e35;
const MULTIPLIER_HIGH: u32 = 0x015a;
const MULTIPLIER_LOW: u32 = 0x4e35;
/// `INCREMENT`.
const INCREMENT: u32 = 1;

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
        // `rand.c`'s `srand`: `Seed.lo = seed; Seed.hi = 0; rand();` — the seed
        // is the low word, and the first `rand()` scrambles it so the first
        // value is not as random as the ones after it.
        self.seed = new_seed;
        self.high = 0;
        self.rand_15bit();
    }

    /// `rand()`: the 32-bit generator over `Seed.lo`, returning bits 16..30.
    ///
    /// `srand` primes with one call, which is the only place the reference
    /// client reaches it: the game uses `random(n)`, i.e. [`Rng::rand`].
    fn rand_15bit(&mut self) -> u32 {
        self.seed = MULTIPLIER.wrapping_mul(self.seed).wrapping_add(INCREMENT);
        (self.seed >> 16) & 0x7fff
    }

    /// Generates a pseudo-random 31-bit unsigned integer in the range
    /// `[0, 2147483647]`.
    ///
    /// This is `_lrand`. The sequence of numbers generated depends on the seed
    /// this [`Rng`] was created or last seeded with.
    pub fn rand(&mut self) -> u32 {
        // `_lrand`'s inline assembly, instruction for instruction:
        //
        //     eax = hi; ecx = 0x15A; ebx = 0x4E35;
        //     if (hi) { edx:eax = hi * 0x4E35 }   // only eax (low word) is kept
        //     xchg ecx, eax; edx:eax = 0x15A * lo // only eax (low word) is kept
        //     eax += ecx;
        //     xchg eax, esi; edx:eax = lo * 0x4E35;
        //     edx += esi; eax += 1; adc edx, 0;
        //     Seed.lo = eax; Seed.hi = edx; return edx & 0x7FFFFFFF;
        //
        // Note what the assembly does *not* do: `hi * 0x4E35` contributes its
        // low word only, the `0x15A * lo` carry is dropped, and the add is a
        // plain 32-bit one. A textbook `Seed = MULTIPLIER * Seed + 1` over a
        // 64-bit `Seed` gives a different sequence.
        let (high, low) = (self.high, self.seed);

        let cross = if high != 0 {
            high.wrapping_mul(MULTIPLIER_LOW)
        } else {
            0
        };
        let rotated = MULTIPLIER_HIGH.wrapping_mul(low);
        let sum = rotated.wrapping_add(cross);

        let product = u64::from(low) * u64::from(MULTIPLIER_LOW);
        let product_low = product as u32;
        let product_high = ((product >> 32) as u32).wrapping_add(sum);

        let (seed_low, carry) = product_low.overflowing_add(INCREMENT);
        let seed_high = product_high.wrapping_add(u32::from(carry));

        self.seed = seed_low;
        self.high = seed_high;

        seed_high & 0x7fff_ffff
    }

    /// Generates a pseudo-random unsigned integer within the inclusive range
    /// `[min, max]`.
    ///
    /// If `min` is greater than `max`, `min` is returned unchanged.
    ///
    /// The range's end **is** included, so `rand_range(0..n)` spans `n + 1`
    /// values. The client's `random(n)` macro is `_lrand() % n`, i.e.
    /// `rand_range(0..n - 1)`; use this to model it.
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
/// assert_eq!(rand(), 1410184549);
/// assert_eq!(rand(), 672584604);
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
        // The runtime's own values: `srand` and `rand` from
        // `ref/Borland5/Source/RTL/source/math/rand.c`, and `_lrand` from the
        // assembly in that file, which the reference client carries verbatim
        // (`mov ecx, 0x15a` / `mov ebx, 0x4e35` at 0x217e98).
        srand(1);
        assert_eq!(rand(), 1410184549);
        assert_eq!(rand(), 672584604);
        assert_eq!(rand(), 1915668986);
        assert_eq!(rand(), 576821944);
        assert_eq!(rand(), 167078712);

        srand(12345);
        assert_eq!(rand(), 1220244142);
        assert_eq!(rand(), 1374406965);
        assert_eq!(rand(), 308141128);
        assert_eq!(rand(), 1558318993);
        assert_eq!(rand(), 598390725);

        srand(0);
        assert_eq!(rand(), 346);
        assert_eq!(rand(), 13854878);
        assert_eq!(rand(), 1621890440);
        assert_eq!(rand(), 816282396);
        assert_eq!(rand(), 1778554774);

        // A seed chosen to exercise wraparound in the priming multiply.
        srand(u32::MAX);
        assert_eq!(rand(), 737319811);
        assert_eq!(rand(), 1903449241);
        assert_eq!(rand(), 1408188579);
        assert_eq!(rand(), 100768176);
        assert_eq!(rand(), 741577241);
    }

    #[test]
    fn rand_is_not_the_textbook_64_bit_lcg() {
        // `Seed = MULTIPLIER * Seed + 1` over a 64-bit `Seed` would give these
        // values; the assembly drops the cross terms, so it must not.
        srand(1);
        assert_ne!(rand(), 1367424485);
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
        assert_eq!(rng.rand(), 1410184549);
        assert_eq!(rng.rand(), 672584604);
        assert_eq!(rng.rand(), 1915668986);
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
