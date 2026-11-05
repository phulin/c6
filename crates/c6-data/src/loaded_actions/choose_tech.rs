//! Source-bounded Linux ChooseTech auto creation and research predicates.
//! Inputs are typed retained state and admitted definitions. Mutable Game.Techs
//! graph state must be supplied separately; SQL prerequisites are not that graph.
//! This is not complete notification Update, live initialization or action parity.
use super::notifications;
use crate::resource_binding::ResourceAuthority;
use crate::{registry::LinuxDefinitionRegistry, unit_initialization_definitions, Rules, SqlValue};
use c6_core::{
    core_player::initialization::{WireBinding, WireRole},
    loaded_state::{ComponentSnapshotWitness, LoadedState},
    notification_lifecycle::PoolId,
    player_prerequisites::IndexedBools,
    player_techs::Techs,
};
use sha2::{Digest, Sha256};

pub const TYPE_HASH: u32 = 0x592d_f4c8;
pub const VTABLE_ADDRESSPOINT: u64 = 0x3af_5060;
pub const DEFAULT_BLOCKING: i32 = 0xeef1_1bf9u32 as i32;
#[derive(Debug)]
pub enum Error {
    Resources(unit_initialization_definitions::Error),
    Rules(crate::Error),
    BaseInitialization(Box<notifications::Error>),
    Graph(c6_core::game_techs::Error),
    Unsupported(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChooseTech: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Caller-supplied current owner and configuration fields. These are typed
/// inputs, not predicate answers or a claim of observed native runtime state.
/// The owner label and binding are checked for consistency only.
pub struct CurrentPlayer<'a> {
    pub binding: &'a WireBinding,
    pub owner: i32,
    pub techs: &'a Techs,
    pub city_count: Option<usize>,
    pub slot_status: Option<i32>,
}

pub struct Definitions {
    binding: WireBinding,
    technologies: LinuxDefinitionRegistry,
    repeatable: Vec<bool>,
    xp2: LinuxDefinitionRegistry,
    hidden_until_prereq: Vec<bool>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        if state.binding() != bound.destination_binding() {
            return Err(Error::Unsupported("rules/retained state binding"));
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, bound)
            .map_err(Error::Resources)?;
        let technologies =
            unit_initialization_definitions::collection(rules, "Technologies", "TechnologyType")
                .map_err(Error::Resources)?;
        let xp2 = unit_initialization_definitions::collection(
            rules,
            "Technologies_XP2",
            "TechnologyType",
        )
        .map_err(Error::Resources)?;
        fn bools(
            rules: &Rules,
            table: &'static str,
            field: &'static str,
        ) -> Result<Vec<bool>, Error> {
            rules
                .gameplay()
                .table(table)
                .ok_or(Error::Unsupported(table))?
                .rows()
                .map(|row| match row.get(field).map_err(Error::Rules)? {
                    SqlValue::Integer(0) => Ok(false),
                    SqlValue::Integer(1) => Ok(true),
                    _ => Err(Error::Unsupported(field)),
                })
                .collect()
        }
        Ok(Self {
            binding: state.binding().clone(),
            technologies,
            repeatable: bools(rules, "Technologies", "Repeatable")?,
            xp2,
            hidden_until_prereq: bools(rules, "Technologies_XP2", "HiddenUntilPrereqComplete")?,
        })
    }
    /// Admitted native collection lookup for the separately source-closed
    /// Game.Techs Read/Validate projection. No SQL row-ID casting is involved.
    pub fn technology_lookup(&self) -> &dyn c6_core::movement_world::DefinitionLookup {
        &self.technologies
    }
    /// Decode a checked source owner without claiming final runtime equivalence.
    pub fn source_read_component(
        &self,
        state: &LoadedState,
        witness: ComponentSnapshotWitness,
        limits: c6_core::unit::Limits,
    ) -> Result<GameTechs, Error> {
        let bytes = checked_component_source(state, &self.binding, &witness)?;
        let wire = c6_core::game_techs::Techs::decode(bytes, limits).map_err(Error::Graph)?;
        let graph = wire
            .project_read(&self.technologies)
            .map_err(Error::Graph)?
            .post_load_without_rebuild()
            .map_err(Error::Graph)?;
        if witness.decoded_prefix_bytes != bytes.len() {
            return Err(Error::Unsupported("GameTechs decoded prefix"));
        }
        let mut input = GameTechs::from_source_read(self.binding.clone(), &graph);
        input.source_witness = Some(witness);
        Ok(input)
    }
    pub fn technology_count(&self) -> usize {
        self.technologies.entries().len()
    }
    pub fn technology_hash(&self, index: usize) -> Option<u32> {
        self.technologies
            .entries()
            .get(index)
            .map(|r| crate::registry::linux_type_hash(&r.type_name))
    }
    /// Native enum identity is a sequential collection position. Row IDs and
    /// serialized hashes never become that identity by casting.
    pub fn researching_tech(&self, techs: &Techs) -> i32 {
        techs
            .research_queue_hashes
            .first()
            .and_then(|h| self.technologies.find_index(*h))
            .and_then(|i| i32::try_from(i).ok())
            .unwrap_or(-1)
    }
    /// Compose current typed player/configuration fields without reading a
    /// saved LoadedState. Graph provenance remains separately inspectable.
    pub fn current_player<'a>(
        &'a self,
        owner: i32,
        current: CurrentPlayer<'a>,
        graph: Option<&'a GameTechs>,
    ) -> Result<Research<'a>, Error> {
        if current.binding != &self.binding || current.owner != owner {
            return Err(Error::Unsupported("current research owner/binding"));
        }
        if graph.is_some_and(|graph| graph.binding != self.binding) {
            return Err(Error::Unsupported("GameTechs session binding"));
        }
        Ok(Research {
            definitions: self,
            techs: current.techs,
            city_count: current.city_count,
            human: current.slot_status.map(|status| status == 3),
            graph,
        })
    }
    /// Convenience adapter over retained source fields. It does not establish
    /// that those fields or a SourceRead graph equal final current runtime.
    pub fn player<'a>(
        &'a self,
        state: &'a LoadedState,
        owner: i32,
        graph: Option<&'a GameTechs>,
    ) -> Result<Research<'a>, Error> {
        if self.binding != *state.binding() {
            return Err(Error::Unsupported("research state binding"));
        }
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.requested_owner == owner)
            .ok_or(Error::Unsupported("retained PlayerTechs owner"))?;
        Ok(Research {
            definitions: self,
            techs: &player.techs,
            city_count: Some(player.cities.records.entries.len()),
            human: state
                .player_slot_status(owner)
                .ok()
                .map(|status| status == 3),
            graph,
        })
    }
}

