/// Swaps the order of contiguous bytes in a sequence of bytes that are divisible by a given
/// multiple value.
///
/// Used when encrypting and decrypting packets and data files.
///
/// # Examples
///
/// ```
/// use eolib::encrypt::swap_multiples;
///
/// let mut bytes = [10, 21, 27];
/// swap_multiples(&mut bytes, 3);
///
/// assert_eq!(bytes, [10, 27, 21]);
/// ```
pub fn swap_multiples(bytes: &mut [u8], multiple: u8) {
    let length = bytes.len();
    let mut sequence_length: usize = 0;
    for i in 0..=length {
        if i != length && bytes[i] % multiple == 0 {
            sequence_length += 1;
        } else {
            if sequence_length > 1 {
                let start = i - sequence_length;
                let end = start + sequence_length - 1;
                for i in 0..sequence_length / 2 {
                    bytes.swap(start + i, end - i);
                }
            }
            sequence_length = 0;
        }
    }
}
