//! Linux HKS math.random/randomseed and the matched libc process-global stream.
//!
//! This stream is distinct from GameCore Random and Cache Lua PCG32. An existing
//! game must import its actual current words and cursor positions; no default
//! initialized stream is supplied. Lua argument conversion remains the caller's
//! real luaL_checkinteger operation, invoked lazily in native order.

/// The five algorithms selected by the matched libc random_data type field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Lcg,
    Additive7,
    Additive15,
    Additive31,
    Additive63,
}
impl Kind {
    pub const fn degree(self) -> usize {
        match self {
            Self::Lcg => 0,
            Self::Additive7 => 7,
            Self::Additive15 => 15,
            Self::Additive31 => 31,
            Self::Additive63 => 63,
        }
    }
    pub const fn separation(self) -> usize {
        match self {
            Self::Additive7 | Self::Additive31 => 3,
            Self::Additive15 | Self::Additive63 => 1,
            Self::Lcg => 0,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidState;

/// Canonical logical generator state. Native addresses, locking and state[-1]
/// serialization metadata are separate from these live draw/seed inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    kind: Kind,
    words: Vec<u32>,
    front: usize,
    rear: usize,
}
impl State {
    pub fn from_parts(
        kind: Kind,
        words: Vec<u32>,
        front: usize,
        rear: usize,
    ) -> Result<Self, InvalidState> {
        let degree = kind.degree();
        if words.len() != degree.max(1)
            || front >= words.len()
            || rear >= words.len()
            || (degree != 0 && (front + degree - rear) % degree != kind.separation())
        {
            return Err(InvalidState);
        }
        Ok(Self {
            kind,
            words,
            front,
            rear,
        })
    }
    /// Explicit new initialization, equivalent to selecting this libc kind then
    /// seeding it. This never stands in for importing an existing process.
    pub fn seeded(kind: Kind, seed: u32) -> Self {
        let mut state = Self {
            kind,
            words: vec![0; kind.degree().max(1)],
            front: kind.separation(),
            rear: 0,
        };
        state.seed(seed);
        state
    }
    pub const fn kind(&self) -> Kind {
        self.kind
    }
    pub fn words(&self) -> &[u32] {
        &self.words
    }
    pub const fn front(&self) -> usize {
        self.front
    }
    pub const fn rear(&self) -> usize {
        self.rear
    }

    /// Matched libc srandom_r. Zero seed becomes one; high seeds enter the
    /// recurrence as signed i32. Nonzero kinds discard exactly 10*degree draws.
    pub fn seed(&mut self, seed: u32) {
        let seed = if seed == 0 { 1 } else { seed };
        self.words[0] = seed;
        let degree = self.kind.degree();
        if degree == 0 {
            return;
        }
        let mut word = seed as i32;
        for cell in &mut self.words[1..] {
            let hi = word / 127773;
            let lo = word % 127773;
            word = 16807 * lo - 2836 * hi;
            if word < 0 {
                word += i32::MAX;
            }
            *cell = word as u32;
        }
        self.front = self.kind.separation();
        self.rear = 0;
        for _ in 0..degree * 10 {
            self.draw();
        }
    }
    /// One matched libc random_r draw; libc's locking adds no RNG words.
    pub fn draw(&mut self) -> u32 {
        if self.kind == Kind::Lcg {
            self.words[0] = self.words[0].wrapping_mul(1103515245).wrapping_add(12345) & 0x7fffffff;
            return self.words[0];
        }
        let value = self.words[self.front].wrapping_add(self.words[self.rear]);
        self.words[self.front] = value;
        self.front += 1;
        self.rear += 1;
        if self.front == self.words.len() {
            self.front = 0;
        } else if self.rear == self.words.len() {
            self.rear = 0;
        }
        value >> 1
    }

    /// HKS draws BEFORE reading/checking any arguments. Extra arguments after
    /// the second are ignored. `integer` is the caller's real, fallible
    /// luaL_checkinteger leaf, with 1-based indices and native call order.
    pub fn math_random<E>(
        &mut self,
        argument_count: usize,
        mut integer: impl FnMut(usize) -> Result<i32, E>,
    ) -> Result<f64, Error<E>> {
        let fraction = f64::from(self.draw() % 0x7fffffff) / 2147483647.0;
        if argument_count == 0 {
            return Ok(fraction);
        }
        let first = integer(1).map_err(Error::Argument)?;
        let (low, width) = if argument_count == 1 {
            if first <= 0 {
                return Err(Error::EmptyInterval { argument: 1 });
            }
            (1.0, f64::from(first))
        } else {
            let second = integer(2).map_err(Error::Argument)?;
            if second < first {
                return Err(Error::EmptyInterval { argument: 2 });
            }
            let low = f64::from(first);
            (low, (f64::from(second) - low) + 1.0)
        };
        Ok((fraction * width).floor() + low)
    }
    /// HKS checks its first integer argument before calling srand. It performs
    /// no extra draw after seeding, and ignores subsequent arguments.
    pub fn math_randomseed<E>(
        &mut self,
        mut integer: impl FnMut(usize) -> Result<i32, E>,
    ) -> Result<(), E> {
        self.seed(integer(1)? as u32);
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error<E> {
    Argument(E),
    EmptyInterval { argument: usize },
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_draw_order_and_argument_failures() {
        let initial = State::seeded(Kind::Additive31, 1);
        let mut expected = initial.clone();
        expected.draw();
        let mut state = initial.clone();
        assert_eq!(
            state.math_random(3, |i| if i == 1 {
                Err("number expected")
            } else {
                panic!()
            }),
            Err(Error::Argument("number expected"))
        );
        assert_eq!(state, expected);
        let mut state = initial.clone();
        assert_eq!(
            state.math_random::<()>(1, |_| Ok(0)),
            Err(Error::EmptyInterval { argument: 1 })
        );
        assert_eq!(state, expected);
        let mut state = initial.clone();
        assert_eq!(
            state.math_random::<()>(2, |i| Ok(if i == 1 { 7 } else { 6 })),
            Err(Error::EmptyInterval { argument: 2 })
        );
        assert_eq!(state, expected);
        let mut state = initial.clone();
        assert_eq!(state.math_randomseed(|_| Err("missing")), Err("missing"));
        assert_eq!(state, initial);
    }
    #[test]
    fn modulo_maximum_and_ignored_extra_arguments() {
        let mut state = State::from_parts(
            Kind::Additive7,
            vec![u32::MAX, 0, 0, u32::MAX, 0, 0, 0],
            3,
            0,
        )
        .unwrap();
        assert_eq!(state.math_random::<()>(0, |_| panic!()), Ok(0.0));
        let mut state = State::seeded(Kind::Additive31, 1);
        let mut indices = Vec::new();
        assert_eq!(
            state.math_random::<()>(9, |i| {
                indices.push(i);
                Ok(-3)
            }),
            Ok(-3.0)
        );
        assert_eq!(indices, [1, 2]);
    }
    #[test]
    fn explicit_import_rejects_impossible_shape_and_preserves_words() {
        assert!(State::from_parts(Kind::Additive7, vec![0; 7], 0, 0).is_err());
        let state = State::from_parts(Kind::Additive7, vec![u32::MAX; 7], 6, 3).unwrap();
        assert_eq!(state.words(), &[u32::MAX; 7]);
        assert_eq!(
            State::seeded(Kind::Additive31, 0),
            State::seeded(Kind::Additive31, 1)
        );
    }
}
