use bytes::{BufMut, Bytes, BytesMut};
use encoding_rs::WINDOWS_1252;
use thiserror::Error;

use super::{encode_number, encode_string, CHAR_MAX, INT_MAX, SHORT_MAX, THREE_MAX};

#[derive(Error, Debug, PartialEq, Eq)]
pub enum EoWriterError {
    #[error("Invalid char value {0} must be between 0 and {}", CHAR_MAX)]
    InvalidCharValue(i32),
    #[error("Invalid short value {0} must be between 0 and {}", SHORT_MAX)]
    InvalidShortValue(i32),
    #[error("Invalid three value {0} must be between 0 and {}", THREE_MAX)]
    InvalidThreeValue(i32),
    #[error("Invalid int value {0} must be between 0 and {}", INT_MAX)]
    InvalidIntValue(i64),
    #[error("{0}")]
    Other(String),
}

impl From<String> for EoWriterError {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}

#[derive(Debug, Default)]
/// A writer for writing data to an EO data stream
///
/// Uses [BytesMut] under the hood for efficient memory usage.
///
/// The data is frozen and converted to [Bytes] when [to_byte_array](EoWriter::to_byte_array) is called.
///
/// # Examples
///
/// ```
/// use eolib::data::EoWriter;
///
/// let mut writer = EoWriter::new();
/// writer.add_byte(1);
/// writer.add_char(42).unwrap();
/// writer.add_short(10);
///
/// let buf = writer.to_byte_array();
///
/// assert_eq!(&buf[..], [1, 43, 11, 254]);
/// ````
pub struct EoWriter {
    data: BytesMut,
    string_sanitization_mode: bool,
}

impl EoWriter {
    /// creates a new [EoWriter]
    pub fn new() -> Self {
        Self::default()
    }

    /// creates a new [EoWriter] with the specified capacity
    pub fn with_capacity(size: usize) -> Self {
        Self {
            data: BytesMut::with_capacity(size),
            ..Default::default()
        }
    }

    /// adds a byte to the data stream
    pub fn add_byte(&mut self, byte: u8) {
        self.data.put_u8(byte);
    }

    /// adds a byte slice to the data stream
    pub fn add_bytes(&mut self, bytes: &[u8]) {
        self.data.put_slice(bytes);
    }

    /// adds a char to the data stream
    pub fn add_char(&mut self, char: i32) -> Result<(), EoWriterError> {
        if !(0..=CHAR_MAX).contains(&char) {
            return Err(EoWriterError::InvalidCharValue(char));
        }

        let encoded = encode_number(char)?;
        self.data.put_slice(&encoded[0..1]);
        Ok(())
    }

    /// adds a short to the data stream
    pub fn add_short(&mut self, short: i32) -> Result<(), EoWriterError> {
        if !(0..=SHORT_MAX).contains(&short) {
            return Err(EoWriterError::InvalidShortValue(short));
        }

        let encoded = encode_number(short)?;
        self.data.put_slice(&encoded[0..2]);
        Ok(())
    }

    /// adds a three to the data stream
    pub fn add_three(&mut self, three: i32) -> Result<(), EoWriterError> {
        if !(0..=THREE_MAX).contains(&three) {
            return Err(EoWriterError::InvalidThreeValue(three));
        }

        let encoded = encode_number(three)?;
        self.data.put_slice(&encoded[0..3]);
        Ok(())
    }

    /// adds an int to the data stream
    pub fn add_int(&mut self, int: i32) -> Result<(), EoWriterError> {
        let encoded = encode_number(int)?;
        self.data.put_slice(&encoded[0..4]);
        Ok(())
    }

    fn sanitize_string(&self, string: &str) -> String {
        if self.string_sanitization_mode {
            string
                .chars()
                .map(|c| if c as i32 == 0xff { 0x79 as char } else { c })
                .collect()
        } else {
            string.to_owned()
        }
    }

    /// adds a string to the data stream
    pub fn add_string(&mut self, string: &str) {
        let string = self.sanitize_string(string);
        let (string, _, _) = WINDOWS_1252.encode(&string);
        self.data.put_slice(&string);
    }

