/// The maximum value of an EO char (1-byte encoded integer type)
pub const CHAR_MAX: i32 = 253;

/// The maximum value of an EO short (2-byte encoded integer type)
pub const SHORT_MAX: i32 = CHAR_MAX * CHAR_MAX;

/// The maximum value of an EO three (3-byte encoded integer type)
pub const THREE_MAX: i32 = CHAR_MAX * CHAR_MAX * CHAR_MAX;

/// The maximum value of an EO int (4-byte encoded integer type)
pub const INT_MAX: i64 = CHAR_MAX as i64 * CHAR_MAX as i64 * CHAR_MAX as i64 * CHAR_MAX as i64;

/// The multiplier each decoded byte is scaled by, by its position in the
/// number: `1`, [CHAR_MAX], [SHORT_MAX], [THREE_MAX].
const DIGIT_MULTIPLIERS: [i32; 4] = [1, CHAR_MAX, SHORT_MAX, THREE_MAX];

/// Returns an encoded byte array from `number`
///
/// EO uses a maximum of four bytes to represent a number
/// in a data stream.
///
/// The value is spread across the four bytes based on if the
/// value is greater than the defined maximum for each amount of bytes
/// see [CHAR_MAX], [SHORT_MAX], [THREE_MAX]
///
/// The four bytes are initialized with a value of 254.
/// This is the padding [decode_number] stops at, and never a digit: the encoder
/// writes `digit + 1`, which is at most 253.
///
/// The number of bytes the field actually occupies is up to the caller: a
/// packet field of `n` bytes uses the first `n` of them ([EoWriter::add_char]
/// and friends), and two numbers in one field are never confused for one
/// another.
///
/// Bytes 4, 3, and 2 are set as follows if `number` is greater than or equal to
/// the corresponding `MAX` constant.
///
/// `bytes[x] = (number / MAX_x) + 1`
///
/// the number is then set to be the remainder of the division as follows
///
/// `number %= MAX_x`
///
/// Byte 1 is simply the remaining `number` plus one.
///
/// `bytes[0] = number + 1`
///
/// # Examples
///
/// ## Number less than CHAR_MAX
/// ```
/// use eolib::data::encode_number;
///
/// let result = encode_number(42).unwrap();
/// assert_eq!(result, [43, 254, 254, 254]);
/// ```
/// since 42 is less than CHAR_MAX it is simply incremented by 1
/// and the remaining bytes are set to 254
///
/// ## Number less than SHORT_MAX
/// ```
/// use eolib::data::encode_number;
/// let result = encode_number(533).unwrap();
/// assert_eq!(result, [28, 3, 254, 254]);
/// ```
///
/// since 533 is grater than CHAR_MAX byte 2 is set to
///
/// `(533 / CHAR_MAX) + 1 // 3`
///
/// byte 1 is set to the the remainder + 1
///
/// `(533 % CHAR_MAX) + 1 // 28`
///
/// and the remaining bytes are set to 254
///
/// ## Number less than THREE_MAX
/// ```
/// use eolib::data::encode_number;
/// let result = encode_number(888888).unwrap();
/// assert_eq!(result, [100, 225, 14, 254]);
/// ```
///
/// since 888888 is grater than SHORT_MAX byte 3 is set to
///
/// `(888888 / SHORT_MAX) + 1 // 14`
///
/// byte 2 is set to
///
/// `((888888 % SHORT_MAX) / CHAR_MAX) + 1 // 225`
///
/// byte 1 is set to the the remainder + 1
///
/// `(888888 % SHORT_MAX % CHAR_MAX) + 1 // 100`
///
/// and the last byte is set to 254
///
/// ## Number less than MAX4
/// ```
/// use eolib::data::encode_number;
/// let result = encode_number(18994242).unwrap();
/// assert_eq!(result, [15, 189, 44, 2]);
/// ```
///
/// since 18994242 is grater than THREE_MAX byte 4 is set to
///
/// `(18994242 / THREE_MAX) + 1 // 2`
///
/// byte 3 is set to
///
/// `((18994242 % THREE_MAX) / SHORT_MAX) + 1 // 44`
///
/// byte 2 is set to
///
/// `((18994242 % THREE_MAX % SHORT_MAX) / CHAR_MAX) + 1 // 189`
///
/// byte 1 is set to the the remainder + 1
///
/// `(18994242 % THREE_MAX % SHORT_MAX % CHAR_MAX) + 1 // 15`
pub fn encode_number(number: i32) -> Result<[u8; 4], EoWriterError> {
    let mut bytes: [u8; 4] = [254, 254, 254, 254];

    // Unwrap negative i32 to positive i64
    let mut number = if number < 0 {
        number.abs() as i64 + i32::MAX as i64
    } else {
        number as i64
    };

    let original_number = number;

    if original_number >= INT_MAX {
        return Err(EoWriterError::InvalidIntValue(original_number));
    }

    if original_number >= THREE_MAX as i64 {
        bytes[3] = (number / THREE_MAX as i64) as u8 + 1;
        number %= THREE_MAX as i64;
    }

    if original_number >= SHORT_MAX as i64 {
        bytes[2] = (number / SHORT_MAX as i64) as u8 + 1;
        number %= SHORT_MAX as i64;
    }

    if original_number >= CHAR_MAX as i64 {
        bytes[1] = (number / CHAR_MAX as i64) as u8 + 1;
        number %= CHAR_MAX as i64;
    }

    bytes[0] = number as u8 + 1;

    Ok(bytes)
}

