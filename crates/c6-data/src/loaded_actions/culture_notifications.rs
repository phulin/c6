//! Linux culture auto-notification queries over retained state and admitted rules.
//! Query readiness yields player decisions; no government, civic or policy is chosen.
//! Source initialization is distinct from observed live runtime and action parity.
use super::{choose_tech, notifications};
use crate::resource_binding::ResourceAuthority;
use crate::{
    registry::LinuxDefinitionRegistry, unit_initialization_definitions as definitions, Row, Rules,
    SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding, loaded_state::LoadedState, player_culture::Culture,
    player_prerequisites::IndexedBools,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    ChooseCivic,
    ConsiderGovernmentChange,
    FillCivicSlot,
}
impl Kind {
    pub fn from_hash(hash: u32) -> Option<Self> {
        match hash {
            0xe1d3d3ac => Some(Self::ChooseCivic),
            0xb6bfb191 => Some(Self::ConsiderGovernmentChange),
            0x6331a2b6 => Some(Self::FillCivicSlot),
            _ => None,
        }
    }
    pub fn type_hash(self) -> u32 {
        match self {
            Self::ChooseCivic => 0xe1d3d3ac,
            Self::ConsiderGovernmentChange => 0xb6bfb191,
            Self::FillCivicSlot => 0x6331a2b6,
        }
    }
    pub fn vtable(self) -> u64 {
        match self {
            Self::ChooseCivic => 0x3af4c50,
            Self::ConsiderGovernmentChange => 0x3af5880,
            Self::FillCivicSlot => 0x3af5c90,
        }
    }
    pub fn blocking(self) -> i32 {
        (match self {
            Self::ChooseCivic => 0x86bfebd1u32,
            Self::ConsiderGovernmentChange => 0x57bcfe42,
            Self::FillCivicSlot => 0xa0064eea,
        }) as i32
    }
}
#[derive(Debug)]
pub enum Error {
    Resources(definitions::Error),
    Configuration(super::configuration::Error),
    Rules(crate::Error),
    Tech(choose_tech::Error),
    Unsupported(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "culture auto notifications: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Apply only the exact subtype stores, after successful base Initialize.
/// The caller retains the candidate in place if a later query/text step fails.
pub fn initialize_in_place(kind: Kind, created: &mut notifications::Created) -> Result<(), Error> {
    if created.record.type_hash != kind.type_hash() || created.vtable_addresspoint != kind.vtable()
    {
        return Err(Error::Unsupported("culture subtype identity"));
    }
    created.record.native_199 = false;
    created.record.native_19a = true;
    created.record.native_18c = 0;
    created.default_blocking = kind.blocking();
    Ok(())
}

/// Actual current configuration/game fields, never precomputed predicate answers.
/// A caller-provided binding authenticates identity only, not native observation.
/// Missing fields stay missing until their source branch actually needs them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartingSource {
    CallerProvided,
    SourceConfigurationAndEraPrefix,
    RuntimeConfigurationAndEraPrefix,
}
/// Current getter fields carry no observation authority merely by construction.
impl StartingContext {
    pub fn from_current_fields(
        binding: WireBinding,
        start_era_hash: u32,
        game_start_turn: Option<i32>,
        current_game_era: Option<i32>,
    ) -> Self {
        Self {
            source: StartingSource::CallerProvided,
            binding,
            start_era_hash,
            game_start_turn,
            current_game_era,
        }
    }
}

/// Typed current owner/global/configuration inputs, never predicate answers.
/// The owner is a caller label: Culture itself carries no owner identity.
pub struct CurrentPlayer<'a> {
    pub binding: &'a WireBinding,
    pub owner: i32,
    pub culture: &'a Culture,
    pub city_count: usize,
    pub current_turn: i32,
    pub slot_status: Option<i32>,
}

