//! Bounded original Linux RNG components, independent of gameplay stream ownership.
//!
//! Contracts target the pinned XP2 ScopedRandom and matching host Math::RandomPCG32.
//! Raw PCG state includes the increment even when it is noncanonical. This module
//! does not assign streams, load Civ VI saves, or supply time-dependent Lua seeds.

/// Four-byte XP2 ScopedRandom state; Get advances even for a zero bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScopedRandom {
    seed: u32,
}

impl ScopedRandom {
    pub const fn new(seed: u32) -> Self {
        Self { seed }
    }

    pub const fn seed(&self) -> u32 {
        self.seed
    }

    pub fn reset(&mut self, seed: u32) {
        self.seed = seed;
    }

    pub fn get(&mut self, bound: u16) -> u32 {
        self.seed = self.seed.wrapping_mul(0x41c6_4e6d).wrapping_add(0x3039);
        ((self.seed >> 16) * u32::from(bound)) >> 16
    }
}

/// Complete two-word host PCG state. Imported increments are preserved verbatim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RandomPcg32 {
    pub state: u64,
    pub increment: u64,
}

impl Default for RandomPcg32 {
    fn default() -> Self {
        Self::new(0)
    }
}

impl RandomPcg32 {
    const MULTIPLIER: u64 = 0x5851_f42d_4c95_7f2d;
    const INITIAL: u64 = 0x08f5_dc87_e5c0_7d8a;

    /// Seeded constructor. The original default constructor is equivalent to seed zero.
    pub const fn new(seed: u32) -> Self {
        Self {
            state: (seed as u64)
                .wrapping_mul(Self::MULTIPLIER)
                .wrapping_add(Self::INITIAL),
            increment: 3,
        }
    }

    pub fn seed(&mut self, seed: u32) {
        *self = Self::new(seed);
    }

    /// Return the permutation of the old state and advance both-word state exactly.
    pub fn rand(&mut self) -> u32 {
        let old = self.state;
        self.state = old
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(self.increment);
        let value = (((old >> 18) ^ old) >> 27) as u32;
        value.rotate_right((old >> 59) as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoped_zero_bound_advances_and_reset_replaces_seed() {
        let mut rng = ScopedRandom::new(u32::MAX);
        assert_eq!(rng.get(0), 0);
        assert_eq!(rng.seed(), 0xbe39_e1cc);
        rng.reset(0);
        assert_eq!(rng.get(u16::MAX), 0);
        assert_eq!(rng.seed(), 12345);
    }

    #[test]
    fn pcg_rotation_wrap_and_raw_increment_survive() {
        let mut rng = RandomPcg32 {
            state: u64::MAX,
            increment: u64::MAX,
        };
        assert_eq!(rng.rand(), 0xfff0_0001);
        assert_eq!(rng.state, 0xa7ae_0bd2_b36a_80d2);
        assert_eq!(rng.increment, u64::MAX);
        rng.seed(u32::MAX);
        assert_eq!(rng, RandomPcg32::new(u32::MAX));
        assert_eq!(rng.increment, 3);
    }

    #[test]
    fn independent_copied_streams_keep_continuity() {
        let mut streams = [RandomPcg32::new(0), RandomPcg32::new(u32::MAX)];
        let mut separate = streams;
        for i in 0..512 {
            let selected = (i * 7 % 11) % 2;
            assert_eq!(streams[selected].rand(), separate[selected].rand());
        }
        assert_eq!(streams, separate);
        assert_eq!(RandomPcg32::default(), RandomPcg32::new(0));
    }
}
