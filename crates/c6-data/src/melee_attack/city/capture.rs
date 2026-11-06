//! Owned capture service on the same combat requirements/effect stores.
use crate::{
    melee_attack::{backend, execution, resolve, transition},
    unit_placement_state as placement, unit_sight_state as sight,
};
use c6_core::loaded_state::LoadedState;

#[derive(Debug)]
pub struct Captured {
    pub disposition: crate::city_capture::PendingDisposition,
    /// Read after removal callbacks, just before the old District's slot release.
    pub retired: crate::city_capture::retired_district::RetiredDistrictScalars,
}
/// The provider wires the concrete public capture transition to the ongoing
/// founding, territory, callback and lifecycle services. It returns the SAME
/// owned stores on either result, preserving every applied failure prefix.
pub trait Provider<E: placement::Effects, R> {
    #[allow(clippy::too_many_arguments)]
    fn capture(
        &mut self,
        state: &mut LoadedState,
        sight: &mut sight::Runtime,
        effects: &mut E,
        runtime: backend::EffectRuntime,
        requirements: R,
        members: &super::Generated,
    ) -> (backend::EffectRuntime, R, Result<Captured, String>);
}
pub struct WithCapture<D, C> {
    pub damage: D,
    pub capture: C,
}
impl<E: placement::Effects, R, D: transition::Damage<E, R>, C: Provider<E, R>>
    transition::Damage<E, R> for WithCapture<D, C>
{
    type Error = D::Error;
    fn apply(
        &mut self,
        state: &mut LoadedState,
        sight: &mut sight::Runtime,
        effects: &mut E,
        runtime: &mut backend::EffectRuntime,
        requirements: &mut R,
        definitions: &placement::Definitions<'_, '_>,
        queries: &placement::Queries<'_>,
        formation: &crate::formation_rules::Definitions,
        pending: execution::RequiresResolve,
    ) -> Result<resolve::RequiresExperience, Self::Error> {
        self.damage.apply(
            state,
            sight,
            effects,
            runtime,
            requirements,
            definitions,
            queries,
            formation,
            pending,
        )
    }
    fn capture(
        &mut self,
        state: &mut LoadedState,
        sight: &mut sight::Runtime,
        effects: &mut E,
        runtime: backend::EffectRuntime,
        requirements: R,
        members: &super::Generated,
    ) -> (backend::EffectRuntime, R, Result<Captured, String>) {
        self.capture
            .capture(state, sight, effects, runtime, requirements, members)
    }
}
