//! Stock production-completion subscribers, over the current canonical state.
use super::{actor, Definitions, Error};
use c6_core::{
    loaded_state::LoadedState,
    movement_world::DefinitionLookup,
    player_ai_targets::{PlotCoord, UnitInfo},
    unit::ComponentId,
};

/// Achievements.OnUnitCreatedInCity28fa780 exits on military formation !=1.
/// Its earlier integrity/configuration/leader getters are read-only, with no
/// RNG or gameplay callbacks. The formation1 unlock path remains unsupported.
pub fn stock_created_in_city<E>(
    state: &LoadedState,
    definitions: &Definitions,
    unit: (i32, u32),
    _city: ComponentId,
) -> Result<(), Error<E>> {
    if definitions.identities != state.binding().inputs {
        return Err(Error::Missing("production definitions binding"));
    }
    if actor(state, unit.0, unit.1)?.e_military_formation != 1 {
        Ok(())
    } else {
        Err(Error::Missing("stock production achievement unlock branch"))
    }
}

/// Targets.UnitCreated21ab510 retains a real observation, including civilians.
/// Absent ArmyKnowledge serialization means the native outer vector is empty.
pub fn player_unit_created<E>(
    state: &mut LoadedState,
    definitions: &Definitions,
    observer: i32,
    unit: (i32, u32),
) -> Result<(), Error<E>> {
    if definitions.identities != state.binding().inputs {
        return Err(Error::Missing("production definitions binding"));
    }
    let current = actor(state, unit.0, unit.1)?;
    let location = current.k_location;
    // PlotCoord.IsValid1fe2340: both signed coordinates are greater than -9999.
    if location.x < -9998 || location.y <= -9999 {
        return Ok(());
    }
    let owner = usize::try_from(current.e_owner)
        .ok()
        .filter(|&v| v < 64)
        .ok_or(Error::Missing("knowledge Unit owner"))?;
    let hash = current.e_type_index;
    let index = definitions
        .units
        .find_index(hash)
        .ok_or(Error::Missing("knowledge Unit definition"))?;
    let targets = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == observer)
        .and_then(|p| p.ai_targets.as_ref())
        .ok_or(Error::Missing("current Targets knowledge"))?;
    let army = &targets.knowledge_base.player_armies.0[owner].army_size;
    let Some(buckets) = army.value.as_ref() else {
        if army.present.0 == 0 {
            return Ok(());
        }
        return Err(Error::Missing("knowledge vector presence"));
    };
    if army.present.0 == 0 {
        return Err(Error::Missing("knowledge vector presence"));
    }
    // Saved hash remapping must represent the complete native type vector.
    if buckets.len() != definitions.units.count()
        || (0..definitions.units.count()).any(|i| {
            buckets
                .iter()
                .filter(|b| Some(b.hash) == definitions.units.type_hash(i))
                .count()
                != 1
        })
    {
        return Err(Error::Missing("complete native Unit knowledge buckets"));
    }
    if index >= buckets.len() {
        return Ok(());
    }
    let map = state.world().map();
    let plot = map
        .plot_at(location.x, location.y)
        .ok_or(Error::Missing("knowledge current Plot"))?;
    let area = map
        .areas
        .entries
        .iter()
        .find(|e| e.index == (plot.area_id as u32 & 0xffff) as i32)
        .ok_or(Error::Missing("knowledge current Area"))?
        .value
        .id as u32;
    let region = map
        .regions
        .entries
        .iter()
        .find(|e| e.index == (plot.region_id & 0xffff) as i32)
        .map_or(u32::MAX, |e| e.value.id);
    let record = UnitInfo {
        unit_id: current.id,
        known_damage: current.i_damage,
        military_formation: current.e_military_formation,
        location: PlotCoord {
            x: location.x,
            y: location.y,
        },
        area_id: area,
        region_id: region,
        turns_since_observation: 0,
    };
    let targets = state
        .combat_ai_targets_mut(observer)
        .map_err(|_| Error::Missing("mutable current Targets knowledge"))?;
    let buckets = targets.knowledge_base.player_armies.0[owner]
        .army_size
        .value
        .as_mut()
        .expect("validated vector");
    let bucket = &mut buckets
        .iter_mut()
        .find(|b| b.hash == hash)
        .expect("validated type")
        .value;
    c6_core::unit_creation::production_effects::observe_unit(bucket, record);
    Ok(())
}
