//! Source-bounded Linux ChooseTech auto creation and research predicates.
//! Inputs are typed retained state and admitted definitions. Mutable Game.Techs
//! graph state must be supplied separately; SQL prerequisites are not that graph.
//! This is not complete notification Update, live initialization or action parity.
use super::{notifications, resources::Bound};
use crate::{registry::LinuxDefinitionRegistry, unit_initialization_definitions, Rules, SqlValue};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    notification_lifecycle::{self as lifecycle, ObjectId, PoolId},
    notifications::{ComponentId, LocalizedString, Notification},
    player_prerequisites::IndexedBools,
    player_techs::Techs,
};

pub const TYPE_HASH: u32 = 0x592d_f4c8;
pub const VTABLE_ADDRESSPOINT: u64 = 0x3af_5060;
pub const DEFAULT_BLOCKING: i32 = 0xeef1_1bf9u32 as i32;
#[derive(Debug)]
pub enum Error {
    Resources(unit_initialization_definitions::Error),
    Rules(crate::Error),
    Unsupported(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChooseTech: {self:?}")
    }
}
impl std::error::Error for Error {}

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
        bound: &Bound<'_>,
        report: &[u8],
    ) -> Result<Self, Error> {
        if state.binding() != bound.destination_binding() {
            return Err(Error::Unsupported("rules/retained state binding"));
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, bound, report)
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
                .player_controls(owner)
                .ok()
                .map(|p| p.slot_status == 3),
            graph,
        })
    }
}

/// Complete original validated mutable graph, not the initial rules edges.
/// The caller must retain its actual source/session authority. This constructor
/// checks shape/binding only; it does not authenticate an invented live export.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TechNode {
    pub native_type: i32,
    pub active: bool,
    pub prerequisites: Vec<i32>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameTechs {
    binding: WireBinding,
    ordered_types: Vec<i32>,
    nodes: Vec<TechNode>,
}
impl GameTechs {
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
            ordered_types,
            nodes,
        })
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
pub enum LogicalAllocation<'a> {
    Fresh(ObjectId),
    Reused {
        object: ObjectId,
        prior: &'a Notification,
    },
    Failed,
}
pub struct Created {
    pub record: Notification,
    pub live: lifecycle::LiveNotification,
    pub default_blocking: i32,
    /// Native +0x18 is not present in the wire Notification record.
    pub group_hash: u32,
    pub vtable_addresspoint: u64,
}
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
    let definition = definitions
        .find(TYPE_HASH)
        .ok_or(Error::Unsupported("ChooseTech notification definition"))?;
    if definition.expires_end_of_turn || definition.expires_end_of_next_turn {
        return Err(Error::Unsupported(
            "base Initialize PreviouslyActiveThisTurn dependency",
        ));
    }
    let (object, mut id, component_type) =
        match allocation.ok_or(Error::Unsupported("logical notification allocation/reuse"))? {
            LogicalAllocation::Fresh(object) => (
                object,
                ComponentId {
                    owner: -1,
                    component_type: 0x26,
                    id: u32::MAX,
                },
                0,
            ),
            LogicalAllocation::Reused { object, prior } => {
                if prior.factory_type != TYPE_HASH
                    || prior.type_hash != TYPE_HASH
                    || !prior.parameters.entries.is_empty()
                {
                    return Err(Error::Unsupported(
                        "reused ChooseTech identity/parameter destruction",
                    ));
                }
                (object, prior.id, prior.component.component_type)
            }
            LogicalAllocation::Failed => {
                return Err(Error::Unsupported("notification allocation failure"))
            }
        };
    if owner != -1 {
        id.owner = owner as i16;
    }
    let severity = definition
        .severity_type
        .as_deref()
        .map(|name| severity_type(crate::registry::linux_type_hash(name.as_bytes())))
        .filter(|v| *v != -1)
        .unwrap_or(0);
    let record = Notification {
        factory_type: TYPE_HASH,
        id,
        type_hash: TYPE_HASH,
        severity,
        message: LocalizedString::default(),
        summary: LocalizedString::default(),
        location: [-9999; 2],
        component: ComponentId {
            owner: -1,
            component_type,
            id: u32::MAX,
        },
        native_188: -1,
        native_18c: 0,
        is_dismissed: false,
        native_199: false,
        native_19a: true,
        native_190: 1,
        native_19b: true,
        native_19c: false,
        parameters: Default::default(),
        native_19d: definition.auto_activate,
        has_been_activated: false,
    };
    Ok(Some(Created {
        record,
        live: lifecycle::LiveNotification {
            object,
            lock_count: 0,
            pool_binding: Some(pool),
            definition_icon: lifecycle::DefinitionIcon::ShowInSinglePlayer(
                definition.show_icon_single_player,
            ),
            activation: lifecycle::ActivationBinding::Base,
            release: lifecycle::ReleaseBinding::Base,
            destruction: lifecycle::DestructionBinding::BaseResources,
        },
        default_blocking: DEFAULT_BLOCKING,
        group_hash: definition
            .group_type
            .as_deref()
            .map(|name| crate::registry::linux_type_hash(name.as_bytes()))
            .unwrap_or(0xb6fa_1284),
        vtable_addresspoint: VTABLE_ADDRESSPOINT,
    }))
}
fn severity_type(hash: u32) -> i32 {
    match hash {
        0xfbd0_c5d4 => 0,
        0x28f6_49bb => 1,
        0x386f_f61e => 2,
        0xba9f_3062 => 3,
        0x6e89_7878 => 4,
        0..5 => hash as i32,
        _ => -1,
    }
}
