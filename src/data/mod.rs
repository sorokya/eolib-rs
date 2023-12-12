/// The maximum value of an EO char (1-byte encoded integer type)
pub const CHAR_MAX: i32 = 253;

/// The maximum value of an EO short (2-byte encoded integer type)
pub const SHORT_MAX: i32 = CHAR_MAX * CHAR_MAX;

/// The maximum value of an EO three (3-byte encoded integer type)
pub const THREE_MAX: i32 = CHAR_MAX * CHAR_MAX * CHAR_MAX;

/// The maximum value of an EO int (4-byte encoded integer type)
pub const INT_MAX: i32 = i32::MAX;

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
/// This is used later in [decode_number] for translating to a 0 value.
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
/// let result = encode_number(42);
/// assert_eq!(result, [43, 254, 254, 254]);
/// ```
/// since 42 is less than CHAR_MAX it is simply incremented by 1
/// and the remaining bytes are set to 254
///
/// ## Number less than SHORT_MAX
/// ```
/// use eolib::data::encode_number;
/// let result = encode_number(533);
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
/// let result = encode_number(888888);
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
/// let result = encode_number(18994242);
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
pub fn encode_number(mut number: i32) -> [u8; 4] {
    let mut bytes: [u8; 4] = [254, 254, 254, 254];
    let original_number = number;

    if original_number >= THREE_MAX {
        bytes[3] = (number / THREE_MAX) as u8 + 1;
        number %= THREE_MAX;
    }

    if original_number >= SHORT_MAX {
        bytes[2] = (number / SHORT_MAX) as u8 + 1;
        number %= SHORT_MAX;
    }

    if original_number >= CHAR_MAX {
        bytes[1] = (number / CHAR_MAX) as u8 + 1;
        number %= CHAR_MAX;
    }

    bytes[0] = number as u8 + 1;

    bytes
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
/// The byte array is iterated over and any byte of 254 is changed to 1, and
/// each byte is decremented by 1.
///
/// The result is then calculated like so
///
/// `(b4 * THREE_MAX) + (b3 * SHORT_MAX) + (b2 * CHAR_MAX) + b1`
///
/// # Examples
/// ```
/// use eolib::data::decode_number;
/// let result = decode_number(&[43, 254, 254, 254]);
/// assert_eq!(result, 42);
/// ```
///
/// * bytes with `254` are swapped to `1`
/// `[43, 1, 1, 1]`
/// * bytes are decremented by 1
/// `[42, 0, 0, 0]`
/// * bytes are multiplied by MAX's and summed
/// `(0 * THREE_MAX) + (0 * SHORT_MAX) + (0 * CHAR_MAX) + 42 == 42`
///
pub fn decode_number(bytes: &[u8]) -> i32 {
    let mut data: [u8; 4] = [254, 254, 254, 254];
    for i in 0..4 {
        if bytes.len() > i && bytes[i] != 0 {
            data[i] = bytes[i];
        }
        if data[i] == 254 {
            data[i] = 1;
        }
        data[i] -= 1;
    }

    (data[3] as i32 * THREE_MAX)
        + (data[2] as i32 * SHORT_MAX)
        + (data[1] as i32 * CHAR_MAX)
        + data[0] as i32
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
pub use eo_reader::EoReader;
mod eo_writer;
pub use eo_writer::EoWriter;