/// Query-relevant fields from a complete ordered graph input. The full owner
/// retains raw costs separately. Source projections and caller-provided native
/// fields remain distinct; neither is authenticated by a shape check alone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TechNode {
    pub native_type: i32,
    pub active: bool,
    pub prerequisites: Vec<i32>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphSource {
    CallerProvided,
    SourceReadNoRebuild,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameTechs {
    binding: WireBinding,
    source: GraphSource,
    source_witness: Option<ComponentSnapshotWitness>,
    projected_owner: Option<c6_core::game_techs::Graph>,
    ordered_types: Vec<i32>,
    nodes: Vec<TechNode>,
}
impl GameTechs {
    pub fn binding(&self) -> &WireBinding { &self.binding }
    pub fn from_complete_native(
        binding: WireBinding,
        ordered_types: Vec<i32>,
        nodes: Vec<TechNode>,
    ) -> Result<Self, Error> {
        if ordered_types.len() > i32::MAX as usize || nodes.len() > i32::MAX as usize {
            return Err(Error::Unsupported("native technology graph size"));
        }
        Ok(Self {
            binding,
            source: GraphSource::CallerProvided,
            source_witness: None,
            projected_owner: None,
            ordered_types,
            nodes,
        })
    }
    /// A query view of the exact owner-local Read/Validate and nonempty
    /// post-load source branch. The caller retains the full typed owner (costs
    /// included) and authenticates its snapshot/session and registry binding.
    /// This does not turn a saved owner into observed final native runtime state.
    pub fn from_source_read(binding: WireBinding, graph: &c6_core::game_techs::Graph) -> Self {
        Self {
            binding,
            source: GraphSource::SourceReadNoRebuild,
            source_witness: None,
            projected_owner: Some(graph.clone()),
            ordered_types: graph.iteration().to_vec(),
            nodes: graph
                .nodes()
                .iter()
                .map(|node| TechNode {
                    native_type: node.native_type,
                    active: node.active,
                    prerequisites: node.prerequisites.clone(),
                })
                .collect(),
        }
    }
    pub fn source(&self) -> GraphSource {
        self.source
    }
    pub fn source_witness(&self) -> Option<&ComponentSnapshotWitness> {
        self.source_witness.as_ref()
    }
    pub fn projected_owner(&self) -> Option<&c6_core::game_techs::Graph> {
        self.projected_owner.as_ref()
    }
    fn node(&self, t: i32) -> Option<&TechNode> {
        if t == -1 {
            return None;
        }
        self.nodes
            .get(t as u32 as usize)
            .filter(|n| n.native_type == t)
            .or_else(|| self.nodes.iter().find(|n| n.native_type == t))
    }
}

pub struct Research<'a> {
    pub definitions: &'a Definitions,
    pub techs: &'a Techs,
    pub city_count: Option<usize>,
    pub human: Option<bool>,
    pub graph: Option<&'a GameTechs>,
}
impl Research<'_> {
    pub(crate) fn binding(&self) -> &WireBinding {
        &self.definitions.binding
    }
    fn graph(&self) -> Result<&GameTechs, Error> {
        let g = self.graph.ok_or(Error::Unsupported(
            "retained validated GameTechs active nodes/prerequisite graph",
        ))?;
        if g.binding != self.definitions.binding {
            return Err(Error::Unsupported("GameTechs session binding"));
        }
        Ok(g)
    }
    fn active(&self, t: i32) -> Result<bool, Error> {
        Ok(self.graph()?.node(t).is_some_and(|n| n.active))
    }
    fn owned_bool(&self, array: &IndexedBools, t: i32) -> Result<bool, Error> {
        let Some(index) = usize::try_from(t)
            .ok()
            .filter(|i| *i < self.definitions.technology_count())
        else {
            return Ok(false);
        };
        // Complete saved hash arrays retain final overwrite precedence. Missing
        // current entries are unknown, not a fabricated false/native reset.
        array
            .pairs
            .iter()
            .rfind(|p| self.definitions.technologies.find_index(p.definition_hash) == Some(index))
            .map(|p| p.value)
            .ok_or(Error::Unsupported("current PlayerTechs indexed bool value"))
    }
    pub fn has_tech(&self, t: i32) -> Result<bool, Error> {
        if t == -1 || t as u32 as usize >= self.definitions.technology_count() {
            return Ok(false);
        }
        if !self.active(t)? {
            return Ok(false);
        }
        self.owned_bool(&self.techs.has_tech, t)
    }
    fn boosted(&self, t: i32) -> Result<bool, Error> {
        if t == -1 || t as u32 as usize >= self.definitions.technology_count() {
            return Ok(false);
        }
        if !self.active(t)? {
            return Ok(false);
        }
        self.owned_bool(&self.techs.boost_triggered, t)
    }
    pub fn more_to_research(&self) -> Result<bool, Error> {
        for &t in &self.graph()?.ordered_types {
            let Some(index) = self.definitions.technologies.find_index(t as u32) else {
                continue;
            };
            if self.definitions.repeatable[index] || !self.has_tech(t)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub fn revealed(&self, t: i32) -> Result<bool, Error> {
        let Some(index) = self.definitions.technologies.find_index(t as u32) else {
            return Ok(false);
        };
        let hash = self
            .definitions
            .technology_hash(index)
            .ok_or(Error::Unsupported("technology hash"))?;
        let hidden = self
            .definitions
            .xp2
            .find_index(hash)
            .is_some_and(|i| self.definitions.hidden_until_prereq[i]);
        if !hidden || self.has_tech(t)? || self.boosted(t)? {
            return Ok(true);
        }
        if !self.active(t)? {
            return Ok(false);
        }
        let prerequisites = &self
            .graph()?
            .node(t)
            .ok_or(Error::Unsupported("active technology node"))?
            .prerequisites;
        if prerequisites.is_empty() {
            return Ok(true);
        }
        for &prerequisite in prerequisites {
            if self.has_tech(prerequisite)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub fn can_research(&self, t: i32) -> Result<bool, Error> {
        if t == -1 || self.definitions.technologies.at(t as u32).is_none() {
            return Ok(false);
        }
        if !self.active(t)? || !self.revealed(t)? {
            return Ok(false);
        }
        if self.has_tech(t)? && !self.definitions.repeatable[t as u32 as usize] {
            return Ok(false);
        }
        for &prerequisite in &self
            .graph()?
            .node(t)
            .ok_or(Error::Unsupported("active technology node"))?
            .prerequisites
        {
            if !self.has_tech(prerequisite)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    /// Native base IsNeeded calls ChooseTech::IsExpired only when +0x19a is set.
    /// Expiry obtains current research before even the city/human short circuits.
    pub fn is_needed(&self, expire_when_unneeded: bool) -> Result<bool, Error> {
        if !expire_when_unneeded {
            return Ok(true);
        }
        let researching = self.definitions.researching_tech(self.techs);
        if self
            .city_count
            .ok_or(Error::Unsupported("current city count"))?
            == 0
        {
            return Ok(false);
        }
        if !self
            .human
            .ok_or(Error::Unsupported("actual human configuration"))?
        {
            return Ok(false);
        }
        if !self.more_to_research()? {
            return Ok(false);
        }
        if researching == -1 {
            return Ok(true);
        }
        Ok(!self.can_research(researching)?)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CreatorBinding {
    ChooseTech,
    BaseFallback,
    Unavailable,
}
pub use super::notifications::{Created, LogicalAllocation};
/// Source-closed auto=true/NULL-parameters initialization. The caller owns the
/// logical allocation/reuse decision and any pool pop; this function does not
/// invent native free-list contents, a successful allocation, or a registered
/// creator. Missing state is an error. Returned objects are not yet Add'ed.
pub fn create_auto(
    definitions: &notifications::Definitions,
    initialized: bool,
    uninitializing: Option<bool>,
    creator: CreatorBinding,
    allocation: Option<LogicalAllocation<'_>>,
    pool: PoolId,
    owner: i32,
) -> Result<Option<Created>, Error> {
    if !initialized {
        return Ok(None);
    }
    if uninitializing.ok_or(Error::Unsupported("game uninitializing state"))? {
        return Ok(None);
    }
    match creator {
        CreatorBinding::ChooseTech => {}
        CreatorBinding::Unavailable => {
            return Err(Error::Unsupported("actual ChooseTech creator registration"))
        }
        CreatorBinding::BaseFallback => {
            return Err(Error::Unsupported("base fallback Instance.Create"))
        }
    }
    let allocation =
        allocation.ok_or(Error::Unsupported("logical notification allocation/reuse"))?;
    let mut created = notifications::initialize_auto_base(
        definitions,
        TYPE_HASH,
        VTABLE_ADDRESSPOINT,
        allocation,
        pool,
        owner,
    )
    .map_err(|error| match error {
        notifications::Error::Unsupported(reason) => Error::Unsupported(reason),
        other => Error::BaseInitialization(Box::new(other)),
    })?;
    created.record.native_199 = false;
    created.record.native_19a = true;
    created.record.native_18c = 0;
    created.default_blocking = DEFAULT_BLOCKING;
    Ok(Some(created))
}

/// Check source consistency; this never authenticates a native observation.
pub(crate) fn checked_component_source<'a>(
    state: &'a LoadedState,
    binding: &WireBinding,
    witness: &ComponentSnapshotWitness,
) -> Result<&'a [u8], Error> {
    if !state.is_pristine() || binding != state.binding() {
        return Err(Error::Unsupported("component source binding"));
    }
    checked_component_bytes(state.source_image(), state.binding().core_sha256, witness)
}
fn checked_component_bytes<'a>(
    source: &'a [u8],
    core_sha256: [u8; 32],
    witness: &ComponentSnapshotWitness,
) -> Result<&'a [u8], Error> {
    if witness.source_role != WireRole::SavedOutputCore
        || witness.snapshot_bytes != source.len()
        || witness.snapshot_sha256 != core_sha256
        || <[u8; 32]>::from(Sha256::digest(source)) != witness.snapshot_sha256
        || witness.owner_range[0] >= witness.owner_range[1]
    {
        return Err(Error::Unsupported("component source binding"));
    }
    let bytes = source
        .get(witness.owner_range[0]..witness.owner_range[1])
        .ok_or(Error::Unsupported("component source extent"))?;
    if witness.decoded_prefix_bytes > bytes.len() {
        return Err(Error::Unsupported("component decoded prefix extent"));
    }
    Ok(bytes)
}

#[cfg(test)]
mod component_tests {
    use super::*;
    #[test]
    fn component_source_checks_identity_role_and_retains_unknown_bytes() {
        let bytes = [1, 2, 3, 0xaa, 0xbb];
        let digest = Sha256::digest(bytes).into();
        let witness = ComponentSnapshotWitness {
            source_role: WireRole::SavedOutputCore,
            snapshot_sha256: digest,
            snapshot_bytes: bytes.len(),
            owner_trace_sha256: [7; 32],
            owner_range: [1, 5],
            decoded_prefix_bytes: 2,
        };
        assert_eq!(
            checked_component_bytes(&bytes, digest, &witness).unwrap(),
            &[2, 3, 0xaa, 0xbb]
        );
        let mut bad = witness.clone();
        bad.source_role = WireRole::ReadInputCore;
        assert!(checked_component_bytes(&bytes, digest, &bad).is_err());
        let mut bad = witness.clone();
        bad.snapshot_bytes += 1;
        assert!(checked_component_bytes(&bytes, digest, &bad).is_err());
        let mut bad = witness.clone();
        bad.snapshot_sha256[0] ^= 1;
        assert!(checked_component_bytes(&bytes, digest, &bad).is_err());
        let mut bad = witness.clone();
        bad.owner_range = [4, 6];
        assert!(checked_component_bytes(&bytes, digest, &bad).is_err());
        let mut bad = witness.clone();
        bad.owner_range = [3, 3];
        assert!(checked_component_bytes(&bytes, digest, &bad).is_err());
        let mut bad = witness.clone();
        bad.decoded_prefix_bytes = 5;
        assert!(checked_component_bytes(&bytes, digest, &bad).is_err());
        let mut changed = bytes;
        changed[4] ^= 1;
        assert!(checked_component_bytes(&changed, digest, &witness).is_err());
    }
}

#[cfg(test)]
pub(super) mod current_tests {
    use super::*;
    use c6_core::core_player::initialization::{DatabaseIdentity, InputIdentities};
    pub(in crate::loaded_actions) fn binding() -> WireBinding {
        WireBinding {
            role: WireRole::SavedOutputCore,
            core_sha256: [1; 32],
            occurrence_id: b"0".to_vec(),
            output_id: None,
            inputs: InputIdentities {
                configuration_sha256: [2; 32],
                rules_inputs_sha256: [3; 32],
                database: DatabaseIdentity::LiveSnapshotUnavailable,
                dlc_inputs_sha256: [4; 32],
                mods_inputs_sha256: [5; 32],
                content_inputs_sha256: [6; 32],
            },
        }
    }
    pub(in crate::loaded_actions) fn registry(name: &[u8]) -> LinuxDefinitionRegistry {
        LinuxDefinitionRegistry::from_native_order(vec![crate::registry::DefinitionEntry {
            database_rowid: crate::RulesRowId(77),
            type_name: name.to_vec(),
        }])
    }
    pub(in crate::loaded_actions) fn definitions(binding: WireBinding) -> Definitions {
        Definitions {
            binding,
            technologies: registry(b"TECH_A"),
            repeatable: vec![false],
            xp2: LinuxDefinitionRegistry::from_native_order(vec![]),
            hidden_until_prereq: vec![],
        }
    }
    pub(in crate::loaded_actions) fn techs() -> Techs {
        Techs {
            wire_version: 6,
            research_queue_hashes: vec![],
            research_overflow_q8: 0,
            science_rate_change_q8: 0,
            modified_boost: 0,
            has_tech: IndexedBools { pairs: vec![] },
            boost_triggered: IndexedBools { pairs: vec![] },
            research_progress_q8: vec![],
            num_random_tech_boosts_on_new_era: 0,
            num_techs_repeated: 0,
            free_tech_boost_wonder_era: 0,
        }
    }
    #[test]
    fn current_inputs_preserve_city_human_graph_order_and_source_provenance() {
        let binding = binding();
        let definitions = definitions(binding.clone());
        let techs = techs();
        let query = |cities, status| {
            definitions
                .current_player(
                    0,
                    CurrentPlayer {
                        binding: &binding,
                        owner: 0,
                        techs: &techs,
                        city_count: cities,
                        slot_status: status,
                    },
                    None,
                )
                .unwrap()
        };
        assert!(query(None, None).is_needed(false).unwrap());
        assert!(!query(Some(0), None).is_needed(true).unwrap());
        assert!(matches!(
            query(Some(1), None).is_needed(true),
            Err(Error::Unsupported("actual human configuration"))
        ));
        assert!(!query(Some(1), Some(2)).is_needed(true).unwrap());
        assert!(matches!(
            query(Some(1), Some(3)).is_needed(true),
            Err(Error::Unsupported(
                "retained validated GameTechs active nodes/prerequisite graph"
            ))
        ));
        let mut graph = GameTechs::from_complete_native(binding.clone(), vec![], vec![]).unwrap();
        graph.source = GraphSource::SourceReadNoRebuild;
        let query = definitions
            .current_player(
                0,
                CurrentPlayer {
                    binding: &binding,
                    owner: 0,
                    techs: &techs,
                    city_count: Some(1),
                    slot_status: Some(3),
                },
                Some(&graph),
            )
            .unwrap();
        assert!(!query.is_needed(true).unwrap());
        assert_eq!(graph.source(), GraphSource::SourceReadNoRebuild);
        assert!(definitions
            .current_player(
                1,
                CurrentPlayer {
                    binding: &binding,
                    owner: 0,
                    techs: &techs,
                    city_count: None,
                    slot_status: None
                },
                None
            )
            .is_err());
        let mut other = binding.clone();
        other.core_sha256[0] ^= 1;
        assert!(definitions
            .current_player(
                0,
                CurrentPlayer {
                    binding: &other,
                    owner: 0,
                    techs: &techs,
                    city_count: None,
                    slot_status: None
                },
                None
            )
            .is_err());
    }
}
