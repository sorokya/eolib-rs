/// Interleaves a sequence of bytes. When encrypting EO data, bytes are "woven" into each
/// other.
///
/// Used when encrypting packets and data files.
///
/// # Examples
///
/// ```
/// use eolib::encrypt::interleave;
///
/// let mut bytes = [0, 1, 2, 3, 4, 5];
/// interleave(&mut bytes);
///
/// assert_eq!(bytes, [0, 5, 1, 4, 2, 3]);
/// ```
///
/// This is an in-place operation and the inverse of [`deinterleave`](crate::encrypt::deinterleave).
pub fn interleave(data: &mut [u8]) {
    let mut buffer = vec![0u8; data.len()];

    let mut i = 0;
    let mut ii = 0;

    while i < data.len() {
        buffer[i] = data[ii];
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
        buffer[i as usize] = data[ii];
        ii += 1;
        i -= 2;
    }

    data.copy_from_slice(&buffer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interleaves_even_length() {
        let mut bytes = [0, 1, 2, 3, 4, 5];
        interleave(&mut bytes);
        assert_eq!(bytes, [0, 5, 1, 4, 2, 3]);
    }

    #[test]
    fn interleaves_odd_length() {
        let mut bytes = [0, 1, 2, 3, 4];
        interleave(&mut bytes);
        assert_eq!(bytes, [0, 4, 1, 3, 2]);
    }

    #[test]
    fn handles_short_inputs() {
        let mut empty: [u8; 0] = [];
        interleave(&mut empty);
        let mut one = [42];
        interleave(&mut one);
        assert_eq!(one, [42]);
        let mut two = [1, 2];
        interleave(&mut two);
        assert_eq!(two, [1, 2]);
    }
}