    /// encodes a string and adds it to the data stream
    pub fn add_encoded_string(&mut self, string: &str) {
        let string = self.sanitize_string(string);
        let (mut string, _, _) = WINDOWS_1252.encode(&string);
        let string = string.to_mut();
        encode_string(&mut *string);
        self.data.put_slice(string);
    }

    /// gets the string sanitization mode
    pub fn get_string_sanitization_mode(&self) -> bool {
        self.string_sanitization_mode
    }

    /// sets the string sanitization mode
    pub fn set_string_sanitization_mode(&mut self, mode: bool) {
        self.string_sanitization_mode = mode;
    }

    /// freezes the data and returns a [Bytes] object that can be freely cloned
    pub fn to_byte_array(self) -> Bytes {
        self.data.freeze()
    }
}

#[cfg(test)]
mod tests {
    use crate::data::{eo_writer::EoWriterError, CHAR_MAX, SHORT_MAX, THREE_MAX};

    use super::EoWriter;

    #[test]
    fn with_capacity() {
        let writer = EoWriter::with_capacity(10);
        assert_eq!(writer.data.capacity(), 10);
    }

    #[test]
    fn add_byte() {
        let mut writer = EoWriter::with_capacity(1);
        writer.add_byte(1);
        assert_eq!(&writer.data[..], [1]);
    }

    #[test]
    fn add_char() {
        let mut writer = EoWriter::with_capacity(1);
        writer.add_char(1).unwrap();
        assert_eq!(&writer.data[..], [2]);
    }

    #[test]
    fn add_short() {
        let mut writer = EoWriter::with_capacity(2);
        writer.add_short(1).unwrap();
        assert_eq!(&writer.data[..], [2, 0xfe]);
    }

    #[test]
    fn add_three() {
        let mut writer = EoWriter::with_capacity(3);
        writer.add_three(1).unwrap();
        assert_eq!(&writer.data[..], [2, 0xfe, 0xfe]);
    }

    #[test]
    fn add_int() {
        let mut writer = EoWriter::with_capacity(4);
        writer.add_int(1).unwrap();
        assert_eq!(&writer.data[..], [2, 0xfe, 0xfe, 0xfe]);
    }

    #[test]
    fn add_negative_char() {
        let mut writer = EoWriter::with_capacity(1);
        let result = writer.add_char(-1).unwrap_err();
        assert_eq!(result, EoWriterError::InvalidCharValue(-1));
    }

    #[test]
    fn add_negative_short() {
        let mut writer = EoWriter::with_capacity(2);
        let result = writer.add_short(-1).unwrap_err();
        assert_eq!(result, EoWriterError::InvalidShortValue(-1));
    }

    #[test]
    fn add_negative_three() {
        let mut writer = EoWriter::with_capacity(3);
        let result = writer.add_three(-1).unwrap_err();
        assert_eq!(result, EoWriterError::InvalidThreeValue(-1));
    }

    #[test]
    fn add_negative_int() {
        let mut writer = EoWriter::with_capacity(4);
        let result = writer.add_int(-1);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn add_large_char() {
        let mut writer = EoWriter::with_capacity(1);
        let result = writer.add_char(CHAR_MAX + 1).unwrap_err();
        assert_eq!(result, EoWriterError::InvalidCharValue(CHAR_MAX + 1));
    }

    #[test]
    fn add_large_short() {
        let mut writer = EoWriter::with_capacity(2);
        let result = writer.add_short(SHORT_MAX + 1).unwrap_err();
        assert_eq!(result, EoWriterError::InvalidShortValue(SHORT_MAX + 1));
    }

    #[test]
    fn add_large_three() {
        let mut writer = EoWriter::with_capacity(3);
        let result = writer.add_three(THREE_MAX + 1).unwrap_err();
        assert_eq!(result, EoWriterError::InvalidThreeValue(THREE_MAX + 1));
    }

    #[test]
    fn add_large_int() {
        let mut writer = EoWriter::with_capacity(4);
        let result = writer.add_int(-i32::MAX).unwrap_err();
        assert_eq!(result, EoWriterError::InvalidIntValue(i32::MAX as i64 * 2));
    }
}
