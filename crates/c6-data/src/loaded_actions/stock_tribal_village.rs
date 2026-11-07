//! Current Linux tribal-village selection and explicit reached continuations.
use super::{movement::Error, Runtime};
use crate::{
    registry::linux_type_hash as hash, unit_initialization_definitions::collection, Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding, loaded_state::LoadedState, tribal_village as kernel,
};
fn error(value: impl std::fmt::Debug) -> Error {
    Error::Requirement(format!("tribal village: {value:?}"))
}
fn missing(value: &'static str) -> Error {
    Error::Missing(value)
}
fn integer(row: crate::Row<'_>, column: &str) -> Result<i32, Error> {
    row.integer(column)
        .map_err(error)?
        .try_into()
        .map_err(error)
}
fn reference(row: crate::Row<'_>, column: &str) -> Result<Option<u32>, Error> {
    match row.get(column).map_err(error)? {
        SqlValue::Null => Ok(None),
        SqlValue::Text(v) => Ok(Some(hash(v))),
        _ => Err(missing("village reference type")),
    }
}
fn reward_recipe(
    rules: &Rules,
    id: Option<u32>,
) -> Result<(Vec<u8>, crate::unit_kill::effects::Effect), String> {
    let row = rules
        .gameplay()
        .table("Modifiers")
        .ok_or("goody Modifiers")?
        .rows()
        .find(|r| {
            r.text("ModifierId")
                .ok()
                .is_some_and(|v| Some(hash(v.as_bytes())) == id)
        })
        .ok_or("goody ModifierID")?;
    let name = row
        .text("ModifierId")
        .map_err(|e| format!("{e:?}"))?
        .as_bytes()
        .to_vec();
    let dynamic = rules
        .gameplay()
        .table("DynamicModifiers")
        .ok_or("goody DynamicModifiers")?
        .rows()
        .find(|r| r.text("ModifierType").ok() == row.text("ModifierType").ok())
        .ok_or("goody DynamicModifier")?;
    if dynamic.text("CollectionType").ok() != Some("COLLECTION_OWNER") {
        return Err("selected village effect is not supported".into());
    }
    for (flag, expected) in [
        ("RunOnce", 1),
        ("Permanent", 1),
        ("NewOnly", 0),
        ("Repeatable", 0),
        ("OwnerStackLimit", 0),
        ("SubjectStackLimit", 0),
    ] {
        let value =
            crate::game_parameters::sql_integer(row.get(flag).map_err(|e| format!("{e:?}"))?)
                .ok_or_else(|| format!("goody modifier integer {flag}"))?;
        if value != expected {
            return Err(format!("goody modifier flag {flag}"));
        }
    }
    for key in ["OwnerRequirementSetId", "SubjectRequirementSetId"] {
        if !matches!(row.get(key), Ok(SqlValue::Null)) && row.text(key).ok() != Some("") {
            return Err(format!("goody modifier requirement {key}"));
        }
    }
    let kind = match dynamic.text("EffectType").ok() {
        Some("EFFECT_ADJUST_UNIT_GRANT_EXPERIENCE") => None,
        Some("EFFECT_ADJUST_PLAYER_RANDOM_TECHNOLOGY_BOOST_GOODY_HUT") => {
            Some(crate::research::Kind::Technology)
        }
        Some("EFFECT_ADJUST_PLAYER_RANDOM_CIVIC_BOOST_GOODY_HUT") => {
            Some(crate::research::Kind::Civic)
        }
        _ => return Err("selected village effect is not supported".into()),
    };
    let mut amount = 0;
    let mut source = 1;
    for arg in rules
        .gameplay()
        .table("ModifierArguments")
        .ok_or("goody ModifierArguments")?
        .rows()
        .filter(|r| r.text("ModifierId").ok().map(str::as_bytes) == Some(name.as_slice()))
    {
        if arg.text("Type").ok() != Some("ARGTYPE_IDENTITY") {
            return Err("dynamic goody experience argument".into());
        }
        if kind.is_some() && arg.text("Name").ok() == Some("Source") {
            source = if arg.text("Value").ok() == Some("CAPTURED_CITY") {
                7
            } else {
                1
            };
            continue;
        }
        if arg.text("Name").ok() != Some("Amount") {
            return Err("unsupported village modifier argument".into());
        }
        amount = arg
            .text("Value")
            .map_err(|e| format!("{e:?}"))?
            .parse()
            .map_err(|_| "goody experience Amount")?;
    }
    if kind.is_none() && amount < 0 {
        return Err("negative village experience amount continuation".into());
    }
    let effect = match kind {
        None => crate::unit_kill::effects::Effect::GrantedExperience(amount),
        Some(kind) => crate::unit_kill::effects::Effect::RandomResearchBoost {
            kind,
            amount,
            source,
        },
    };
    Ok((name, effect))
}
struct Reward {
    hash: u32,
    show_moment: bool,
    modifier: Result<(Vec<u8>, crate::unit_kill::effects::Effect), String>,
    turn: i32,
    city: bool,
    upgrade: bool,
    experience: bool,
    heal: i32,
    relic: bool,
    trader: bool,
    city_state: bool,
    strategic: bool,
}
pub struct Definitions {
    binding: WireBinding,
    families: Vec<kernel::Family>,
    rewards: Vec<Reward>,
    units: Vec<(u32, bool)>,
    maximum_damage: i32,
    exploration_experience: i32,
    speed: i32,
    resources: crate::city_turn::resources::Definitions,
    strategic: Vec<i32>,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules, speed: u32) -> Result<Self, Error> {
        let families = collection(rules, "GoodyHuts", "GoodyHutType").map_err(error)?;
        let subtypes = collection(rules, "GoodyHutSubTypes", "SubTypeGoodyHut").map_err(error)?;
        let table = |name| {
            rules
                .gameplay()
                .table(name)
                .ok_or(missing("village rules table"))
        };
        let mut definitions = Vec::new();
        let mut rewards = Vec::new();
        for family in families.entries() {
            let row = table("GoodyHuts")?
                .row_by_id(family.database_rowid)
                .ok_or(missing("village family row"))?;
            definitions.push(kernel::Family {
                hash: hash(&family.type_name),
                improvement: reference(row, "ImprovementType")?,
                weight: integer(row, "Weight")?,
                rewards: Vec::new(),
            });
        }
        for subtype in subtypes.entries() {
            let row = table("GoodyHutSubTypes")?
                .row_by_id(subtype.database_rowid)
                .ok_or(missing("village subtype row"))?;
            let family = reference(row, "GoodyHut")?.ok_or(missing("village subtype family"))?;
            definitions
                .iter_mut()
                .find(|v| v.hash == family)
                .ok_or(missing("village family reference"))?
                .rewards
                .push(kernel::Reward {
                    hash: hash(&subtype.type_name),
                    weight: integer(row, "Weight")?,
                });
            let xp2 = table("GoodyHutSubTypes_XP2")?.rows().find(|r| {
                r.text("SubTypeGoodyHut").ok().map(str::as_bytes) == Some(&subtype.type_name)
            });
            rewards.push(Reward {
                hash: hash(&subtype.type_name),
                show_moment: {
                    let family = table("GoodyHuts")?
                        .rows()
                        .find(|r| reference(*r, "GoodyHutType").ok().flatten() == Some(family))
                        .ok_or(missing("village family Moment"))?;
                    integer(family, "ShowMoment")? != 0
                },
                modifier: reward_recipe(rules, reference(row, "ModifierID")?),
                turn: integer(row, "Turn")?,
                city: integer(row, "MinOneCity")? != 0,
                upgrade: integer(row, "UpgradeUnit")? != 0,
                experience: integer(row, "Experience")? != 0,
                heal: integer(row, "Heal")?,
                relic: integer(row, "Relic")? != 0,
                trader: integer(row, "Trader")? != 0,
                city_state: xp2
                    .map(|r| integer(r, "CityState"))
                    .transpose()?
                    .unwrap_or(0)
                    != 0,
                strategic: xp2
                    .map(|r| integer(r, "StrategicResources"))
                    .transpose()?
                    .unwrap_or(0)
                    != 0,
            });
        }
        let mut units = Vec::new();
        for entry in collection(rules, "Units", "UnitType")
            .map_err(error)?
            .entries()
        {
            let row = table("Units")?
                .row_by_id(entry.database_rowid)
                .ok_or(missing("village Unit row"))?;
            let xp2 = table("Units_XP2")?
                .rows()
                .find(|r| r.text("UnitType").ok().map(str::as_bytes) == Some(&entry.type_name));
            units.push((
                hash(&entry.type_name),
                integer(row, "CanEarnExperience")? != 0
                    && xp2
                        .map(|r| integer(r, "CanEarnExperience"))
                        .transpose()?
                        .unwrap_or(1)
                        != 0,
            ));
        }
        let speed_defs = collection(rules, "GameSpeeds", "GameSpeedType").map_err(error)?;
        let speed_entry = speed_defs
            .find(speed)
            .ok_or(missing("village game speed"))?;
        let speed = integer(
            table("GameSpeeds")?
                .row_by_id(speed_entry.database_rowid)
                .ok_or(missing("village speed row"))?,
            "CostMultiplier",
        )?;
        let maximum_damage = rules
            .gameplay()
            .table("GlobalParameters")
            .and_then(|t| {
                t.rows()
                    .find(|r| r.text("Name").ok() == Some("COMBAT_MAX_HIT_POINTS"))
            })
            .and_then(|r| r.get("Value").ok())
            .and_then(crate::game_parameters::sql_integer)
            .ok_or(missing("village maximum damage"))?;
        let mut strategic = Vec::new();
        for (index, entry) in collection(rules, "Resources", "ResourceType")
            .map_err(error)?
            .entries()
            .iter()
            .enumerate()
        {
            let row = table("Resources")?
                .row_by_id(entry.database_rowid)
                .ok_or(missing("village resource row"))?;
            if row.text("ResourceClassType").map_err(error)? == "RESOURCECLASS_STRATEGIC" {
                strategic.push(index as i32);
            }
        }
        Ok(Self {
            binding: state.binding().clone(),
            families: definitions,
            rewards,
            units,
            maximum_damage,
            exploration_experience: rules
                .gameplay()
                .table("GlobalParameters")
                .and_then(|t| {
                    t.rows()
                        .find(|r| r.text("Name").ok() == Some("EXPERIENCE_ACTIVATE_GOODY_HUT"))
                })
                .and_then(|r| r.get("Value").ok())
                .and_then(crate::game_parameters::sql_integer)
                .ok_or(missing("village exploration experience"))?,
            speed,
            resources: crate::city_turn::resources::Definitions::from_rules(rules)
                .map_err(error)?,
            strategic,
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Subscriber {
    AttachGameEffects,
    History,
    Summary,
    Script,
    Unsupported(u64),
}
/// Actual stock presentation/report consumers, with no added synchronous
/// gameplay consumers. The adapter must qualify all reached routes.
#[derive(Clone, Copy, Debug)]
pub enum Scope {
    StockPlainHeadlessExternalPoliciesNoAddedConsumers,
}
pub struct Inputs<'a> {
    pub definitions: &'a Definitions,
    pub scope: Scope,
    pub subscribers: Vec<Subscriber>,
    pub history: &'a crate::melee_attack::resolve::completion::history::Definitions,
    pub history_inputs: crate::melee_attack::resolve::completion::history::Inputs<'a>,
    pub history_recorded:
        &'a [crate::melee_attack::resolve::completion::history::RecordedListener],
    /// Actual configuration initializer flag, not an eligibility override.
    pub ignore_turn: bool,
    /// Current original alive-minor roster in native order.
    pub alive_minor_ids: Vec<i32>,
    /// Unsaved Goody_Hut::m_applyingGoody for each current owner.
    pub applying: Vec<(i32, bool)>,
}
pub struct State<'a> {
    inputs: Inputs<'a>,
    pub selections: Vec<((i32, u32), Option<kernel::Selection>)>,
    pub history_reports: Vec<crate::loaded_actions::government_actions::history::Report>,
}
struct Current<'a, 's> {
    state: &'s mut LoadedState,
    definitions: &'a Definitions,
    owner: i32,
    actor: u32,
    ignore_turn: bool,
    minor_ids: &'a [i32],
}
impl kernel::Context for Current<'_, '_> {
    type Error = Error;
    fn eligible(&mut self, reward: u32) -> Result<bool, Error> {
        let d = self
            .definitions
            .rewards
            .iter()
            .find(|v| v.hash == reward)
            .ok_or(missing("village current reward"))?;
        let player = self
            .state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == self.owner)
            .ok_or(missing("village Player"))?;
        let unit = self
            .state
            .world()
            .player(self.owner)
            .map_err(error)?
            .unit(self.actor)
            .map_err(error)?
            .ok_or(missing("village Unit"))?
            .record;
        if !self.ignore_turn
            && d.turn > 0
            && self.state.retained().core.current_turn
                < c6_core::research_acquisition::units::adjust_for_game_speed(
                    d.turn,
                    self.definitions.speed,
                )
        {
            return Ok(false);
        }
        if d.city && player.cities.records.entries.is_empty() {
            return Ok(false);
        }
        if d.upgrade {
            return Err(missing("positive village unit-upgrade eligibility"));
        }
        if d.experience
            && (!self
                .definitions
                .units
                .iter()
                .find(|u| u.0 == unit.e_type_index)
                .ok_or(missing("village Unit definition"))?
                .1
                || unit.k_experience.b_experience_locked)
        {
            return Ok(false);
        }
        if d.heal > 0 && unit.i_damage < self.definitions.maximum_damage / d.heal {
            return Ok(false);
        }
        if d.relic {
            return Err(missing("positive village nearest relic-slot eligibility"));
        }
        if d.trader {
            return Err(missing("positive village trader capacity eligibility"));
        }
        if d.city_state {
            let mut met = false;
            for &other in self.minor_ids {
                if self
                    .state
                    .game_relations()
                    .map_err(error)?
                    .has_met(self.owner, other)
                    .map_err(error)?
                {
                    met = true;
                    break;
                }
            }
            if !met {
                return Ok(false);
            }
        }
        if d.strategic {
            let mut visible = false;
            for &resource in &self.definitions.strategic {
                if self
                    .definitions
                    .resources
                    .visible_resource(self.state, self.owner, resource)
                    .map_err(error)?
                    != -1
                {
                    visible = true;
                    break;
                }
            }
            if !visible {
                return Ok(false);
            }
            return Err(missing("positive village strategic stockpile eligibility"));
        }
        Ok(true)
    }
    fn prior_rewards(&mut self, family: u32) -> Result<i32, Error> {
        Ok(self
            .state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == self.owner)
            .ok_or(missing("village Player"))?
            .goody_hut
            .player_goodies
            .iter()
            .rfind(|p| p.definition_hash == family)
            .ok_or(missing("village current family reward count"))?
            .value)
    }
    fn random(&mut self, bound: u16, _: kernel::Draw) -> Result<u16, Error> {
        self.state.game_sync_rand(bound).map_err(error)
    }
    fn increment_rewards(&mut self, family: u32) -> Result<(), Error> {
        let count = self
            .state
            .player_goody_hut_mut(self.owner)
            .map_err(error)?
            .player_goodies
            .iter_mut()
            .rfind(|p| p.definition_hash == family)
            .ok_or(missing("village current family reward count"))?;
        count.value = count.value.wrapping_add(1);
        Ok(())
    }
}
impl<'a, P> Runtime<'a, P> {
    pub fn bind_tribal_village(
        &mut self,
        state: &LoadedState,
        inputs: Inputs<'a>,
    ) -> Result<(), Error> {
        self.check(state).map_err(error)?;
        if self.tribal_village.is_some() || state.binding() != &inputs.definitions.binding {
            return Err(missing("village owner binding"));
        }
        self.tribal_village = Some(State {
            inputs,
            selections: Vec::new(),
            history_reports: Vec::new(),
        });
        Ok(())
    }
    pub fn tribal_village(&self) -> Option<&State<'a>> {
        self.tribal_village.as_ref()
    }
}

