//! Ordinary SetImprovementType first placement, without Barbarian manager hooks.
use super::{Definitions, Error};
use crate::barbarian_camp_placement::{self as shared, OwnerContext, VisualContext};
use c6_core::{
    loaded_state::LoadedState,
    map::{DefinitionValue, PlotFlag},
    unit::variants::{ArrayVariant, TypedEntry, TypedVariant, TypedVariantMap, VariantPayload},
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
        // SetDirection(NO_DIRECTION,true) calls EditProperties even when the
        // direction key is absent (Linux2a3e85f). That creates the nested
        // Improvement typed map before erasing its direction property.
        let properties = p.properties.get_or_insert_with(TypedVariantMap::default);
        if !properties.entries.iter().any(|e| e.key == 0xd9a34c75) {
            properties.entries.push(TypedEntry { key: 0xd9a34c75, name: None,
                value: TypedVariant { type_bits: 10, modifier_bits: 0,
                    value: ArrayVariant { count_flags: 0x05000000, element_size: 0,
                        payload: VariantPayload::TypedVariantMap(Box::default()) } } });
        }
        let entry = properties.entries.iter_mut().find(|e|e.key==0xd9a34c75)
            .ok_or(Error::Missing("Farm improvement property map"))?;
        if entry.value.type_bits != 10 || entry.value.modifier_bits != 0 {
            return Err(Error::Unsupported("Farm non-map improvement properties"));
        }
        let VariantPayload::TypedVariantMap(improvement) = &mut entry.value.value.payload else {
            return Err(Error::Unsupported("Farm improvement property payload"));
        };
        improvement.entries.retain(|e|e.key!=0xf9fa9bc3);
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
