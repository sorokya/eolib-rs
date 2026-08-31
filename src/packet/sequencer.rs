use std::cmp;

use crate::{data::CHAR_MAX, rng};

#[derive(Debug)]
/// Used for packet sequencing
///
/// The sequence value is sent at the start of every client packet
/// and verified on the server.
///
/// The starting value can be reset at different stages in game play.
///
/// In the original game protocol these places are:
/// - Handshake (Init_Init packet)
/// - Account creation
/// - Server pings
pub struct Sequencer {
    start: i32,
    counter: i32,
}

impl Sequencer {
    /// creates a new [Sequencer] with the specified starting value
    pub fn new(start: i32) -> Self {
        Self { start, counter: 0 }
    }

    /// returns the next sequence value
    ///
    /// Returns the current `start + counter` value and *then* increments the
    /// counter (0 → 9, looping). This matches the reference client
    /// (`eolib` TypeScript `PacketSequencer.nextSequence`), where the first
    /// call returns `start` and the counter only advances afterward.
    pub fn next_sequence(&mut self) -> i32 {
        let sequence = self.start + self.counter;
        self.counter = (self.counter + 1) % 10;
        sequence
    }

    /// sets a new starting value for the sequencer
    pub fn set_start(&mut self, start: i32) {
        self.start = start;
    }

    /// gets the current starting value for the sequencer
    pub fn get_start(&self) -> i32 {
        self.start
    }
}

/// returns a random sequence start value
pub fn generate_sequence_start() -> i32 {
    rng::internal_rand_range(0..(CHAR_MAX - 10) as u32) as i32
}

/// returns sequence bytes from a starting value
///
/// used by the server for Init_Init packet
pub fn get_init_sequence_bytes(start: i32) -> [i32; 2] {
    let seq1_min = cmp::max(0, (start - (CHAR_MAX - 1) + 13 + 6) / 7);
    let seq1_max = (start + 13) / 7;
    let seq1 = rng::internal_rand_range(0..(seq1_max - seq1_min) as u32) as i32 + seq1_min;
    let seq2 = start - seq1 * 7 + 13;
    [seq1, seq2]
}

/// returns the initial sequence start value from sequence bytes
///
/// used by the client after receiving Init_Init packet
pub fn get_init_sequence_start(s1: i32, s2: i32) -> i32 {
    s1 * 7 + s2 - 13
}

/// returns sequence bytes from a starting value
///
/// used by the server for Ping packet
pub fn get_ping_sequence_bytes(start: i32) -> [i32; 2] {
    let seq1_max = start + 252;
    let seq1_min = start;
    let seq1 = rng::internal_rand_range(seq1_min as u32..seq1_max as u32) as i32;
    let seq2 = seq1 - start;
    [seq1, seq2]
}

/// returns the ping sequence start value from sequence bytes
///
/// used by the client after receiving Ping packet
pub fn get_ping_sequence_start(s1: i32, s2: i32) -> i32 {
    s1 - s2
}

#[cfg(test)]
mod tests {
    use super::{
        Sequencer, generate_sequence_start, get_init_sequence_bytes, get_init_sequence_start,
        get_ping_sequence_bytes, get_ping_sequence_start,
    };
    use crate::data::CHAR_MAX;

    #[test]
    fn next_sequence_returns_start_before_incrementing() {
        let mut sequencer = Sequencer::new(100);

        // First call returns the start value, then advances the counter.
        assert_eq!(sequencer.next_sequence(), 100);
        assert_eq!(sequencer.next_sequence(), 101);
        assert_eq!(sequencer.next_sequence(), 102);
    }

    #[test]
    fn counter_wraps_after_ten() {
        let mut sequencer = Sequencer::new(0);

        for i in 0..10 {
            assert_eq!(sequencer.next_sequence(), i);
        }
        // Wrapped back around to 0.
        assert_eq!(sequencer.next_sequence(), 0);
    }

    #[test]
    fn set_start_does_not_reset_counter() {
        let mut sequencer = Sequencer::new(0);
        sequencer.next_sequence(); // 0, counter -> 1

        sequencer.set_start(50);
        // Counter is still 1, so the next value is 50 + 1.
        assert_eq!(sequencer.next_sequence(), 51);
    }

    #[test]
    fn generate_sequence_start_stays_within_bounds() {
        for _ in 0..1000 {
            let start = generate_sequence_start();
            assert!((0..=CHAR_MAX - 10).contains(&start));
        }
    }

    #[test]
    fn init_sequence_bytes_round_trip() {
        for start in [0, 1, 13, 100, CHAR_MAX - 10] {
            let [s1, s2] = get_init_sequence_bytes(start);
            assert_eq!(get_init_sequence_start(s1, s2), start);
        }
    }

    #[test]
    fn ping_sequence_bytes_round_trip() {
        for start in [0, 1, 100, CHAR_MAX - 10] {
            let [s1, s2] = get_ping_sequence_bytes(start);
            assert_eq!(get_ping_sequence_start(s1, s2), start);
        }
    }
}