pub struct StartingContext {
    source: StartingSource,
    pub binding: WireBinding,
    pub start_era_hash: u32,
    pub game_start_turn: Option<i32>,
    pub current_game_era: Option<i32>,
}
#[derive(Clone, Debug)]
struct Policy {
    slot: Option<i32>,
    explicit: bool,
    requires_government: Option<bool>,
    civic: Option<i32>,
    tech: Option<i32>,
    age_restricted: bool,
}
#[derive(Clone, Debug)]
struct Slot {
    any: bool,
}
pub struct Definitions {
    binding: WireBinding,
    civics: LinuxDefinitionRegistry,
    repeatable: Vec<bool>,
    xp2: LinuxDefinitionRegistry,
    hidden: Vec<bool>,
    governments: LinuxDefinitionRegistry,
    government_civics: Vec<Option<i32>>,
    policies: LinuxDefinitionRegistry,
    policy: Vec<Policy>,
    slots: Vec<Slot>,
    eras: LinuxDefinitionRegistry,
    start_eras: LinuxDefinitionRegistry,
    starting_change: Vec<u32>,
    require_cities: bool,
    allow_empty: bool,
    calendar: CalendarDefinitions,
    phases: Vec<Option<crate::unit_operation_manager::TurnPhaseDefinition>>,
}
fn boolean(row: Row<'_>, field: &'static str) -> Result<Option<bool>, Error> {
    match row.get(field).map_err(Error::Rules)? {
        SqlValue::Integer(0) => Ok(Some(false)),
        SqlValue::Integer(1) => Ok(Some(true)),
        SqlValue::Null => Ok(None),
        _ => Err(Error::Unsupported(field)),
    }
}
fn required_bool(row: Row<'_>, field: &'static str) -> Result<bool, Error> {
    boolean(row, field)?.ok_or(Error::Unsupported(field))
}
fn reference(
    row: Row<'_>,
    field: &'static str,
    registry: &LinuxDefinitionRegistry,
) -> Result<Option<i32>, Error> {
    match row.get(field).map_err(Error::Rules)? {
        SqlValue::Null => Ok(None),
        SqlValue::Text(v) => Ok(registry
            .find_index(crate::registry::linux_type_hash(v))
            .map(|i| i as i32)),
        _ => Err(Error::Unsupported(field)),
    }
}
fn rows<'a>(rules: &'a Rules, table: &'static str) -> Result<impl Iterator<Item = Row<'a>>, Error> {
    Ok(rules
        .gameplay()
        .table(table)
        .ok_or(Error::Unsupported(table))?
        .rows())
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        if state.binding() != bound.destination_binding() {
            return Err(Error::Unsupported("culture rules/state binding"));
        }
        definitions::Definitions::validate_resources(rules, bound).map_err(Error::Resources)?;
        let collection =
            |table, key| definitions::collection(rules, table, key).map_err(Error::Resources);
        let civics = collection("Civics", "CivicType")?;
        let xp2 = collection("Civics_XP2", "CivicType")?;
        let governments = collection("Governments", "GovernmentType")?;
        let policies = collection("Policies", "PolicyType")?;
        let policy_xp1 = collection("Policies_XP1", "PolicyType")?;
        let slots_registry = collection("GovernmentSlots", "GovernmentSlotType")?;
        let techs = collection("Technologies", "TechnologyType")?;
        let eras = collection("Eras", "EraType")?;
        let start_eras = collection("StartEras", "EraType")?;
        let repeatable = rows(rules, "Civics")?
            .map(|r| required_bool(r, "Repeatable"))
            .collect::<Result<_, _>>()?;
        let hidden = rows(rules, "Civics_XP2")?
            .map(|r| required_bool(r, "HiddenUntilPrereqComplete"))
            .collect::<Result<_, _>>()?;
        let government_civics = rows(rules, "Governments")?
            .map(|r| reference(r, "PrereqCivic", &civics))
            .collect::<Result<_, _>>()?;
        let age_restricted = rows(rules, "Policies_XP1")?
            .map(|r| {
                Ok(required_bool(r, "RequiresDarkAge")? || required_bool(r, "RequiresGoldenAge")?)
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let policy = rows(rules, "Policies")?
            .map(|r| {
                let hash = match r.get("PolicyType").map_err(Error::Rules)? {
                    SqlValue::Text(v) => crate::registry::linux_type_hash(v),
                    _ => return Err(Error::Unsupported("PolicyType")),
                };
                Ok(Policy {
                    slot: reference(r, "GovernmentSlotType", &slots_registry)?,
                    explicit: required_bool(r, "ExplicitUnlock")?,
                    requires_government: boolean(r, "RequiresGovernmentUnlock")?,
                    civic: reference(r, "PrereqCivic", &civics)?,
                    tech: reference(r, "PrereqTech", &techs)?,
                    age_restricted: policy_xp1
                        .find_index(hash)
                        .is_some_and(|i| age_restricted[i]),
                })
            })
            .collect::<Result<_, Error>>()?;
        let slots = rows(rules, "GovernmentSlots")?
            .map(|r| {
                Ok(Slot {
                    any: required_bool(r, "AllowsAnyPolicy")?,
                })
            })
            .collect::<Result<_, Error>>()?;
        let mut starting_change = Vec::new();
        for row in rows(rules, "StartingGovernments")? {
            if required_bool(row, "Change")? {
                if let SqlValue::Text(v) = row.get("Era").map_err(Error::Rules)? {
                    starting_change.push(crate::registry::linux_type_hash(v));
                } else {
                    return Err(Error::Unsupported("StartingGovernments.Era"));
                }
            }
        }
        let parameter = |name: &'static str| -> Result<bool, Error> {
            for row in rows(rules, "GlobalParameters")? {
                if matches!(row.get("Name").map_err(Error::Rules)?,SqlValue::Text(v) if v==name.as_bytes())
                {
                    return match row.get("Value").map_err(Error::Rules)? {
                        SqlValue::Text(v) if v == b"0" => Ok(false),
                        SqlValue::Text(v) if v == b"1" => Ok(true),
                        _ => Err(Error::Unsupported(name)),
                    };
                }
            }
            Err(Error::Unsupported(name))
        };
        Ok(Self {
            binding: state.binding().clone(),
            civics,
            repeatable,
            xp2,
            hidden,
            governments,
            government_civics,
            policies,
            policy,
            slots,
            eras,
            start_eras,
            starting_change,
            require_cities: parameter("POLICY_REQUIRE_CITIES")?,
            allow_empty: parameter("GOVERNMENT_ALLOW_EMPTY_POLICY_SLOTS")?,
            calendar: CalendarDefinitions::from_rules(rules)?,
            phases: bound.phases().slots().to_vec(),
        })
    }
    /// Source-only owner-local civic graph prefix Read/Validate projection.
    /// The GreatWorks tail is retained unclassified; no complete owner Read or
    /// observed final native runtime is certified by this bounded operation.
    /// Project a source prefix and retain the complete unclassified owner suffix.
    pub fn source_read_component(
        &self,
        state: &LoadedState,
        mut witness: c6_core::loaded_state::ComponentSnapshotWitness,
        limits: c6_core::unit::Limits,
    ) -> Result<GameCulture, Error> {
        let bytes = super::choose_tech::checked_component_source(state, &self.binding, &witness)
            .map_err(|_| Error::Unsupported("culture source binding"))?;
        let (nodes, costs_raw, ordered_types, used) =
            decode_graph_prefix(bytes, &self.civics, limits)?;
        if nodes.is_empty() || ordered_types.is_empty() {
            return Err(Error::Unsupported("Game.Culture.BuildCivics required"));
        }
        if witness.decoded_prefix_bytes != 0 && witness.decoded_prefix_bytes != used {
            return Err(Error::Unsupported("culture decoded prefix"));
        }
        witness.decoded_prefix_bytes = used;
        Ok(GameCulture {
            binding: self.binding.clone(),
            nodes,
            costs_raw,
            ordered_types,
            source: GraphSource::SourceReadPrefixNoRebuild,
            source_owner: bytes.to_vec(),
            source_witness: Some(witness),
        })
    }
    /// All three culture +0x30 wrappers pass GetPlayerID to the strategic
    /// processor leaf. The processor ignores owner and reads only current
    /// phase and its active segment; absence is true only for proven nulls.
    pub fn valid_for_phase(&self, state: &LoadedState) -> Result<bool, Error> {
        if self.binding != *state.binding() {
            return Err(Error::Unsupported("culture phase state binding"));
        }
        strategic_phase(
            &self.phases,
            state.retained().core.phase_processor.current_phase,
        )
    }
    pub fn civic_lookup(&self) -> &dyn c6_core::movement_world::DefinitionLookup {
        &self.civics
    }
    /// Compose current typed player, global and configuration fields. Checks
    /// establish consistent labels/bindings, not native observation authority.
    /// A source Read graph remains a source Read graph after composition.
    pub fn current_player<'a>(
        &'a self,
        owner: i32,
        current: CurrentPlayer<'a>,
        graph: Option<&'a GameCulture>,
        techs: Option<&'a choose_tech::Research<'a>>,
        starting: Option<&'a StartingContext>,
    ) -> Result<Queries<'a>, Error> {
        if current.binding != &self.binding || current.owner != owner {
            return Err(Error::Unsupported("current culture owner/binding"));
        }
        if graph.is_some_and(|graph| graph.binding != self.binding)
            || starting.is_some_and(|context| context.binding != self.binding)
            || techs.is_some_and(|research| research.binding() != &self.binding)
        {
            return Err(Error::Unsupported("current culture dependency binding"));
        }
        Ok(Queries {
            definitions: self,
            culture: current.culture,
            city_count: current.city_count,
            current_turn: current.current_turn,
            current_game_era: None,
            human: current.slot_status.map(|status| status == 3),
            graph,
            techs,
            starting,
        })
    }
    /// Retained-source adapter; it does not certify current runtime fields.
    pub fn player<'a>(
        &'a self,
        state: &'a LoadedState,
        owner: i32,
        graph: Option<&'a GameCulture>,
        techs: Option<&'a choose_tech::Research<'a>>,
        starting: Option<&'a StartingContext>,
    ) -> Result<Queries<'a>, Error> {
        if self.binding != *state.binding() {
            return Err(Error::Unsupported("culture state binding"));
        }
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.requested_owner == owner)
            .ok_or(Error::Unsupported("retained PlayerCulture owner"))?;
        Ok(Queries {
            definitions: self,
            culture: &player.culture,
            current_turn: state.retained().core.current_turn,
            current_game_era: state
                .game_eras()
                .ok()
                .and_then(|eras| eras.current_era()),
            human: state
                .player_slot_status(owner)
                .ok()
                .map(|status| status == 3),
            city_count: player.cities.records.entries.len(),
            graph,
            techs,
            starting,
        })
    }
}

