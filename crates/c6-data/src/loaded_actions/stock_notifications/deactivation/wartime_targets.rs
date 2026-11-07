//! Human new-war query callbacks on canonical proposal/modifier owners.
//! Native planner scores and choices remain external; forecast proposal
//! registration, requirement effects, destruction, IDs and RNG stay in scope.
use super::super::{human_activation, unit_creation::ModifierOwners, Runtime};
use crate::melee_attack::{backend, proposals};
use c6_core::{
    core_player::initialization::WireBinding,
    game_update::Error,
    loaded_state::{ActionStamp, LoadedState},
    map::UnitReference,
    unit::{ComponentId, PlotCoord},
};
use std::cell::RefCell;
fn query_error(error: impl std::fmt::Debug) -> String {
    format!("wartime planning query: {error:?}")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Plan {
    pub unit: u32,
    pub search_turns: i32,
    pub delayed_death: bool,
}
/// Source-ordered numerical forecast requests, not chosen actions.
pub struct Forecast {
    pub attacker: UnitReference,
    pub defender: ComponentId,
    pub attack_location: PlotCoord,
    pub retaliation_locations: Vec<PlotCoord>,
    pub district_can_retaliate: bool,
}
/// The stock formation planning BFS supplies these requests from the SAME
/// current world, placement, sight and movement definitions. Unknown target or
/// path branches must return errors, never captured/native forecast answers.
pub trait Queries {
    fn forecasts(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        plan: Plan,
    ) -> Result<Vec<Forecast>, String>;
}
pub struct Profile<'a> {
    binding: WireBinding,
    origin: ActionStamp,
    owner: i32,
    plans: Vec<Plan>,
    used: bool,
    requirements: &'a RefCell<crate::game_update_state::activation::requirement_changes::Runtime>,
    queries: Box<dyn Queries + 'a>,
    inputs: Option<backend::Inputs>,
}
impl<'a> Profile<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn from_initial(
        state: &LoadedState,
        rules: &crate::Rules,
        owner: i32,
        plans: Vec<Plan>,
        requirements: &'a RefCell<
            crate::game_update_state::activation::requirement_changes::Runtime,
        >,
        queries: Box<dyn Queries + 'a>,
        inputs: backend::Inputs,
        _: human_activation::Scope,
    ) -> Result<Self, Error> {
        if !state.is_pristine() || state.player_slot_status(owner).ok() != Some(3) {
            return Err(Error::Unsupported("human wartime initial owner"));
        }
        if rules
            .gameplay()
            .table("UnitRetreats_XP1")
            .ok_or(Error::Unsupported("human wartime retreat rules"))?
            .rows()
            .next()
            .is_some()
        {
            return Err(Error::Unsupported(
                "human wartime forecast retreat continuation",
            ));
        }
        for (index, plan) in plans.iter().enumerate() {
            if plans[..index].iter().any(|old| old.unit == plan.unit) || plan.search_turns != 2 {
                return Err(Error::Unsupported("human wartime planner shape"));
            }
            let unit = state
                .world()
                .player(owner)
                .map_err(|_| Error::Unsupported("human wartime Player"))?
                .unit(plan.unit)
                .map_err(|_| Error::Unsupported("human wartime Unit"))?
                .ok_or(Error::Unsupported("human wartime initial Unit missing"))?
                .record;
            if unit.b_delayed_death != plan.delayed_death {
                return Err(Error::Unsupported("human wartime initial death state"));
            }
            if !plan.delayed_death
                && unit.e_type_index != crate::registry::linux_type_hash(b"UNIT_WARRIOR")
            {
                return Err(Error::Unsupported("human wartime live planner type"));
            }
        }
        Ok(Self {
            binding: state.binding().clone(),
            origin: state.action_stamp(),
            owner,
            plans,
            used: false,
            requirements,
            queries,
            inputs: Some(inputs),
        })
    }
    fn run<P>(
        &mut self,
        runtime: &mut Runtime<'_, P>,
        state: &mut LoadedState,
    ) -> Result<(), Error> {
        if self.used {
            return Err(Error::Unsupported("human wartime profile already consumed"));
        }
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == self.owner)
            .ok_or(Error::Unsupported("human wartime current Player"))?;
        if !player
            .ai_military
            .as_ref()
            .ok_or(Error::Unsupported("human wartime Military"))?
            .active_operations
            .entries
            .is_empty()
            || !player
                .ai_behavior_tree
                .as_ref()
                .ok_or(Error::Unsupported("human wartime behavior trees"))?
                .behavior_trees
                .entries
                .is_empty()
        {
            return Err(Error::Unsupported(
                "human wartime active military/tree continuation",
            ));
        }
        let mut forecasts = Vec::new();
        for &plan in &self.plans {
            if plan.delayed_death {
                let unit = state
                    .world()
                    .player(self.owner)
                    .map_err(|_| Error::Unsupported("human wartime units"))?
                    .unit(plan.unit)
                    .map_err(|_| Error::Unsupported("human wartime dead planner"))?;
                if unit.is_some_and(|u| !u.record.b_delayed_death) {
                    return Err(Error::Unsupported("human wartime revived planner"));
                }
                continue;
            }
            forecasts.extend(
                self.queries
                    .forecasts(state, self.owner, plan)
                    .map_err(|error| {
                        runtime.last_error = Some(error);
                        Error::Unsupported(
                            "human wartime forecast query; inspect backend.last_error",
                        )
                    })?,
            );
        }
        let inputs = self
            .inputs
            .take()
            .ok_or(Error::Unsupported("human wartime callback inputs borrowed"))?;
        let mut shared = self.requirements.borrow_mut();
        let mut owned_inputs = Some(inputs);
        let mut detail = None;
        let result=runtime.with_modifier_requirement_sets(shared.sets_mut(),|runtime| {
            runtime.with_owned_modifier_owners(|runtime,owners| {
                let ModifierOwners{effects,requirements,collections,unit_created}=owners;
                let combat=backend::Backend::resume_owned(state,self.origin,crate::melee_attack::transition::Reports(runtime),requirements,effects,owned_inputs.take().expect("single loan"));
                let (mut combat,mut result)=match combat {
                    Ok(combat)=>(combat,Ok(())),
                    Err(combat)=>(combat,Err(Error::Unsupported("human wartime combat World"))),
                };
                if result.is_ok() {
                    result=(|| {
                        for forecast in forecasts {
                            let (id,_)=proposals::Context::new(state,&mut combat)
                                .propose_unit_district(forecast.attacker,forecast.defender,forecast.attack_location)
                                .map_err(|error|{detail=Some(format!("{error:?}"));Error::Unsupported("human wartime attack proposal callback")})?;
                            proposals::collection_modifiers::forget_combat(state,&mut combat,id)
                                .map_err(|error|{detail=Some(format!("{error:?}"));Error::Unsupported("human wartime attack proposal destruction")})?;
                            if forecast.district_can_retaliate {
                                for at in forecast.retaliation_locations {
                                    let (id,_)=proposals::Context::new(state,&mut combat)
                                        .propose_district_unit(forecast.defender,forecast.attacker,at)
                                        .map_err(|error|{detail=Some(format!("{error:?}"));Error::Unsupported("human wartime retaliation proposal callback")})?;
                                    proposals::collection_modifiers::forget_combat(state,&mut combat,id)
                                        .map_err(|error|{detail=Some(format!("{error:?}"));Error::Unsupported("human wartime retaliation proposal destruction")})?;
                                }
                            }
                        }
                        Ok(())
                    })();
                }
                let (_,requirements,effects,inputs)=combat.into_parts();
                owned_inputs=Some(inputs);
                (ModifierOwners{effects,requirements,collections,unit_created},result)
            }).map_err(|_|Error::Unsupported("human wartime canonical modifier loan"))?
        }).map_err(|_|Error::Unsupported("human wartime canonical requirement loan"));
        self.inputs = owned_inputs;
        if let Some(detail) = detail {
            runtime.last_error = Some(detail);
        }
        result??;
        self.used = true;
        runtime
            .reached
            .push("stock human wartime forecast proposal callbacks");
        Ok(())
    }
}
impl<'a, P> Runtime<'a, P> {
    pub fn bind_human_wartime(
        &mut self,
        state: &LoadedState,
        profile: Profile<'a>,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("human wartime backend binding"))?;
        if profile.binding != *state.binding()
            || self.human_wartime.iter().any(|p| p.owner == profile.owner)
        {
            return Err(Error::Unsupported("human wartime profile binding"));
        }
        self.human_wartime.push(profile);
        Ok(())
    }
    pub(super) fn continue_human_wartime(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Unsupported("human wartime current binding"))?;
        let at = self
            .human_wartime
            .iter()
            .position(|p| p.owner == owner)
            .ok_or(Error::Unsupported("current human wartime planner inputs"))?;
        let mut profile = self.human_wartime.remove(at);
        let result = profile.run(self, state);
        self.human_wartime.insert(at, profile);
        result
    }
}

