use std::{cell::Cell, cmp};

use bytes::Bytes;
use encoding_rs::WINDOWS_1252;
use thiserror::Error;

use super::{decode_number, decode_string};

#[derive(Error, Debug)]
pub enum EoReaderError {
    #[error("Chunked reading mode is disabled")]
    ChunkedReadingDisabled,
    #[error("{0}")]
    Other(String),
}

impl From<String> for EoReaderError {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}

#[derive(Debug)]
/// A reader for reading data from an EO data stream
///
/// # Examples
///
/// ```
/// use bytes::Bytes;
/// use eolib::data::EoReader;
///
/// let data = Bytes::from_static(&[1, 43, 11, 254]);
/// let reader = EoReader::new(data);
///
/// assert_eq!(reader.get_byte().unwrap(), 1);
/// assert_eq!(reader.get_char().unwrap(), 42);
/// assert_eq!(reader.get_short().unwrap(), 10);
/// assert_eq!(reader.remaining().unwrap(), 0);
/// ```
///
/// ## Chunked reading mode
///
/// ```
/// use bytes::Bytes;
/// use eolib::data::EoReader;
///
/// let data = Bytes::from_static(&[43, 255, 72, 101, 108, 108, 111, 255, 2]);
/// let reader = EoReader::new(data);
///
/// reader.set_chunked_reading_mode(true);
///
/// // Reads an integer (4 bytes) but only advances the cursor by one byte, accounting for
/// // the first chunk being a single byte.
/// assert_eq!(reader.get_int().unwrap(), 42);
///
/// // Advances the cursor to the next chunk
/// reader.next_chunk().unwrap();
///
/// assert_eq!(reader.get_string().unwrap(), "Hello");
///
/// // Advances the cursor to the next chunk
/// reader.next_chunk().unwrap();
///
/// // Reads an integer (4 bytes) but only advances the cursor by one byte, accounting for
/// // the last chunk
/// assert_eq!(reader.get_int().unwrap(), 1);
/// ````
pub struct EoReader {
    data: Bytes,
    position: Cell<usize>,
    chunked_reading_mode: Cell<bool>,
    chunk_start: Cell<usize>,
    next_break: Cell<Option<usize>>,
}

impl EoReader {
    /// creates a new [EoReader] with the specified data
    pub fn new(data: Bytes) -> Self {
        Self {
            data,
            position: Cell::new(0),
            chunked_reading_mode: Cell::new(false),
            chunk_start: Cell::new(0),
            next_break: Cell::new(None),
        }
    }

    /// returns the number of bytes remaining in the input data or chunk if chunked reading is
    /// enabled
    pub fn remaining(&self) -> usize {
        let position = self.position.get();
        let chunked_reading_mode = self.chunked_reading_mode.get();
        if chunked_reading_mode {
            let next_break = match self.next_break.get() {
                Some(next_break) => next_break,
                None => position,
            };

            next_break - cmp::min(position, next_break)
        } else {
            self.data.len() - position
        }
    }

    /// returns the current chunked reading mode for the reader
    pub fn get_chunked_reading_mode(&self) -> bool {
        self.chunked_reading_mode.get()
    }

    /// sets the chunked reading mode for the reader
    ///
    /// in chunked reading mode:
    /// * the reader will treat `0xFF` bytes as the end of the current chunk
    /// * [next_chunk](EoReader::next_chunk) can be called to move to the next chunk
    pub fn set_chunked_reading_mode(&self, enabled: bool) {
        self.chunked_reading_mode.set(enabled);
        let next_break = self.next_break.get();
        if next_break.is_none() {
            self.next_break.set(Some(self.find_next_break_index()))
        }
    }

