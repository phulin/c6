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
    /// Required when occupied-city pressure reaches its current garrison query.
    pub garrison: Option<&'a crate::loaded_actions::city_range_attack::Definitions>,
}
pub struct ControlInputs<'a> {
    pub movement: &'a crate::unit_movement_state::Definitions,
    /// Complete observed d70ba51f native order: either empty or the sole
    /// OnUnitUpdate27ff430 receiver, using the existing snapshot provider.
    pub processed_report_snapshot: bool,
}
/// Immutable inputs for the current canonical research continuation.
#[derive(Clone, Copy)]
pub struct ResearchInputs<'a> {
    pub definitions: &'a crate::research::Definitions,
    pub choices: &'a crate::loaded_actions::choose_tech::Definitions,
    pub culture: Option<&'a super::research::CultureInputs<'a>>,
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
    /// Current moment callbacks; absent remains unsupported when a request is reached.
    pub history: Option<stock::History<'a>>,
    pub research: Option<ResearchInputs<'a>>,
}
// The slot accessor keeps the transfer scope independent of backend services;
// the external owner reference cannot be accessed by the callback.
fn with_research_loan<C, R>(
    backend: &mut C,
    research: &mut crate::research::Runtime,
    slot: fn(&mut C) -> &mut Option<crate::research::Runtime>,
    apply: impl FnOnce(&mut C) -> R,
) -> R {
    let holder = crate::research::Runtime {
        binding: research.binding.clone(),
        civic_graph: None,
        alive_majors: None,
        alive_major_ids: None,
        city_order: Vec::new(),
        game_era: None,
        start_era_hash: None,
        civilization_levels: Vec::new(),
        scratch_players: Vec::new(),
        manager_logging: None,
    };
    *slot(backend) = Some(std::mem::replace(research, holder));
    let result = apply(backend);
    *research = slot(backend)
        .take()
        .expect("canonical research loan retained through synchronous callback");
    result
}
impl<'a, P> Runtime<'a, P> {
    /// Loan the existing research owner to synchronous factory callbacks.
    /// The caller's mutable reference stays borrowed for the whole scope. Its
    /// empty identity holder is never exposed to the callback, and every normal
    /// return (including an error) restores the mutated canonical owner.
    pub fn with_action_research<R>(
        &mut self,
        research: &mut crate::research::Runtime,
        apply: impl FnOnce(&mut Self) -> R,
    ) -> Result<R, movement::Error> {
        if self.action_research.is_some() || research.binding != self.binding {
            return Err(movement::Error::Missing("canonical research loan binding"));
        }
        Ok(with_research_loan(
            self,
            research,
            |backend| &mut backend.action_research,
            apply,
        ))
    }

