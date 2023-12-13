mod server_verification_hash;
pub use server_verification_hash::server_verification_hash;
mod swap_multiples;
pub use swap_multiples::swap_multiples;
mod generate_swap_multiple;
pub use generate_swap_multiple::generate_swap_multiple;
mod encrypt_packet;
pub use encrypt_packet::encrypt_packet;
mod decrypt_packet;
pub use decrypt_packet::decrypt_packet;

pub(crate) fn valid_for_encryption(buf: &[u8]) -> bool {
    buf.len() > 2 && buf[0..=1] != [0xff, 0xff]
}
