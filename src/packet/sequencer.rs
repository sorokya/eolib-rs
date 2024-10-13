use std::cmp;

use rand::Rng;

use crate::data::CHAR_MAX;

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
    pub fn next_sequence(&mut self) -> i32 {
        self.counter = (self.counter + 1) % 10;
        self.start + self.counter
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
    let mut rng = rand::thread_rng();
    rng.gen_range(0..=CHAR_MAX - 10)
}

/// returns sequence bytes from a starting value
///
/// used by the server for Init_Init packet
pub fn get_init_sequence_bytes(start: i32) -> [i32; 2] {
    let mut rng = rand::thread_rng();
    let seq1_min = cmp::max(0, (start - (CHAR_MAX - 1) + 13 + 6) / 7);
    let seq1_max = (start + 13) / 7;
    let seq1 = rng.gen_range(0..=seq1_max - seq1_min) + seq1_min;
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
    let mut rng = rand::thread_rng();
    let seq1_max = start + 252;
    let seq1_min = start;
    let seq1 = rng.gen_range(seq1_min..=seq1_max);
    let seq2 = seq1 - start;
    [seq1, seq2]
}

/// returns the ping sequence start value from sequence bytes
///
/// used by the client after receiving Ping packet
pub fn get_ping_sequence_start(s1: i32, s2: i32) -> i32 {
    s1 - s2
}