/// Ordered query projection of actual Game.Culture nodes. No SQL edges inferred.
/// Caller-provided native fields are not an observed fixture or source Read proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CivicNode {
    pub native_type: i32,
    pub active: bool,
    pub prerequisites: Vec<i32>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphSource {
    CallerProvided,
    SourceReadPrefixNoRebuild,
}
pub struct GameCulture {
    binding: WireBinding,
    ordered_types: Vec<i32>,
    nodes: Vec<CivicNode>,
    source: GraphSource,
    source_witness: Option<c6_core::loaded_state::ComponentSnapshotWitness>,
    source_owner: Vec<u8>,
    costs_raw: Vec<i32>,
}
impl GameCulture {
    pub fn from_complete_native(
        binding: WireBinding,
        ordered_types: Vec<i32>,
        nodes: Vec<CivicNode>,
    ) -> Result<Self, Error> {
        if ordered_types.len() > i32::MAX as usize || nodes.len() > i32::MAX as usize {
            return Err(Error::Unsupported("native civic graph size"));
        }
        Ok(Self {
            binding,
            ordered_types,
            nodes,
            source: GraphSource::CallerProvided,
            source_witness: None,
            source_owner: Vec::new(),
            costs_raw: Vec::new(),
        })
    }
    pub fn binding(&self) -> &WireBinding {
        &self.binding
    }
    /// Original node order, retaining duplicates and exact stored active bytes.
    pub fn nodes(&self) -> &[CivicNode] {
        &self.nodes
    }
    /// Native HasCivic: positional matching node, then first matching type;
    /// absent nodes and -1 return false. Current source data must be established
    /// by the caller; this does not promote costs/prerequisites or graph rebuild.
    pub fn has_civic(&self, civic: i32) -> bool {
        self.node(civic).is_some_and(|node| node.active)
    }
    pub fn source(&self) -> GraphSource {
        self.source
    }
    pub fn source_witness(&self) -> Option<&c6_core::loaded_state::ComponentSnapshotWitness> {
        self.source_witness.as_ref()
    }
    /// Complete original owner preimage, including uninterpreted GreatWorks tail.
    pub fn source_owner(&self) -> &[u8] {
        &self.source_owner
    }
    pub fn costs_raw(&self) -> &[i32] {
        &self.costs_raw
    }
    fn node(&self, t: i32) -> Option<&CivicNode> {
        if t == -1 {
            return None;
        }
        self.nodes
            .get(t as u32 as usize)
            .filter(|n| n.native_type == t)
            .or_else(|| self.nodes.iter().find(|n| n.native_type == t))
    }
}
pub struct Queries<'a> {
    definitions: &'a Definitions,
    current_turn: i32,
    current_game_era: Option<i32>,
    human: Option<bool>,
    culture: &'a Culture,
    city_count: usize,
    graph: Option<&'a GameCulture>,
    techs: Option<&'a choose_tech::Research<'a>>,
    starting: Option<&'a StartingContext>,
}
impl Queries<'_> {
    fn graph(&self) -> Result<&GameCulture, Error> {
        let g = self.graph.ok_or(Error::Unsupported(
            "retained Game.Culture active nodes/prerequisites/order",
        ))?;
        if g.binding != self.definitions.binding {
            return Err(Error::Unsupported("Game.Culture binding"));
        }
        Ok(g)
    }
    fn context(&self) -> Result<&StartingContext, Error> {
        let c = self.starting.ok_or(Error::Unsupported(
            "current Configuration.StartEra and Game.GameStartTurn",
        ))?;
        if c.binding != self.definitions.binding {
            return Err(Error::Unsupported("culture configuration binding"));
        }
        Ok(c)
    }
    fn active(&self, t: i32) -> Result<bool, Error> {
        Ok(self.graph()?.node(t).is_some_and(|n| n.active))
    }
    fn owned_bool(
        &self,
        array: &IndexedBools,
        registry: &LinuxDefinitionRegistry,
        t: i32,
    ) -> Result<bool, Error> {
        let Some(index) = usize::try_from(t)
            .ok()
            .filter(|i| *i < registry.entries().len())
        else {
            return Ok(false);
        };
        array
            .pairs
            .iter()
            .rfind(|p| registry.find_index(p.definition_hash) == Some(index))
            .map(|p| p.value)
            .ok_or(Error::Unsupported(
                "current PlayerCulture indexed bool value",
            ))
    }
    pub fn has_civic(&self, t: i32) -> Result<bool, Error> {
        if t == -1 || t as u32 as usize >= self.definitions.civics.entries().len() {
            return Ok(false);
        }
        if !self.active(t)? {
            return Ok(false);
        }
        self.owned_bool(&self.culture.has_civic, &self.definitions.civics, t)
    }
    fn boosted(&self, t: i32) -> Result<bool, Error> {
        if t == -1 || t as u32 as usize >= self.definitions.civics.entries().len() {
            return Ok(false);
        }
        if !self.active(t)? {
            return Ok(false);
        }
        self.owned_bool(&self.culture.boost_triggered, &self.definitions.civics, t)
    }
    pub fn revealed(&self, t: i32) -> Result<bool, Error> {
        let Some(index) = self.definitions.civics.find_index(t as u32) else {
            return Ok(false);
        };
        let hash =
            crate::registry::linux_type_hash(&self.definitions.civics.entries()[index].type_name);
        let hidden = self
            .definitions
            .xp2
            .find_index(hash)
            .is_some_and(|i| self.definitions.hidden[i]);
        if !hidden || self.has_civic(t)? || self.boosted(t)? {
            return Ok(true);
        }
        if !self.active(t)? {
            return Ok(false);
        }
        let prereqs = &self
            .graph()?
            .node(t)
            .ok_or(Error::Unsupported("active civic node"))?
            .prerequisites;
        if prereqs.is_empty() {
            return Ok(true);
        }
        for &p in prereqs {
            if self.has_civic(p)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub fn can_progress(&self, t: i32) -> Result<bool, Error> {
        if t == -1 || self.definitions.civics.at(t as u32).is_none() {
            return Ok(false);
        }
        if !self.active(t)? || !self.revealed(t)? {
            return Ok(false);
        }
        if self.has_civic(t)? && !self.definitions.repeatable[t as u32 as usize] {
            return Ok(false);
        }
        for &p in &self
            .graph()?
            .node(t)
            .ok_or(Error::Unsupported("active civic node"))?
            .prerequisites
        {
            if !self.has_civic(p)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub fn more_civics(&self) -> Result<bool, Error> {
        for &t in &self.graph()?.ordered_types {
            if let Some(i) = self.definitions.civics.find_index(t as u32) {
                if self.definitions.repeatable[i] || !self.has_civic(t)? {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    fn mapped_civic(&self, hash: u32) -> i32 {
        self.definitions
            .civics
            .find_index(hash)
            .map(|i| i as i32)
            .unwrap_or(-1)
    }
    pub fn progressing_civic(&self) -> i32 {
        self.culture
            .civic_queue_hashes
            .first()
            .map(|h| self.mapped_civic(*h))
            .unwrap_or(-1)
    }
    fn starting_government_notification(&self) -> Result<bool, Error> {
        let context = self.context()?;
        let Some(era) = self.definitions.eras.find_index(context.start_era_hash) else {
            return Ok(false);
        };
        // StartEra::Find receives the era native index, preserving Find's direct-index priority.
        if self.definitions.start_eras.find_index(era as u32).is_none() {
            return Ok(false);
        }
        let start = context
            .game_start_turn
            .ok_or(Error::Unsupported("current Game.GameStartTurn"))?;
        if self.current_turn.wrapping_sub(start) > 1 {
            return Ok(false);
        }
        if !self
            .human
            .ok_or(Error::Unsupported("current human configuration"))?
        {
            return Ok(false);
        }
        let era_hash =
            crate::registry::linux_type_hash(&self.definitions.eras.entries()[era].type_name);
        Ok(self.definitions.starting_change.contains(&era_hash))
    }
    fn current_government(&self) -> i32 {
        self.definitions
            .governments
            .find_index(self.culture.current_government_hash)
            .map(|i| i as i32)
            .unwrap_or(-1)
    }
    pub fn num_policy_slots(&self) -> i32 {
        if self.current_government() == -1 {
            0
        } else {
            self.culture.active_policies.entries.len() as i32
        }
    }
    pub fn num_policy_slots_open(&self) -> i32 {
        if self.num_policy_slots() == 0 {
            return 0;
        }
        self.culture
            .active_policies
            .entries
            .iter()
            .fold(0i32, |count, s| {
                count.wrapping_add(i32::from(
                    self.definitions
                        .policies
                        .find_index(s.policy_hash)
                        .is_none(),
                ))
            })
    }
    pub fn policy_unlocked(&self, t: i32) -> Result<bool, Error> {
        if self.owned_bool(&self.culture.policy_unlocked, &self.definitions.policies, t)? {
            return Ok(true);
        }
        let Some(i) = self.definitions.policies.find_index(t as u32) else {
            return Err(Error::Unsupported(
                "native null policy definition dereference",
            ));
        };
        let p = &self.definitions.policy[i];
        if p.explicit || p.requires_government == Some(true) {
            return Ok(false);
        }
        if let Some(civic) = p.civic {
            if !self.has_civic(civic)? {
                return Ok(false);
            }
        }
        if let Some(tech) = p.tech {
            if !self
                .techs
                .ok_or(Error::Unsupported("current PlayerTechs/GameTechs query"))?
                .has_tech(tech)
                .map_err(Error::Tech)?
            {
                return Ok(false);
            }
        }
        if p.age_restricted {
            let c = self.context()?;
            let current = self
                .current_game_era
                .or(c.current_game_era)
                .ok_or(Error::Unsupported("current Game.Eras.CurrentEra"))?;
            let start = self
                .definitions
                .eras
                .find_index(c.start_era_hash)
                .map(|i| i as i32)
                .unwrap_or(-1);
            if current == start {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub fn can_fill_all_policy_slots(&self) -> Result<bool, Error> {
        let mut counts = Vec::with_capacity(self.definitions.slots.len());
        for (i, slot) in self.definitions.slots.iter().enumerate() {
            let needed = if self.current_government() == -1 {
                0
            } else {
                self.culture
                    .active_policies
                    .entries
                    .iter()
                    .filter(|s| s.slot_ordinal == i as i32)
                    .count() as i32
            };
            let mut available = 0i32;
            for (p, definition) in self.definitions.policy.iter().enumerate() {
                if self.policy_unlocked(p as i32)? && definition.slot == Some(i as i32) {
                    available = available.wrapping_add(1)
                }
            }
            counts.push(FillInfo {
                any: slot.any,
                needed,
                available,
            });
        }
        Ok(fill_counts(&mut counts))
    }
    pub fn is_expired(&self, kind: Kind) -> Result<bool, Error> {
        match kind {
            Kind::ChooseCivic => {
                let progressing = self.progressing_civic();
                if self.city_count == 0 || !self.more_civics()? {
                    return Ok(true);
                }
                if progressing == -1 {
                    return Ok(false);
                }
                self.can_progress(progressing)
            }
            Kind::ConsiderGovernmentChange => {
                let starting = self.starting_government_notification()?;
                if self.city_count == 0 || !self.culture.civic_completed_this_turn {
                    return Ok(true);
                }
                let civic = self.mapped_civic(self.culture.civic_completed_this_turn_hash);
                let unlocks = self.definitions.government_civics.contains(&Some(civic));
                Ok(!(starting || unlocks) || self.culture.government_change_considered)
            }
            Kind::FillCivicSlot => {
                let no_cities = self.definitions.require_cities && self.city_count == 0;
                let anarchy = self.culture.anarchy_end_turn >= 0;
                if no_cities || anarchy {
                    return Ok(true);
                }
                let open = self.num_policy_slots_open();
                let needed = if self.definitions.allow_empty {
                    open == self.num_policy_slots()
                } else {
                    open > 0
                };
                // Native always calls CanFillAllPolicySlots, even when needed=false.
                let can_fill = self.can_fill_all_policy_slots()?;
                Ok(!(needed && can_fill))
            }
        }
    }
    /// FillCivicSlot2d2c9f0 and ConsiderGovernmentChange2d2b600 are five-byte
    /// tail jumps to Instance::IsNeeded; ChooseCivic inherits it directly.
    /// No wrapper calls IsValidForPhase or evaluates expiry before this flag.
    pub fn is_needed(&self, kind: Kind, expire_when_unneeded: bool) -> Result<bool, Error> {
        if !expire_when_unneeded {
            return Ok(true);
        }
        Ok(!self.is_expired(kind)?)
    }
}
#[derive(Clone, Copy, Debug)]
struct FillInfo {
    any: bool,
    needed: i32,
    available: i32,
}
fn fill_counts(counts: &mut [FillInfo]) -> bool {
    for count in counts.iter_mut() {
        if !count.any && count.needed > 0 {
            let used = count.available.min(count.needed);
            count.available = count.available.wrapping_sub(used);
            count.needed = count.needed.wrapping_sub(used);
        }
    }
    for i in 0..counts.len() {
        if counts[i].any && counts[i].needed > 0 {
            for j in 0..counts.len() {
                let used = counts[j].available.min(counts[i].needed);
                counts[j].available = counts[j].available.wrapping_sub(used);
                counts[i].needed = counts[i].needed.wrapping_sub(used);
                if counts[i].needed == 0 {
                    break;
                }
            }
        }
    }
    !counts.iter().any(|c| c.needed > 0)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn policy_slot_matching_reserves_specific_cards_before_wildcards() {
        let mut slots = [
            FillInfo {
                any: true,
                needed: 2,
                available: 0,
            },
            FillInfo {
                any: false,
                needed: 1,
                available: 2,
            },
            FillInfo {
                any: false,
                needed: 0,
                available: 1,
            },
        ];
        assert!(fill_counts(&mut slots));
        assert!(slots.iter().all(|c| c.needed == 0));
        let mut insufficient = [
            FillInfo {
                any: true,
                needed: 2,
                available: 0,
            },
            FillInfo {
                any: false,
                needed: 1,
                available: 2,
            },
        ];
        assert!(!fill_counts(&mut insufficient));
        let mut wrong_type = [
            FillInfo {
                any: false,
                needed: 1,
                available: 0,
            },
            FillInfo {
                any: false,
                needed: 0,
                available: 100,
            },
        ];
        assert!(!fill_counts(&mut wrong_type));
    }
}

// Current version6 graph prefix. Native version<5 rebuild and historical cost
// conversion are deliberately unavailable here; no unknown suffix is consumed.
struct GraphReader<'a> {
    bytes: &'a [u8],
    pos: usize,
    items: usize,
    limits: c6_core::unit::Limits,
}
impl GraphReader<'_> {
    fn raw(&mut self, n: usize) -> Result<&[u8], Error> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or(Error::Unsupported("culture graph resource limit"))?;
        if end > self.limits.max_bytes {
            return Err(Error::Unsupported("culture graph byte limit"));
        }
        let bytes = self
            .bytes
            .get(self.pos..end)
            .ok_or(Error::Unsupported("truncated culture graph"))?;
        self.pos = end;
        Ok(bytes)
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.raw(4)?.try_into().unwrap()))
    }
    fn version(&mut self) -> Result<(), Error> {
        if self.u32()? != 6 {
            return Err(Error::Unsupported("culture graph current version6"));
        }
        Ok(())
    }
    fn count<T>(&mut self, minimum: usize) -> Result<usize, Error> {
        let n = self.u32()? as usize;
        self.items = self
            .items
            .checked_add(n)
            .ok_or(Error::Unsupported("culture graph item limit"))?;
        let bytes = n
            .checked_mul(minimum)
            .and_then(|n| n.checked_add(self.pos))
            .ok_or(Error::Unsupported("culture graph byte limit"))?;
        let allocation = n
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(Error::Unsupported("culture graph allocation limit"))?;
        if self.items > self.limits.max_items
            || bytes > self.limits.max_bytes
            || allocation > self.limits.max_bytes
            || allocation > isize::MAX as usize
        {
            return Err(Error::Unsupported("culture graph resource limit"));
        }
        if bytes > self.bytes.len() {
            return Err(Error::Unsupported("truncated culture graph"));
        }
        Ok(n)
    }
}
type GraphPrefix = (Vec<CivicNode>, Vec<i32>, Vec<i32>, usize);
fn decode_graph_prefix(
    bytes: &[u8],
    registry: &LinuxDefinitionRegistry,
    limits: c6_core::unit::Limits,
) -> Result<GraphPrefix, Error> {
    let mut r = GraphReader {
        bytes,
        pos: 0,
        items: 0,
        limits,
    };
    r.version()?;
    if registry.entries().len() > i32::MAX as usize {
        return Err(Error::Unsupported("civic registry size"));
    }
    let remap = |hash| registry.find_index(hash).map(|i| i as i32).unwrap_or(-1);
    let valid = |index: i32| registry.find_index(index as u32).is_some();
    let n = r.count::<CivicNode>(17)?;
    let mut nodes = Vec::with_capacity(n);
    let mut costs = Vec::with_capacity(n);
    for _ in 0..n {
        r.version()?;
        let native_type = remap(r.u32()?);
        let active = match r.raw(1)?[0] {
            0 => false,
            1 => true,
            _ => return Err(Error::Unsupported("culture graph bool")),
        };
        let cost = r.u32()? as i32;
        let count = r.count::<i32>(4)?;
        let mut prerequisites = Vec::with_capacity(count);
        for _ in 0..count {
            let t = remap(r.u32()?);
            if valid(t) {
                prerequisites.push(t)
            }
        }
        if valid(native_type) {
            nodes.push(CivicNode {
                native_type,
                active,
                prerequisites,
            });
            costs.push(cost)
        }
    }
    let count = r.count::<i32>(4)?;
    let mut iteration = Vec::with_capacity(count);
    for _ in 0..count {
        let t = remap(r.u32()?);
        if valid(t) {
            iteration.push(t)
        }
    }
    Ok((nodes, costs, iteration, r.pos))
}

#[cfg(test)]
mod graph_tests {
    use super::*;
    #[test]
    fn activity_lookup_preserves_positional_duplicates_and_first_fallback() {
        use c6_core::core_player::initialization::{DatabaseIdentity, InputIdentities, WireRole};
        let binding = WireBinding {
            role: WireRole::SavedOutputCore,
            core_sha256: [1; 32],
            occurrence_id: b"1".to_vec(),
            output_id: Some(b"1".to_vec()),
            inputs: InputIdentities {
                configuration_sha256: [2; 32],
                rules_inputs_sha256: [3; 32],
                database: DatabaseIdentity::LiveSnapshotUnavailable,
                dlc_inputs_sha256: [4; 32],
                mods_inputs_sha256: [5; 32],
                content_inputs_sha256: [6; 32],
            },
        };
        let graph = GameCulture::from_complete_native(
            binding.clone(),
            vec![],
            vec![
                CivicNode {
                    native_type: 1,
                    active: false,
                    prerequisites: vec![],
                },
                CivicNode {
                    native_type: 1,
                    active: true,
                    prerequisites: vec![],
                },
                CivicNode {
                    native_type: 0,
                    active: true,
                    prerequisites: vec![],
                },
            ],
        )
        .unwrap();
        assert!(graph.has_civic(1)); // Matching position overrides earlier false duplicate.
        assert!(graph.has_civic(0)); // Position mismatches, then first matching type.
        assert!(!graph.has_civic(-1));
        assert!(!graph.has_civic(2));
        assert!(!graph.has_civic(i32::MAX));
        assert_eq!(graph.nodes().len(), 3);
        assert_eq!(graph.binding(), &binding);
    }

    #[test]
    fn v6_prefix_remaps_and_filters_stably_without_consuming_suffix() {
        let registry = LinuxDefinitionRegistry::from_native_order(vec![
            crate::registry::DefinitionEntry {
                database_rowid: crate::RulesRowId(100),
                type_name: b"CIVIC_A".to_vec(),
            },
            crate::registry::DefinitionEntry {
                database_rowid: crate::RulesRowId(9),
                type_name: b"CIVIC_B".to_vec(),
            },
        ]);
        let a = crate::registry::linux_type_hash(b"CIVIC_A");
        let b = crate::registry::linux_type_hash(b"CIVIC_B");
        let mut bytes = Vec::new();
        let word = |bytes: &mut Vec<u8>, v: u32| bytes.extend_from_slice(&v.to_le_bytes());
        word(&mut bytes, 6);
        word(&mut bytes, 3);
        for (hash, active, cost, prereqs) in [
            (b, 1u8, -123i32, vec![a, 0xdeadbeef]),
            (0xdeadbeef, 0, 5, vec![]),
            (a, 1, 44, vec![]),
        ] {
            word(&mut bytes, 6);
            word(&mut bytes, hash);
            bytes.push(active);
            word(&mut bytes, cost as u32);
            word(&mut bytes, prereqs.len() as u32);
            for p in prereqs {
                word(&mut bytes, p)
            }
        }
        word(&mut bytes, 4);
        for p in [b, a, b, 0xdeadbeef] {
            word(&mut bytes, p)
        }
        let prefix = bytes.len();
        bytes.extend_from_slice(&[255; 7]);
        let limits = c6_core::unit::Limits {
            max_bytes: 1 << 16,
            max_items: 100,
        };
        let (nodes, costs, iteration, used) =
            decode_graph_prefix(&bytes, &registry, limits).unwrap();
        assert_eq!(used, prefix);
        assert_eq!(costs, [-123, 44]);
        assert_eq!(iteration, [1, 0, 1]);
        assert_eq!(nodes[0].prerequisites, [0]);
        assert_eq!(nodes[0].native_type, 1);
        for end in 0..prefix {
            assert!(
                decode_graph_prefix(&bytes[..end], &registry, limits).is_err(),
                "truncation {end}"
            );
        }
        assert!(decode_graph_prefix(
            &bytes,
            &registry,
            c6_core::unit::Limits {
                max_items: 1,
                ..limits
            }
        )
        .is_err());
        bytes[16] = 2;
        assert!(decode_graph_prefix(&bytes, &registry, limits).is_err());
    }
}

#[derive(Clone, Debug)]
struct SpeedTurn {
    month_increment: i32,
    turns: i32,
}
struct CalendarDefinitions {
    months: i32,
    speeds: LinuxDefinitionRegistry,
    speed_turns: Vec<Vec<SpeedTurn>>,
    start_years: Vec<(u32, i32)>,
}
fn integer(row: Row<'_>, field: &'static str) -> Result<i32, Error> {
    match row.get(field).map_err(Error::Rules)? {
        SqlValue::Integer(v) => i32::try_from(*v).map_err(|_| Error::Unsupported(field)),
        _ => Err(Error::Unsupported(field)),
    }
}
impl CalendarDefinitions {
    fn from_rules(rules: &Rules) -> Result<Self, Error> {
        let speeds = definitions::collection(rules, "GameSpeeds", "GameSpeedType")
            .map_err(Error::Resources)?;
        let mut speed_turns = vec![Vec::new(); speeds.entries().len()];
        // Actual GameSpeedGameSpeedTurnCollection property SQL2293e00 filters
        // by GameSpeedType and orders T1.rowid ASC; no numeric SQL row-ID cast.
        for row in rows(rules, "GameSpeed_Turns")? {
            if let Some(i) = reference(row, "GameSpeedType", &speeds)? {
                speed_turns[i as usize].push(SpeedTurn {
                    month_increment: integer(row, "MonthIncrement")?,
                    turns: integer(row, "TurnsPerIncrement")?,
                });
            }
        }
        let months = i32::try_from(rows(rules, "Months")?.count())
            .map_err(|_| Error::Unsupported("month count"))?
            .max(1);
        let mut start_years = Vec::new();
        for row in rows(rules, "StartEras")? {
            let hash = match row.get("EraType").map_err(Error::Rules)? {
                SqlValue::Text(v) => crate::registry::linux_type_hash(v),
                _ => return Err(Error::Unsupported("StartEras.EraType")),
            };
            start_years.push((hash, integer(row, "Year")?));
        }
        Ok(Self {
            months,
            speeds,
            speed_turns,
            start_years,
        })
    }
    fn initializer_start_year(&self, explicit: Option<i32>, era: u32) -> i32 {
        explicit.unwrap_or_else(|| {
            if era == u32::MAX {
                -4000
            } else {
                self.start_years
                    .iter()
                    .find(|(h, _)| *h == era)
                    .map(|(_, year)| *year)
                    .unwrap_or(-4000)
            }
        })
    }
    fn calendar_start_year(&self, calendar: u32, era_hash: u32) -> i32 {
        if calendar == 0 || calendar == 0x32ea8797 {
            return -4000;
        }
        self.start_years
            .iter()
            .find(|(h, _)| *h == era_hash)
            .map(|(_, year)| *year)
            .unwrap_or(-4000)
    }
    /// Exact GetTurnFromDate path for month0 used by Initializer::GetStartTurn.
    /// Signed idiv truncates toward zero; every native add/sub/mul wraps i32.
    fn start_turn(
        &self,
        calendar: u32,
        year: i32,
        speed: u32,
        era_hash: u32,
    ) -> Result<i32, Error> {
        let origin = self.calendar_start_year(calendar, era_hash);
        let delta = year.max(origin).wrapping_sub(origin);
        let months = delta.wrapping_mul(self.months);
        let turn = match calendar {
            0 | 0x32ea8797 | 0x9e758cfc => {
                let index = self.speeds.find_index(speed).unwrap_or(0);
                if let Some(rows) = self.speed_turns.get(index) {
                    speed_turn_from_months(months, delta, rows)?
                } else {
                    0
                }
            }
            0xdc276bed | 0xfc6001f0 => delta,
            0xa4d8e9ac => 0, // month0 remainder divided by positive season count
            0x0b2a46b2 => months,
            _ => 0,
        };
        Ok(turn.wrapping_add(1))
    }
}
fn signed_div(numerator: i32, denominator: i32) -> Result<i32, Error> {
    numerator
        .checked_div(denominator)
        .ok_or(Error::Unsupported("native calendar division trap"))
}
fn speed_turn_from_months(months: i32, delta: i32, rows: &[SpeedTurn]) -> Result<i32, Error> {
    if rows.is_empty() {
        return Ok(delta);
    }
    let mut consumed = 0i32;
    let mut turns = 0i32;
    for row in rows {
        let threshold = consumed.wrapping_add(row.month_increment.wrapping_mul(row.turns));
        if months <= threshold {
            return Ok(
                signed_div(months.wrapping_sub(consumed), row.month_increment.max(1))?
                    .wrapping_add(turns),
            );
        }
        consumed = threshold;
        turns = turns.wrapping_add(row.turns);
    }
    let last = &rows[rows.len() - 1];
    Ok(signed_div(months.wrapping_sub(consumed), last.month_increment.max(1))?.wrapping_add(turns))
}
impl Definitions {
    /// Source-derived start turn from raw configuration fields and admitted
    /// calendar/speed/era definitions; no computed start-turn answer is accepted.
    pub fn starting_context_from_raw_configuration(
        &self,
        state: &LoadedState,
        start_era: u32,
        start_year: Option<i32>,
        calendar_type: u32,
        game_speed_type: u32,
        current_game_era: Option<i32>,
    ) -> Result<StartingContext, Error> {
        if self.binding != *state.binding() {
            return Err(Error::Unsupported("calendar state binding"));
        }
        let year = self.calendar.initializer_start_year(start_year, start_era);
        // Calendar.GetStartYear first maps either index or hash through Eras.
        let era_hash = if (start_era as usize) < self.eras.entries().len() {
            crate::registry::linux_type_hash(&self.eras.entries()[start_era as usize].type_name)
        } else if start_era > 0x7ff {
            start_era
        } else {
            u32::MAX
        };
        let turn = self
            .calendar
            .start_turn(calendar_type, year, game_speed_type, era_hash)?;
        Ok(StartingContext {
            binding: self.binding.clone(),
            start_era_hash: start_era,
            game_start_turn: Some(turn),
            current_game_era,
            source: StartingSource::CallerProvided,
        })
    }
}
#[cfg(test)]
mod calendar_tests {
    use super::*;
    #[test]
    fn ordered_segments_keep_boundary_rounding_and_last_segment_extension() {
        let rows = [
            SpeedTurn {
                month_increment: 120,
                turns: 2,
            },
            SpeedTurn {
                month_increment: 60,
                turns: 3,
            },
        ];
        for (months, expected) in [
            (0, 0),
            (119, 0),
            (120, 1),
            (239, 1),
            (240, 2),
            (299, 2),
            (300, 3),
            (420, 5),
            (480, 6),
        ] {
            assert_eq!(speed_turn_from_months(months, 0, &rows).unwrap(), expected);
        }
        assert_eq!(speed_turn_from_months(-1, 0, &rows).unwrap(), 0);
        assert_eq!(speed_turn_from_months(12, -7, &[]).unwrap(), -7);
        assert!(signed_div(i32::MIN, -1).is_err());
        assert!(signed_div(1, 0).is_err());
    }
}

/// Version30 Game.Eras current-era prefix only. All owner bytes are retained;
/// unsourced later fields and post-load callbacks are not certified by it.
pub struct SourceEras {
    current_era: i32,
    owner: Vec<u8>,
    witness: c6_core::loaded_state::ComponentSnapshotWitness,
}
impl SourceEras {
    pub fn current_era(&self) -> i32 {
        self.current_era
    }
    pub fn source_owner(&self) -> &[u8] {
        &self.owner
    }
    pub fn source_witness(&self) -> &c6_core::loaded_state::ComponentSnapshotWitness {
        &self.witness
    }
}
impl StartingContext {
    pub fn source(&self) -> StartingSource {
        self.source
    }
}
impl Definitions {
    /// Decode the version-30 current-era prefix, retaining unknown owner bytes.
    pub fn source_read_eras_component(
        &self,
        state: &LoadedState,
        witness: c6_core::loaded_state::ComponentSnapshotWitness,
    ) -> Result<SourceEras, Error> {
        let owner = super::choose_tech::checked_component_source(state, &self.binding, &witness)
            .map_err(|_| Error::Unsupported("eras source binding"))?;
        if owner.len() < 8 || owner[..4] != 30u32.to_le_bytes() || witness.decoded_prefix_bytes != 8
        {
            return Err(Error::Unsupported("Game.Eras v30 prefix"));
        }
        let hash = u32::from_le_bytes(owner[4..8].try_into().unwrap());
        let current_era = self.eras.find_index(hash).map(|i| i as i32).unwrap_or(-1);
        Ok(SourceEras {
            current_era,
            owner: owner.to_vec(),
            witness,
        })
    }
    /// Source configuration and saved era-prefix context. This runs the exact
    /// calendar calculation, but does not assert current runtime equivalence.
    pub fn starting_context_from_source(
        &self,
        state: &LoadedState,
        configuration: &super::configuration::Configuration,
        eras: &SourceEras,
    ) -> Result<StartingContext, Error> {
        if eras.witness.snapshot_sha256 != state.binding().core_sha256 {
            return Err(Error::Unsupported("Game.Eras context snapshot"));
        }
        configuration
            .require_binding(state.binding())
            .map_err(Error::Configuration)?;
        let game = configuration.game_fields();
        let required = |value: Option<u32>, field| {
            value.ok_or(Error::Configuration(super::configuration::Error::Missing(
                field,
            )))
        };
        let mut context = self.starting_context_from_raw_configuration(
            state,
            required(game.start_era, "start_era")?,
            game.start_year
                .ok_or(Error::Configuration(super::configuration::Error::Missing(
                    "start_year presence",
                )))?,
            required(game.calendar_type, "calendar_type")?,
            required(game.game_speed_type, "game_speed_type")?,
            Some(eras.current_era),
        )?;
        context.source = match configuration.source() {
            super::configuration::ConfigurationSource::SourceRead => {
                StartingSource::SourceConfigurationAndEraPrefix
            }
            super::configuration::ConfigurationSource::RuntimeSnapshot => {
                StartingSource::RuntimeConfigurationAndEraPrefix
            }
        };
        Ok(context)
    }
}

fn strategic_phase(
    phases: &[Option<crate::unit_operation_manager::TurnPhaseDefinition>],
    current_phase: i32,
) -> Result<bool, Error> {
    if current_phase == -1 {
        return Ok(true);
    }
    let phase = usize::try_from(current_phase)
        .ok()
        .and_then(|index| phases.get(index))
        .ok_or(Error::Unsupported("unchecked native current phase index"))?;
    Ok(phase
        .as_ref()
        .and_then(|phase| phase.active_segment.as_ref())
        .is_none_or(|segment| segment.flags & 1 != 0))
}
#[cfg(test)]
mod phase_tests {
    use super::*;
    use crate::unit_operation_manager::{TurnPhaseDefinition, TurnSegmentDefinition};
    #[test]
    fn strategic_phase_uses_bit_zero_and_preserves_actual_null_branches() {
        let phase = |flags: Option<u8>| {
            Some(TurnPhaseDefinition {
                row_id: crate::RulesRowId(41),
                phase_type_hash: 0x12345678,
                phase_order: 4,
                active_segment: flags.map(|flags| TurnSegmentDefinition {
                    row_id: crate::RulesRowId(99),
                    flags,
                }),
            })
        };
        let phases = [
            None,
            phase(None),
            phase(Some(2)),
            phase(Some(1)),
            phase(Some(0xff)),
        ];
        for (index, expected) in [
            (-1, true),
            (0, true),
            (1, true),
            (2, false),
            (3, true),
            (4, true),
        ] {
            assert_eq!(strategic_phase(&phases, index).unwrap(), expected);
        }
        assert!(strategic_phase(&phases, -2).is_err());
        assert!(strategic_phase(&phases, 5).is_err());
    }
}

#[cfg(test)]
mod current_tests {
    use super::*;
    use choose_tech::current_tests::{binding, registry};
    fn culture() -> Culture {
        Culture {
            wire_version: 45,
            current_government_hash: 0,
            post_anarchy_government_hash: 0,
            anarchy_end_turn: 0,
            turn_adopted: 0,
            modified_boost: 0,
            government_change_considered: false,
            government_unlocked: IndexedBools { pairs: vec![] },
            government_usage_count: vec![],
            last_government_used: vec![],
            policy_unlocked: IndexedBools { pairs: vec![] },
            policy_purchased: IndexedBools { pairs: vec![] },
            policy_has_been_enacted: IndexedBools { pairs: vec![] },
            policy_is_in_place: IndexedBools { pairs: vec![] },
            policy_banned: IndexedBools { pairs: vec![] },
            active_policies: c6_core::player_culture::GovernmentSlots { entries: vec![] },
            modified_slots: c6_core::player_culture::GovernmentSlots { entries: vec![] },
            modified_removed_slots: c6_core::player_culture::GovernmentSlots { entries: vec![] },
            has_civic: IndexedBools { pairs: vec![] },
            boost_triggered: IndexedBools { pairs: vec![] },
            cultural_progress_bits: vec![],
            civic_queue_hashes: vec![],
            cultural_overflow_bits: 0,
            culture_rate_change_bits: 0,
            civic_completed_this_turn: false,
            civic_completed_this_turn_hash: 0,
            turn_last_policy_change: 0,
            num_random_civic_boosts_on_new_era: 0,
            free_civic_boost_wonder_era: 0,
            tourism_on_bits: vec![],
            dominant_over: vec![],
            turns_until_dominant: vec![],
            tourism_modifier: 0,
            cached_alliance_tourism_bonus_bits: 0,
            cached_alliance_culture_bonus_bits: 0,
            religious_tourism_reduction: 0,
            overall_tourism_reduction: 0,
            always_full_religious_tourism: 0,
            lifetime_culture_bits: 0,
            staycationers: 0,
            last_turn_staycationers: 0,
            last_turn_tourists_to: 0,
            tourists_from: vec![],
            extra_trade_route_tourism_modifier: 0,
            auto_themed_building_hash: 0,
            tourism_locations: vec![],
            culture_bomb_triggers: vec![],
            tourism_into_favor: vec![],
            culture_bomb_converts_city: false,
            unlock_features: vec![],
            tourism_national_parks_modifier_bits: 0,
            great_work_object_tourism_modifiers: vec![],
            policy_favor_list: vec![],
            building_favor_list: vec![],
            other_government_intolerance_multiplier_bits: 0,
            other_government_intolerance_modifier: 0,
            previous_government_policies: c6_core::player_culture::GovernmentSlots {
                entries: vec![],
            },
            num_civics_repeated: 0,
            auto_themed_buildings_with_x_slots: 0,
            auto_themed_wonders_with_x_slots: 0,
            all_great_works_tourism_modifier: 0,
            all_great_works_yield_modifier: 0,
            replaced_slots: vec![],
            government_type_modified_slots: vec![],
        }
    }
    fn definitions(binding: WireBinding) -> Definitions {
        let empty = || LinuxDefinitionRegistry::from_native_order(vec![]);
        Definitions {
            binding,
            civics: registry(b"CIVIC_A"),
            repeatable: vec![false],
            xp2: empty(),
            hidden: vec![],
            governments: empty(),
            government_civics: vec![],
            policies: empty(),
            policy: vec![],
            slots: vec![],
            eras: registry(b"ERA_A"),
            start_eras: registry(b"ERA_A"),
            starting_change: vec![crate::registry::linux_type_hash(b"ERA_A")],
            require_cities: true,
            allow_empty: false,
            phases: vec![],
            calendar: CalendarDefinitions {
                months: 12,
                speeds: empty(),
                speed_turns: vec![],
                start_years: vec![],
            },
        }
    }
    #[test]
    fn current_culture_uses_supplied_turn_and_preserves_eager_starting_query() {
        let binding = binding();
        let definitions = definitions(binding.clone());
        let culture = culture();
        let context = StartingContext::from_current_fields(binding.clone(), 0, Some(10), None);
        let query = |turn, status, starting| {
            definitions
                .current_player(
                    0,
                    CurrentPlayer {
                        binding: &binding,
                        owner: 0,
                        culture: &culture,
                        city_count: 0,
                        current_turn: turn,
                        slot_status: status,
                    },
                    None,
                    None,
                    starting,
                )
                .unwrap()
        };
        // ChooseCivic's city branch skips graph, while government queries starting context eagerly.
        assert!(query(10, None, None).is_expired(Kind::ChooseCivic).unwrap());
        assert!(query(10, None, None)
            .is_needed(Kind::ConsiderGovernmentChange, false)
            .unwrap());
        assert!(query(10, None, None)
            .is_expired(Kind::ConsiderGovernmentChange)
            .is_err());
        assert!(query(12, None, Some(&context))
            .is_expired(Kind::ConsiderGovernmentChange)
            .unwrap());
        assert!(matches!(
            query(11, None, Some(&context)).is_expired(Kind::ConsiderGovernmentChange),
            Err(Error::Unsupported("current human configuration"))
        ));
        assert!(query(11, Some(3), Some(&context))
            .starting_government_notification()
            .unwrap());
        assert!(!query(11, Some(2), Some(&context))
            .starting_government_notification()
            .unwrap());
        assert!(!query(12, Some(3), Some(&context))
            .starting_government_notification()
            .unwrap());
        assert_eq!(context.source(), StartingSource::CallerProvided);
        let current = || CurrentPlayer {
            binding: &binding,
            owner: 0,
            culture: &culture,
            city_count: 0,
            current_turn: 12,
            slot_status: None,
        };
        assert!(definitions
            .current_player(1, current(), None, None, None)
            .is_err());
        let mut other = binding.clone();
        other.core_sha256[0] ^= 1;
        let other_definitions = choose_tech::current_tests::definitions(other);
        let techs = choose_tech::current_tests::techs();
        let research = choose_tech::Research {
            definitions: &other_definitions,
            techs: &techs,
            city_count: None,
            human: None,
            graph: None,
        };
        assert!(definitions
            .current_player(0, current(), None, Some(&research), None)
            .is_err());
    }
}
