//! City branch of AttackInto, including an owned capture loan and restoration.
use super::{resolve as city_resolve, Definitions};
use crate::{
    loaded_upkeep as upkeep,
    melee_attack::{
        backend, experience, resolve,
        transition::{self, Reports, Stage},
    },
    unit_placement_state as placement, unit_sight_state as sight,
};
use c6_core::{loaded_state::LoadedState, map::UnitReference, unit::ComponentId};
type ProviderError<E> = <E as sight::Effects>::Error;
fn failure(stage: Stage, error: impl std::fmt::Debug) -> transition::Error {
    transition::Error {
        stage,
        detail: format!("{error:?}"),
    }
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn execute<'e, E, R, D>(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    mut backend: backend::Backend<Reports<'e, E>, R>,
    damage: &mut D,
    definitions: &Definitions,
    inputs: &transition::Inputs<'_, '_, '_>,
    bindings: &mut resolve::completion::stock::Bindings,
    attacker: UnitReference,
    defender: ComponentId,
) -> (
    backend::Backend<Reports<'e, E>, R>,
    Result<transition::Outcome, transition::Error>,
)
where
    R: backend::Requirements,
    R::Error: std::fmt::Debug,
    D: transition::Damage<E, R>,
    E: city_resolve::Effects
        + placement::Effects
        + upkeep::Effects<Error = ProviderError<E>>
        + upkeep::OperationReports<Error = ProviderError<E>>
        + experience::Reports<Error = ProviderError<E>>
        + resolve::surviving::Effects
        + backend::PostCombatYields
        + resolve::completion::stock::UnitDistrictEffects,
    ProviderError<E>: std::fmt::Debug,
    <E as resolve::completion::stock::Effects>::Error: std::fmt::Debug,
{
    let prepared = (|| {
        let members = super::generate(
            state,
            &mut backend,
            definitions,
            inputs.generation,
            inputs.experience,
            attacker,
            defender,
        )
        .map_err(|e| failure(Stage::Generate, e))?;
        let (reports, _, _, _) = backend.kill_parts();
        city_resolve::prepare(state, sight, reports.0, inputs, members)
            .map_err(|e| failure(Stage::Damage, e))
    })();
    let mut prepared = match prepared {
        Ok(p) => p,
        Err(e) => return (backend, Err(e)),
    };
    if prepared.capture_required {
        let origin = backend.origin();
        let (reports, requirements, runtime, backend_inputs) = backend.into_parts();
        let (runtime, requirements, captured) = damage.capture(
            state,
            sight,
            reports.0,
            runtime,
            requirements,
            &prepared.members,
        );
        let restored = backend::Backend::resume_owned(
            state,
            origin,
            reports,
            requirements,
            runtime,
            backend_inputs,
        );
        let (restored, accepted) = match restored {
            Ok(b) => (b, true),
            Err(b) => (b, false),
        };
        backend = restored;
        if !accepted {
            return (
                backend,
                Err(failure(Stage::Backend, "capture owner identity changed")),
            );
        }
        let captured = match captured {
            Ok(c) => c,
            Err(e) => return (backend, Err(failure(Stage::CaptureProposal, e))),
        };
        if captured.retired.district_id != defender.id
            || captured.retired.location != prepared.members.location
            || captured.disposition.city.component_type != 2
            || captured.disposition.district.component_type != 3
            || i32::from(captured.disposition.city.owner) != attacker.player()
            || captured.disposition.city.owner != captured.disposition.district.owner
        {
            return (
                backend,
                Err(failure(
                    Stage::CaptureProposal,
                    "capture returned inconsistent logical identities",
                )),
            );
        }
        prepared.captured = Some(captured);
    }
    let (reports, _, _, _) = backend.kill_parts();
    let result = city_resolve::complete(
        state,
        sight,
        reports.0,
        definitions,
        inputs,
        bindings,
        prepared,
    )
    .map(transition::Outcome::CityResolved)
    .map_err(|e| failure(Stage::Completion, e));
    (backend, result)
}