    /// moves the reader position to the start of the next chunk in the input data
    pub fn next_chunk(&self) -> Result<(), EoReaderError> {
        let chunked_reading_mode = self.chunked_reading_mode.get();
        if !chunked_reading_mode {
            return Err(EoReaderError::ChunkedReadingDisabled);
        }

        let next_break = match self.next_break.get() {
            Some(next_break) => next_break,
            None => self.position.get(),
        };

        let mut position = next_break;
        self.chunk_start.set(position);

        if position < self.data.len() {
            position += 1;
        }

        self.position.set(position);
        self.next_break.set(Some(self.find_next_break_index()));

        Ok(())
    }

    fn find_next_break_index(&self) -> usize {
        let position = self.position.get();
        match self.data.iter().skip(position).position(|b| *b == 0xff) {
            Some(index) => position + index,
            None => self.data.len(),
        }
    }

    /// returns a single [u8] from the data stream
    ///
    /// increases the read position by 1
    pub fn get_byte(&self) -> u8 {
        match self.read_bytes(1) {
            Some(buf) => buf[0],
            None => 0,
        }
    }

    /// returns a [u8] slice from the data stream
    ///
    /// increases the read position by `length`
    pub fn get_bytes(&self, length: usize) -> Vec<u8> {
        match self.read_bytes(length) {
            Some(buf) => buf.to_vec(),
            None => Vec::new(),
        }
    }

    /// returns a single [u8] from the data stream decoded into an [i32]
    ///
    /// increases the read position by 1
    pub fn get_char(&self) -> i32 {
        match self.read_bytes(1) {
            Some(buf) => decode_number(buf),
            None => 0,
        }
    }

    /// returns two [u8]s from the data stream decoded into an [i32]
    ///
    /// increases the read position by 2
    pub fn get_short(&self) -> i32 {
        match self.read_bytes(2) {
            Some(buf) => decode_number(buf),
            None => 0,
        }
    }

    /// returns three [u8]s from the data stream decoded into an [i32]
    ///
    /// increases the read position by 3
    pub fn get_three(&self) -> i32 {
        match self.read_bytes(3) {
            Some(buf) => decode_number(buf),
            None => 0,
        }
    }

    /// returns four [u8]s from the data stream decoded into an [i32]
    ///
    /// increases the read position by 4
    pub fn get_int(&self) -> i32 {
        match self.read_bytes(4) {
            Some(buf) => decode_number(buf),
            None => 0,
        }
    }

    /// returns a [String] from the data stream
    pub fn get_string(&self) -> String {
        let remaining = self.remaining();
        self.get_fixed_string(remaining)
    }

    /// returns a [String] from the data stream with a fixed length
    ///
    /// if `length` is `0` then an empty [String] is returned
    /// increases the read position by length
    pub fn get_fixed_string(&self, length: usize) -> String {
        if length == 0 {
            return String::new();
        }

        let buf = match self.read_bytes(length) {
            Some(buf) => buf,
            None => return String::new(),
        };

        let (cow, _, _) = WINDOWS_1252.decode(buf);
        cow.to_string()
    }

    /// returns an encoded [String] from the data stream
    pub fn get_encoded_string(&self) -> String {
        self.get_fixed_encoded_string(self.remaining())
    }

    /// returns an encoded [String] from the data stream with a fixed length
    pub fn get_fixed_encoded_string(&self, length: usize) -> String {
        if length == 0 {
            return String::new();
        }

        let mut buf = match self.read_bytes(length) {
            Some(buf) => buf.to_vec(),
            None => return String::new(),
        };

        decode_string(&mut buf);
        let position_of_break = match buf.iter().position(|b| *b == 0xff) {
            Some(position_of_break) => position_of_break,
            None => length - 1,
        };
        let (cow, _, _) = WINDOWS_1252.decode(&buf[..position_of_break]);
        cow.to_string()
    }

    fn read_bytes(&self, length: usize) -> Option<&[u8]> {
        let position = self.position.get();
        let length = cmp::min(length, self.remaining());
        let buf = match self.data.get(position..position + length) {
            Some(buf) => buf,
            None => return None,
        };
        self.position.set(position + length);
        Some(buf)
    }
}
