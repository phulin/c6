//! Stock fresh-Unit callbacks on the ongoing Script and modifier owners.
use super::{movement, Runtime};
use crate::{
    unit_creation as creation,
    unit_creation::{collections, signals, stock},
};
use c6_core::{loaded_state::LoadedState, map::UnitReference, unit::ComponentId};

/// Canonical mutable modifier owners, shared by unit and building creation.
/// Definition/registration admission data is stored separately in `State`.
pub struct ModifierOwners {
    pub effects: crate::melee_attack::backend::EffectRuntime,
    pub requirements: crate::melee_attack::backend::requirements::Runtime,
    pub collections: c6_core::melee_attack::proposals::collections::Runtime,
    pub unit_created: collections::Runtime,
}
pub struct Achievement {
    pub context_present: bool,
    pub integrity: bool,
    pub editor: bool,
    pub ruleset: String,
    pub civilizations: Vec<(i32, String)>,
}
pub struct PlacementInputs<'a> {
    pub definitions: &'a crate::unit_creation::placement_effects::Definitions,
    pub properties: &'a [(i32, Vec<(u32, i32)>)],
}
pub struct ControlInputs<'a> {
    pub movement: &'a crate::unit_movement_state::Definitions,
    /// Complete observed d70ba51f native order: either empty or the sole
    /// OnUnitUpdate27ff430 receiver, using the existing snapshot provider.
    pub processed_report_snapshot: bool,
}
pub struct State<'a> {
    pub definitions: &'a creation::Definitions,
    pub signals: &'a signals::Definitions,
    pub combat: &'a crate::formation_rules::Definitions,
    pub build_maps: &'a signals::BuildMaps,
    pub tags: &'a crate::loaded_actions::reporting::hold_cycling::commands::Tags,
    pub levels: &'a [(i32, u32)],
    pub scratch: Vec<(i32, bool)>,
    pub quests: Vec<crate::city_turn::religion_callbacks::CurrentQuest>,
    pub game_consumers: Vec<stock::Consumer>,
    pub player_consumers: Vec<(i32, Vec<stock::PlayerConsumer>)>,
    pub achievement: Achievement,
    /// Qualified actual 23e4180 AttachGameEffects subscriber. Additional
    /// subscribers must be represented explicitly before this flag is admitted.
    pub ability_attach_game_effects: bool,
    pub city_initialized_script: bool,
    pub placement: Option<PlacementInputs<'a>>,
    pub control: Option<ControlInputs<'a>>,
}
impl<'a, P> Runtime<'a, P> {
    pub fn bind_modifier_owners(
        &mut self,
        state: &LoadedState,
        owners: ModifierOwners,
    ) -> Result<(), movement::Error> {
        self.check(state)
            .map_err(|_| movement::Error::Missing("modifier owner binding"))?;
        if self.modifier_owners.is_some() {
            return Err(movement::Error::Missing("modifier owners already bound"));
        }
        self.modifier_owners = Some(owners);
        Ok(())
    }
    pub fn modifier_owners(&self) -> Result<&ModifierOwners, movement::Error> {
        self.modifier_owners.as_ref().ok_or(movement::Error::Missing("canonical modifier owners"))
    }
    pub fn modifier_owners_mut(&mut self) -> Result<&mut ModifierOwners, movement::Error> {
        self.modifier_owners
            .as_mut()
            .ok_or(movement::Error::Missing("canonical modifier owners"))
    }
    /// Loan the SAME canonical owners during a synchronous lifecycle callback.
    /// The provider remains borrowed for reports/sight; no owner is cloned or
    /// rebuilt, and errors restore the mutated owner before returning.
    pub(crate) fn with_modifier_owners<R>(
        &mut self,
        apply: impl FnOnce(&mut Self, &mut ModifierOwners) -> R,
    ) -> Result<R, movement::Error> {
        let mut owners=self.modifier_owners.take()
            .ok_or(movement::Error::Missing("canonical modifier owners"))?;
        let result=apply(self,&mut owners);
        self.modifier_owners=Some(owners);
        Ok(result)
    }
    /// Execute a synchronous factory scope against the ongoing turn's actual
    /// requirement store. Only immutable factory metadata is joined. All
    /// returned errors preserve mutations and restore the canonical store.
    pub fn with_modifier_requirement_sets<R>(
        &mut self,
        shared: &mut c6_core::melee_attack::proposals::subject_sets::Runtime,
        apply: impl FnOnce(&mut Self) -> R,
    ) -> Result<R, movement::Error> {
        let owners = self.modifier_owners_mut()?;
        shared
            .install_factory_definitions(owners.requirements.sets())
            .map_err(movement::Error::Missing)?;
        std::mem::swap(shared, owners.requirements.sets_mut());
        let result = apply(self);
        // No fallible operation lies between the callback and restoration.
        // Public accessors cannot remove the bound owner during this scope.
        let owners = self
            .modifier_owners
            .as_mut()
            .expect("bound modifier owner retained through synchronous factory");
        std::mem::swap(shared, owners.requirements.sets_mut());
        Ok(result)
    }
    pub fn bind_unit_creation(
        &mut self,
        state: &LoadedState,
        input: State<'a>,
    ) -> Result<(), movement::Error> {
        self.check(state)
            .map_err(|_| movement::Error::Missing("Unit creation binding"))?;
        if self.unit_creation.is_some() || input.build_maps.binding != *state.binding() {
            return Err(movement::Error::Missing(
                "Unit creation inputs already bound or identity mismatch",
            ));
        }
        self.unit_creation = Some(input);
        Ok(())
    }
}
struct CollectionEffects<'a, 'r> {
    definitions: &'a creation::Definitions,
    tags: &'a crate::loaded_actions::reporting::hold_cycling::commands::Tags,
    moved: &'a mut c6_core::unit_moved_signal::Registry,
    reporting: &'a mut crate::loaded_actions::reporting::Provider<'r>,
    reporting_suspended: bool,
}
impl collections::Effects for CollectionEffects<'_, '_> {
    fn can_have_ability(
        &mut self,
        owners: &mut collections::Owners<'_>,
        unit: UnitReference,
        index: usize,
    ) -> Result<bool, collections::Error> {
        self.definitions
            .can_have_ability::<std::convert::Infallible>(
                owners.state,
                (unit.player(), unit.id),
                self.tags,
                index,
            )
            .map_err(|_| collections::Error::Missing("current ability eligibility"))
    }
    fn subscribe_home_movement(
        &mut self,
        _: &mut collections::Owners<'_>,
        requirement: u64,
    ) -> Result<(), collections::Error> {
        self.moved
            .global_mut()
            .map_err(collections::Error::Ring)?
            .subscribe(c6_core::unit_moved_signal::Subscriber::Requirement {
                receiver: requirement,
            })
            .map_err(collections::Error::Ring)?;
        Ok(())
    }
    fn ability_changed(
        &mut self,
        owners: &mut collections::Owners<'_>,
        runtime: &mut collections::Runtime,
        unit: UnitReference,
        index: usize,
        added: bool,
    ) -> Result<(), collections::Error> {
        if !added {
            return Err(collections::Error::Missing("ability removal attachment"));
        }
        collections::on_ability_earned(owners, runtime, self, self.definitions, unit, index)
    }
    fn ability_report(
        &mut self,
        owners: &mut collections::Owners<'_>,
        unit: UnitReference,
        index: usize,
        added: bool,
    ) -> Result<(), collections::Error> {
        if !added {
            return Err(collections::Error::Missing("ability removal report"));
        }
        if !self.reporting_suspended {
            self.reporting
                .send_ability_gained_report(owners.state, unit, index as i32)
                .map_err(|_| collections::Error::Missing("ability gained report"))?;
        }
        Ok(())
    }
}
fn callback(error: impl std::fmt::Debug) -> creation::Error<movement::Error> {
    creation::Error::Effects(movement::Error::Requirement(format!(
        "Unit creation: {error:?}"
    )))
}
impl<P> creation::Effects for Runtime<'_, P> {
    type Error = movement::Error;
    fn ability_changed(
        &mut self,
        state: &mut LoadedState,
        unit: ComponentId,
        index: usize,
    ) -> Result<(), creation::Error<Self::Error>> {
        let input = self
            .unit_creation
            .as_ref()
            .ok_or(creation::Error::Missing("Unit creation inputs"))?;
        if !input.ability_attach_game_effects {
            return Err(creation::Error::Missing(
                "current AbilityChanged subscribers",
            ));
        }
        let modifier = self
            .modifier_owners
            .as_mut()
            .ok_or(creation::Error::Missing("canonical modifier owners"))?;
        let movement = self
            .movement
            .as_mut()
            .ok_or(creation::Error::Missing("current movement callbacks"))?;
        let mut effects = CollectionEffects {
            definitions: input.definitions,
            tags: input.tags,
            moved: &mut movement.signal,
            reporting: &mut self.reporting,
            reporting_suspended: movement.reporting_suspended,
        };
        let mut owners = collections::Owners {
            state,
            effects: &mut modifier.effects,
            requirements: &mut modifier.requirements,
            collections: &mut modifier.collections,
        };
        collections::on_ability_earned(
            &mut owners,
            &mut modifier.unit_created,
            &mut effects,
            input.definitions,
            UnitReference::from_component_id(unit),
            index,
        )
        .map_err(callback)
    }
}
impl<P> creation::PlayerCreationEffects for Runtime<'_, P> {
    fn game_unit_created(
        &mut self,
        state: &mut LoadedState,
        unit: Option<ComponentId>,
        city: Option<ComponentId>,
    ) -> Result<(), creation::Error<movement::Error>> {
        let input = self
            .unit_creation
            .as_ref()
            .ok_or(creation::Error::Missing("Unit creation inputs"))?;
        let civilization = input
            .achievement
            .civilizations
            .iter()
            .find(|r| Some(r.0) == unit.map(|u| i32::from(u.owner)))
            .map(|r| r.1.as_str())
            .ok_or(creation::Error::Missing("Unit achievement civilization"))?;
        let modifier = self
            .modifier_owners
            .as_mut()
            .ok_or(creation::Error::Missing("canonical modifier owners"))?;
        let movement = self
            .movement
            .as_mut()
            .ok_or(creation::Error::Missing("current movement callbacks"))?;
        let mut effects = CollectionEffects {
            definitions: input.definitions,
            tags: input.tags,
            moved: &mut movement.signal,
            reporting: &mut self.reporting,
            reporting_suspended: movement.reporting_suspended,
        };
        let mut owners = collections::Owners {
            state,
            effects: &mut modifier.effects,
            requirements: &mut modifier.requirements,
            collections: &mut modifier.collections,
        };
        let inputs = stock::Inputs {
            definitions: input.definitions,
            signals: input.signals,
            combat: input.combat,
            build_maps: input.build_maps,
            levels: crate::movement_placement_state::CurrentCivilizationLevels {
                binding: &self.binding,
                values: input.levels,
            },
            scratch: &input.scratch,
            quests: &input.quests,
            achievements: signals::AchievementInputs {
                context_present: input.achievement.context_present,
                integrity: input.achievement.integrity,
                editor: input.achievement.editor,
                ruleset: &input.achievement.ruleset,
                civilization,
            },
        };
        stock::game_created(
            &mut owners,
            &mut modifier.unit_created,
            &mut effects,
            &mut self.script,
            &inputs,
            &input.game_consumers,
            unit,
            city,
        )
        .map_err(callback)
    }
    fn player_unit_created(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        unit: Option<ComponentId>,
    ) -> Result<(), creation::Error<movement::Error>> {
        let input = self
            .unit_creation
            .as_ref()
            .ok_or(creation::Error::Missing("Unit creation inputs"))?;
        let consumers = input.player_consumers.iter().find(|p| p.0 == owner).ok_or(
            creation::Error::Missing("current PlayerUnitCreated consumers"),
        )?;
        let signals = input.signals;
        let consumers = consumers.1.clone();
        for consumer in &consumers {
            let result =
                stock::player_created(state, signals, std::slice::from_ref(consumer), owner, unit);
            match result {
                Err(stock::Error::Signal(signals::Error::Control(c6_core::loaded_state::unit_creation_control::Error::RecruiterRequiresUnitContinuation))) => {
                    let reference=UnitReference::from_component_id(unit.ok_or(creation::Error::Missing("UnitCreated control Unit"))?);
                    let control=self.unit_creation.as_ref().and_then(|v|v.control.as_ref()).ok_or(creation::Error::Missing("current UnitCreated control effects"))?;
                    let movement=control.movement;
                    let snapshot=control.processed_report_snapshot;
                    let actor=state.world().unit(reference).map_err(creation::Error::World)?.ok_or(creation::Error::Missing("current UnitCreated control actor"))?;
                    if !actor.record.b_processed_turn {
                        state.unit_mut(reference.player(),reference.id).map_err(creation::Error::State)?.b_processed_turn=true;
                        if !self.movement().map_err(creation::Error::Effects)?.reporting_suspended && snapshot {
                            self.reporting.observe_unit_snapshot(state,reference).map_err(|e|callback(format!("processed-turn report: {e:?}")))?;
                        }
                    }
                    crate::loaded_upkeep::finish_unit_moves(state,reference.player(),reference.id,&crate::unit_movement_state::Inputs{definitions:movement},self)
                        .map_err(|e|callback(format!("UnitCreated FinishMoves: {e:?}")))?;
                    state.complete_created_unit_control(owner).map_err(callback)?;
                }
                other => other.map_err(callback)?,
            }
        }
        Ok(())
    }
}
impl<P> creation::FactoryEffects for Runtime<'_, P> {
    fn city_unit_created(
        &mut self,
        state: &mut LoadedState,
        unit: ComponentId,
        city: ComponentId,
    ) -> Result<(), creation::Error<movement::Error>> {
        if !self
            .unit_creation
            .as_ref()
            .ok_or(creation::Error::Missing("Unit creation inputs"))?
            .city_initialized_script
        {
            return Err(creation::Error::Missing(
                "current CityUnitCreated consumers",
            ));
        }
        signals::script_initialized(
            state,
            &mut self.script,
            UnitReference::from_component_id(unit),
            Some(city),
        )
        .map_err(callback)
    }
}

