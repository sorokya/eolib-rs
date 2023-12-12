/// This hash function is how the game client checks that it's communicating with a genuine server
/// during connection initialization.
///
/// - The client sends an integer value to the server in the INIT_INIT client packet, where it is referred to as the `challenge`
/// - The server hashes the value and sends the hash back in the INIT_INIT server packet.
/// - The client hashes the value and compares it to the hash sent by the server.
/// - If the hashes don't match, the client drops the connection.
///
/// # Examples
///
/// ```
/// use eolib::encrypt::server_verification_hash;
///
/// let challenge = 123456;
/// assert_eq!(server_verification_hash(challenge), 300733);
/// ````
///
/// # Warning
/// Oversized challenges may result in negative hash values, which cannot be represented properly in the EO protocol.
pub fn server_verification_hash(mut challenge: i32) -> i32 {
    challenge += 1;
    110905
        + ((challenge % 9) + 1) * ((11092004 - challenge) % (((challenge % 11) + 1) * 119)) * 119
        + (challenge % 2004)
}
