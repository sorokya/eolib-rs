use super::{swap_multiples, valid_for_encryption};

/// Decrypts a packet.
///
/// Packets are decrypted in three steps:
/// 1. Flipping
/// 2. Interleaving
/// 3. "dickwinding"
///
/// ## Flipping
/// Each byte of the packet has their most significant bits flipped
/// ```text
/// for i in 0..length {
///     bytes[i] ^= 0x80;
/// }
/// ```
///
/// ## Interleaving
/// Bytes are "woven" in to each-other e.g.
/// ```text
/// abcde -> acedb
///   or
/// abcdef -> acefdb
/// ```
///
/// ## Dickwinding
/// This was named by Sausage and first implemented in the EOProxy project.
/// There are two numbers sent from the server to the client on connect
/// between 6 and 12 that represent a "send packet swap multiple"
/// and a "receive packet swap multiple".
///
/// Any two bytes next to each other in the packet data that are
/// divisible by that number are swapped.
///
/// For more details see [Packet](https://eoserv.net/wiki/wiki?page=Packet)
///
/// # Examples
/// ```
/// use eolib::encrypt::decrypt_packet;
///
/// let mut buf = [149, 161, 146, 228, 17, 242, 200, 236, 229, 239, 236, 247, 236, 160, 239, 172];
///
/// decrypt_packet(&mut buf, 6);
///
/// assert_eq!(buf, [21, 18, 145, 72, 101, 108, 108, 111, 44, 32, 119, 111, 114, 108, 100, 33]);
/// ```
pub fn decrypt_packet(buf: &mut [u8], swap_multiple: u8) {
    if !valid_for_encryption(buf) {
        return;
    }

    let length = buf.len();
    let mut tmp: Vec<u8> = vec![0; length];
    let big_half = (length + 1) / 2;
    let little_half = length / 2;
    for i in 0..big_half {
        tmp[i] = buf[i * 2];
        if tmp[i] & 0x7f != 0 {
            tmp[i] ^= 0x80;
        }
    }
    for i in 0..little_half {
        tmp[length - 1 - i] = buf[(i * 2) + 1];
        if tmp[length - 1 - i] & 0x7f != 0 {
            tmp[length - 1 - i] ^= 0x80;
        }
    }
    swap_multiples(&mut tmp, swap_multiple);
    buf.copy_from_slice(&tmp);
}
