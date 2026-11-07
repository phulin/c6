//! Linux XP2 RandomEvents::DoTurn (2a0f470), including deterministic candidate
//! compilation and its GameSync weighted draw. Positive event application is
//! an explicit continuation; it never becomes a player-policy choice or no-op.
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    unit_initialization_definitions::collection,
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    game_capabilities::Membership,
    game_random_events::{OmittedReadDefaults, RandomEvents},
    loaded_state::LoadedState,
    map::{Component, FeatureComponent, RiverComponent},
};

pub const CAPABILITY: u32 = 0x72ce2afb;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PossibleEvent {
    pub event: i32,
    pub severity: i32,
    pub river: i32,
    pub volcano: i32,
    pub natural_wonder: i32,
    pub start_plot: i32,
}
impl PossibleEvent {
    pub const NONE: Self = Self {
        event: -1,
        severity: -1,
        river: -1,
        volcano: -1,
        natural_wonder: -1,
        start_plot: -1,
    };
    fn event(event: i32, severity: i32) -> Self {
        Self {
            event,
            severity,
            ..Self::NONE
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Weight {
    pub index: i32,
    pub weight: i32,
}
#[derive(Clone, Copy, Debug)]
pub struct Scheduling {
    pub game_start_turn: i32,
    pub game_end_turn: i32,
    pub game_speed_hash: u32,
    pub local_player: i32,
    pub is_any_multiplayer: bool,
    pub configuration_realism: i32,
}
pub struct Initial {
    pub binding: WireBinding,
    pub wire: RandomEvents,
    pub omitted: OmittedReadDefaults,
    pub possible: Vec<PossibleEvent>,
    pub triggerable: Vec<PossibleEvent>,
    pub weights: Vec<Weight>,
    pub scheduling: Scheduling,
    /// None means the host allowance fallback is unavailable, never zero.
    pub fire_allowance: Option<i32>,
}
pub struct Runtime {
    binding: WireBinding,
    pub wire: RandomEvents,
    pub omitted: OmittedReadDefaults,
    pub possible: Vec<PossibleEvent>,
    pub triggerable: Vec<PossibleEvent>,
    pub weights: Vec<Weight>,
    pub scheduling: Scheduling,
    pub fire_allowance: Option<i32>,
    pub pending: Option<PossibleEvent>,
    /// A zero status draw reached an unsupported positive setter. Retain the
    /// direction so a retry cannot consume this GameSync draw again.
    pub pending_volcano_status: Option<bool>,
}
#[derive(Debug)]
pub enum Error {
    Missing(&'static str),
    Rules(crate::Error),
    Definitions(crate::unit_initialization_definitions::Error),
    State(c6_core::loaded_state::Error),
    Random(c6_core::loaded_state::game_sync::AccessError),
    Requires(&'static str),
    Pending(PossibleEvent),
}
impl Runtime {
    pub fn from_initial(state: &LoadedState, input: Initial) -> Result<Self, Error> {
        if input.binding != *state.binding() {
            return Err(Error::Missing("RandomEvents initial binding"));
        }
        if input.wire.wire_version != 5 {
            return Err(Error::Requires("RandomEvents wire version"));
        }
        if input
            .weights
            .iter()
            .any(|w| w.index < 0 || w.index as usize >= input.possible.len())
        {
            return Err(Error::Missing("RandomEvents weighted indices"));
        }
        Ok(Self {
            binding: input.binding,
            wire: input.wire,
            omitted: input.omitted,
            possible: input.possible,
            triggerable: input.triggerable,
            weights: input.weights,
            scheduling: input.scheduling,
            fire_allowance: input.fire_allowance,
            pending: None,
            pending_volcano_status: None,
        })
    }
    fn realism(&self) -> i32 {
        if self.omitted.realism_override_ordinal == -1 {
            self.scheduling.configuration_realism
        } else {
            self.omitted.realism_override_ordinal
        }
    }
    pub fn rates_q8(&self) -> [i32; 12] {
        [
            self.wire.storm_percent_chance_q8,
            self.wire.flood_percent_chance_q8,
            self.wire.eruption_percent_chance_q8,
            self.omitted.drought_percent_chance_q8,
            self.omitted.fire_percent_chance_q8,
            self.omitted.one_off_percent_chance_q8,
            self.wire.storm_climate_increased_chance_q8,
            self.wire.flood_climate_increased_chance_q8,
            self.wire.eruption_climate_increased_chance_q8,
            self.omitted.drought_climate_increased_chance_q8,
            self.omitted.fire_climate_increased_chance_q8,
            self.omitted.one_off_climate_increased_chance_q8,
        ]
    }
    fn set_rates(&mut self, r: [i32; 12]) {
        self.wire.storm_percent_chance_q8 = r[0];
        self.wire.flood_percent_chance_q8 = r[1];
        self.wire.eruption_percent_chance_q8 = r[2];
        self.omitted.drought_percent_chance_q8 = r[3];
        self.omitted.fire_percent_chance_q8 = r[4];
        self.omitted.one_off_percent_chance_q8 = r[5];
        self.wire.storm_climate_increased_chance_q8 = r[6];
        self.wire.flood_climate_increased_chance_q8 = r[7];
        self.wire.eruption_climate_increased_chance_q8 = r[8];
        self.omitted.drought_climate_increased_chance_q8 = r[9];
        self.omitted.fire_climate_increased_chance_q8 = r[10];
        self.omitted.one_off_climate_increased_chance_q8 = r[11];
    }
}
struct Event {
    index: i32,
    hash: u32,
    hexes: i32,
    duration: i32,
    operator: u32,
    severity: i32,
    natural_wonder: Option<u32>,
    unit_triggered: bool,
    chance_per_degree: i32,
    climate_points: i32,
    frequencies: Vec<(i32, f32)>,
}
struct Realism {
    percent_active: i32,
    climate_points: i32,
}
pub struct Definitions {
    start_turn: i32,
    first_boost: i32,
    events: Vec<Event>,
    realism: Vec<Realism>,
    standard_tiles: Option<i32>,
    river_names: LinuxDefinitionRegistry,
    volcano_names: LinuxDefinitionRegistry,
    features: LinuxDefinitionRegistry,
    speed_turns: Vec<(u32, i32)>,
}
fn int(row: crate::Row<'_>, column: &'static str) -> Result<i32, Error> {
    i32::try_from(row.integer(column).map_err(Error::Rules)?).map_err(|_| Error::Missing(column))
}
fn hash(row: crate::Row<'_>, column: &'static str) -> Result<Option<u32>, Error> {
    match row.get(column).map_err(Error::Rules)? {
        SqlValue::Null => Ok(None),
        SqlValue::Text(t) => Ok(Some(linux_type_hash(t))),
        _ => Err(Error::Missing(column)),
    }
}
impl Definitions {
    pub fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let table = |n| rules.gameplay().table(n).ok_or(Error::Missing(n));
        let registry = |t, c| collection(rules, t, c).map_err(Error::Definitions);
        let events = registry("RandomEvents", "RandomEventType")?;
        let realism = registry("RealismSettings", "RealismSettingType")?;
        let parameter = |name| -> Result<i32, Error> {
            let r = table("GlobalParameters")?
                .rows()
                .find(|r| r.text("Name").ok() == Some(name))
                .ok_or(Error::Missing(name))?;
            crate::game_parameters::sql_integer(r.get("Value").map_err(Error::Rules)?)
                .ok_or(Error::Missing(name))
        };
        let mut realisms = Vec::new();
        for entry in realism.entries() {
            let row = table("RealismSettings")?
                .rows()
                .find(|r| r.row_id() == Some(entry.database_rowid))
                .ok_or(Error::Missing("RealismSetting row"))?;
            realisms.push(Realism {
                percent_active: int(row, "PercentVolcanoesActive")?,
                climate_points: int(row, "ClimateChangePoints")?,
            });
        }
        let mut definitions = Vec::new();
        for (index, entry) in events.entries().iter().enumerate() {
            let row = table("RandomEvents")?
                .rows()
                .find(|r| r.row_id() == Some(entry.database_rowid))
                .ok_or(Error::Missing("RandomEvent row"))?;
            let mut frequencies = Vec::new();
            for frequency in table("RandomEvent_Frequencies")?.rows().filter(|r| {
                r.text("RandomEventType")
                    .ok()
                    .is_some_and(|n| n.as_bytes() == entry.type_name)
            }) {
                let level = hash(frequency, "RealismSettingType")?
                    .and_then(|h| realism.find_index(h))
                    .ok_or(Error::Missing("event realism reference"))?
                    as i32;
                let value = match frequency.get("OccurrencesPerGame").map_err(Error::Rules)? {
                    SqlValue::RealBits(v) => f64::from_bits(*v) as f32,
                    SqlValue::Integer(v) => *v as f32,
                    _ => return Err(Error::Missing("event frequency")),
                };
                frequencies.push((level, value));
            }
            definitions.push(Event {
                index: index as i32,
                hash: linux_type_hash(&entry.type_name),
                hexes: int(row, "Hexes")?,
                duration: int(row, "Duration")?,
                operator: hash(row, "EffectOperatorType")?.unwrap_or(u32::MAX),
                severity: int(row, "Severity")?,
                natural_wonder: hash(row, "NaturalWonder")?,
                unit_triggered: int(row, "UnitTriggered")? != 0,
                chance_per_degree: int(row, "ChanceIncreasePerDegree")?,
                climate_points: int(row, "ClimateChangePoints")?,
                frequencies,
            });
        }
        let standard = table("Maps")?
            .rows()
            .find(|r| r.text("MapSizeType").ok() == Some("MAPSIZE_STANDARD"));
        let standard_tiles = standard
            .map(|r| Ok::<_, Error>(int(r, "GridWidth")?.wrapping_mul(int(r, "GridHeight")?)))
            .transpose()?;
        let mut speed_turns = Vec::new();
        for row in table("GameSpeed_Turns")?.rows() {
            let speed = hash(row, "GameSpeedType")?.ok_or(Error::Missing("expected-turn speed"))?;
            let count = int(row, "TurnsPerIncrement")?;
            if let Some((_, v)) = speed_turns.iter_mut().find(|(h, _)| *h == speed) {
                *v = i32::wrapping_add(*v, count);
            } else {
                speed_turns.push((speed, count));
            }
        }
        Ok(Self {
            start_turn: parameter("RANDOM_EVENT_START_TURN")?,
            first_boost: parameter("RANDOM_EVENT_FIRST_TIME_OCCURRENCE_BOOST")?,
            events: definitions,
            realism: realisms,
            standard_tiles,
            river_names: registry("NamedRivers", "NamedRiverType")?,
            volcano_names: registry("NamedVolcanoes", "NamedVolcanoType")?,
            features: registry("Features", "FeatureType")?,
            speed_turns,
        })
    }
    fn expected_turns(&self, r: &Runtime) -> i32 {
        let n = r
            .scheduling
            .game_end_turn
            .wrapping_sub(r.scheduling.game_start_turn);
        if n > 0 {
            return n;
        }
        self.speed_turns
            .iter()
            .find(|(h, _)| *h == r.scheduling.game_speed_hash)
            .map(|(_, v)| *v)
            .filter(|v| *v > 0)
            .unwrap_or(500)
    }
}

/// This is the SAME Climate owner used by Climate.DoTurn and cityless
/// consumption. Implementations retain active-event effects and pending output.
pub trait Climate<P> {
    fn continue_events(
        &mut self,
        state: &mut LoadedState,
        random_events: &mut Runtime,
        backend: &mut crate::loaded_actions::stock_notifications::Runtime<'_, P>,
    ) -> Result<(), Error>;
    fn temperature_change(&self, state: &LoadedState) -> Result<f32, Error>;
    fn climate_level(&self, state: &LoadedState, realism_points: i32) -> Result<i32, Error>;
    fn last_sea_level_climate(&self) -> i32;
    fn send_notifications(
        &mut self,
        state: &mut LoadedState,
        backend: &mut crate::loaded_actions::stock_notifications::Runtime<'_, P>,
    ) -> Result<(), Error>;
    fn apply_event(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        definitions: &Definitions,
        event: PossibleEvent,
        backend: &mut crate::loaded_actions::stock_notifications::Runtime<'_, P>,
    ) -> Result<(), Error>;
}
fn cvtt(v: f32) -> i32 {
    if !v.is_finite() || v < i32::MIN as f32 || v >= 2_147_483_648.0 {
        i32::MIN
    } else {
        v as i32
    }
}
fn feature(map: &c6_core::map::Map) -> Result<&FeatureComponent, Error> {
    map.components
        .iter()
        .find_map(|c| {
            if let Component::Feature(f) = c {
                Some(f.as_ref())
            } else {
                None
            }
        })
        .ok_or(Error::Missing("RandomEvents map feature manager"))
}
fn rivers(map: &c6_core::map::Map) -> Result<&RiverComponent, Error> {
    map.components
        .iter()
        .find_map(|c| {
            if let Component::River(r) = c {
                Some(r.as_ref())
            } else {
                None
            }
        })
        .ok_or(Error::Missing("RandomEvents river manager"))
}
fn add(runtime: &mut Runtime, possible: PossibleEvent, weight: i32, triggerable: bool) {
    if triggerable {
        runtime.triggerable.push(possible);
    } else {
        let index = runtime.possible.len() as i32;
        runtime.possible.push(possible);
        runtime.weights.push(Weight { index, weight });
    }
}
fn allowed(runtime: &Runtime, event: &Event) -> Result<bool, Error> {
    if matches!(
        event.operator,
        0x91b41b84 | 0x91ddd92e | 0x7784fe95 | 0x312a5e66
    ) && runtime.scheduling.local_player != -1
    {
        let allowance = runtime
            .fire_allowance
            .ok_or(Error::Requires("event capability allowance fallback"))?;
        Ok(if runtime.scheduling.is_any_multiplayer {
            allowance == 1 || allowance == 2
        } else {
            allowance == 1
        })
    } else {
        Ok(true)
    }
}
fn rate_slot(operator: u32) -> Option<usize> {
    match operator {
        0xf927ed0a => Some(0),
        0x4916dab4 => Some(1),
        0x31fbdf53 | 0xdf6d7483 => Some(2),
        0x949223ba => Some(3),
        0x91b41b84 => Some(4),
        0x91ddd92e | 0xa4a5401b | 0x312a5e66 | 0x7784fe95 => Some(5),
        _ => None,
    }
}
fn climate_extra(base: i32, chance: i32, temperature: f32) -> i32 {
    cvtt(((base as f32) * (chance as f32)) * temperature) / 100
}

pub fn compile_possible_events(
    state: &LoadedState,
    defs: &Definitions,
    runtime: &mut Runtime,
    fallout: &super::round_fallout::Runtime,
    temperature: f32,
    climate_level: i32,
    last_sea_level: i32,
) -> Result<(), Error> {
    let map = state.world().map();
    let feature = feature(map)?;
    let river = rivers(map)?;
    runtime.set_rates([0; 12]);
    runtime.possible.clear();
    runtime.triggerable.clear();
    runtime.weights.clear();
    let realism = runtime.realism();
    let mut rates = [0i32; 12];
    let nuclear = 0i32;
    for event in &defs.events {
        let frequency = event
            .frequencies
            .iter()
            .find(|(r, _)| *r == realism)
            .map_or(0.0, |(_, f)| *f);
        if frequency.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
            || !allowed(runtime, event)?
        {
            continue;
        }
        let raw = cvtt(frequency * 10.0);
        let mut total = 0i32;
        let mut delta = 0i32;
        match event.operator {
            0x4916dab4 => {
                for r in &river.rivers {
                    if r.name_hash == u32::MAX || r.floodplains.is_empty() {
                        continue;
                    }
                    let name = defs
                        .river_names
                        .find_index(r.name_hash)
                        .ok_or(Error::Missing("event river name"))?
                        as i32;
                    let prior = r.floods.iter().any(|f| f.severity == event.severity);
                    let boost = if prior {
                        100
                    } else {
                        100i32.wrapping_add(defs.first_boost)
                    };
                    let base = boost.wrapping_mul(raw) / 100;
                    let extra = climate_extra(base, event.chance_per_degree, temperature);
                    let weight = base.wrapping_add(extra);
                    let mut p = PossibleEvent::event(event.index, event.severity);
                    p.river = name;
                    add(runtime, p, weight, event.unit_triggered);
                    total = total.wrapping_add(weight);
                    delta = delta.wrapping_add(extra);
                }
            }
            0x31fbdf53 => {
                for v in &feature.volcanoes {
                    if v.turn_active <= 0 || v.name_hash == u32::MAX {
                        continue;
                    }
                    let name = defs
                        .volcano_names
                        .find_index(v.name_hash)
                        .ok_or(Error::Missing("event volcano name"))?
                        as i32;
                    let prior = v.eruptions.iter().any(|e| e.severity == event.severity);
                    let boost = if prior {
                        100
                    } else {
                        100i32.wrapping_add(defs.first_boost)
                    };
                    let base = boost.wrapping_mul(raw) / 100;
                    let extra = climate_extra(base, event.chance_per_degree, temperature);
                    let weight = base.wrapping_add(extra);
                    let mut p = PossibleEvent::event(event.index, event.severity);
                    p.volcano = name;
                    add(runtime, p, weight, event.unit_triggered);
                    total = total.wrapping_add(weight);
                    delta = delta.wrapping_add(extra);
                }
            }
            0xdf6d7483 => {
                if let Some(hash) = event.natural_wonder {
                    for v in feature
                        .natural_wonder_volcanoes
                        .iter()
                        .filter(|v| v.feature_hash == hash)
                    {
                        let index = defs
                            .features
                            .find_index(hash)
                            .ok_or(Error::Missing("event natural wonder type"))?
                            as i32;
                        let prior = v.eruptions.iter().any(|e| e.severity == event.severity);
                        let boost = if prior {
                            100
                        } else {
                            100i32.wrapping_add(defs.first_boost)
                        };
                        let base = boost.wrapping_mul(raw) / 100;
                        let extra = climate_extra(base, event.chance_per_degree, temperature);
                        let weight = base.wrapping_add(extra);
                        let mut p = PossibleEvent::event(event.index, event.severity);
                        p.natural_wonder = index;
                        add(runtime, p, weight, event.unit_triggered);
                        total = total.wrapping_add(weight);
                        delta = delta.wrapping_add(extra);
                    }
                }
            }
            0x3965526f => {
                if fallout.reactor_count() != 0 {
                    return Err(Error::Requires("nuclear reactor event population"));
                }
            }
            0x876c673a => {} // SEA_LEVEL is handled by the ordered override pass.
            operator if rate_slot(operator).is_some() => {
                let base = if let Some(standard) = defs.standard_tiles {
                    if standard == 0 {
                        return Err(Error::Requires("zero standard map area"));
                    }
                    map.header
                        .width
                        .wrapping_mul(raw)
                        .wrapping_mul(map.header.height)
                        / standard
                } else {
                    raw
                };
                let extra = climate_extra(base, event.chance_per_degree, temperature);
                total = base.wrapping_add(extra);
                delta = extra;
                add(
                    runtime,
                    PossibleEvent::event(event.index, event.severity),
                    total,
                    event.unit_triggered,
                );
            }
            _ => return Err(Error::Requires("random event operator")),
        }
        if !event.unit_triggered {
            if let Some(slot) = rate_slot(event.operator) {
                rates[slot] = rates[slot].wrapping_add(total.wrapping_shl(8));
                rates[slot + 6] = rates[slot + 6].wrapping_add(delta.wrapping_shl(8));
            }
        }
    }
    let mut sea = false;
    for event in &defs.events {
        let frequency = event
            .frequencies
            .iter()
            .find(|(r, _)| *r == realism)
            .map_or(0.0, |(_, f)| *f);
        if frequency > 0.0
            && event.operator == 0x876c673a
            && last_sea_level < event.climate_points
            && climate_level >= event.climate_points
        {
            runtime.possible.clear();
            runtime.weights.clear();
            add(
                runtime,
                PossibleEvent::event(event.index, event.severity),
                1,
                false,
            );
            sea = true;
            break;
        }
    }
    if !runtime.possible.is_empty() {
        let maximum = defs.expected_turns(runtime).wrapping_mul(10);
        let total = rates[..6]
            .iter()
            .fold(nuclear, |s, &v| s.wrapping_add(v >> 8));
        let denominator = if maximum <= total {
            if total == 0 {
                1
            } else {
                total
            }
        } else {
            if !sea {
                add(
                    runtime,
                    PossibleEvent::NONE,
                    maximum.wrapping_sub(total),
                    false,
                );
            }
            if maximum == 0 {
                1
            } else {
                maximum
            }
        };
        for v in &mut rates {
            *v = ((*v >> 8).wrapping_mul(100) / denominator).wrapping_shl(8);
        }
    }
    runtime.set_rates(rates);
    Ok(())
}

/// ChangeVolcanoStatus2a0f590. Normal named count gates the operation;
/// percentage includes all normal volcanoes and always-active natural wonders.
fn volcano_status_draw(
    counts: (i32, i32, i32, i32),
    expected_turns: i32,
    target_percent: i32,
) -> Result<Option<(bool, u16)>, Error> {
    let (normal, named, active, wonders) = counts;
    if named <= 0 || expected_turns <= 0 {
        return Ok(None);
    }
    let divisor = normal.wrapping_mul(2);
    let mut bound = expected_turns
        .checked_div(divisor)
        .ok_or(Error::Requires("volcano status native division trap"))?;
    let total = normal.wrapping_add(wonders) as u32;
    let percent = if total == 0 {
        100
    } else {
        ((active.wrapping_add(wonders).wrapping_mul(100) as u32) / total) as i32
    };
    let activate = percent < target_percent;
    if activate {
        if named <= active {
            return Ok(None);
        }
        let deficit = target_percent.wrapping_sub(percent).wrapping_mul(named);
        if deficit >= 200 {
            bound = bound
                .checked_div(deficit / 100)
                .ok_or(Error::Requires("volcano status native division trap"))?;
        }
    } else if active <= 0 {
        return Ok(None);
    }
    // Native passes only the low16 bits to Game.GetRandNum, including zero.
    Ok(Some((activate, bound as u16)))
}

/// Executes the mandatory world-system leaf over current canonical state.
/// A selected positive event remains in `runtime.pending` until its effects
/// succeed; retrying the turn cannot silently consume its RNG a second time.
pub fn run<P, C: Climate<P>>(
    state: &mut LoadedState,
    definitions: &Definitions,
    runtime: &mut Runtime,
    fallout: &super::round_fallout::Runtime,
    climate: &mut C,
    capabilities: &Membership,
    backend: &mut crate::loaded_actions::stock_notifications::Runtime<'_, P>,
) -> Result<(), Error> {
    if runtime.binding != *state.binding() {
        return Err(Error::Missing("RandomEvents current binding"));
    }
    if runtime.pending_volcano_status.is_some() {
        return Err(Error::Requires("pending named volcano status setter"));
    }
    if let Some(pending) = runtime.pending {
        climate.apply_event(state, runtime, definitions, pending, backend)?;
        runtime.pending = None;
        climate.send_notifications(state, backend)?;
        backend.reached.push("Game.RandomEvents.DoTurn");
        return Ok(());
    }
    if !capabilities.has(CAPABILITY) {
        return Ok(());
    }
    let realism = runtime.realism();
    let level = definitions
        .realism
        .get(realism as usize)
        .ok_or(Error::Missing("current realism definition"))?;
    let threshold = runtime
        .scheduling
        .game_start_turn
        .wrapping_add(definitions.start_turn)
        .wrapping_sub(1);
    if state.retained().core.current_turn < threshold {
        return Ok(());
    }
    climate.continue_events(state, runtime, backend)?;
    let f = feature(state.world().map())?;
    let counts = (
        f.volcanoes.len() as i32,
        f.volcanoes
            .iter()
            .filter(|v| v.name_hash != u32::MAX)
            .count() as i32,
        f.volcanoes.iter().filter(|v| v.turn_active > 0).count() as i32,
        f.natural_wonder_volcanoes.len() as i32,
    );
    if let Some((activate, bound)) = volcano_status_draw(
        counts,
        definitions.expected_turns(runtime),
        level.percent_active,
    )? {
        if state.game_sync_rand(bound).map_err(Error::Random)? == 0 {
            runtime.pending_volcano_status = Some(activate);
            return Err(Error::Requires("named volcano status setter"));
        }
    }
    let temperature = climate.temperature_change(state)?;
    let current = climate.climate_level(state, level.climate_points)?;
    compile_possible_events(
        state,
        definitions,
        runtime,
        fallout,
        temperature,
        current,
        climate.last_sea_level_climate(),
    )?;
    if !runtime.possible.is_empty() {
        let total = runtime
            .weights
            .iter()
            .fold(0i32, |s, w| s.wrapping_add(w.weight));
        let draw = state.game_sync_rand(total as u16).map_err(Error::Random)?;
        let mut remaining = i32::from(draw);
        let mut selected = runtime
            .weights
            .last()
            .map(|w| w.index)
            .ok_or(Error::Missing("event weights"))?;
        for weight in &runtime.weights {
            selected = weight.index;
            remaining = remaining.wrapping_sub(weight.weight);
            if remaining < 0 {
                break;
            }
        }
        let event = *runtime
            .possible
            .get(selected as usize)
            .ok_or(Error::Missing("selected event index"))?;
        if event.event != -1 {
            runtime.pending = Some(event);
            climate.apply_event(state, runtime, definitions, event, backend)?;
            runtime.pending = None;
        }
    }
    climate.send_notifications(state, backend)?;
    backend.reached.push("Game.RandomEvents.DoTurn");
    Ok(())
}

/// Concrete stock bridge to the single Climate owner shared with world upkeep.
pub struct CurrentClimate<'a> {
    pub runtime: &'a mut super::round_climate::Runtime,
    pub cityless: &'a super::activation::cityless_managers::Runtime,
}
fn climate_error(error: super::round_climate::Error) -> Error {
    match error {
        super::round_climate::Error::Missing(m) => Error::Missing(m),
        super::round_climate::Error::Requires(m) => Error::Requires(m),
    }
}
impl<P> Climate<P> for CurrentClimate<'_> {
    fn continue_events(
        &mut self,
        state: &mut LoadedState,
        random_events: &mut Runtime,
        _backend: &mut crate::loaded_actions::stock_notifications::Runtime<'_, P>,
    ) -> Result<(), Error> {
        if !self.runtime.live.active_droughts.is_empty() {
            return Err(Error::Requires("Climate CheckForEndingDroughts"));
        }
        super::round_storm::continue_storms(state, self.runtime, random_events)?;
        if !self.runtime.live.active_one_offs.is_empty() {
            return Err(Error::Requires("Climate DoActiveOneOffs"));
        }
        Ok(())
    }
    fn temperature_change(&self, state: &LoadedState) -> Result<f32, Error> {
        super::round_climate::temperature_change(state, self.runtime, self.cityless)
            .map_err(climate_error)
    }
    fn climate_level(&self, state: &LoadedState, points: i32) -> Result<i32, Error> {
        super::round_climate::climate_change_level(state, self.runtime, self.cityless, points)
            .map_err(climate_error)
    }
    fn last_sea_level_climate(&self) -> i32 {
        self.runtime.manager.climate_change_for_last_sea_level_event
    }
    fn send_notifications(
        &mut self,
        state: &mut LoadedState,
        _backend: &mut crate::loaded_actions::stock_notifications::Runtime<'_, P>,
    ) -> Result<(), Error> {
        super::round_climate::send_notifications(state, self.runtime).map_err(climate_error)
    }
    fn apply_event(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        definitions: &Definitions,
        event: PossibleEvent,
        _backend: &mut crate::loaded_actions::stock_notifications::Runtime<'_, P>,
    ) -> Result<(), Error> {
        self.runtime.validate(state).map_err(climate_error)?;
        let definition = definitions
            .events
            .get(event.event as usize)
            .ok_or(Error::Missing("selected event definition"))?;
        if !allowed(runtime, definition)? {
            return Ok(());
        }
        // Linux296d883..296d89c derives the required tile count from Hexes.
        // FindRequiredTilesAroundPlot2971409..297140b returns false for zero,
        // even when its terrain predicate accepts the plot. Therefore ALL
        // scores are zero for Hexes<=2: no target draw, direction draw, active
        // storm, damage, name generation, callback, or notification is reached.
        // ApplyEvent nevertheless appends its initialized historical record.
        if definition.operator != 0xf927ed0a {
            return Err(Error::Pending(event));
        }
        let mut history = targetless_storm_history(
            definition,
            event,
            state.retained().core.current_turn,
            definitions,
        )?;
        if storm_required_tiles(definition.hexes) != 0 || event.start_plot != -1 {
            super::round_storm::apply(state, self.runtime, event, &mut history)?;
        }
        runtime.wire.historical_events.push(history);
        Ok(())
    }
}

fn storm_required_tiles(hexes: i32) -> i32 {
    if hexes > 18 {
        2
    } else {
        i32::from(hexes > 2)
    }
}

fn targetless_storm_history(
    definition: &Event,
    event: PossibleEvent,
    turn: i32,
    definitions: &Definitions,
) -> Result<c6_core::game_random_events::HistoricalEvent, Error> {
    let remap = |registry: &LinuxDefinitionRegistry, index: i32| {
        if index == -1 {
            Ok(u32::MAX)
        } else {
            registry
                .entries()
                .get(index as usize)
                .map(|entry| linux_type_hash(&entry.type_name))
                .ok_or(Error::Missing("event history definition remap"))
        }
    };
    // HistoricalEvent constructor2a13760, ApplyEvent2a0fa00. A failed
    // TriggerEvent leaves the initialized plots/direction/counters/name intact.
    Ok(c6_core::game_random_events::HistoricalEvent {
        wire_version: c6_core::game_random_events::HISTORY_VERSION,
        random_event_hash: definition.hash,
        named_river_hash: remap(&definitions.river_names, event.river)?,
        named_volcano_hash: remap(&definitions.volcano_names, event.volcano)?,
        natural_wonder_volcano_feature_hash: remap(&definitions.features, event.natural_wonder)?,
        start_plot: -1,
        current_plot: -1,
        current_direction_ordinal: -1,
        start_turn: turn,
        end_turn: turn.wrapping_add(definition.duration),
        tiles_damaged: 0,
        population_lost: 0,
        units_lost: 0,
        fertility_added: 0,
        name_bytes: Vec::new(),
        spread_raw: Some(u8::from(event.severity == -2)),
    })
}

#[cfg(test)]
mod continuation_tests {
    use super::*;

    #[test]
    fn volcano_status_uses_named_gate_all_normal_active_count_and_wonders() {
        assert_eq!(volcano_status_draw((8, 0, 0, 0), 500, 60).unwrap(), None);
        assert_eq!(volcano_status_draw((8, 1, 0, 0), 0, 60).unwrap(), None);
        assert_eq!(
            volcano_status_draw((8, 1, 0, 0), 500, 60).unwrap(),
            Some((true, 31))
        );
        // Already-active unnamed normals count too; named<=active suppresses activation.
        assert_eq!(volcano_status_draw((8, 1, 1, 0), 500, 60).unwrap(), None);
        // Natural wonders are always active and contribute to percentage only.
        assert_eq!(volcano_status_draw((2, 1, 0, 3), 500, 60).unwrap(), None);
        assert_eq!(
            volcano_status_draw((2, 1, 1, 0), 500, 50).unwrap(),
            Some((false, 125))
        );
    }
    #[test]
    fn volcano_status_accelerates_activation_and_preserves_low16_bound() {
        assert_eq!(
            volcano_status_draw((10, 10, 0, 0), 500, 60).unwrap(),
            Some((true, 4))
        );
        assert_eq!(
            volcano_status_draw((1, 1, 0, 0), 131074, 60).unwrap(),
            Some((true, 1))
        );
        assert_eq!(
            volcano_status_draw((10, 10, 0, 0), 1, 60).unwrap(),
            Some((true, 0))
        );
    }

    #[test]
    fn storm_required_tile_boundaries_match_linux_signed_comparisons() {
        for (hexes, required) in [(-1, 0), (0, 0), (1, 0), (2, 0), (3, 1), (18, 1), (19, 2)] {
            assert_eq!(storm_required_tiles(hexes), required);
        }
    }
}
