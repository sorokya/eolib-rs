use crate::rng;

/// returns a random swap multiple
pub fn generate_swap_multiple() -> u8 {
    rng::internal_rand_range(6..12) as u8
}

#[cfg(test)]
mod tests {
    use super::generate_swap_multiple;

    #[test]
    fn stays_within_bounds() {
        for _ in 0..1000 {
            let value = generate_swap_multiple();
            assert!((6..=12).contains(&value));
        }
    }
}
