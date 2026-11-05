//! Ordinary SetImprovementType first placement, without Barbarian manager hooks.
use super::{Definitions, Error};
use crate::barbarian_camp_placement::{self as shared, OwnerContext, VisualContext};
use c6_core::{
    loaded_state::LoadedState,
    map::{DefinitionValue, PlotFlag},
};

/// Neutral owner/revelation primitives are shared with camp placement. The
/// constructed dispatcher and monopoly calculation are the actual ordinary
/// services; they must use the same retained modifiers, requirements and RNG.
pub trait Effects: OwnerContext + VisualContext<Error = <Self as OwnerContext>::Error> {
    fn constructed(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        definitions: &Definitions,
    ) -> Result<(), <Self as OwnerContext>::Error>;
    fn calculate_monopolies(
        &mut self,
        state: &mut LoadedState,
    ) -> Result<(), <Self as OwnerContext>::Error>;
}
fn shared_error<E>(error: shared::Error<E>) -> Error<E> {
    match error {
        shared::Error::Missing(message) => Error::Missing(message),
        shared::Error::Unsupported(message) => Error::Unsupported(message),
        shared::Error::State(error) => Error::State(error),
        shared::Error::Effect(error) => Error::Effect {
            stage: "Farm improvement service",
            error,
        },
        shared::Error::Random(_) => {
            Error::Unsupported("unexpected neutral improvement random access")
        }
    }
}
/// Linux 0x2a391d0 first-placement branch. The improvement is assigned before
/// owner sight and listeners. Any error retains that exact completed prefix.
pub fn first<C: Effects>(
    state: &mut LoadedState,
    context: &mut C,
    plot: usize,
    definitions: &Definitions,
    owner: i32,
) -> Result<(), Error<<C as OwnerContext>::Error>> {
    if state.binding() != &definitions.binding {
        return Err(Error::Unsupported("Farm placement binding"));
    }
    let current = state
        .world()
        .map()
        .plots
        .get(plot)
        .ok_or(Error::Missing("Farm plot"))?;
    if current.improvement_hash != u32::MAX || current.improvement_owner != -1 {
        return Err(Error::Unsupported(
            "Farm first placement requires unimproved plot",
        ));
    }
    let area = current.area_id;
    {
        let world = state.world_mut().map_err(Error::State)?;
        let p = world.plot_mut(plot).ok_or(Error::Missing("Farm plot"))?;
        p.improvement_hash = definitions.farm_hash;
        p.flags.set(PlotFlag::ImprovementPillaged, false);
        if let Some(properties) = &mut p.properties {
            properties.entries.retain(|e| e.key != 0xf9fa9bc3);
        }
        if let Some(area) = world.area_mut(area) {
            if let Some(count) = area
                .improvement_counts
                .iter_mut()
                .find(|c| c.hash == definitions.farm_hash)
            {
                count.value = count.value.wrapping_add(1);
            } else {
                area.improvement_counts.push(DefinitionValue {
                    hash: definitions.farm_hash,
                    value: 1,
                });
            }
        }
    }
    shared::set_improvement_owner(state, context, plot, owner).map_err(shared_error)?;
    context
        .constructed(state, plot, definitions)
        .map_err(|error| Error::Effect {
            stage: "Farm constructed subscribers",
            error,
        })?;
    shared::restore_visual_state(state, context, plot, definitions.farm_index)
        .map_err(shared_error)?;
    context
        .calculate_monopolies(state)
        .map_err(|error| Error::Effect {
            stage: "Farm resource monopolies",
            error,
        })
}
