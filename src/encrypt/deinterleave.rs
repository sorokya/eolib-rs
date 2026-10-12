/// Deinterleaves a sequence of bytes. This is the reverse of
/// [`interleave`](crate::encrypt::interleave).
///
/// Used when decrypting packets and data files.
///
/// # Examples
///
/// ```
/// use eolib::encrypt::deinterleave;
///
/// let mut bytes = [0, 1, 2, 3, 4, 5];
/// deinterleave(&mut bytes);
///
/// assert_eq!(bytes, [0, 2, 4, 5, 3, 1]);
/// ```
///
/// This is an in-place operation.
pub fn deinterleave(data: &mut [u8]) {
    let mut buffer = vec![0u8; data.len()];

    let mut i = 0;
    let mut ii = 0;

    while i < data.len() {
        buffer[ii] = data[i];
        i += 2;
        ii += 1;
    }

    // The largest odd index below the length, walked back towards zero.
    let mut i: isize = if data.len() % 2 == 1 {
        data.len() as isize - 2
    } else {
        data.len() as isize - 1
    };

    while i >= 0 {
        buffer[ii] = data[i as usize];
        ii += 1;
        i -= 2;
    }

    data.copy_from_slice(&buffer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deinterleaves_even_length() {
        let mut bytes = [0, 1, 2, 3, 4, 5];
        deinterleave(&mut bytes);
        assert_eq!(bytes, [0, 2, 4, 5, 3, 1]);
    }

    #[test]
    fn deinterleaves_odd_length() {
        let mut bytes = [0, 1, 2, 3, 4];
        deinterleave(&mut bytes);
        assert_eq!(bytes, [0, 2, 4, 3, 1]);
    }

    #[test]
    fn handles_short_inputs() {
        let mut empty: [u8; 0] = [];
        deinterleave(&mut empty);
        let mut one = [42];
        deinterleave(&mut one);
        assert_eq!(one, [42]);
        let mut two = [1, 2];
        deinterleave(&mut two);
        assert_eq!(two, [1, 2]);
    }

    #[test]
    fn roundtrips_with_interleave() {
        let original: Vec<u8> = (0..64).collect();
        for length in 0..=original.len() {
            let mut bytes = original[..length].to_vec();
            crate::encrypt::interleave(&mut bytes);
            deinterleave(&mut bytes);
            assert_eq!(bytes, original[..length], "length {length}");
        }
    }
}
