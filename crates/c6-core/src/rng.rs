//! Gameplay RNG primitive recovered from the pinned XP2 DLL.
//! See `evidence/native/xp2-random.asm` and `docs/architecture.md`.

/// Logical fields of GameCore::Random; not a C++ object or memory layout.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RandomState {
    pub seed: u32,
    pub calls: u32,
    pub resets: u32,
    pub lock_count: u32,
    pub synchronous: bool,
}

impl RandomState {
    /// Equivalent logical state after Initialize on an asynchronous instance.
    pub const fn new(seed: u32) -> Self {
        Self {
            seed,
            calls: 0,
            resets: 0,
            lock_count: 0,
            synchronous: false,
        }
    }

    /// Execute Get's arithmetic and counters. Logging callbacks are excluded.
    /// Even bound zero consumes a draw. The native bound is an unsigned short.
    #[inline]
    pub fn draw(&mut self, bound: u16) -> u16 {
        self.seed = self.seed.wrapping_mul(0x41c6_4e6d).wrapping_add(0x3039);
        self.calls = self.calls.wrapping_add(1);
        (((self.seed >> 16) * u32::from(bound)) >> 16) as u16
    }

    pub fn lock(&mut self) {
        self.lock_count = self.lock_count.wrapping_add(1);
    }

    pub fn unlock(&mut self) {
        self.lock_count = self.lock_count.saturating_sub(1);
    }

    pub fn reset(&mut self, seed: u32) {
        self.seed = seed;
        self.resets = self.resets.wrapping_add(1);
    }

    pub fn initialize(&mut self, seed: u32) {
        *self = Self {
            synchronous: self.synchronous,
            ..Self::new(seed)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_bound_consumes_state_and_counters_wrap() {
        let mut rng = RandomState {
            calls: u32::MAX,
            ..RandomState::new(0)
        };
        assert_eq!(rng.draw(0), 0);
        assert_eq!(rng.seed, 12345);
        assert_eq!(rng.calls, 0);
    }
    #[test]
    fn reset_and_initialize_have_distinct_counter_semantics() {
        let mut rng = RandomState {
            calls: 9,
            resets: u32::MAX,
            lock_count: 2,
            synchronous: true,
            seed: 5,
        };
        rng.reset(7);
        assert_eq!(
            rng,
            RandomState {
                seed: 7,
                resets: 0,
                calls: 9,
                lock_count: 2,
                synchronous: true
            }
        );
        rng.initialize(12);
        assert_eq!(
            rng,
            RandomState {
                seed: 12,
                synchronous: true,
                ..RandomState::default()
            }
        );
    }
    #[test]
    fn lock_wraps_and_unlock_saturates() {
        let mut rng = RandomState {
            lock_count: u32::MAX,
            ..RandomState::new(0)
        };
        rng.lock();
        assert_eq!(rng.lock_count, 0);
        rng.unlock();
        assert_eq!(rng.lock_count, 0);
        rng.lock();
        rng.unlock();
        assert_eq!(rng.lock_count, 0);
    }
    #[test]
    fn seeded_sequence_and_max_bound() {
        let mut rng = RandomState::new(1);
        assert_eq!(rng.draw(u16::MAX), 16837);
        assert_eq!(rng.seed, 1103527590);
        assert_eq!(rng.draw(u16::MAX), 38525);
    }
}