impl<'a, P> Runtime<'a, P> {
    pub(super) fn apply_tribal_village(
        &mut self,
        state: &mut LoadedState,
        sight: &mut crate::unit_sight_state::Runtime,
        actor: (i32, u32),
        plot: u32,
        improvement: u32,
    ) -> Result<(), Error> {
        let mut owner = self
            .tribal_village
            .take()
            .ok_or(missing("current tribal village services"))?;
        let result = owner.apply(state, sight, self, actor, plot, improvement);
        self.tribal_village = Some(owner);
        result
    }
}
impl State<'_> {
    fn apply<P>(
        &mut self,
        state: &mut LoadedState,
        sight: &mut crate::unit_sight_state::Runtime,
        backend: &mut Runtime<'_, P>,
        actor: (i32, u32),
        plot: u32,
        improvement: u32,
    ) -> Result<(), Error> {
        let applying = self
            .inputs
            .applying
            .iter_mut()
            .find(|p| p.0 == actor.0)
            .ok_or(missing("current unsaved applyingGoody"))?;
        if applying.1 {
            return Ok(());
        }
        applying.1 = true;
        let selected = kernel::select(
            &mut Current {
                state,
                definitions: self.inputs.definitions,
                owner: actor.0,
                actor: actor.1,
                ignore_turn: self.inputs.ignore_turn,
                minor_ids: &self.inputs.alive_minor_ids,
            },
            &self.inputs.definitions.families,
            improvement,
        )?;
        self.selections.push((actor, selected));
        // Original retains applyingGoody=true when GetNewType yields null.
        let Some(selected) = selected else {
            return Ok(());
        };
        crate::loaded_upkeep::complete_unit_movement(state, actor.0, actor.1, backend)
            .map_err(error)?;
        let reward = self
            .inputs
            .definitions
            .rewards
            .iter()
            .find(|r| r.hash == selected.reward)
            .ok_or(missing("selected village subtype"))?;
        for &subscriber in &self.inputs.subscribers {
            match subscriber {
                Subscriber::AttachGameEffects => {
                    let (definition, effect) = reward.modifier.as_ref().map_err(error)?;
                    backend.attach_village_reward(
                        state,
                        actor,
                        definition,
                        *effect,
                        self.inputs.definitions,
                        sight,
                    )?;
                }
                Subscriber::History if reward.show_moment => {
                    use c6_core::game_history::{Moment, MomentData, MOMENT_VERSION};
                    let map = state.world().map();
                    let at = c6_core::unit::PlotCoord {
                        x: plot as i32 % map.header.width,
                        y: plot as i32 / map.header.width,
                    };
                    let mut effects =
                        crate::loaded_actions::government_actions::history::StockEffects {
                            recorded: self.inputs.history_recorded,
                            reports: Vec::new(),
                        };
                    crate::melee_attack::resolve::completion::history::handle_moment(
                        state,
                        self.inputs.history,
                        self.inputs.history_inputs,
                        Moment {
                            wire_version: MOMENT_VERSION,
                            type_hash: 0x9c51164c,
                            turn: -1,
                            action_plot: at,
                            acting_player_ordinal: actor.0,
                            id: -1,
                            has_ever_been_commemorated_raw: 0,
                            game_era_hash: u32::MAX,
                            extra_data: vec![MomentData {
                                kind_raw: 0x4de4292b,
                                value: actor.0,
                            }],
                            city_name_bytes: Vec::new(),
                            unit_citizen_name_bytes: Vec::new(),
                            other_city_name_bytes: Vec::new(),
                            era_score: 0,
                        },
                        &mut effects,
                    )
                    .map_err(error)?;
                    self.history_reports.extend(effects.reports);
                }
                Subscriber::History => {}
                Subscriber::Summary => {
                    // GeneralPlayerDataProvider's qualified dataset update is
                    // presentation-only; no gameplay subscriber or RNG.
                    backend
                        .reached
                        .push("stock tribal village summary boundary");
                }
                Subscriber::Script => {
                    let index = self
                        .inputs
                        .definitions
                        .rewards
                        .iter()
                        .position(|r| r.hash == selected.reward)
                        .ok_or(missing("village subtype index"))?;
                    backend
                        .script
                        .call_empty(
                            state,
                            "UnitTriggerGoodyHut",
                            &[actor.0, actor.1 as i32, index as i32],
                        )
                        .map_err(error)?;
                }
                _ => return Err(error(("village callback", subscriber))),
            }
        }
        use crate::unit_sight_state::Effects;
        let human = state.player_slot_status(actor.0).map_err(error)? == 3;
        if human {
            if backend.promotion_class(state, actor)? == Some("PROMOTION_CLASS_RECON") {
                let modifier = backend.experience_modifier(state, actor)?;
                let amount = c6_core::unit_sight::experience_amount(
                    self.inputs.definitions.exploration_experience,
                    modifier,
                );
                backend.change_experience(state, sight, actor, amount)?;
            }
            // CheckPromoteNotification is also called after the exploration
            // change. The currently reached subtype leaves XP below threshold.
            if state
                .world()
                .player(actor.0)
                .map_err(error)?
                .unit(actor.1)
                .map_err(error)?
                .ok_or(missing("goody Unit"))?
                .record
                .k_experience
                .b_can_promote_notify_needed
            {
                return Err(missing("positive goody promotion notice continuation"));
            }
        }
        backend
            .reached
            .push("stock tribal village reward presentation boundary");
        self.inputs
            .applying
            .iter_mut()
            .find(|p| p.0 == actor.0)
            .ok_or(missing("current unsaved applyingGoody"))?
            .1 = false;
        Ok(())
    }
}