/// Shared current services for the bounded Warrior/one enemy combat District
/// forecast query. These borrow the ordinary action/turn providers.
pub struct CurrentQueries<'a, 'r> {
    pub binding: WireBinding,
    pub path: &'a crate::formation_path_state::Definitions<'a, 'r>,
    pub visibility: &'a RefCell<crate::unit_sight_state::Runtime>,
    pub levels: &'a [(i32, u32)],
    pub city_domains: Option<&'a [Option<c6_core::city_district::DomainMask>]>,
    pub combat: &'a crate::loaded_actions::city_range_attack::Definitions,
    pub capabilities: Option<&'a c6_core::game_capabilities::Membership>,
}
impl Queries for CurrentQueries<'_, '_> {
    fn forecasts(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        plan: Plan,
    ) -> Result<Vec<Forecast>, String> {
        if self.binding != *state.binding() {
            return Err("wartime planning query binding".into());
        }
        let formation = state
            .unit_runtime(owner, plan.unit)
            .map_err(query_error)?
            .formation
            .ok_or("wartime planning Unit formation")?;
        let unit = state
            .world()
            .player(owner)
            .map_err(query_error)?
            .unit(plan.unit)
            .map_err(query_error)?
            .ok_or("wartime planning Unit missing")?
            .record;
        let origin = unit.k_location;
        let map = state.world().map();
        let near = |at: (i32, i32)| {
            c6_core::geometry::plot_distance(
                map.header.width,
                map.header.wrap_x,
                origin.x,
                origin.y,
                at.0,
                at.1,
            ) <= 4
        };
        let targets = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .and_then(|p| p.ai_targets.as_ref())
            .ok_or("wartime planning current Targets")?;
        let desired = targets
            .desired_targets
            .iter()
            .find(|r| r.hash == crate::registry::linux_type_hash(b"TARGET_ENEMY_COMBAT_DISTRICT"))
            .ok_or("wartime planning desired combat District type")?
            .value
            .0;
        let sight = self.visibility.borrow();
        let mut districts = Vec::new();
        for other in &state.retained().players {
            let enemy = other.header.owner;
            if enemy == owner
                || !state
                    .game_relations()
                    .map_err(query_error)?
                    .is_at_war(owner, enemy)
                    .map_err(query_error)?
            {
                continue;
            }
            for unit in state
                .world()
                .player(enemy)
                .map_err(query_error)?
                .unit_records()
                .map_err(query_error)?
            {
                if !unit.value.b_delayed_death
                    && near((unit.value.k_location.x, unit.value.k_location.y))
                {
                    return Err("wartime planning nearby enemy Unit continuation".into());
                }
            }
            if let Some(army) = usize::try_from(enemy)
                .ok()
                .and_then(|i| targets.knowledge_base.player_armies.0.get(i))
            {
                if army.army_size.value.as_ref().is_some_and(|groups| {
                    groups.iter().any(|group| {
                        group
                            .value
                            .iter()
                            .any(|unit| near((unit.location.x, unit.location.y)))
                    })
                }) {
                    return Err("wartime planning nearby unseen Unit continuation".into());
                }
            }
            if desired == 0 {
                continue;
            }
            for row in &other.districts.records.entries {
                let district = &row.value;
                if !near(district.location) {
                    continue;
                }
                let x = usize::try_from(district.location.0).map_err(|_| "wartime District x")?;
                let y = usize::try_from(district.location.1).map_err(|_| "wartime District y")?;
                let index = y * map.header.width as usize + x;
                let revealed = *crate::formation_path_state::mapped_visibility(state, owner)
                    .map_err(query_error)?
                    .revealed
                    .get(index)
                    .ok_or("wartime District reveal extent")?;
                if revealed != 1
                    && !crate::formation_path_state::shared_plot_visible(&sight, owner, index)
                        .map_err(query_error)?
                {
                    continue;
                }
                let retaliation =
                    crate::loaded_actions::city_range_attack::attack_strength_current_q8(
                        state,
                        district,
                        self.combat,
                        self.levels,
                    )
                    .map_err(query_error)?
                        != 0;
                districts.push((enemy, district.id, retaliation));
            }
        }
        if districts.len() > 1 {
            return Err("wartime planning multiple nearby combat Districts".into());
        }
        let mut result = Vec::new();
        for (enemy, id, retaliation) in districts {
            let stamp = state.action_stamp();
            let runtime = crate::formation_path_state::Runtime {
                visibility: &sight,
                civilization_levels: Some(
                    crate::movement_placement_state::CurrentCivilizationLevels {
                        binding: &self.binding,
                        values: self.levels,
                    },
                ),
                city_domains: self.city_domains.map(|masks| {
                    crate::formation_path_state::CurrentCityDomains {
                        binding: &self.binding,
                        stamp,
                        masks,
                    }
                }),
                execution: crate::formation_path_state::ExecutionContext::Exclusive,
                is_evaluating_operation: false,
            };
            let mut context = crate::formation_path_state::Context::new(
                state,
                self.path,
                runtime,
                (formation.owner, formation.resolved_id),
            )
            .map_err(query_error)?;
            if let Some(capabilities) = self.capabilities {
                context = context.with_capabilities(capabilities);
            }
            if let Some(forecast) = crate::formation_path_state::planning::district_forecast(
                context,
                plan.unit,
                (enemy, id),
                plan.search_turns,
            )
            .map_err(query_error)?
            {
                result.push(Forecast {
                    attacker: UnitReference::from_component_id(ComponentId {
                        owner: owner as i16,
                        component_type: 1,
                        id: plan.unit,
                    }),
                    defender: ComponentId {
                        owner: enemy as i16,
                        component_type: 3,
                        id,
                    },
                    attack_location: forecast.attack_location,
                    retaliation_locations: forecast.retaliation_locations,
                    district_can_retaliate: retaliation,
                });
            }
        }
        Ok(result)
    }
}
