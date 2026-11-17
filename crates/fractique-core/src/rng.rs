//! xoshiro256++ seeded through SplitMix64. Written out here so that no dependency update can change a render.

#[derive(Clone, Debug)]
pub struct Rng {
    s: [u64; 4],
}

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Rng {
    /// An independent stream for each (seed, domain, index).
    pub fn derive(seed: u64, domain: u64, index: u64) -> Self {
        let mut state = seed;
        state = splitmix(&mut state) ^ domain.wrapping_mul(0xD6E8_FEB8_6659_FD93);
        state = splitmix(&mut state) ^ index.wrapping_mul(0xA076_1D64_78BD_642F);
        let mut s = [0u64; 4];
        for word in &mut s {
            *word = splitmix(&mut state);
        }
        if s == [0; 4] {
            s[0] = 1;
        }
        Self { s }
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let s = &mut self.s;
        let result = s[0].wrapping_add(s[3]).rotate_left(23).wrapping_add(s[0]);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        result
    }

    /// Uniform in [0, 1).
    #[inline]
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform in [lo, hi).
    #[inline]
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_f64()
    }

    /// Uniform in 0..n; n must not be 0.
    #[inline]
    pub fn below(&mut self, n: u64) -> u64 {
        ((u128::from(self.next_u64()) * u128::from(n)) >> 64) as u64
    }

    #[inline]
    pub fn next_bool(&mut self) -> bool {
        self.next_u64() >> 63 == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_are_fixed() {
        let mut rng = Rng::derive(42, 1, 2);
        let first: Vec<u64> = (0..3).map(|_| rng.next_u64()).collect();
        let mut again = Rng::derive(42, 1, 2);
        assert_eq!(first, (0..3).map(|_| again.next_u64()).collect::<Vec<_>>());
        let mut other = Rng::derive(42, 1, 3);
        assert_ne!(first[0], other.next_u64());
    }

    #[test]
    fn floats_stay_in_range() {
        let mut rng = Rng::derive(7, 0, 0);
        for _ in 0..10_000 {
            let v = rng.next_f64();
            assert!((0.0..1.0).contains(&v));
            assert!(rng.below(5) < 5);
        }
    }
}
