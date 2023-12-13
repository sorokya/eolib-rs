use rand::Rng;

/// returns a random swap multiple
pub fn generate_swap_multiple() -> u8 {
    let mut rng = rand::thread_rng();
    rng.gen_range(6..=12)
}