impl<P> Runtime<'_, P> {
    fn attach_village_reward(
        &mut self,
        state: &mut LoadedState,
        actor: (i32, u32),
        definition: &[u8],
        reward: crate::unit_kill::effects::Effect,
        definitions: &Definitions,
        sight: &mut crate::unit_sight_state::Runtime,
    ) -> Result<(), Error> {
        use c6_core::{
            map::UnitReference,
            unit::ComponentId,
            unit_kill::objects::{Callback, Modifier, Tracked},
            unit_moved_signal::modifiers::{Modifier as Control, TrackedSubject},
        };
        let unit = UnitReference::from_component_id(ComponentId {
            owner: actor.0 as i16,
            component_type: 1,
            id: actor.1,
        });
        {
            let owner = self
                .modifier_owners
                .as_mut()
                .ok_or(missing("canonical village modifier owner"))?;
            let context = owner
                .effects
                .make_current_unit_object(state, unit, [0, 0])
                .map_err(error)?;
            let instance = owner.effects.allocate_modifier_id(state).map_err(error)?;
            let receiver = (1u64 << 59) | u64::from(instance as u32);
            let effect = (1u64 << 58) | u64::from(instance as u32);
            owner
                .unit_created
                .register_village_reward(receiver)
                .map_err(error)?;
            owner
                .effects
                .register_unit_effect(state, effect, reward)
                .map_err(error)?;
            owner
                .effects
                .register_unit_modifier(
                    state,
                    Modifier {
                        receiver,
                        context,
                        instance,
                        control: Control {
                            enabled: true,
                            suppressed: false,
                            owner_requirement_set: None,
                        },
                        permanent: true,
                        definition: definition.to_vec(),
                        stack_limit: 0,
                        effect,
                        tracked: Vec::new(),
                        applied: Vec::new(),
                    },
                )
                .map_err(error)?;
            let lifecycle = owner
                .effects
                .object_mut(context)
                .map_err(error)?
                .unit_lifecycle
                .as_mut()
                .ok_or(missing("village Unit wrapper lifecycle"))?;
            lifecycle.owned_modifiers.push(instance);
            lifecycle
                .destroyed
                .subscribe(Callback::OwnerDestroyed(receiver))
                .map_err(error)?;
            let token = lifecycle
                .destroyed
                .subscribe(Callback::TrackedDestroyed(receiver))
                .map_err(error)?;
            lifecycle
                .affected
                .mark(definition, instance)
                .map_err(error)?;
            let modifier = owner.effects.unit_modifier_mut(receiver).map_err(error)?;
            modifier.tracked.push(Tracked {
                object: context,
                state: TrackedSubject {
                    requirement_set: None,
                    applied: 1,
                    removed: false,
                },
                destruction_subscription: Some(token),
            });
            modifier.applied.push(context);
        }
        if let crate::unit_kill::effects::Effect::RandomResearchBoost {
            kind,
            amount,
            source,
        } = reward
        {
            let inputs = self
                .unit_creation
                .as_ref()
                .and_then(|input| input.research)
                .ok_or(missing("village canonical research definitions"))?;
            let mut runtime = self
                .action_research
                .take()
                .ok_or(missing("village canonical research loan"))?;
            let result = {
                let mut effects = super::research::Effects::new(
                    self,
                    &mut runtime,
                    inputs.choices,
                    inputs.culture,
                );
                crate::research::Context {
                    state,
                    definitions: inputs.definitions,
                    effects: &mut effects,
                }
                .grant_random_boost_goody_hut(actor.0, kind, amount, source)
                .map_err(error)
            };
            self.action_research = Some(runtime);
            return result;
        }
        let crate::unit_kill::effects::Effect::GrantedExperience(amount) = reward else {
            return Err(missing("village attached effect"));
        };
        let actor_unit = state
            .world()
            .player(actor.0)
            .map_err(error)?
            .unit(actor.1)
            .map_err(error)?
            .ok_or(missing("village Unit"))?
            .record;
        let earn = definitions
            .units
            .iter()
            .find(|u| u.0 == actor_unit.e_type_index)
            .ok_or(missing("village Unit experience definition"))?
            .1
            && !actor_unit.k_experience.b_experience_locked;
        if earn {
            crate::unit_sight_state::Effects::change_experience(self, state, sight, actor, amount)?;
        }
        Ok(())
    }
}

impl State<'_> {
    /// Native definition index for the most recent selected reward, derived
    /// from this binding's canonical subtype collection order.
    pub fn selected_reward_index(&self) -> Option<usize> {
        let hash = self.selections.last()?.1?.reward;
        self.inputs
            .definitions
            .rewards
            .iter()
            .position(|r| r.hash == hash)
    }
    /// Unsaved guard is retained even after an incomplete selected effect or a
    /// native null-selection prefix; it is never reset to manufacture success.
    pub fn applying(&self, owner: i32) -> Option<bool> {
        self.inputs
            .applying
            .iter()
            .find(|p| p.0 == owner)
            .map(|p| p.1)
    }
}
