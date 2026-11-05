//! Ordered activation managers. Positive city/trader branches report their
//! unfinished native prefix; the empty collections are read from current state.
#[path = "resource_cities.rs"]
mod resource_cities;
use crate::{
    game_parameters::sql_integer,
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    unit_initialization_definitions::collection,
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    game_update::Error,
    loaded_components::RetainedPlayer,
    loaded_state::{ActionStamp, LoadedState},
    player_diplomacy_blackboard::DiplomaticModifierTracker,
};

fn missing(s: &'static str) -> Error {
    Error::Unsupported(s)
}
fn player(state: &LoadedState, owner: i32) -> Result<&RetainedPlayer, Error> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(missing("activation manager Player"))
}
fn state_error(_: c6_core::loaded_state::Error) -> Error {
    missing("activation manager state")
}
pub struct Definitions {
    binding: WireBinding,
    units: LinuxDefinitionRegistry,
    traders: Vec<bool>,
    resources: Vec<(u32, bool)>,
    city_resources: resource_cities::Definitions,
    denounce: i32,
    earliest_major_dow_turn: i32,
    casus_prerequisites: Vec<(u32, Option<u32>, bool)>,
    denounce_by_speed: Vec<(u32, i32)>,
    congress_initial_era: i32,
    special_session_min_turns: i32,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let units =
            collection(rules, "Units", "UnitType").map_err(|_| missing("activation Units"))?;
        let table = rules
            .gameplay()
            .table("Units")
            .ok_or(missing("activation Units"))?;
        let traders = units
            .entries()
            .iter()
            .map(|e| {
                table
                    .row_by_id(e.database_rowid)
                    .and_then(|r| r.get("MakeTradeRoute").ok())
                    .and_then(sql_integer)
                    .map(|v| v != 0)
                    .ok_or(missing("MakeTradeRoute"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let res = collection(rules, "Resources", "ResourceType")
            .map_err(|_| missing("activation Resources"))?;
        let consumption = rules
            .gameplay()
            .table("Resource_Consumption")
            .ok_or(missing("Resource_Consumption"))?;
        let resources = res
            .entries()
            .iter()
            .map(|e| {
                let row = consumption.rows().find(|r| {
                    r.text("ResourceType").ok().map(str::as_bytes) == Some(e.type_name.as_slice())
                });
                let accumulates = match row {
                    None => false,
                    Some(r) => {
                        r.get("Accumulate")
                            .ok()
                            .and_then(sql_integer)
                            .ok_or(missing("resource Accumulate"))?
                            != 0
                    }
                };
                Ok((linux_type_hash(&e.type_name), accumulates))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let denounce = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(missing("diplomacy parameters"))?
            .rows()
            .find(|r| r.text("Name").ok() == Some("DIPLOMACY_DENOUNCE_TIME_LIMIT"))
            .and_then(|r| r.get("Value").ok())
            .and_then(sql_integer)
            .ok_or(missing("denounce time limit"))?;
        let speeds = rules
            .gameplay()
            .table("GameSpeeds")
            .ok_or(missing("GameSpeeds"))?;
        let scalings = rules
            .gameplay()
            .table("GameSpeed_Scalings")
            .ok_or(missing("GameSpeed_Scalings"))?;
        let durations = rules
            .gameplay()
            .table("GameSpeed_Durations")
            .ok_or(missing("GameSpeed_Durations"))?;
        let mut denounce_by_speed = Vec::new();
        for speed in speeds.rows() {
            let name = speed
                .text("GameSpeedType")
                .map_err(|_| missing("GameSpeedType"))?;
            let cost = speed
                .get("CostMultiplier")
                .ok()
                .and_then(sql_integer)
                .ok_or(missing("GameSpeed cost"))?;
            let scaling = scalings.rows().find(|r| {
                r.text("GameSpeedType").ok() == Some(name)
                    && r.text("ScalingType")
                        .ok()
                        .is_some_and(|s| linux_type_hash(s.as_bytes()) == 0xafc894aa)
            });
            let mut scaled = 0;
            if let Some(row) = scaling {
                let key = row
                    .text("GameSpeedScalingType")
                    .map_err(|_| missing("GameSpeedScalingType"))?;
                if let Some(duration) = durations.rows().find(|r| {
                    r.text("GameSpeedScalingType").ok() == Some(key)
                        && r.get("NumberOfTurnsOnStandard").ok().and_then(sql_integer)
                            == Some(denounce)
                }) {
                    scaled = duration
                        .get("NumberOfTurnsScaled")
                        .ok()
                        .and_then(sql_integer)
                        .ok_or(missing("scaled duration"))?;
                } else {
                    let percent = row
                        .get("DefaultCostMultiplier")
                        .ok()
                        .and_then(sql_integer)
                        .ok_or(missing("scaling multiplier"))?;
                    scaled = scale(denounce, percent);
                }
            }
            if scaled == 0 {
                scaled = scale(denounce, cost);
            }
            denounce_by_speed.push((linux_type_hash(name.as_bytes()), scaled));
        }
        let parameter = |name| {
            rules
                .gameplay()
                .table("GlobalParameters")
                .and_then(|t| t.rows().find(|r| r.text("Name").ok() == Some(name)))
                .and_then(|r| r.get("Value").ok())
                .and_then(sql_integer)
                .ok_or(missing("Congress parameter"))
        };
        let actions = rules
            .gameplay()
            .table("DiplomaticActions")
            .ok_or(missing("DiplomaticActions"))?;
        let xp1 = rules
            .gameplay()
            .table("DiplomaticActions_XP1")
            .ok_or(missing("DiplomaticActions_XP1"))?;
        let mut casus_prerequisites = Vec::new();
        for name in [
            "DIPLOACTION_DECLARE_COLONIAL_WAR",
            "DIPLOACTION_DECLARE_HOLY_WAR",
            "DIPLOACTION_DECLARE_LIBERATION_WAR",
            "DIPLOACTION_DECLARE_PROTECTORATE_WAR",
            "DIPLOACTION_DECLARE_RECONQUEST_WAR",
            "DIPLOACTION_DECLARE_TERRITORIAL_WAR",
            "DIPLOACTION_DECLARE_GOLDEN_AGE_WAR",
            "DIPLOACTION_DECLARE_WAR_OF_RETRIBUTION",
            "DIPLOACTION_DECLARE_IDEOLOGICAL_WAR",
        ] {
            let row = actions
                .rows()
                .find(|r| r.text("DiplomaticActionType").ok() == Some(name))
                .ok_or(missing("casus belli action"))?;
            let prerequisite = match row
                .get("InitiatorPrereqCivic")
                .map_err(|_| missing("casus civic prerequisite"))?
            {
                SqlValue::Null => None,
                SqlValue::Text(value) => Some(linux_type_hash(value)),
                _ => return Err(missing("casus civic prerequisite")),
            };
            let golden = xp1
                .rows()
                .find(|r| r.text("DiplomaticActionType").ok() == Some(name))
                .is_some_and(|r| {
                    matches!(
                        r.get("RequiresGoldenAgeCommemorationType"),
                        Ok(SqlValue::Text(_))
                    )
                });
            casus_prerequisites.push((linux_type_hash(name.as_bytes()), prerequisite, golden));
        }
        let city_resources = resource_cities::Definitions::from_rules(state, rules, &resources)?;
        Ok(Self {
            city_resources,
            congress_initial_era: parameter("WORLD_CONGRESS_INITIAL_ERA")?,
            special_session_min_turns: parameter(
                "WORLD_CONGRESS_MIN_TIME_BETWEEN_SPECIAL_SESSIONS",
            )?,
            binding: state.binding().clone(),
            units,
            traders,
            resources,
            denounce,
            earliest_major_dow_turn: parameter("DIPLOMACY_EARLIEST_MAJOR_DOW_TURN")?,
            casus_prerequisites,
            denounce_by_speed,
        })
    }
    fn validate(&self, state: &LoadedState) -> Result<(), Error> {
        if &self.binding != state.binding() {
            Err(missing("activation manager definition binding"))
        } else {
            Ok(())
        }
    }
}
/// Current unsaved Climate resource words in their native order. Native reset
/// writes only words 1 and 3 (row+12 and row+20), preserving words 0 and 2.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClimateConsumption {
    pub owner: i32,
    pub resource: i32,
    pub words: [i32; 4],
}
fn reset_climate(rows: &mut [ClimateConsumption], owner: i32) {
    for row in rows {
        if row.owner == owner {
            row.words[1] = 0;
            row.words[3] = 0;
        }
    }
}
pub struct CongressInputs {
    pub last_special_session: i32,
    pub emergencies: Vec<c6_core::game_emergency::Instance>,
}
pub struct Runtime {
    instance: ActionStamp,
    pub climate: Vec<ClimateConsumption>,
    pub parks: Vec<c6_core::game_national_parks::ParkData>,
    congress: Option<CongressInputs>,
    pub(crate) congress_round: Option<crate::game_update_state::round_congress::Current>,
    emergency_signals: Option<Vec<c6_core::game_emergency::Signal>>,
}
impl Runtime {
    pub fn initialize_congress_round(
        &mut self,
        state: &LoadedState,
        current: crate::game_update_state::round_congress::Current,
    ) -> Result<(), Error> {
        self.validate(state)?;
        if self.congress_round.is_some() {
            return Err(missing("Congress round already initialized"));
        }
        self.congress_round = Some(current);
        Ok(())
    }
    pub fn congress(&self, state: &LoadedState) -> Result<&CongressInputs, Error> {
        self.validate(state)?;
        self.congress
            .as_ref()
            .ok_or(missing("current Congress/Emergency owner"))
    }
    /// The signal list belongs to the same live Emergency manager as instances.
    pub fn initialize_emergency_signals(
        &mut self,
        state: &LoadedState,
        signals: Vec<c6_core::game_emergency::Signal>,
    ) -> Result<(), Error> {
        self.validate(state)?;
        if self.emergency_signals.is_some() {
            return Err(missing("Emergency signals already initialized"));
        }
        self.emergency_signals = Some(signals);
        Ok(())
    }
    pub fn emergency_signals(
        &self,
        state: &LoadedState,
    ) -> Result<&[c6_core::game_emergency::Signal], Error> {
        self.validate(state)?;
        self.emergency_signals
            .as_deref()
            .ok_or(missing("current Emergency signals"))
    }
    pub(crate) fn clear_emergency_signals(&mut self, state: &LoadedState) -> Result<(), Error> {
        self.validate(state)?;
        self.emergency_signals
            .as_mut()
            .ok_or(missing("current Emergency signals"))?
            .clear();
        Ok(())
    }
    pub fn from_initial(
        state: &LoadedState,
        climate: Vec<ClimateConsumption>,
        parks: Vec<c6_core::game_national_parks::ParkData>,
    ) -> Self {
        Self {
            instance: state.action_stamp(),
            climate,
            parks,
            congress: None,
            congress_round: None,
            emergency_signals: None,
        }
    }
    pub fn initialize_congress(
        &mut self,
        state: &LoadedState,
        inputs: CongressInputs,
    ) -> Result<(), Error> {
        self.validate(state)?;
        if self.congress.is_some() {
            return Err(missing("Congress owner already initialized"));
        }
        self.congress = Some(inputs);
        Ok(())
    }
    pub(crate) fn validate(&self, state: &LoadedState) -> Result<(), Error> {
        if !self.instance.same_instance(state.action_stamp()) {
            Err(missing("activation Climate/Parks instance"))
        } else {
            Ok(())
        }
    }
}
pub struct Queries<'a> {
    pub alive_major_ids: &'a [i32],
    pub game_speed_type: u32,
}
pub trait Effects {
    fn congress_event(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        kind: u32,
    ) -> Result<(), Error>;
    fn report_block(
        &mut self,
        state: &mut LoadedState,
        hash: u32,
        begin: bool,
    ) -> Result<(), Error>;
    fn diplomacy_tracker_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        other: i32,
        key: u32,
        old: i32,
        new: i32,
    ) -> Result<(), Error>;
    fn congress_points_changed(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        delta: i32,
    ) -> Result<(), Error>;
    fn notification(&mut self, state: &mut LoadedState, owner: i32, kind: u32)
        -> Result<(), Error>;
}
fn met(state: &LoadedState, owner: i32, other: i32) -> Result<bool, Error> {
    if owner == other {
        return Ok(false);
    }
    state
        .world()
        .player(other)
        .map_err(|_| missing("diplomacy target Player"))?;
    state
        .game_relations()
        .map_err(state_error)?
        .has_met(owner, other)
        .map_err(|_| missing("current diplomatic relations"))
}
// Integer overload 2d57930: signed 64-bit product/division, then ClampToInt.
fn scale(value: i32, percentage: i32) -> i32 {
    (i64::from(value) * i64::from(percentage) / 100).clamp(i64::from(i32::MIN), i64::from(i32::MAX))
        as i32
}

fn multiply(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 8) as i32
}
fn update_tracker(t: &mut DiplomaticModifierTracker) {
    t.accumulation_turn_count = t.accumulation_turn_count.wrapping_add(1);
    if t.is_active
        && t.accumulation_turn_count >= t.increment_turns
        && t.current_value != t.max_value
    {
        t.accumulation_turn_count = 0;
        t.current_value = t.current_value.wrapping_add(t.increment);
        t.current_value = if t.increment == 0 || t.max_value == 0 {
            t.max_value
        } else if t.max_value > 0 {
            t.current_value.min(t.max_value)
        } else {
            t.current_value.max(t.max_value)
        };
    }
    if !t.is_active && t.current_value != 0 && t.accumulation_turn_count >= t.cooldown_turns {
        t.accumulation_turn_count = 0;
        let old = t.current_value;
        t.current_value = if multiply(old, t.reduction) > 0 {
            old.wrapping_sub(t.reduction)
        } else {
            old.wrapping_add(t.reduction)
        };
        if multiply(t.current_value, old) < 0 {
            t.current_value = 0;
        }
    }
}
fn diplomacy(
    state: &mut LoadedState,
    defs: &Definitions,
    q: &Queries<'_>,
    e: &mut impl Effects,
    owner: i32,
) -> Result<(), Error> {
    let counts = player(state, owner)?
        .diplomacy
        .blackboard
        .diplomatic_modifiers
        .iter()
        .map(Vec::len)
        .collect::<Vec<_>>();
    if counts.len() != 64 {
        return Err(missing("Diplomacy Blackboard player map count"));
    }
    for (other, count) in counts.into_iter().enumerate() {
        for index in 0..count {
            let (key, old, new) = {
                let m = state.activation_managers_mut(owner).map_err(state_error)?;
                let entry = &mut m.diplomacy.blackboard.diplomatic_modifiers[other][index];
                let old = entry.tracker.current_value;
                update_tracker(&mut entry.tracker);
                (entry.key, old, entry.tracker.current_value)
            };
            if old != new {
                e.diplomacy_tracker_changed(state, owner, other as i32, key, old, new)?;
            }
        }
    }
    for &other in q.alive_major_ids {
        if met(state, owner, other)?
            && *player(state, owner)?
                .diplomacy
                .war_weariness
                .get(other as usize)
                .ok_or(missing("war weariness owner"))?
                > 0
        {
            return Err(missing("positive war weariness decay"));
        }
    }
    state
        .activation_managers_mut(owner)
        .map_err(state_error)?
        .diplomacy
        .city_war_weariness
        .clear();
    if player(state, owner)?
        .diplomacy
        .war_weariness
        .iter()
        .any(|&v| v != 0)
    {
        return Err(missing("positive war weariness amenities"));
    }
    let threshold = state.retained().core.current_turn.wrapping_sub(
        defs.denounce_by_speed
            .iter()
            .find(|r| r.0 == q.game_speed_type)
            .map(|r| r.1)
            .unwrap_or_else(|| scale(defs.denounce, 100)),
    );
    if threshold > 0
        && q.alive_major_ids.iter().any(|&other| {
            player(state, owner)
                .ok()
                .and_then(|p| p.diplomacy.denounced.get(other as usize))
                .is_some_and(|&v| v >= 0 && v < threshold)
        })
    {
        return Err(missing("expired denunciation signal"));
    }
    if !player(state, owner)?.diplomacy.held_messages.is_empty() {
        return Err(missing("held diplomatic messages"));
    }
    for &other in q.alive_major_ids {
        if met(state, owner, other)? {
            // These gates only read current rules/flags. A positive or
            // overridden prerequisite keeps the later formal-war rules explicit.
            if state.retained().core.current_turn >= defs.earliest_major_dow_turn {
                let p = player(state, owner)?;
                for &(action, civic, golden) in &defs.casus_prerequisites {
                    let unmet_civic = if let Some(hash) = civic {
                        let acquired = p
                            .culture
                            .has_civic
                            .pairs
                            .iter()
                            .rev()
                            .find(|v| v.definition_hash == hash)
                            .ok_or(missing("casus current civic flag"))?
                            .value;
                        !acquired
                            && !p
                                .diplomacy
                                .action_prereq_overrides
                                .iter()
                                .any(|v| v.action_hash == action)
                    } else {
                        false
                    };
                    if unmet_civic {
                        continue;
                    }
                    if golden
                        && !state
                            .player_has_golden_age(owner)
                            .map_err(|_| missing("casus current golden age"))?
                    {
                        continue;
                    }
                    return Err(missing("met-player casus belli positive war legality"));
                }
            }
            // UpdateAvailableCasusBelli writes each false result; notifications
            // occur only for newly true availability, not true-to-false changes.
            let index = usize::try_from(other).map_err(|_| missing("casus belli player"))?;
            let row = state
                .activation_managers_mut(owner)
                .map_err(state_error)?
                .diplomacy
                .casus_belli
                .get_mut(index)
                .ok_or(missing("casus belli player"))?;
            clear_casus_belli(row);
        }
    }
    Ok(())
}
fn clear_casus_belli(row: &mut c6_core::player_diplomacy::CasusBelliData) {
    row.colonial_war = false;
    row.holy_war = false;
    row.liberation_war = false;
    row.protectorate_war = false;
    row.reconquest_war = false;
    row.territorial_war = false;
    row.golden_age_war = false;
    row.retribution_war = false;
    row.ideological_war = false;
}
pub fn before_units(
    state: &mut LoadedState,
    defs: &Definitions,
    q: &Queries<'_>,
    runtime: &mut Runtime,
    e: &mut impl Effects,
    owner: i32,
) -> Result<(), Error> {
    defs.validate(state)?;
    runtime.validate(state)?;
    diplomacy(state, defs, q, e, owner)?;
    // Improvements.HandleImprovementVisResets and Heroes.DoTurn are RET.
    let p = player(state, owner)?;
    let v = &p.stats.diplomatic_victory_points;
    let mut points = v.value_q8;
    if let Some(max) = v.max_q8 {
        points = points.min(max);
    }
    if let Some(min) = v.min_q8 {
        points = points.max(min);
    }
    let delta = (points >> 8).wrapping_sub(p.congress.diplo_vp_last_turn);
    if delta != 0 {
        e.congress_points_changed(state, owner, delta)?;
        let p = state.activation_managers_mut(owner).map_err(state_error)?;
        p.congress.diplo_vp_last_turn = p.congress.diplo_vp_last_turn.wrapping_add(delta);
    }
    let turn = state.retained().core.current_turn;
    state
        .activation_managers_mut(owner)
        .map_err(state_error)?
        .espionage
        .cities_with_sources
        .retain(|r| r.expire_turn > turn);
    if !player(state, owner)?.espionage.off_map_spies.is_empty() {
        return Err(missing("returning off-map spies"));
    }
    reset_climate(&mut runtime.climate, owner);
    for row in &mut state
        .player_resources_mut(owner)
        .map_err(state_error)?
        .accumulated_resource_per_turn_q8
    {
        row.value = 0;
    }
    e.report_block(state, 0x1153df52, true)?;
    for &(hash, accumulates) in &defs.resources {
        if !accumulates {
            continue;
        }
        let p = player(state, owner)?;
        for rows in [
            &p.resources.import_resource_extracted,
            &p.resources.free_resources_extracted,
        ] {
            if rows
                .iter()
                .find(|r| r.resource_hash == hash)
                .ok_or(missing("extracted resource entry"))?
                .value
                > 0
            {
                return Err(missing("positive extracted resource accumulation"));
            }
        }
    }
    resource_cities::accumulate(state, &defs.city_resources, owner)?;
    e.report_block(state, 0x1153df52, false)?;
    state
        .player_resources_mut(owner)
        .map_err(state_error)?
        .luxury_allocations
        .clear();
    let r = &player(state, owner)?.resources;
    if r.full_access_one_strategic
        || r.resource_ownership
            .iter()
            .any(|r| r.value.iter().any(|&v| v != 0))
    {
        return Err(missing("owned-resource luxury allocation"));
    }
    if !runtime.parks.is_empty() {
        return Err(missing("national park amenities"));
    }
    Ok(())
}
pub fn after_units(state: &mut LoadedState, defs: &Definitions, owner: i32) -> Result<(), Error> {
    defs.validate(state)?;
    if !player(state, owner)?.governors.entries.is_empty() {
        return Err(missing("governor activation countdown"));
    }
    for slot in state
        .world()
        .player(owner)
        .map_err(|_| missing("trade Player"))?
        .unit_records()
        .map_err(|_| missing("trade Units"))?
    {
        let u = &slot.value;
        if u.b_delayed_death {
            continue;
        }
        let index = state
            .unit_runtime(owner, u.id)
            .map_err(state_error)?
            .definition_index;
        if index == -1 {
            continue;
        }
        let index = usize::try_from(index).map_err(|_| missing("trade definition index"))?;
        if *defs
            .traders
            .get(index)
            .ok_or(missing("trade unit definition"))?
        {
            return Err(missing("idle-trader path update"));
        }
        let hash = c6_core::movement_world::DefinitionLookup::type_hash(&defs.units, index)
            .ok_or(missing("trade definition identity"))?;
        if u.e_type_index != hash && u.e_type_index != index as u32 {
            return Err(missing("trade saved/native unit identity"));
        }
    }
    Ok(())
}
/// Final non-Barbarian resource work after City/Improvement activation.
pub fn after_cities(
    state: &mut LoadedState,
    defs: &Definitions,
    e: &mut impl Effects,
    owner: i32,
) -> Result<(), Error> {
    defs.validate(state)?;
    // DoTurnResourceMaintenance resets the Power demand IndexList, then pays
    // Building costs and supplements City power in current City order.
    for row in &mut state
        .player_resources_mut(owner)
        .map_err(state_error)?
        .power_resource_demand_per_turn_q8
    {
        row.value = 0;
    }
    resource_cities::maintain(state, &defs.city_resources, owner)?;
    for &(hash, accumulates) in &defs.resources {
        if !accumulates {
            continue;
        }
        let p = player(state, owner)?;
        let unit = p
            .units
            .unit_fuel_demands
            .iter()
            .find(|r| r.definition_hash == hash)
            .ok_or(missing("Unit resource demand"))?
            .value
            .wrapping_shl(8);
        let power = p
            .resources
            .power_resource_demand_per_turn_q8
            .iter()
            .find(|r| r.resource_hash == hash)
            .ok_or(missing("Power resource demand"))?
            .value;
        let accumulation = p
            .resources
            .accumulated_resource_per_turn_q8
            .iter()
            .find(|r| r.resource_hash == hash)
            .ok_or(missing("Resource accumulation"))?
            .value;
        let demand = unit.wrapping_add(power);
        if accumulation < (demand >> 8).wrapping_shl(8) {
            return Err(missing("resource shortage Script hook"));
        }
    }
    let p = player(state, owner)?;
    let capacity = p.trade.outgoing_route_capacity;
    let old = p.trade.notified_outgoing_route_capacity;
    if capacity != old {
        if capacity > 0 && capacity > old {
            // The live graph must answer CountOutgoingRoutes before Send.
            return Err(missing("positive trade-capacity outgoing route count"));
        }
        // Native rereads capacity after the optional synchronous notification.
        let capacity = player(state, owner)?.trade.outgoing_route_capacity;
        state
            .activation_managers_mut(owner)
            .map_err(state_error)?
            .trade
            .notified_outgoing_route_capacity = capacity;
    }
    let _ = e;
    Ok(())
}
fn can_call_special_session(turn: i32, last: i32, minimum: i32, era: i32, initial: i32) -> bool {
    (turn.wrapping_sub(last) > minimum || turn == last) && era >= initial
}
/// The finite +9c0 Congress subscriber. This reads the same current Phase
/// definition at each source query, not the saved last-valid-segment scalar.
pub fn on_congress_turn_started(
    state: &mut LoadedState,
    defs: &Definitions,
    q: &Queries<'_>,
    runtime: &Runtime,
    phases: &super::Phases,
    e: &mut impl Effects,
    owner: i32,
) -> Result<(), Error> {
    defs.validate(state)?;
    runtime.validate(state)?;
    let current = runtime
        .congress
        .as_ref()
        .ok_or(missing("current Congress/Emergency owner"))?;
    let disabled = state
        .retained()
        .core
        .phase_processor
        .turn_segments
        .iter()
        .find(|v| v.hash == 0x2c2337c1)
        .is_none_or(|v| v.disabled);
    let elapsed = state
        .retained()
        .core
        .current_turn
        .wrapping_sub(current.last_special_session);
    let first_available = disabled && elapsed.wrapping_sub(defs.special_session_min_turns) == 1;
    // GetDiscussionsToDecideOn calls CanCallSpecialSession before segment gates.
    let era = state
        .game_eras()
        .map_err(|_| missing("Congress current era"))?
        .current_era()
        .ok_or(missing("Congress current era"))?;
    let _can_call = can_call_special_session(
        state.retained().core.current_turn,
        current.last_special_session,
        defs.special_session_min_turns,
        era,
        defs.congress_initial_era,
    );
    let segment = phases.current_segment(state)?;
    if segment != 0xb52a667b && segment != 0x2c2337c1 {
        for &other in q.alive_major_ids {
            // Player.Diplomacy.HasMet runs even for the eventual self case.
            let has_met = met(state, other, owner)?;
            if has_met || other == owner {
                // FindDiscussionOptions only considers current Emergency
                // instances reached through real Discussion definitions. With
                // an observed empty manager every EditEmergency returns null;
                // ConfirmEmergency and its mutations cannot be reached.
                if !current.emergencies.is_empty() {
                    return Err(missing("positive Congress Emergency discussion options"));
                }
            }
        }
    }
    // No options were produced by the reached current-state branch. The
    // non-disabled form still attempts the ordinary all-requirements notification.
    if !disabled {
        e.notification(state, owner, 0x7179594d)?;
    }
    let _ = first_available; // False options emit nothing in either disabled arm.
    match phases.current_segment(state)? {
        0xb52a667b => {
            e.congress_event(state, owner, 0xdabc2341)?;
            e.notification(state, owner, 0x3b627e38)?;
        }
        0x2c2337c1 => {
            e.congress_event(state, owner, 0x43b572fb)?;
            e.notification(state, owner, 0x3b627e38)?;
        }
        0xd74f83e5 => {
            e.congress_event(state, owner, 0x34b2426d)?;
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn unavailable_casus_belli_clears_stale_flags_and_preserves_wire_tag() {
        let mut row = c6_core::player_diplomacy::CasusBelliData {
            wire_version: 41,
            colonial_war: true,
            holy_war: true,
            liberation_war: true,
            protectorate_war: true,
            reconquest_war: true,
            territorial_war: true,
            golden_age_war: true,
            retribution_war: true,
            ideological_war: true,
        };
        super::clear_casus_belli(&mut row);
        assert_eq!(
            row,
            c6_core::player_diplomacy::CasusBelliData {
                wire_version: 41,
                ..Default::default()
            }
        );
    }

    use super::*;
    fn tracker() -> DiplomaticModifierTracker {
        DiplomaticModifierTracker {
            is_active: true,
            current_value: 0,
            initial_value: 0,
            max_value: 768,
            increment: 512,
            increment_turns: 2,
            cooldown_turns: 2,
            accumulation_turn_count: 0,
            reduction: 512,
            modifier_name: vec![],
            visibility: 0,
        }
    }
    #[test]
    fn tracker_count_and_native_clamps() {
        let mut t = tracker();
        update_tracker(&mut t);
        assert_eq!((t.current_value, t.accumulation_turn_count), (0, 1));
        update_tracker(&mut t);
        assert_eq!((t.current_value, t.accumulation_turn_count), (512, 0));
        update_tracker(&mut t);
        update_tracker(&mut t);
        assert_eq!(t.current_value, 768);
        t.is_active = false;
        t.current_value = 256;
        t.accumulation_turn_count = 1;
        update_tracker(&mut t);
        assert_eq!((t.current_value, t.accumulation_turn_count), (0, 0));
        t.current_value = -256;
        t.accumulation_turn_count = 1;
        update_tracker(&mut t);
        assert_eq!(t.current_value, 0);
    }
    #[test]
    fn special_session_uses_strict_elapsed_and_current_era() {
        assert!(!can_call_special_session(15, 0, 15, 2, 2));
        assert!(can_call_special_session(16, 0, 15, 2, 2));
        assert!(can_call_special_session(0, 0, 15, 2, 2));
        assert!(!can_call_special_session(16, 0, 15, 1, 2));
    }
    #[test]
    fn integer_speed_scaling_truncates_and_clamps_without_q8_quantization() {
        assert_eq!(scale(-1, 50), 0);
        assert_eq!(scale(30, 150), 45);
        assert_eq!(scale(i32::MAX, 200), i32::MAX);
        assert_eq!(scale(i32::MIN, 200), i32::MIN);
    }
    #[test]
    fn climate_reset_preserves_other_player_and_other_usage_words() {
        let mut rows = [
            ClimateConsumption {
                owner: 1,
                resource: 4,
                words: [123, 456, -789, -4],
            },
            ClimateConsumption {
                owner: 0,
                resource: 4,
                words: [1, 2, 3, 4],
            },
        ];
        reset_climate(&mut rows, 1);
        assert_eq!(rows[0].words, [123, 0, -789, 0]);
        assert_eq!(rows[1].words, [1, 2, 3, 4]);
    }
}