    /// Bind immutable research definitions after the factory's current inputs.
    pub fn bind_unit_creation_research(
        &mut self,
        state: &LoadedState,
        inputs: ResearchInputs<'a>,
    ) -> Result<(), movement::Error> {
        self.check(state)
            .map_err(|_| movement::Error::Missing("Unit creation research binding"))?;
        if inputs.definitions.binding() != state.binding() {
            return Err(movement::Error::Missing(
                "Unit creation research rules binding",
            ));
        }
        let creation = self
            .unit_creation
            .as_mut()
            .ok_or(movement::Error::Missing("Unit creation inputs"))?;
        if creation.research.is_some() {
            return Err(movement::Error::Missing(
                "Unit creation research already bound",
            ));
        }
        creation.research = Some(inputs);
        Ok(())
    }
    /// Bind current City placement callbacks independently of Unit creation.
    /// Yield, amenity and alive-player services remain on this same runtime.
    pub fn bind_city_placement(
        &mut self,
        state: &LoadedState,
        input: PlacementInputs<'a>,
    ) -> Result<(), movement::Error> {
        self.check(state)
            .map_err(|_| movement::Error::Missing("City placement binding"))?;
        if self.city_placement.is_some() || input.definitions.binding() != state.binding() {
            return Err(movement::Error::Missing(
                "City placement inputs already bound or identity mismatch",
            ));
        }
        self.city_placement = Some(input);
        Ok(())
    }
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
        self.modifier_owners
            .as_ref()
            .ok_or(movement::Error::Missing("canonical modifier owners"))
    }
    pub fn modifier_owners_mut(&mut self) -> Result<&mut ModifierOwners, movement::Error> {
        self.modifier_owners
            .as_mut()
            .ok_or(movement::Error::Missing("canonical modifier owners"))
    }
    /// Loan the SAME canonical owners during a synchronous lifecycle callback.
    /// The provider remains borrowed for reports/sight; no owner is cloned or
    /// rebuilt, and errors restore the mutated owner before returning.
    pub fn with_modifier_owners<R>(
        &mut self,
        apply: impl FnOnce(&mut Self, &mut ModifierOwners) -> R,
    ) -> Result<R, movement::Error> {
        let mut owners = self
            .modifier_owners
            .take()
            .ok_or(movement::Error::Missing("canonical modifier owners"))?;
        let result = apply(self, &mut owners);
        self.modifier_owners = Some(owners);
        Ok(result)
    }
    /// Temporarily transfer the canonical modifier owners to a synchronous
    /// adapter that requires ownership. The adapter must return those same
    /// mutated owners with its result, including on a failed operation.
    pub(crate) fn with_owned_modifier_owners<R>(
        &mut self,
        apply: impl FnOnce(&mut Self, ModifierOwners) -> (ModifierOwners, R),
    ) -> Result<R, movement::Error> {
        let owners = self
            .modifier_owners
            .take()
            .ok_or(movement::Error::Missing("canonical modifier owners"))?;
        let (owners, result) = apply(self, owners);
        self.modifier_owners = Some(owners);
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
pub(super) struct CollectionEffects<'a, 'r> {
    pub(super) definitions: &'a creation::Definitions,
    pub(super) tags: &'a crate::loaded_actions::reporting::hold_cycling::commands::Tags,
    pub(super) moved: &'a mut c6_core::unit_moved_signal::Registry,
    pub(super) reporting: &'a mut crate::loaded_actions::reporting::Provider<'r>,
    pub(super) reporting_suspended: bool,
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
impl<P> Runtime<'_, P> {
    fn created_research_boost(
        &mut self,
        state: &mut LoadedState,
        inputs: Option<ResearchInputs<'_>>,
        owner: i32,
        index: usize,
    ) -> Result<(), creation::Error<movement::Error>> {
        use crate::melee_attack::resolve::completion::boosts::Trigger;
        let inputs = inputs.ok_or(creation::Error::Missing("UnitCreated research inputs"))?;
        let mut runtime = self.action_research.take().ok_or(creation::Error::Missing(
            "UnitCreated canonical research loan",
        ))?;
        let result = {
            let mut effects =
                super::research::Effects::new(self, &mut runtime, inputs.choices, inputs.culture);
            crate::melee_attack::resolve::completion::boosts::ResearchTrigger {
                definitions: inputs.definitions,
                effects: &mut effects,
            }
            .trigger(state, owner, index)
            .map_err(callback)
        };
        self.action_research = Some(runtime);
        result
    }
    fn game_unit_created_current(
        &mut self,
        state: &mut LoadedState,
        input: &State<'_>,
        unit: Option<ComponentId>,
        city: Option<ComponentId>,
    ) -> Result<(), creation::Error<movement::Error>> {
        let gossip_history_empty = self.current_gossip_history_empty();
        let civilization = input
            .achievement
            .civilizations
            .iter()
            .find(|r| Some(r.0) == unit.map(|u| i32::from(u.owner)))
            .map(|r| r.1.as_str())
            .ok_or(creation::Error::Missing("Unit achievement civilization"))?;
        for consumer in &input.game_consumers {
            if let stock::Consumer::Boost { kind, row } = *consumer {
                let reference = UnitReference::from_component_id(
                    unit.ok_or(creation::Error::Missing("nonnull stock UnitCreated Unit"))?,
                );
                if let Some(index) = input
                    .signals
                    .boost_created(state, input.combat, input.build_maps, kind, row, reference)
                    .map_err(callback)?
                {
                    self.created_research_boost(state, input.research, reference.player(), index)?;
                }
                continue;
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
                history: input.history.as_ref(),
                gossip_history_empty,
            };
            stock::game_created(
                &mut owners,
                &mut modifier.unit_created,
                &mut effects,
                &mut self.script,
                &inputs,
                std::slice::from_ref(consumer),
                unit,
                city,
            )
            .map_err(callback)?;
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
        // Policy-created PlayerUnits subscriptions append after the imported
        // current Game.UnitCreated ring, in actual factory registration order.
        let routes = modifier.unit_created.policy_unit_subscriptions().to_vec();
        if let Some(unit) = unit {
            for route in routes {
                collections::policy::unit_created(
                    &mut owners,
                    &mut modifier.unit_created,
                    &mut effects,
                    input.definitions,
                    route,
                    UnitReference::from_component_id(unit),
                )
                .map_err(callback)?;
            }
        }
        Ok(())
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
            .take()
            .ok_or(creation::Error::Missing("Unit creation inputs"))?;
        let result = self.game_unit_created_current(state, &input, unit, city);
        self.unit_creation = Some(input);
        result
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
    fn has_martial_law_garrison(
        &mut self,
        state: &LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<bool, crate::city_turn::Error> {
        let input = self
            .runtime
            .city_placement
            .as_ref()
            .or_else(|| {
                self.runtime
                    .unit_creation
                    .as_ref()
                    .and_then(|v| v.placement.as_ref())
            })
            .ok_or(crate::city_turn::Error::Unsupported(
                "current City placement inputs",
            ))?;
        let definitions = input.garrison.ok_or(crate::city_turn::Error::Unsupported(
            "current placement garrison definitions",
        ))?;
        let movement = self.runtime.movement().map_err(|_| {
            crate::city_turn::Error::Unsupported("current placement combat definitions")
        })?;
        crate::loaded_actions::city_range_attack::city_has_garrison(
            state,
            owner,
            city,
            definitions,
            movement.definitions.formation,
        )
        .map_err(|_| crate::city_turn::Error::Unsupported("current placement City garrison"))
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
        let territory = inputs.current.territory.read()?;
        let current = crate::city_turn::yield_calculation::Current {
            territory: &territory,
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
        let _ = changed;
        // Runtime's fixed stock/no-added-consumers scope admits only the
        // 0242e68c -> 2717400 derived-cache output. Conversion state was already
        // written by RecalculateConversionTrends; this report has no gameplay
        // subscriber or RNG. See the same closure in current_upkeep::Context.
        Ok(())
    }
}
impl<P> Runtime<'_, P> {
    pub(crate) fn current_city_unit_changed(
        &self,
        state: &mut LoadedState,
        city: ComponentId,
    ) -> Result<(), crate::unit_placement_state::Error<movement::Error>> {
        let input = self
            .city_placement
            .as_ref()
            .or_else(|| {
                self.unit_creation
                    .as_ref()
                    .and_then(|v| v.placement.as_ref())
            })
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

impl<P> Runtime<'_, P> {
    /// Synchronous SetPromotion gameplay subscriber over the canonical owners.
    pub fn upgrade_promotion_earned(
        &mut self,
        state: &mut LoadedState,
        rules: &crate::Rules,
        unit: ComponentId,
        name: &[u8],
    ) -> Result<(), String> {
        self.check(state)
            .map_err(|e| format!("promotion binding: {e:?}"))?;
        let input = self
            .unit_creation
            .as_ref()
            .ok_or("current Unit creation inputs")?;
        let modifier = self
            .modifier_owners
            .as_mut()
            .ok_or("canonical modifier owners")?;
        let movement = self.movement.as_mut().ok_or("current movement callbacks")?;
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
        collections::ability::on_promotion_earned(
            &mut owners,
            &mut modifier.unit_created,
            &mut effects,
            input.definitions,
            rules,
            UnitReference::from_component_id(unit),
            name,
        )
        .map_err(|e| format!("promotion attachment: {e:?}"))
    }
    pub fn upgrade_processed_turn(
        &mut self,
        state: &mut LoadedState,
        unit: ComponentId,
    ) -> Result<(), String> {
        self.check(state)
            .map_err(|e| format!("processed binding: {e:?}"))?;
        let snapshot = self
            .unit_creation
            .as_ref()
            .and_then(|v| v.control.as_ref())
            .ok_or("current processed-turn report inputs")?
            .processed_report_snapshot;
        if !snapshot {
            return Err("unqualified processed-turn report consumers".into());
        }
        let reference = UnitReference::from_component_id(unit);
        let current = state
            .unit_mut(reference.player(), reference.id)
            .map_err(|e| format!("processed Unit: {e:?}"))?;
        if !current.b_processed_turn {
            current.b_processed_turn = true;
            if !self
                .movement()
                .map_err(|e| format!("processed movement: {e:?}"))?
                .reporting_suspended
            {
                self.reporting
                    .observe_unit_snapshot(state, reference)
                    .map_err(|e| format!("processed report: {e:?}"))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::with_research_loan;
    use c6_core::core_player::initialization::{
        DatabaseIdentity, InputIdentities, WireBinding, WireRole,
    };

    #[test]
    fn research_loan_restores_canonical_mutations_after_callback_error() {
        let binding = WireBinding {
            role: WireRole::ReadInputCore,
            core_sha256: [1; 32],
            occurrence_id: b"synthetic-research-loan".to_vec(),
            output_id: None,
            inputs: InputIdentities {
                configuration_sha256: [2; 32],
                rules_inputs_sha256: [3; 32],
                database: DatabaseIdentity::Snapshot([4; 32]),
                dlc_inputs_sha256: [5; 32],
                mods_inputs_sha256: [6; 32],
                content_inputs_sha256: [7; 32],
            },
        };
        let mut research = crate::research::Runtime {
            binding: binding.clone(),
            civic_graph: Some(crate::research::CivicGraph {
                nodes: vec![],
                iteration: vec![3, 1],
            }),
            alive_majors: Some(vec![0, 2]),
            alive_major_ids: Some(vec![2, 0]),
            city_order: vec![(0, vec![])],
            game_era: Some(1),
            start_era_hash: Some(17),
            civilization_levels: vec![(63, 9)],
            scratch_players: vec![(63, false)],
            manager_logging: Some(false),
        };
        let allocation = research.alive_majors.as_ref().unwrap().as_ptr();
        let mut backend = None;
        let result = with_research_loan(
            &mut backend,
            &mut research,
            |slot| slot,
            |slot| {
                let owner = slot.as_mut().unwrap();
                assert_eq!(owner.alive_majors.as_ref().unwrap().as_ptr(), allocation);
                owner.game_era = Some(2);
                owner.scratch_players[0].1 = true;
                Err::<(), _>("reached callback failure")
            },
        );
        assert_eq!(result, Err("reached callback failure"));
        assert!(backend.is_none());
        assert_eq!(research.binding, binding);
        assert_eq!(research.alive_majors.as_ref().unwrap().as_ptr(), allocation);
        assert_eq!(research.alive_major_ids, Some(vec![2, 0]));
        assert_eq!(research.civic_graph.unwrap().iteration, vec![3, 1]);
        assert_eq!(research.city_order, vec![(0, vec![])]);
        assert_eq!(research.game_era, Some(2));
        assert_eq!(research.start_era_hash, Some(17));
        assert_eq!(research.civilization_levels, vec![(63, 9)]);
        assert_eq!(research.scratch_players, vec![(63, true)]);
        assert_eq!(research.manager_logging, Some(false));
    }
}
