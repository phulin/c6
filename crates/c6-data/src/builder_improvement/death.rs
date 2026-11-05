//! Last-charge Builder removal over the existing Unit lifecycle and SAME owners.
use super::stock::Error;
use crate::{influence_map_update as influence, loaded_actions::{city_range_attack,stock_notifications::Runtime},
    unit_kill,unit_sight_state as sight};
use c6_core::{loaded_state::LoadedState,movement_world::DefinitionTable,skip_turn::placement::UnitProperties};
/// Immutable/current service joins retained by the caller across actions.
pub struct Inputs<'a,'r> {
    pub metadata:&'a unit_kill::stock::Metadata,
    pub unit_definitions:&'a crate::unit_initialization_definitions::Definitions,
    pub units:&'a DefinitionTable<'r,Option<UnitProperties>>,
    pub sight_definitions:&'a city_range_attack::Definitions,
    pub influence:&'a influence::Definitions,
    pub capabilities:&'a c6_core::game_capabilities::Membership,
    pub movement:&'a crate::formation_movement_cost::Definitions<'r>,
    pub formation:&'a crate::formation_rules::Definitions,
    pub cities:&'a city_range_attack::Queries<'a>,
}
pub fn delayed_destroy<P>(
    state:&mut LoadedState,sight:&mut sight::Runtime,backend:&mut Runtime<'_,P>,
    input:&Inputs<'_,'_>,actor:(i32,u32),
)->Result<(),Error>{
    backend.with_modifier_owners(|backend,owners|{
        let mut influence=|state:&mut LoadedState,visibility:&sight::Runtime,
            services:&crate::unit_placement_services::Runtime,owner:i32| {
            let alive=services.alive_players.as_ref()
                .ok_or(unit_kill::stock::Error::Input("current Influence alive order"))?;
            influence::update(state,input.influence,&influence::Queries {
                alive_players:alive,capabilities:Some(input.capabilities),visibility,
                movement:input.movement,formation:input.formation,city_range:input.cities,
                path:None,path_runtime:None,strength:None,
            },owner).map_err(|e|unit_kill::stock::Error::Influence(format!("{e:?}")))
        };
        let mut stock=unit_kill::stock::Stock::new(&mut owners.effects,&mut owners.requirements,
            input.unit_definitions,input.metadata)
            .map_err(|e|Error("UnitRemoved bindings",format!("{e:?}")))?
            .with_collections(&mut owners.collections)
            .with_unit_definitions(input.units)
            .with_influence(&mut influence);
        unit_kill::delayed_destroy_with_continuations(state,sight,backend,&mut stock,
            input.sight_definitions,actor)
            .map_err(|e|Error("DelayedDestroy",format!("{e:?}")))
    }).map_err(|e|Error("canonical lifecycle owners",format!("{e:?}")))?
}
