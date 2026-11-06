//! Current TechResearched diplomatic-visibility recheck. Source ordering is
//! retained; positive spy/trade-route continuations remain explicit.
use crate::{
    registry::linux_type_hash,
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{self, collection},
    Rules, SqlValue,
};
use c6_core::loaded_state::LoadedState;
#[derive(Debug)]
pub enum Error<E> {
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Relations(c6_core::game_diplomatic_relations::Error),
    Definitions(unit_initialization_definitions::Error),
    Rules(crate::Error),
    Missing(&'static str),
    Effect(E),
}
#[derive(Clone, Copy)]
enum Active {
    Dynamic,
    Trader,
    Delegate,
    Ally,
    Spy(i32),
    Inactive,
}
struct Source {
    hash: u32,
    active: Active,
    dynamic: bool,
    trade_post: bool,
}
pub struct Definitions {
    sources: Vec<Source>,
    spies: Vec<u32>,
    unit_hashes: Vec<u32>,
    binding: c6_core::core_player::initialization::WireBinding,
    delegation_removal_civics: Vec<u32>,
}
impl Definitions {
    pub fn from_rules(
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, Error<std::convert::Infallible>> {
        unit_initialization_definitions::Definitions::validate_resources(rules, bound)
            .map_err(Error::Definitions)?;
        let registry = collection(rules, "DiplomaticVisibilitySources", "VisibilitySourceType")
            .map_err(Error::Definitions)?;
        let rows = rules
            .gameplay()
            .table("DiplomaticVisibilitySources")
            .ok_or(Error::Missing("DiplomaticVisibilitySources"))?;
        let xp = rules
            .gameplay()
            .table("DiplomaticVisibilitySources_XP1")
            .ok_or(Error::Missing("DiplomaticVisibilitySources_XP1"))?;
        let mut sources = Vec::new();
        for entry in registry.entries() {
            let row = rows
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("visibility source row"))?;
            let yes = |field| match row.get(field).map_err(Error::Rules)? {
                SqlValue::Integer(v) => Ok(*v != 0),
                _ => Err(Error::Missing("visibility source bool")),
            };
            let reference = |field| match row.get(field).map_err(Error::Rules)? {
                SqlValue::Text(_) => Ok(true),
                SqlValue::Null => Ok(false),
                _ => Err(Error::Missing("visibility reference")),
            };
            let prereq = reference("PrereqTech")?;
            let trait_ = reference("TraitType")?;
            let great = reference("GreatPersonIndividualType")?;
            let active = if prereq {
                Active::Dynamic
            } else if yes("Trader")? {
                Active::Trader
            } else if yes("Delegate")? {
                Active::Delegate
            } else if trait_ {
                Active::Dynamic
            } else if yes("Ally")? {
                Active::Ally
            } else if yes("Spy")? {
                let SqlValue::Integer(level) = row.get("LevelRequired").map_err(Error::Rules)?
                else {
                    return Err(Error::Missing("spy level"));
                };
                Active::Spy(i32::try_from(*level).map_err(|_| Error::Missing("spy level"))?)
            } else if great {
                Active::Dynamic
            } else {
                Active::Inactive
            };
            let hash = linux_type_hash(&entry.type_name);
            let mut trade_post = false;
            for row in xp.rows() {
                if matches!(row.get("VisibilitySourceType").map_err(Error::Rules)?,SqlValue::Text(name) if linux_type_hash(name)==hash)
                {
                    trade_post = matches!(
                        row.get("TradePostTrait").map_err(Error::Rules)?,
                        SqlValue::Text(_)
                    );
                }
            }
            sources.push(Source {
                hash,
                active,
                dynamic: prereq || trait_ || great || trade_post,
                trade_post,
            });
        }
        let units = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Missing("Units"))?;
        let mut spies = Vec::new();
        let mut unit_hashes = Vec::new();
        for row in units.rows() {
            let SqlValue::Text(name) = row.get("UnitType").map_err(Error::Rules)? else {
                return Err(Error::Missing("UnitType"));
            };
            unit_hashes.push(linux_type_hash(name));
            if matches!(row.get("Spy").map_err(Error::Rules)?,SqlValue::Integer(v)if *v!=0) {
                let SqlValue::Text(name) = row.get("UnitType").map_err(Error::Rules)? else {
                    return Err(Error::Missing("UnitType"));
                };
                spies.push(linux_type_hash(name));
            }
        }
        // Retain the actual DiplomaticAction collection order and repeats.
        let actions = collection(rules, "DiplomaticActions", "DiplomaticActionType")
            .map_err(Error::Definitions)?;
        let table = rules
            .gameplay()
            .table("DiplomaticActions")
            .ok_or(Error::Missing("DiplomaticActions"))?;
        let mut delegation_removal_civics = Vec::new();
        for action in actions.entries() {
            let row = table
                .row_by_id(action.database_rowid)
                .ok_or(Error::Missing("DiplomaticAction row"))?;
            if let SqlValue::Text(civic) =
                row.get("InitiatorObsoleteCivic").map_err(Error::Rules)?
            {
                if matches!(row.get("RemoveOnEntry").map_err(Error::Rules)?, SqlValue::Integer(v) if *v != 0)
                {
                    delegation_removal_civics.push(linux_type_hash(civic));
                }
            }
        }
        Ok(Self {
            delegation_removal_civics,
            sources,
            spies,
            unit_hashes,
            binding: bound.destination_binding().clone(),
        })
    }
    pub(super) fn delegation_removal_count(&self, civic: u32) -> usize {
        self.delegation_removal_civics
            .iter()
            .filter(|hash| **hash == civic)
            .count()
    }
}
/// RemoveOnEntry sets the delegation flag and dispatches its removal signal
/// even when already false. Visibility rechecking follows that signal for each
/// other alive player, unconditionally, in the manager's current order.
pub fn remove_delegations_on_civic<E>(
    state: &mut LoadedState,
    definitions: &Definitions,
    levels: &[(i32, u32)],
    alive: &[i32],
    owner: i32,
    mut removed: impl FnMut(&mut LoadedState, i32, i32) -> Result<(), E>,
    mut visibility_report: impl FnMut(&mut LoadedState, i32, i32) -> Result<(), E>,
) -> Result<(), Error<E>> {
    if state.binding() != &definitions.binding {
        return Err(Error::Missing("delegation removal rules binding"));
    }
    for &other in alive {
        if other == owner {
            continue;
        }
        state
            .set_current_diplomatic_delegation(owner, other, false)
            .map_err(Error::State)?;
        removed(state, owner, other).map_err(Error::Effect)?;
        recheck(
            state,
            definitions,
            levels,
            owner,
            other,
            &mut visibility_report,
        )?;
    }
    Ok(())
}
/// No city means CountOutgoingRoutesTo's first source iterator is empty. Spy
/// units are filtered by the admitted Unit definition; a real spy refuses here.
pub fn recheck<E>(
    state: &mut LoadedState,
    definitions: &Definitions,
    levels: &[(i32, u32)],
    owner: i32,
    other: i32,
    mut report: impl FnMut(&mut LoadedState, i32, i32) -> Result<(), E>,
) -> Result<(), Error<E>> {
    if state.binding() != &definitions.binding {
        return Err(Error::Missing("visibility rules binding"));
    }
    let level = levels
        .iter()
        .find(|p| p.0 == other)
        .ok_or(Error::Missing("viewed civilization level"))?
        .1;
    if level != 0x253718b0 {
        return Ok(());
    }
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("viewing player"))?;
    let target = usize::try_from(other).map_err(|_| Error::Missing("viewed player index"))?;
    let mut total = 0i32;
    let mut last = u32::MAX;
    for source in &definitions.sources {
        let dynamic = || {
            player
                .diplomacy
                .visibility_additions
                .iter()
                .filter(|p| p.source_hash == source.hash)
        };
        let active = match source.active {
            Active::Dynamic => dynamic().next().is_some(),
            Active::Trader => {
                // CountOutgoingRoutesTo visits the current source and viewed
                // city rosters, testing each ordered pair's active graph edge.
                let viewed = state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == other)
                    .ok_or(Error::Missing("viewed trade-route player"))?;
                if player.cities.records.entries.is_empty()
                    || viewed.cities.records.entries.is_empty()
                {
                    false
                } else {
                    let graph = state.trade_graph().map_err(Error::State)?;
                    let mut count = 0i32;
                    for a in &player.cities.records.entries {
                        for b in &viewed.cities.records.entries {
                            let vertex = |p: i32, id: u32| {
                                c6_core::trade_graph_state::city_vertex(
                                    c6_core::trade::ComponentId {
                                        owner: p as i16,
                                        component_type: 2,
                                        id,
                                    },
                                )
                            };
                            if graph
                                .graph()
                                .get_edge(
                                    vertex(owner, a.value.header.id),
                                    vertex(other, b.value.header.id),
                                )
                                .is_some_and(|edge| edge.has_active_route)
                            {
                                count = count.wrapping_add(1);
                            }
                        }
                    }
                    count > 0
                }
            }
            Active::Delegate => {
                *player
                    .diplomacy
                    .has_delegation
                    .get(target)
                    .ok_or(Error::Missing("delegation cell"))?
                    || *player
                        .diplomacy
                        .has_embassy
                        .get(target)
                        .ok_or(Error::Missing("embassy cell"))?
            }
            Active::Ally => state
                .game_relations()
                .map_err(Error::State)?
                .is_allied(owner, other)
                .map_err(Error::Relations)?,
            Active::Spy(required) => {
                for unit in state
                    .world()
                    .player(owner)
                    .map_err(Error::World)?
                    .unit_records()
                    .map_err(Error::World)?
                {
                    if unit.value.b_delayed_death || unit.value.e_type_index == u32::MAX {
                        continue;
                    }
                    if !definitions.unit_hashes.contains(&unit.value.e_type_index) {
                        return Err(Error::Missing("current Unit definition for spying"));
                    }
                    if definitions.spies.contains(&unit.value.e_type_index) {
                        return Err(Error::Missing("current spying-on level"));
                    }
                }
                let spy_level = 0;
                if required == 0 {
                    spy_level > 0
                } else {
                    spy_level >= required
                }
            }
            Active::Inactive => false,
        };
        if !active && source.trade_post && dynamic().next().is_some() {
            return Err(Error::Missing("current trade-post trait visibility"));
        }
        if active {
            if source.dynamic {
                if let Some(first) = dynamic().next() {
                    total = total.wrapping_add(first.amount);
                    last = source.hash;
                }
            } else {
                total = total.wrapping_add(1);
                last = source.hash;
            }
        }
    }
    let changed = state
        .set_current_diplomatic_visibility(owner, other, total)
        .map_err(Error::State)?;
    if changed {
        report(state, owner, other).map_err(Error::Effect)?;
    }
    state
        .set_current_diplomatic_visibility_source(owner, other, last)
        .map_err(Error::State)
}