/// Returns a decoded number from an EO Byte array
///
/// EO uses a maximum of four bytes to represent a number
/// in a data stream.
///
/// You can provide any number of [u8]s in `bytes`
/// but only the first four are used.
///
/// If you provide less than four than the remaining bytes default to 254
///
/// The byte array is iterated over and each byte is decremented by 1, then
/// multiplied by the multiplier for its position and summed.
///
/// The result is then calculated like so
///
/// `(b4 * THREE_MAX) + (b3 * SHORT_MAX) + (b2 * CHAR_MAX) + b1`
///
/// A byte of `254` **terminates** the number rather than contributing a digit:
/// the encoder pads with `254` past the significant bytes, so nothing of value
/// can follow one. This matches `EO_DecodeNumber`, which breaks out of its loop
/// on `0xfe` (see [encoding.md](../eo-protocol/docs/encoding.md#decode)), so a
/// `254` followed by a significant byte decodes to the bytes before it, not
/// through it.
///
/// # Examples
/// ```
/// use eolib::data::decode_number;
/// let result = decode_number(&[43, 254, 254, 254]);
/// assert_eq!(result, 42);
/// ```
///
/// * the first byte is decremented by 1
///   `[42, 254, 254, 254]`
/// * the `254` at index 0 terminates the number, so the rest is padding
/// * the result is `42`
///
/// ```
/// # use eolib::data::decode_number;
/// // Two digits: 28 - 1 = 27, then (3 - 1) * CHAR_MAX.
/// assert_eq!(decode_number(&[28, 3, 254, 254]), 533);
/// // The padding stops the decode even when bytes follow it.
/// assert_eq!(decode_number(&[28, 254, 3, 3]), 27);
/// ```
pub fn decode_number(bytes: &[u8]) -> i32 {
    let mut result: i32 = 0;
    for (i, &byte) in bytes.iter().take(4).enumerate() {
        if byte == 254 {
            break;
        }
        // A zero byte (which a well-formed stream never carries) subtracts one,
        // as the client's `int b = ch; b -= 1;` does.
        result = result.wrapping_add((i32::from(byte) - 1).wrapping_mul(DIGIT_MULTIPLIERS[i]));
    }

    result
}