struct CityPlacement<'a, 'b, P> {
    runtime: &'a Runtime<'b, P>,
    properties: &'a [(i32, Vec<(u32, i32)>)],
    alive: &'a [i32],
}
impl<P> crate::found_city::cultural_identity::Context for CityPlacement<'_, '_, P> {
    fn alive_ids(&self) -> &[i32] {
        self.alive
    }
    fn civilization_level(&self, owner: i32) -> Result<u32, crate::city_turn::Error> {
        self.runtime
            .movement()
            .map_err(|_| crate::city_turn::Error::Unsupported("current placement levels"))?
            .civilization_levels
            .iter()
            .find(|v| v.0 == owner)
            .map(|v| v.1)
            .ok_or(crate::city_turn::Error::Unsupported(
                "current placement civilization level",
            ))
    }
    fn player_property_int(
        &mut self,
        _: &LoadedState,
        owner: i32,
        key: u32,
    ) -> Result<Option<i32>, crate::city_turn::Error> {
        creation::placement_effects::current_player_property(self.properties, owner, key)
    }
    fn food_surplus_q8(
        &mut self,
        state: &LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<i32, crate::city_turn::Error> {
        let inputs =
            self.runtime
                .city_query_inputs()
                .ok_or(crate::city_turn::Error::Unsupported(
                    "current placement City queries",
                ))?;
        let current = crate::city_turn::yield_calculation::Current {
            territory: inputs.current.territory,
            drought_ids: inputs.current.drought_ids,
            trade: state.trade_graph()?,
            capabilities: inputs.current.capabilities,
            current_era_index: state
                .game_eras()
                .map_err(|_| crate::city_turn::Error::Unsupported("current placement Era owner"))?
                .current_era()
                .ok_or(crate::city_turn::Error::Unsupported(
                    "current placement Era",
                ))?,
        };
        creation::placement_effects::food_surplus_q8(
            state,
            owner,
            city,
            inputs.definitions,
            &current,
        )
    }
    fn happiness(
        &mut self,
        state: &LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<i32, crate::city_turn::Error> {
        let inputs =
            self.runtime
                .city_query_inputs()
                .ok_or(crate::city_turn::Error::Unsupported(
                    "current placement City queries",
                ))?;
        creation::placement_effects::happiness(
            state,
            owner,
            city,
            inputs.definitions,
            inputs.current.capabilities,
        )
    }
    fn trend_changed(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
        _: i32,
        changed: Option<bool>,
    ) -> Result<(), crate::city_turn::Error> {
        if changed == Some(false) {
            Ok(())
        } else {
            Err(crate::city_turn::Error::Unsupported(
                "placement conversion trend changed report",
            ))
        }
    }
}
impl<P> Runtime<'_, P> {
    pub(crate) fn current_city_unit_changed(
        &self,
        state: &mut LoadedState,
        city: ComponentId,
    ) -> Result<(), crate::unit_placement_state::Error<movement::Error>> {
        let input = self
            .unit_creation
            .as_ref()
            .and_then(|v| v.placement.as_ref())
            .ok_or(crate::unit_placement_state::Error::Missing(
                "current City placement loyalty metadata",
            ))?;
        let alive = self
            .player_services
            .as_ref()
            .and_then(|v| v.alive_players.as_ref())
            .ok_or(crate::unit_placement_state::Error::Missing(
                "current placement alive order",
            ))?
            .values();
        creation::placement_effects::city_unit_changed(
            state,
            city,
            input.definitions,
            &mut CityPlacement {
                runtime: self,
                properties: input.properties,
                alive,
            },
        )
        .map_err(|e| {
            crate::unit_placement_state::Error::Effect(movement::Error::Requirement(format!(
                "City placement loyalty: {e:?}"
            )))
        })
    }
}
