//! Optional observed HKS math.random owner. No default/native libc is invoked.
use super::ScriptState;
use c6_core::{
    loaded_state::{LoadedState, ObservedSnapshotWitness},
    lua_math_random::{self, NativeSnapshot, State},
};
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScriptMathRngError<E> {
    Missing,
    Binding,
    AlreadyBound,
    Random(lua_math_random::Error<E>),
    Argument(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for ScriptMathRngError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "script math RNG: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for ScriptMathRngError<E> {}
pub(super) struct OwnedMathRng(Option<(State, u32)>);
impl OwnedMathRng {
    pub(super) fn unobserved() -> Self {
        Self(None)
    }
    fn install(&mut self, snapshot: NativeSnapshot) -> Result<(), ScriptMathRngError<()>> {
        let header = snapshot.state_header();
        self.install_supplied(snapshot.into_state(), header)
    }
    fn install_supplied(
        &mut self,
        state: State,
        header: u32,
    ) -> Result<(), ScriptMathRngError<()>> {
        if self.0.is_some() {
            return Err(ScriptMathRngError::AlreadyBound);
        }
        self.0 = Some((state, header));
        Ok(())
    }
    fn random<E>(
        &mut self,
        count: usize,
        integer: impl FnMut(usize) -> Result<i32, E>,
    ) -> Result<f64, ScriptMathRngError<E>> {
        self.0
            .as_mut()
            .ok_or(ScriptMathRngError::Missing)?
            .0
            .math_random(count, integer)
            .map_err(ScriptMathRngError::Random)
    }
    fn seed<E>(
        &mut self,
        integer: impl FnMut(usize) -> Result<i32, E>,
    ) -> Result<(), ScriptMathRngError<E>> {
        self.0
            .as_mut()
            .ok_or(ScriptMathRngError::Missing)?
            .0
            .math_randomseed(integer)
            .map_err(ScriptMathRngError::Argument)
    }
}
impl ScriptState {
    /// Import exactly this observed initial VM's matched-libc owner. The
    /// adapter checks module mapping, epoch, descriptor and raw-byte stability.
    /// Logical state moves here; it is never replaced by native afterstate.
    pub fn bind_native_math_rng(
        &mut self,
        state: &LoadedState,
        witness: &ObservedSnapshotWitness,
        snapshot: NativeSnapshot,
    ) -> Result<(), ScriptMathRngError<()>> {
        if !state.is_pristine()
            || !self.matches(state)
            || self.origin.as_ref() != Some(witness)
            || state.observed_snapshot() != Some(witness)
        {
            return Err(ScriptMathRngError::Binding);
        }
        self.math_rng.install(snapshot)
    }
    /// Install the caller's exact initialized logical stream and serialization
    /// header. This consistency check neither authenticates a native observation
    /// nor creates an observation witness. No seed or host RNG is substituted.
    pub fn bind_supplied_math_rng(
        &mut self,
        state: &LoadedState,
        random: State,
        state_header: u32,
    ) -> Result<(), ScriptMathRngError<()>> {
        if !state.is_pristine() || !self.matches(state) {
            return Err(ScriptMathRngError::Binding);
        }
        self.math_rng.install_supplied(random, state_header)
    }
    pub fn math_rng(&self) -> Option<&State> {
        self.math_rng.0.as_ref().map(|(state, _)| state)
    }
    /// Native state[-1] metadata is retained independently of live draw words.
    pub fn math_rng_header(&self) -> Option<u32> {
        self.math_rng.0.as_ref().map(|(_, header)| *header)
    }
    /// This advances the same owner even when a later integer check fails,
    /// matching HKS's draw-before-argument order. No state means no implicit seed.
    pub fn math_random<E>(
        &mut self,
        state: &LoadedState,
        argument_count: usize,
        integer: impl FnMut(usize) -> Result<i32, E>,
    ) -> Result<f64, ScriptMathRngError<E>> {
        if !self.matches(state) {
            return Err(ScriptMathRngError::Binding);
        }
        self.math_rng.random(argument_count, integer)
    }
    pub fn math_randomseed<E>(
        &mut self,
        state: &LoadedState,
        integer: impl FnMut(usize) -> Result<i32, E>,
    ) -> Result<(), ScriptMathRngError<E>> {
        if !self.matches(state) {
            return Err(ScriptMathRngError::Binding);
        }
        self.math_rng.seed(integer)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot() -> NativeSnapshot {
        let mut descriptor = [0; 48];
        descriptor[16..24].copy_from_slice(&0x1004u64.to_le_bytes());
        descriptor[40..48].copy_from_slice(&0x1004u64.to_le_bytes());
        let raw = [0u32.to_le_bytes(), 17u32.to_le_bytes()].concat();
        NativeSnapshot::from_native_parts(
            lua_math_random::LINUX_LIBC_SHA256,
            descriptor,
            &raw,
            [0; 4],
        )
        .unwrap()
    }
    #[test]
    fn supplied_logical_state_retains_words_cursors_and_header() {
        let state = State::from_parts(
            lua_math_random::Kind::Additive7,
            vec![u32::MAX, 2, 3, 4, 5, 6, 7],
            5,
            2,
        )
        .unwrap();
        let mut owner = OwnedMathRng::unobserved();
        owner.install_supplied(state.clone(), 91).unwrap();
        assert_eq!(owner.0, Some((state.clone(), 91)));
        assert_eq!(
            owner.install_supplied(state.clone(), 92),
            Err(ScriptMathRngError::AlreadyBound)
        );
        let mut expected = state;
        let expected_result = expected.math_random::<()>(0, |_| unreachable!());
        assert_eq!(
            owner.random::<()>(0, |_| unreachable!()),
            expected_result.map_err(ScriptMathRngError::Random)
        );
        assert_eq!(owner.0, Some((expected, 91)));
    }
    #[test]
    fn missing_owner_does_not_check_arguments_or_seed_a_default() {
        let mut owner = OwnedMathRng::unobserved();
        assert_eq!(
            owner.random(1, |_| -> Result<i32, ()> {
                panic!("unavailable RNG must not evaluate arguments")
            }),
            Err(ScriptMathRngError::Missing)
        );
        assert_eq!(
            owner.seed(|_| -> Result<i32, ()> { panic!("unavailable RNG must not evaluate seed") }),
            Err(ScriptMathRngError::Missing)
        );
    }
    #[test]
    fn observed_owner_advances_before_argument_failure_and_is_not_replaced() {
        let initial = snapshot();
        let mut expected = initial.state().clone();
        expected.draw();
        let mut owner = OwnedMathRng::unobserved();
        owner.install(initial).unwrap();
        assert_eq!(
            owner.random(1, |_| Err("argument")),
            Err(ScriptMathRngError::Random(
                lua_math_random::Error::Argument("argument")
            ))
        );
        assert_eq!(owner.0.as_ref().map(|(state, _)| state), Some(&expected));
        assert_eq!(
            owner.install(snapshot()),
            Err(ScriptMathRngError::AlreadyBound)
        );
        let before = owner.0.clone();
        assert_eq!(
            owner.seed(|_| Err("seed")),
            Err(ScriptMathRngError::Argument("seed"))
        );
        assert_eq!(owner.0, before);
    }
}