/// Decodes a string in place
///
/// This is used for map names and sign text in map files
///
/// # Examples
///
/// ```
/// use eolib::data::decode_string;
///
/// let mut buf = [0x69, 0x36, 0x5E, 0x49];
/// decode_string(&mut buf);
///
/// let name = String::from_utf8_lossy(&buf).to_string();
/// assert_eq!(name, "Void");
/// ````
pub fn decode_string(buf: &mut [u8]) {
    buf.reverse();

    let parity = (buf.len() + 1) % 2;

    for (i, c) in buf.iter_mut().enumerate() {
        *c = match (i % 2 != parity, *c) {
            (true, ch @ 34..=125) => 125 - ch + 34,
            (false, ch @ 34..=79) => 79 - ch + 34,
            (false, ch @ 80..=125) => 125 - ch + 80,
            _ => *c,
        };
    }
}

/// Encodes a string in place
///
/// This is used for map names and sign text in map files
///
/// # Examples
///
/// ```
/// use eolib::data::encode_string;
///
/// let mut buf = b"Void".to_vec();
/// encode_string(&mut buf);
///
/// assert_eq!(buf, [0x69, 0x36, 0x5E, 0x49]);
/// ````
pub fn encode_string(buf: &mut [u8]) {
    let parity = (buf.len() + 1) % 2;

    for (i, c) in buf.iter_mut().enumerate() {
        *c = match (i % 2 != parity, *c) {
            (true, ch @ 34..=125) => 125 - ch + 34,
            (false, ch @ 34..=79) => 79 - ch + 34,
            (false, ch @ 80..=125) => 125 - ch + 80,
            _ => *c,
        };
    }

    buf.reverse();
}

mod eo_reader;
pub use eo_reader::{EoReader, EoReaderError};
mod eo_writer;
pub use eo_writer::{EoWriter, EoWriterError};
mod eo_serialize;
pub use eo_serialize::{EoSerialize, EoSerializeError};

#[cfg(test)]
mod tests {
    use super::{CHAR_MAX, SHORT_MAX, THREE_MAX, decode_number, encode_number};

    #[test]
    fn encode_spreads_the_value_across_four_bytes() {
        assert_eq!(encode_number(42).unwrap(), [43, 254, 254, 254]);
        assert_eq!(encode_number(533).unwrap(), [28, 3, 254, 254]);
        assert_eq!(encode_number(888888).unwrap(), [100, 225, 14, 254]);
        assert_eq!(encode_number(18994242).unwrap(), [15, 189, 44, 2]);
        // The largest value each position can hold.
        let max = CHAR_MAX as u8;
        assert_eq!(encode_number(CHAR_MAX - 1).unwrap(), [max, 254, 254, 254]);
        assert_eq!(encode_number(SHORT_MAX - 1).unwrap(), [max, max, 254, 254]);
        assert_eq!(encode_number(THREE_MAX - 1).unwrap(), [max, max, max, 254]);
    }

    #[test]
    fn decode_reads_what_encode_wrote() {
        for value in [0, 1, 42, 252, 253, 533, 888888, 18994242, THREE_MAX - 1] {
            assert_eq!(decode_number(&encode_number(value).unwrap()), value);
        }
    }

    #[test]
    fn decode_stops_at_the_padding_byte() {
        assert_eq!(decode_number(&[43, 254, 254, 254]), 42);
        assert_eq!(decode_number(&[28, 3, 254, 254]), 533);
        assert_eq!(decode_number(&[15, 189, 44, 2]), 18994242);
        // Everything after the first 254 is padding, not a digit.
        assert_eq!(decode_number(&[43, 254, 5, 5]), 42);
        assert_eq!(decode_number(&[28, 254, 3, 3]), 27);
        assert_eq!(decode_number(&[254, 3, 3, 3]), 0);
    }

    #[test]
    fn decode_uses_only_the_first_four_bytes() {
        assert_eq!(decode_number(&[43, 254]), 42);
        assert_eq!(decode_number(&[]), 0);
        let all_four = 42 * (1 + CHAR_MAX + SHORT_MAX + THREE_MAX);
        assert_eq!(decode_number(&[43, 43, 43, 43, 43]), all_four);
    }

    #[test]
    fn decode_subtracts_one_from_a_zero_byte() {
        // A well-formed stream never carries one (the encoder writes digit + 1),
        // but the client's `int b = ch; b -= 1;` gives -1 for it.
        assert_eq!(decode_number(&[0]), -1);
    }
}
