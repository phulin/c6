//! Ordinary Linux XP2 improvement Pillage operation, over the current loaded world.
//!
//! This operation body requires its reached synchronous effects. It does not
//! accept missing subscribers, district pillaging, or incomplete outer queue
//! processing as a completed action.
use crate::{game_parameters::Parameters, Rules};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    map::{PlotFlag, UnitReference},
    skip_turn::StartData,
    unit::{ComponentId, OperationNode, Unit},
};

pub mod current;
pub mod notification;
pub mod plunder;

pub mod stock;

pub const OPERATION_HASH: u32 = 0x4885d724;
#[derive(Debug)]
pub enum Error<E> {
    Binding,
    Missing(&'static str),
    Unsupported(&'static str),
    State(c6_core::loaded_state::Error),
    Query(E),
    /// Execution has begun; the caller must retain and expose this prefix.
    Effect {
        stage: &'static str,
        error: E,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Report {
    ObserverPillage { x: i32, y: i32 },
    Operation { unit: ComponentId, improvement: i32 },
    VisibleAnimation(ComponentId),
    ImprovementChanged { plot: usize },
}
pub struct Definitions {
    binding: WireBinding,
    movement_cost: i32,
    advanced_movement_cost: i32,
    improvements: crate::registry::LinuxDefinitionRegistry,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error<()>> {
        let p = Parameters::from_rules(rules)
            .map_err(|_| Error::Missing("Pillage Linux parameters"))?;
        Ok(Self {
            binding: state.binding().clone(),
            movement_cost: p
                .required_integer("PILLAGE_MOVEMENT_COST")
                .map_err(|_| Error::Missing("PILLAGE_MOVEMENT_COST"))?,
            advanced_movement_cost: p
                .required_integer("PILLAGE_ADVANCED_MOVEMENT_COST")
                .map_err(|_| Error::Missing("PILLAGE_ADVANCED_MOVEMENT_COST"))?,
            improvements: crate::unit_initialization_definitions::collection(
                rules,
                "Improvements",
                "ImprovementType",
            )
            .map_err(|_| Error::Missing("Pillage improvement definitions"))?,
        })
    }
    fn cost(&self, u: &Unit) -> i32 {
        if u.k_combat_modifers.b_advanced_pillaging {
            self.advanced_movement_cost
        } else {
            self.movement_cost
        }
    }
}
/// Source gameplay dependencies; implementations borrow persistent owners and
/// perform the actual callbacks in native registration order.
pub trait Queries {
    type Error;
    fn is_combat(&self, state: &LoadedState, unit: &Unit) -> Result<bool, Self::Error>;
    fn at_war(&self, state: &LoadedState, owner: i32, other: i32) -> Result<bool, Self::Error>;
    fn max_moves(&self, state: &LoadedState, unit: &Unit) -> Result<i32, Self::Error>;
    /// XP2 barbarian bribe/clans guard, queried only for a barbarian actor.
    fn is_barbarian(&self, state: &LoadedState, unit: &Unit) -> Result<bool, Self::Error>;
    fn barbarian_permitted(
        &self,
        state: &LoadedState,
        unit: &Unit,
        plot_owner: i32,
    ) -> Result<bool, Self::Error>;
    fn clans_active(&self) -> Result<bool, Self::Error>;
    fn barbarian_camp_improvement(&self, state: &LoadedState) -> Result<u32, Self::Error>;
}
/// Synchronous execution effects, evaluated against the same canonical owners.
pub trait Effects: Queries {
    fn observer_can_see_owner(&self, state: &LoadedState, owner: i32) -> Result<bool, Self::Error>;
    fn local_unit_visible(
        &self,
        state: &LoadedState,
        actor: UnitReference,
    ) -> Result<bool, Self::Error>;
    fn report(&mut self, state: &mut LoadedState, report: Report) -> Result<(), Self::Error>;
    fn improvement_pillaged(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        improvement: u32,
    ) -> Result<(), Self::Error>;
    fn improvement_notification(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        actor_owner: i32,
    ) -> Result<(), Self::Error>;
    fn visibility_messages_disabled(&self) -> bool;
    fn update_appeal(&mut self, state: &mut LoadedState, plot: usize) -> Result<(), Self::Error>;
    fn on_pillage(
        &mut self,
        state: &mut LoadedState,
        actor: ComponentId,
        plot: usize,
        improvement: u32,
    ) -> Result<(), Self::Error>;
    fn post_pillage_effects(
        &mut self,
        state: &mut LoadedState,
        actor: UnitReference,
        plot: usize,
    ) -> Result<(), Self::Error>;
    fn change_moves(
        &mut self,
        state: &mut LoadedState,
        actor: UnitReference,
        delta_q8: i32,
    ) -> Result<(), Self::Error>;
    fn award_plunder(
        &mut self,
        state: &mut LoadedState,
        actor: UnitReference,
        plot: usize,
    ) -> Result<(), Self::Error>;
}
fn check<E>(state: &LoadedState, defs: &Definitions) -> Result<(), Error<E>> {
    if state.binding() != &defs.binding {
        Err(Error::Binding)
    } else {
        Ok(())
    }
}
/// XP2 CanStart without optional presentation tooltip construction. District
/// plots remain explicit unsupported inputs; improvement legality does not use
/// the request's optional improvement hash.
pub fn can_start<E: Queries>(
    state: &LoadedState,
    defs: &Definitions,
    unit: &Unit,
    visible_only: bool,
    effects: &E,
) -> Result<bool, Error<E::Error>> {
    check(state, defs)?;
    if !effects.is_combat(state, unit).map_err(Error::Query)?
        || (crate::unit_movement_state::unit_is_embarked(unit)
            && !unit.k_combat_modifers.b_fight_while_embarked)
        || state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?
            .domain
            == 1
    {
        return Ok(false);
    }
    let (_, plot) = c6_core::movement_world::plot_at(state.world().map(), unit.k_location)
        .map_err(|_| Error::Missing("Pillage plot"))?;
    if plot.flags.get(PlotFlag::Impassable) {
        return Ok(false);
    }
    let owner = crate::movement_placement_state::plot_owner(plot)
        .map_err(|_| Error::Missing("Pillage political owner"))?;
    if owner != -1 {
        if !effects
            .at_war(state, unit.e_owner, owner)
            .map_err(Error::Query)?
        {
            return Ok(false);
        }
        if effects.is_barbarian(state, unit).map_err(Error::Query)?
            && !effects
                .barbarian_permitted(state, unit, owner)
                .map_err(Error::Query)?
        {
            return Ok(false);
        }
    }
    if !c6_core::skip_turn::legality::can_move(unit) {
        return Ok(false);
    }
    if plot.improvement_hash == u32::MAX {
        return if plot
            .city_attributes
            .as_ref()
            .is_some_and(|a| a.district != u32::MAX)
        {
            Err(Error::Unsupported("Pillage district/building path"))
        } else {
            Ok(false)
        };
    }
    if plot.flags.get(PlotFlag::ImprovementPillaged) {
        return Ok(false);
    }
    if effects.clans_active().map_err(Error::Query)?
        && plot.improvement_hash
            == effects
                .barbarian_camp_improvement(state)
                .map_err(Error::Query)?
    {
        return Ok(false);
    }
    if visible_only {
        return Ok(true);
    }
    let max = effects
        .max_moves(state, unit)
        .map_err(Error::Query)?
        .wrapping_shl(8);
    Ok(unit.x_moves_remaining >= max || unit.x_moves_remaining >= defs.cost(unit).wrapping_shl(8))
}
/// Inherited Pillage.Start for an improvement. StartData.action becomes true
/// only after every reached gameplay continuation has completed.
pub fn start<E: Effects>(
    state: &mut LoadedState,
    defs: &Definitions,
    actor: UnitReference,
    node: &OperationNode,
    effects: &mut E,
) -> Result<StartData, Error<E::Error>> {
    check(state, defs)?;
    if node.operation_hash != OPERATION_HASH {
        return Err(Error::Unsupported("Pillage operation identity"));
    }
    let unit = state
        .world()
        .unit(actor)
        .map_err(|_| Error::Missing("Pillage actor"))?
        .ok_or(Error::Missing("Pillage actor"))?
        .record;
    let location = unit.k_location;
    let identity = ComponentId {
        owner: actor.player() as i16,
        component_type: 1,
        id: actor.id,
    };
    let (plot, record) = c6_core::movement_world::plot_at(state.world().map(), location)
        .map_err(|_| Error::Missing("Pillage actor plot"))?;
    let improvement = record.improvement_hash;
    if improvement == u32::MAX {
        return Err(Error::Unsupported("Pillage district/building Start"));
    }
    let index = defs
        .improvements
        .find_index(improvement)
        .ok_or(Error::Missing("Pillage improvement definition"))? as i32;
    macro_rules! effect {
        ($stage:literal,$call:expr) => {
            $call.map_err(|error| Error::Effect {
                stage: $stage,
                error,
            })?
        };
    }
    if effects
        .observer_can_see_owner(state, actor.player())
        .map_err(Error::Query)?
    {
        effect!(
            "observer report",
            effects.report(
                state,
                Report::ObserverPillage {
                    x: location.x,
                    y: location.y
                }
            )
        );
    }
    effect!(
        "operation report",
        effects.report(
            state,
            Report::Operation {
                unit: identity,
                improvement: index
            }
        )
    );
    let stats = state
        .player_stats_mut(actor.player())
        .map_err(Error::State)?;
    stats.tiles_pillaged = stats.tiles_pillaged.wrapping_add(1);
    let record = &state.world().map().plots[plot];
    if !record.flags.get(PlotFlag::ImprovementPillaged) && !record.flags.get(PlotFlag::Impassable) {
        state
            .world_mut()
            .map_err(Error::State)?
            .plot_mut(plot)
            .ok_or(Error::Missing("Pillage current plot"))?
            .flags
            .set(PlotFlag::ImprovementPillaged, true);
        effect!(
            "ImprovementPillaged",
            effects.improvement_pillaged(state, plot, improvement)
        );
        let city = state.world().map().plots[plot]
            .city_attributes
            .as_ref()
            .filter(|a| a.player != -1 && a.owning_city != u32::MAX)
            .map(|a| (i32::from(a.player), a.owning_city));
        let has_city = match city {
            None => false,
            Some((owner, id)) => match state.city(owner, id) {
                Ok(_) => true,
                Err(c6_core::loaded_state::Error::Missing(
                    c6_core::loaded_state::MissingInput::City { .. },
                )) => false,
                Err(error) => return Err(Error::State(error)),
            },
        };
        if actor.player() != -1 && has_city {
            effect!(
                "improvement notification",
                effects.improvement_notification(state, plot, actor.player())
            );
        }
        if !effects.visibility_messages_disabled() {
            effect!(
                "improvement report",
                effects.report(state, Report::ImprovementChanged { plot })
            );
            effect!("appeal update", effects.update_appeal(state, plot));
        }
    }
    if effect!("local visibility", effects.local_unit_visible(state, actor)) {
        effect!(
            "animation report",
            effects.report(state, Report::VisibleAnimation(identity))
        );
    }
    effect!(
        "OnPillage",
        effects.on_pillage(state, identity, plot, improvement)
    );
    effect!(
        "post-pillage effects",
        effects.post_pillage_effects(state, actor, plot)
    );
    let current = state
        .world()
        .unit(actor)
        .map_err(|_| Error::Missing("Pillage current actor"))?
        .ok_or(Error::Missing("Pillage current actor"))?
        .record;
    let cost = defs.cost(current).wrapping_neg().wrapping_shl(8);
    effect!("movement cost", effects.change_moves(state, actor, cost));
    effect!("plunder", effects.award_plunder(state, actor, plot));
    Ok(StartData {
        delete: true,
        action: true,
    })
}
