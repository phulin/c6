//! Formation path search against the owned Linux gameplay world.
//!
//! Definitions are admitted once. Each expansion reads current Units, Plots,
//! diplomacy and visibility; unsupported reached dependencies remain errors.
//! This read-only search plus Formation cache writes is not complete MoveTo.
pub mod breadth_first;
mod shore;
mod visible_units;
pub use visible_units::has_visible_enemy_unit;
use crate::{
    city_movement::NativeTerrainDefinitions,
    formation_movement_cost as cost, formation_rules, movement_placement_state as placement,
    resource_binding::{ResourceAuthority, ResourceIdentity},
    unit_initialization_definitions::collection,
    unit_movement_state as unit_moves, Rules,
};
use c6_core::{
    formation_path_cache::{self, GenerationContext},
    formation_path_callbacks::{
        self as logic, LazyPathQueries, PathBoolQuery as B, PathIntQuery as I,
    },
    loaded_state::{self, LoadedState},
    map::{Plot, PlotFlag, Visibility},
    movement::{MoveBudget, MovementPoints, ZocValue},
    movement_shore::PlacementRequest,
    movement_world,
    pathfinder::{NodeId, PathCallbacks, TwoLayer},
    skip_turn::placement as unit_placement,
    unit::{Formation, PlotCoord},
    world,
};

#[derive(Debug)]
pub enum Error {
    State(loaded_state::Error),
    World(world::MissingInput),
    Placement(placement::Error),
    PlacementQuery(unit_placement::Error),
    Cost(cost::Error),
    UnitMoves(unit_moves::Error),
    Rules(crate::loaded_actions::Error),
    Missing(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Formation path state: {self:?}")
    }
}
impl std::error::Error for Error {}

pub struct Definitions<'a, 'r> {
    pub placement: placement::Definitions<'a, 'r>,
    pub movement: cost::Definitions<'a>,
    pub units: unit_moves::Definitions,
    pub formation: formation_rules::Definitions,
    pub fortify_turn_max: i32,
    features: crate::registry::LinuxDefinitionRegistry,
    feature_danger: Vec<i32>,
    shallow_terrains: Vec<bool>,
    // Only the first native reference is consumed. Nonempty reference tables
    // require an admitted order; this boundary rejects their unsupported import.
    feature_movements_empty: bool,
}
impl<'a, 'r> Definitions<'a, 'r> {
    pub fn new(
        state: &LoadedState,
        rules: &Rules,
        resources: &'a ResourceAuthority<'r>,
        terrains: &'a NativeTerrainDefinitions,
        terrain_resources: &ResourceIdentity,
    ) -> Result<Self, Error> {
        let placement =
            placement::Definitions::new(state, rules, resources, terrains, terrain_resources)
                .map_err(Error::Placement)?;
        let movement = cost::Definitions::from_rules(state, rules, resources, terrains, &placement)
            .map_err(Error::Cost)?;
        let features = collection(rules, "Features", "FeatureType")
            .map_err(|_| Error::Missing("Features native collection"))?;
        let rows = rules
            .gameplay()
            .table("Features")
            .ok_or(Error::Missing("Features table"))?;
        let feature_danger = features
            .entries()
            .iter()
            .map(|e| {
                let row = rows
                    .row_by_id(e.database_rowid)
                    .ok_or(Error::Missing("Features row"))?;
                i32::try_from(
                    row.integer("DangerValue")
                        .map_err(|_| Error::Missing("Features.DangerValue"))?,
                )
                .map_err(|_| Error::Missing("Features.DangerValue range"))
            })
            .collect::<Result<_, _>>()?;
        let movements = rules
            .gameplay()
            .table("Feature_UnitMovements")
            .ok_or(Error::Missing("Feature_UnitMovements table"))?;
        let parameters = crate::game_parameters::Parameters::from_rules(rules)
            .map_err(|_| Error::Missing("static gameplay parameters"))?;
        let fortify_turn_max = parameters
            .integer(crate::game_parameters::IntegerParameter::FortifyTurnMax)
            .map_err(|_| Error::Missing("FORTIFY_TURN_MAX"))?;
        let terrain_registry = collection(rules, "Terrains", "TerrainType")
            .map_err(|_| Error::Missing("Terrains native collection"))?;
        let terrain_rows = rules
            .gameplay()
            .table("Terrains")
            .ok_or(Error::Missing("Terrains table"))?;
        let shallow_terrains = terrain_registry
            .entries()
            .iter()
            .map(|entry| {
                terrain_rows
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Missing("Terrains row"))?
                    .integer("ShallowWater")
                    .map(|v| v != 0)
                    .map_err(|_| Error::Missing("Terrains.ShallowWater"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            shallow_terrains,
            fortify_turn_max,
            placement,
            movement,
            units: unit_moves::Definitions::from_rules(state, rules, resources)
                .map_err(Error::UnitMoves)?,
            formation: formation_rules::Definitions::from_rules(state, rules, resources)
                .map_err(Error::Rules)?,
            features,
            feature_danger,
            feature_movements_empty: movements.rows().next().is_none(),
        })
    }
}

/// Runtime service inputs are explicit. Shared counts are the actual live
/// group1 arrays. Civilization levels are current configuration IDs, not query
/// answers. No callback or path result is supplied by this interface.
pub struct CurrentCityDomains<'a> {
    pub binding: &'a c6_core::core_player::initialization::WireBinding,
    pub stamp: loaded_state::ActionStamp,
    pub masks: &'a [Option<c6_core::city_district::DomainMask>],
}
/// TwoLayer's host-only reentrancy guard. An exclusive standalone caller has
/// no concurrent application-thread query. This replaces native host pointer
/// and thread-ID storage with the execution relationship it guards.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionContext {
    Exclusive,
    ApplicationThread { game_core_executing: bool },
}
pub struct Runtime<'a> {
    pub visibility: &'a crate::unit_sight_state::Runtime,
    pub civilization_levels: Option<placement::CurrentCivilizationLevels<'a>>,
    pub city_domains: Option<CurrentCityDomains<'a>>,
    pub execution: ExecutionContext,
    pub is_evaluating_operation: bool,
}

pub fn mapped_visibility(state: &LoadedState, owner: i32) -> Result<&Visibility, Error> {
    let world = state.world();
    let index = usize::try_from(owner)
        .ok()
        .and_then(|i| world.player_mapping.as_ref()?.get(i))
        .and_then(|i| usize::try_from(*i).ok())
        .ok_or(Error::Missing("visibility owner mapping"))?;
    let row = world
        .map()
        .visibility
        .get(index)
        .filter(|r| r.player == owner && r.visibility.player_id == owner)
        .ok_or(Error::Missing("mapped visibility occurrence"))?;
    if world
        .map()
        .visibility
        .iter()
        .filter(|r| r.player == owner)
        .count()
        != 1
    {
        return Err(Error::Missing("ambiguous visibility owner"));
    }
    Ok(&row.visibility)
}
fn revealed(state: &LoadedState, owner: i32, index: usize) -> Result<bool, Error> {
    match mapped_visibility(state, owner)?.revealed.get(index) {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => Err(Error::Missing("canonical revealed byte")),
    }
}
fn visible(runtime: &Runtime<'_>, owner: i32, index: usize) -> Result<bool, Error> {
    shared_plot_visible(runtime.visibility, owner, index)
}

/// Native IsVisible over the actual live shared normal layer, with signed i16 counts.
pub fn shared_plot_visible(
    runtime: &crate::unit_sight_state::Runtime,
    owner: i32,
    index: usize,
) -> Result<bool, Error> {
    let layers = &runtime.shared(owner).map_err(Error::Missing)?.0;
    let mut normal = layers
        .iter()
        .filter(|l| l.type_hash == c6_core::visibility_change::BASE_LAYER_HASH);
    let layer = normal
        .next()
        .ok_or(Error::Missing("shared normal visibility"))?;
    if normal.next().is_some() {
        return Err(Error::Missing("duplicate shared normal visibility"));
    }
    Ok(*layer
        .values
        .get(index)
        .ok_or(Error::Missing("shared normal plot"))?
        > 0)
}

/// Reborrow the current placement state with the exact path runtime inputs.
pub fn placement_context<'s, 'a, 'r>(
    state: &'s LoadedState,
    definitions: &'a Definitions<'a, 'r>,
    runtime: &Runtime<'a>,
) -> Result<placement::Context<'s, 'a, 'r>, Error> {
    let mut p = placement::Context::from_definitions(state, &definitions.placement)
        .map_err(Error::Placement)?
        .with_district_definitions(&definitions.movement)
        .with_combat_definitions(&definitions.formation);
    if let Some(levels) = &runtime.civilization_levels {
        p = p
            .with_civilization_levels(placement::CurrentCivilizationLevels {
                binding: levels.binding,
                values: levels.values,
            })
            .map_err(Error::Placement)?;
    }
    if let Some(domains) = &runtime.city_domains {
        p = p
            .with_city_domain_masks(domains.binding, domains.stamp, domains.masks)
            .map_err(Error::Placement)?;
    }
    Ok(p)
}

pub struct Context<'s, 'a, 'r> {
    pub state: &'s mut LoadedState,
    pub definitions: &'a Definitions<'a, 'r>,
    pub runtime: Runtime<'a>,
    pub formation: (i32, u32),
    initialized: Option<logic::InitializedMembers>,
    includes: [bool; 5],
    human: bool,
    tribe: i32,
    religion: u32,
    layer: u32,
    memo: logic::MovementMemo,
    starting_remaining_override: Option<i32>,
    capabilities: Option<&'a c6_core::game_capabilities::Membership>,
}
impl<'s, 'a, 'r> Context<'s, 'a, 'r> {
    pub fn new(
        state: &'s mut LoadedState,
        definitions: &'a Definitions<'a, 'r>,
        runtime: Runtime<'a>,
        formation: (i32, u32),
    ) -> Result<Self, Error> {
        state
            .world()
            .player(formation.0)
            .map_err(Error::World)?
            .formation(formation.1)
            .map_err(Error::World)?
            .ok_or(Error::Missing("Formation"))?;
        placement::Context::from_definitions(state, &definitions.placement)
            .map_err(Error::Placement)?;
        Ok(Self {
            state,
            definitions,
            runtime,
            formation,
            initialized: None,
            includes: [false; 5],
            human: false,
            tribe: -1,
            religion: u32::MAX,
            layer: u32::MAX,
            memo: logic::MovementMemo::default(),
            starting_remaining_override: None,
            capabilities: None,
        })
    }
    fn record(&self) -> &Formation {
        self.state
            .world()
            .player(self.formation.0)
            .expect("validated owner")
            .formation(self.formation.1)
            .expect("validated container")
            .expect("retained Formation")
            .record
    }
    pub fn with_capabilities(
        mut self,
        capabilities: &'a c6_core::game_capabilities::Membership,
    ) -> Self {
        self.capabilities = Some(capabilities);
        self
    }
    pub fn placement(&self) -> Result<placement::Context<'_, 'a, 'r>, Error> {
        let view = placement_context(self.state, self.definitions, &self.runtime)?;
        Ok(match self.capabilities {
            Some(capabilities) => view.with_capabilities(capabilities),
            None => view,
        })
    }
    fn init(&self) -> Result<logic::InitializedMembers, Error> {
        self.initialized.ok_or(Error::Missing("PathInitialize"))
    }
    fn plot(&self, at: PlotCoord) -> Result<Option<(usize, &Plot)>, Error> {
        let map = self.state.world().map();
        if at.x == -9999 || at.y == -9999 {
            return Ok(None);
        }
        if map.header.width <= 0 || map.header.height <= 0 {
            return Err(Error::Missing("map geometry"));
        }
        let x = movement_world::coord_range(at.x, map.header.width, map.header.wrap_x);
        let y = movement_world::coord_range(at.y, map.header.height, map.header.wrap_y);
        Ok(map
            .plot_at(x, y)
            .map(|p| ((y * map.header.width + x) as usize, p)))
    }
    fn require_plot(&self, at: PlotCoord) -> Result<(usize, &Plot), Error> {
        self.plot(at)?.ok_or(Error::Missing("path plot"))
    }
    fn mountain(&self, at: PlotCoord) -> Result<bool, Error> {
        let (_, p) = self.require_plot(at)?;
        let index = self
            .definitions
            .placement
            .terrain_definitions()
            .plot_index(p.terrain_hash)
            .map_err(|_| Error::Missing("terrain index"))?;
        Ok(matches!(index, 2 | 5 | 8 | 11 | 14))
    }
    fn water(&self, at: PlotCoord) -> Result<bool, Error> {
        self.placement()?
            .water(self.require_plot(at)?.1)
            .map_err(Error::Placement)
    }
    fn node_cache(s: &TwoLayer, id: NodeId) -> logic::NodeCache {
        let n = s.node(id);
        logic::NodeCache {
            flags: n.cache_flags,
            friendly_units_of_type: n.cache_other as u16,
        }
    }
    fn cache(
        &mut self,
        s: &mut TwoLayer,
        parent: Option<NodeId>,
        child: NodeId,
    ) -> Result<(), Error> {
        if s.node(child).cache_flags & 1 != 0 {
            return Ok(());
        }
        let mut cache = Self::node_cache(s, child);
        let init = self.init()?;
        let facts = logic::LazyNodeCacheFacts {
            formation_owner: self.record().owner,
            formation_location: self.record().location,
            location: s.node(child).location(),
            destination: s.destination,
            plot_unit_limit: self.definitions.movement.plot_unit_limit(),
            is_human: self.human,
            is_automated: init.automated,
        };
        let parent_cache = parent.map(|id| Self::node_cache(s, id));
        let result = logic::initialize_node_cache_lazy(
            &mut cache,
            parent_cache,
            facts,
            &mut Queries {
                context: self,
                search: s,
                parent,
                child,
            },
        );
        s.node_mut(child).cache_flags = cache.flags;
        s.node_mut(child).cache_other =
            (s.node(child).cache_other & 0xffff0000) | u32::from(cache.friendly_units_of_type);
        result
    }
    fn movement(
        &mut self,
        s: &mut TwoLayer,
        parent: Option<NodeId>,
        child: NodeId,
    ) -> Result<logic::MovementResult, Error> {
        let p = parent.map(|id| {
            let n = s.node(id);
            logic::ParentMovement {
                location: n.location(),
                remaining: MovementPoints(n.moves_remaining_raw),
                exit_cost: MovementPoints(n.exit_cost_raw),
                turns: n.turn,
                cache_flags: n.cache_flags,
            }
        });
        let facts = logic::LazyMovementFacts {
            formation_location: self.record().location,
            path_flags: s.flags,
            child_cache_flags: s.node(child).cache_flags,
            starting_remaining: MovementPoints(
                self.starting_remaining_override
                    .unwrap_or(self.record().moves_remaining_raw),
            ),
        };
        // Move the small identity memo out to permit the lazy live queries to
        // borrow this context; no Unit, World or definition collection is cloned.
        let mut memo = std::mem::take(&mut self.memo);
        let result = memo.calculate_lazy(
            parent,
            child,
            p,
            facts,
            &mut Queries {
                context: self,
                search: s,
                parent,
                child,
            },
        );
        self.memo = memo;
        result
    }
    pub fn generate(
        &mut self,
        s: &mut TwoLayer,
        destination: PlotCoord,
        flags: u32,
        reuse: bool,
    ) -> Result<bool, Error> {
        let result =
            formation_path_cache::generate_formation_path(s, self, destination, flags, reuse, None);
        if result.is_err() {
            s.invalidate();
        }
        result
    }
}
impl PathCallbacks for Context<'_, '_, '_> {
    type Error = Error;
    fn initialize(&mut self, _: &mut TwoLayer) -> Result<(), Error> {
        for domain in [2, 0, 1, 4] {
            self.includes[domain as usize] = self.includes_unit_of_domain(domain)?;
        }
        let player = self
            .state
            .world()
            .player(self.record().owner)
            .map_err(Error::World)?;
        let mut religion = u32::MAX;
        for &id in &self.record().unit_ids {
            if let Some(found) = player.unit(id).map_err(Error::World)? {
                religion = found.record.k_religion.e_religion_type;
                break;
            }
        }
        self.religion = religion;
        let mut tribe = -1;
        for &id in &self.record().unit_ids {
            let unit = player
                .unit(id)
                .map_err(Error::World)?
                .ok_or(Error::Missing("barbarian member"))?
                .record;
            let level = self
                .runtime
                .civilization_levels
                .as_ref()
                .and_then(|l| l.values.iter().find(|v| v.0 == unit.e_owner))
                .map(|v| v.1)
                .ok_or(Error::Missing("current member civilization level"))?;
            if level == 0xd959ec24 {
                tribe = unit.i_barbarian_tribe_index;
                break;
            }
        }
        self.tribe = tribe;
        self.human = player
            .slot_status
            .ok_or(Error::Missing("owner slot status"))?
            == 3;
        let p = self.placement()?;
        let mut members = Vec::with_capacity(self.record().unit_ids.len());
        let inputs = unit_moves::Inputs {
            definitions: &self.definitions.units,
        };
        for &id in &self.record().unit_ids {
            let member = player
                .unit(id)
                .map_err(Error::World)?
                .map(|found| {
                    let u = found.record;
                    let mut base_moves = [0; 5];
                    for domain in 0..5 {
                        base_moves[domain as usize] = unit_moves::unit_base_moves_with_can_embark(
                            self.state,
                            u,
                            domain,
                            &inputs,
                            || p.can_embark(u),
                        )
                        .map_err(Error::UnitMoves)?;
                    }
                    Ok(logic::InitializationMember {
                        embarked: unit_moves::unit_is_embarked(u),
                        automated: u.e_automation != -1,
                        can_attack: self
                            .definitions
                            .formation
                            .can_attack(self.state, u)
                            .map_err(Error::Rules)?,
                        max_moves: unit_moves::unit_max_moves(self.state, u, &inputs)
                            .map_err(Error::UnitMoves)?,
                        base_moves,
                    })
                })
                .transpose()?;
            members.push(member);
        }
        if self.initialized.is_none()
            && !self.includes[0]
            && !self.includes[1]
            && !self.includes[2]
            && !self.includes[4]
            && !members.iter().flatten().any(|m| m.embarked)
        {
            return Err(Error::Missing("previous path scratch domain"));
        }
        self.initialized = Some(logic::initialize_members(
            members,
            self.includes,
            self.initialized.map_or(self.record().domain, |v| v.domain),
        ));
        self.layer = unit_placement::formation_layer(self.state.world(), self.record())
            .map_err(|_| Error::Missing("formation layer"))?
            .unwrap_or(u32::MAX);
        self.memo = logic::MovementMemo::default();
        Ok(())
    }
    fn uninitialize(&mut self, _: &mut TwoLayer) -> Result<(), Error> {
        Ok(())
    }
    fn destination_valid(&mut self, s: &mut TwoLayer) -> Result<bool, Error> {
        if s.flags & 0x4000 != 0 {
            if let Some((index, _)) = self.plot(s.destination)? {
                if !visible(&self.runtime, self.record().owner, index)? {
                    return Ok(true);
                }
            }
        }
        let flags = ((s.flags & 4) << 1) | ((s.flags >> 4) & 3) | ((s.flags >> 6) & 0x80);
        Ok(self
            .placement()?
            .formation_placement(
                self.record().owner,
                self.record().id,
                s.destination,
                PlacementRequest::from_validate_options(flags),
            )
            .map_err(Error::Placement)?
            == 1)
    }
    fn is_destination(&mut self, s: &mut TwoLayer, p: PlotCoord) -> Result<bool, Error> {
        Ok(p == s.destination)
    }
    fn heuristic(&mut self, s: &mut TwoLayer, p: PlotCoord, edge: i32) -> Result<i32, Error> {
        let map = self.state.world().map();
        Ok(logic::heuristic(
            c6_core::geometry::plot_distance(
                map.header.width,
                map.header.wrap_x,
                p.x,
                p.y,
                s.destination.x,
                s.destination.y,
            ),
            MovementPoints(edge),
        ))
    }
    fn valid(
        &mut self,
        s: &mut TwoLayer,
        parent: Option<NodeId>,
        child: NodeId,
    ) -> Result<i32, Error> {
        self.cache(s, parent, child)?;
        let movement = self.movement(s, parent, child)?;
        let mut ancestors = Vec::new();
        let mut current = parent;
        while let Some(id) = current {
            let n = s.node(id);
            ancestors.push(logic::ValidityNode {
                location: n.location(),
                turns: n.turn,
                cache: Self::node_cache(s, id),
            });
            current = n.parent;
        }
        let init = self.init()?;
        let facts = logic::LazyValidFacts {
            path_flags: s.flags,
            formation_location: self.record().location,
            destination: s.destination,
            calculated_turns: movement.turns,
            is_human: self.human,
            is_automated: init.automated,
            can_attack: init.can_attack,
            domain: init.domain,
            plot_unit_limit: self.definitions.movement.plot_unit_limit(),
            includes_land_unit: self.includes[2],
            includes_naval_unit: self.includes[0],
            formation_owner: self.record().owner,
            barbarian_tribe_index: self.tribe,
        };
        let node = logic::ValidityNode {
            location: s.node(child).location(),
            turns: movement.turns,
            cache: Self::node_cache(s, child),
        };
        let result = logic::path_valid_lazy(
            node,
            &ancestors,
            facts,
            &mut Queries {
                context: self,
                search: s,
                parent,
                child,
            },
        )?;
        if result.abort {
            s.aborted = true;
        }
        Ok(i32::from(result.valid))
    }
    fn cost(&mut self, s: &mut TwoLayer, parent: NodeId, child: NodeId) -> Result<i32, Error> {
        let movement = self.movement(s, Some(parent), child)?;
        let init = self.init()?;
        let facts = logic::LazyCostFacts {
            path_flags: s.flags,
            child_cache_flags: s.node(child).cache_flags,
            includes_land_unit: self.includes[2],
            is_human: self.human,
            is_automated: init.automated,
            formation_owner: self.record().owner,
            plot_unit_limit: self.definitions.movement.plot_unit_limit(),
        };
        Ok(logic::path_cost_lazy(
            movement,
            facts,
            &mut Queries {
                context: self,
                search: s,
                parent: Some(parent),
                child,
            },
        )?
        .0)
    }
    fn add(
        &mut self,
        s: &mut TwoLayer,
        parent: Option<NodeId>,
        child: NodeId,
        event: i32,
    ) -> Result<(), Error> {
        if event != 5 {
            self.movement(s, parent, child)?;
            self.memo.add(s.node_mut(child), event);
        }
        Ok(())
    }
    fn node_add(
        &mut self,
        s: &mut TwoLayer,
        _: Option<NodeId>,
        child: NodeId,
        event: i32,
    ) -> Result<(), Error> {
        if event as u32 > 1 || s.node(child).moves_remaining_raw <= 0 {
            return Ok(());
        }
        let at = s.node(child).location();
        if self.is_destination(s, at)? {
            return Ok(());
        }
        if s.flags & 4 != 0 && !self.mountain(at)? {
            return Ok(());
        } // Linux IsCanyon is literal false.
        if let Some(partial) = logic::prepare_partial_node(s, child) {
            s.add_to_open(self, partial)?;
        }
        Ok(())
    }
    fn extra_children_count(&mut self, s: &mut TwoLayer, parent: NodeId) -> Result<i32, Error> {
        if self.includes[0] {
            return Ok(0);
        }
        Ok(self
            .portal(s.node(parent).location())?
            .map_or(0, |p| p.exit_plot_indices.len() as i32))
    }
    fn extra_child(
        &mut self,
        s: &mut TwoLayer,
        parent: NodeId,
        index: i32,
    ) -> Result<PlotCoord, Error> {
        let portal = self
            .portal(s.node(parent).location())?
            .ok_or(Error::Missing("portal"))?;
        let raw = *portal
            .exit_plot_indices
            .get(index as usize)
            .ok_or(Error::Missing("portal exit"))?;
        let map = self.state.world().map();
        let (x, y) = map
            .coordinates(raw as u32)
            .ok_or(Error::Missing("portal exit plot"))?;
        Ok(PlotCoord { x, y })
    }
}
impl Context<'_, '_, '_> {
    fn portal(&self, at: PlotCoord) -> Result<Option<&c6_core::map::AreaPortal>, Error> {
        let mut routes = self.state.world().map().components.iter().filter_map(|c| {
            if let c6_core::map::Component::Route(r) = c {
                Some(r)
            } else {
                None
            }
        });
        let route = routes.next().ok_or(Error::Missing("Route manager"))?;
        if routes.next().is_some() {
            return Err(Error::Missing("ambiguous Route manager"));
        }
        // Empty storage is a complete absence proof. Nonempty AreaInstance
        // source-index reconstruction is not yet admitted by this importer.
        if !route.portals.is_empty() {
            return Err(Error::Missing("live portal area lookup"));
        }
        let _ = at;
        Ok(None)
    }
}
/// Native IncludesUnitOfDomain ordered member lookup. Null members are not
/// silently skipped by this getter, unlike PathInitialize's scalar reduction.
pub fn includes_unit_of_domain(
    state: &LoadedState,
    formation: &Formation,
    domain: i32,
) -> Result<bool, Error> {
    let player = state
        .world()
        .player(formation.owner)
        .map_err(Error::World)?;
    for &id in &formation.unit_ids {
        let unit = player
            .unit(id)
            .map_err(Error::World)?
            .ok_or(Error::Missing("domain member"))?
            .record;
        if state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?
            .domain
            == domain
        {
            return Ok(true);
        }
    }
    Ok(false)
}

impl GenerationContext for Context<'_, '_, '_> {
    fn formation(&self) -> &Formation {
        self.record()
    }
    fn formation_mut(&mut self) -> Result<&mut Formation, Error> {
        self.state
            .formation_mut(self.formation.0, self.formation.1)
            .map_err(Error::State)
    }
    fn includes_unit_of_domain(&mut self, domain: i32) -> Result<bool, Error> {
        includes_unit_of_domain(self.state, self.record(), domain)
    }
    fn is_plot(&mut self, at: PlotCoord) -> Result<bool, Error> {
        Ok(self.plot(at)?.is_some())
    }
    fn is_visible(&mut self, at: PlotCoord) -> Result<bool, Error> {
        let (index, _) = self.require_plot(at)?;
        visible(&self.runtime, self.record().owner, index)
    }
    fn can_generate(&mut self) -> Result<bool, Error> {
        Ok(!matches!(
            self.runtime.execution,
            ExecutionContext::ApplicationThread {
                game_core_executing: true
            }
        ))
    }
}
struct Queries<'q, 's, 'a, 'r> {
    context: &'q mut Context<'s, 'a, 'r>,
    search: &'q mut TwoLayer,
    parent: Option<NodeId>,
    child: NodeId,
}
impl Queries<'_, '_, '_, '_> {
    fn child(&self) -> PlotCoord {
        self.search.node(self.child).location()
    }
    fn parent(&self) -> Result<PlotCoord, Error> {
        self.parent
            .map(|p| self.search.node(p).location())
            .ok_or(Error::Missing("parent node"))
    }
    fn owner(&self, at: PlotCoord) -> Result<i32, Error> {
        placement::plot_owner(self.context.require_plot(at)?.1).map_err(Error::Placement)
    }
    fn feature_index(&self) -> Result<Option<usize>, Error> {
        let hash = self.context.require_plot(self.child())?.1.feature_hash;
        Ok(self.context.definitions.features.find_index(hash))
    }
}
impl LazyPathQueries for Queries<'_, '_, '_, '_> {
    type Error = Error;
    fn boolean(&mut self, q: B) -> Result<bool, Error> {
        let c = &self.context;
        match q {
            B::ChildVisible => {
                let (i, _) = c.require_plot(self.child())?;
                visible(&c.runtime, c.record().owner, i)
            }
            B::ChildRevealed => {
                let (i, _) = c.require_plot(self.child())?;
                revealed(c.state, c.record().owner, i)
            }
            B::ChildMountain => c.mountain(self.child()),
            B::ChildImpassable => Ok(c
                .require_plot(self.child())?
                .1
                .flags
                .get(PlotFlag::Impassable)),
            B::ChildWater => c.water(self.child()),
            B::ParentWater => c.water(self.parent()?),
            B::ChildHills => Ok(c.require_plot(self.child())?.1.flags.get(PlotFlag::Hills)),
            B::ChildCanyon => Ok(false), // target 0x2bc9c00 is xor eax,eax;ret.
            B::AtWar { other, reverse } => {
                let (owner, target) = if reverse {
                    (other, c.record().owner)
                } else {
                    (c.record().owner, other)
                };
                let relations = c.state.game_relations().ok();
                let war = relations.and_then(|r| {
                    r.war().map(|w| unit_placement::WarRelations {
                        player_count: r.player_count(),
                        war_types: w.values(),
                    })
                });
                unit_placement::Inputs {
                    world: c.state.world(),
                    definitions: None,
                    relations: war.as_ref(),
                    plot_unit_limit: None,
                }
                .at_war_with(owner, target)
                .map_err(Error::PlacementQuery)
            }
            B::ParentEnforcesBorders { owner } => Ok(c
                .state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == owner)
                .ok_or(Error::Missing("border owner"))?
                .diplomacy
                .can_enforce_borders),
            B::OpenBordersFrom { owner } => c
                .placement()?
                .open_borders(c.record().owner, owner)
                .map_err(Error::Placement),
            B::AdjacentToFormation => {
                movement_world::adjacent(c.state.world().map(), self.child(), c.record().location)
                    .map_err(|_| Error::Missing("adjacency"))
            }
            B::SlowedByZoc => cost::slowed_by_zoc_cached(
                c.state,
                &c.definitions.movement,
                &c.placement()?,
                c.record(),
                self.child(),
                Some(self.search.destination),
                cost::ZocIdentity {
                    owner: c.record().owner,
                    domain: c.init()?.domain,
                    religion: c.religion,
                    layer: c.layer,
                    ignores_zoc: c.record().ignores_zoc,
                },
                Some(c.runtime.visibility),
            )
            .map_err(Error::Cost),
            B::CanEmbarkTo => shore::query(c, self.parent()?, self.child(), None),
            B::CanDisembarkTo { allow_unembarked } => shore::query(
                c,
                self.parent()?,
                self.child(),
                Some((allow_unembarked, self.search.flags & (1 << 13) != 0)),
            ),
            B::BarbarianClansActive => c
                .capabilities
                .map(|capabilities| capabilities.has(c6_core::game_capabilities::BARBARIAN_CLANS))
                .ok_or(Error::Missing("current Barbarian Clans activation")),
            B::FeatureAllowPassthrough | B::FeatureAllowDestination => {
                Err(Error::Missing("ordered Feature_UnitMovement import"))
            }
        }
    }
    fn integer(&mut self, q: I) -> Result<i32, Error> {
        let c = &self.context;
        match q {
            I::ParentOwner => self.owner(self.parent()?),
            I::ChildOwner => self.owner(self.child()),
            I::FriendlyUnitsOfType => c
                .placement()?
                .formation_count(c.record(), self.child(), c.layer)
                .map(|v| v as i32)
                .map_err(Error::Placement),
            I::PlacementResult => c
                .placement()?
                .formation_placement(
                    c.record().owner,
                    c.record().id,
                    self.child(),
                    PlacementRequest::shore_query(false),
                )
                .map_err(Error::Placement),
            I::BaseMoves { domain } => c
                .init()?
                .base_moves
                .get(domain as usize)
                .copied()
                .ok_or(Error::Missing("base move domain")),
            I::ForeignCityOwner => {
                let (_, plot) = c.require_plot(self.child())?;
                let Some(d) = c
                    .definitions
                    .movement
                    .district_at(c.state, plot)
                    .map_err(Error::Cost)?
                else {
                    return Ok(-1);
                };
                // CityAttributes::IsCity tests the resolved native District hash.
                if !c
                    .definitions
                    .movement
                    .district_is_city(d)
                    .map_err(Error::Cost)?
                {
                    return Ok(-1);
                }
                plot.city_attributes
                    .as_ref()
                    .map(|a| i32::from(a.player))
                    .ok_or(Error::Missing("city attributes"))
            }
            I::ForeignDefensibleDistrictOwner => {
                let (_, plot) = c.require_plot(self.child())?;
                let Some(d) = c
                    .definitions
                    .movement
                    .district_at(c.state, plot)
                    .map_err(Error::Cost)?
                else {
                    return Ok(-1);
                };
                if !c
                    .definitions
                    .movement
                    .district_defensible(d)
                    .map_err(Error::Cost)?
                {
                    return Ok(-1);
                }
                plot.city_attributes
                    .as_ref()
                    .map(|a| i32::from(a.player))
                    .ok_or(Error::Missing("district attributes"))
            }
            I::FeatureDanger => self
                .feature_index()?
                .and_then(|i| c.definitions.feature_danger.get(i).copied())
                .ok_or(Error::Missing("feature danger")),
            I::AdjacentNonRevealedCount => {
                let mut count = 0;
                for at in unit_placement::query_plots(c.state.world().map(), self.child())
                    .map_err(|_| Error::Missing("adjacent plots"))?
                    .into_iter()
                    .skip(1)
                    .flatten()
                {
                    let (index, _) = c.require_plot(at)?;
                    if !revealed(c.state, c.record().owner, index)? {
                        count += 1;
                    }
                }
                Ok(count)
            }
            I::Influence => {
                let owner = c.record().owner;
                let player = c
                    .state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == owner)
                    .ok_or(Error::Missing("current influence player"))?;
                let map = player
                    .ai_influence_map
                    .as_ref()
                    .ok_or(Error::Missing("current pathfinder influence map"))?;
                let (index, _) = c.require_plot(self.child())?;
                Ok(map.influence_at(index as i32))
            }
            I::BribeTurnsRemaining { .. } => {
                Err(Error::Missing("current Barbarian Clan bribe state"))
            }
        }
    }
    fn feature_movement_count(&mut self) -> Result<Option<u32>, Error> {
        if self.feature_index()?.is_none() {
            return Ok(None);
        }
        if !self.context.definitions.feature_movements_empty {
            return Err(Error::Missing("ordered Feature_UnitMovement references"));
        }
        Ok(Some(0))
    }
    fn visible_enemy(&mut self) -> Result<(bool, bool), Error> {
        visible_units::enemy(self.context, self.child())
    }
    fn movement_cost(
        &mut self,
        base_moves: i32,
        available: MovementPoints,
        zoc_value: i32,
    ) -> Result<(MovementPoints, MovementPoints), Error> {
        let c = &self.context;
        let zoc = match zoc_value {
            0 => ZocValue::Query,
            1 => ZocValue::NotSlowed,
            2 => ZocValue::Slowed,
            _ => return Err(Error::Missing("ZOC enum")),
        };
        let result = cost::movement_cost(
            c.state,
            &c.definitions.movement,
            &c.placement()?,
            c.record(),
            self.parent()?,
            self.child(),
            Some(self.search.destination),
            MoveBudget {
                base_moves,
                max_moves: MovementPoints::integer(c.init()?.max_moves),
                remaining: available,
                exit_moves: MovementPoints(0),
            },
            zoc,
            self.search.flags,
            Some(c.runtime.visibility),
        )
        .map_err(Error::Cost)?;
        Ok((result.final_cost(), result.to_plot_exit))
    }
}

impl Context<'_, '_, '_> {
    /// Preserve evaluating-operation replans and their intermediate cache writes.
    pub fn update_path_cache(
        &mut self,
        search: &mut TwoLayer,
        destination: PlotCoord,
        flags: u32,
    ) -> Result<bool, Error> {
        struct Cache<'q, 's, 'a, 'r> {
            context: &'q mut Context<'s, 'a, 'r>,
            search: &'q mut TwoLayer,
        }
        impl formation_path_cache::PathCacheContext for Cache<'_, '_, '_, '_> {
            type Error = Error;
            fn formation(&self) -> &Formation {
                self.context.record()
            }
            fn is_evaluating_operation(&self) -> Result<bool, Error> {
                Ok(self.context.runtime.is_evaluating_operation)
            }
            fn plot_is_visible(&mut self, at: PlotCoord) -> Result<bool, Error> {
                let Some((index, _)) = self.context.plot(at)? else {
                    return Ok(false);
                };
                visible(&self.context.runtime, self.context.record().owner, index)
            }
            fn is_plot(&mut self, at: PlotCoord) -> Result<bool, Error> {
                GenerationContext::is_plot(self.context, at)
            }
            fn generate_path(&mut self, at: PlotCoord, flags: u32) -> Result<bool, Error> {
                self.context.generate(self.search, at, flags, false)
            }
        }
        formation_path_cache::update_path_cache(
            &mut Cache {
                context: self,
                search,
            },
            destination,
            flags,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c6_core::{
        map::VisibilityLayer,
        visibility_change::{SharedLayers, BASE_LAYER_HASH},
    };

    #[test]
    fn path_visibility_uses_signed_live_shared_normal_counts() {
        let make = || VisibilityLayer {
            type_hash: BASE_LAYER_HASH,
            index: 0,
            count: 3,
            grid_width: 3,
            values: vec![-1, 0, 1],
        };
        let visibility = crate::unit_sight_state::Runtime {
            shared: vec![crate::unit_sight_state::SharedVisibility {
                owner: 7,
                layers: SharedLayers(vec![make()]),
            }],
            hidden_layer_hashes: vec![],
            messages_disabled: false,
            local_observer: None,
        };
        let runtime = Runtime {
            visibility: &visibility,
            civilization_levels: None,
            city_domains: None,
            execution: ExecutionContext::ApplicationThread {
                game_core_executing: true,
            },
            is_evaluating_operation: false,
        };
        assert!(!visible(&runtime, 7, 0).unwrap());
        assert!(!visible(&runtime, 7, 1).unwrap());
        assert!(visible(&runtime, 7, 2).unwrap());
        assert!(visible(&runtime, 0, 2).is_err());
        assert!(visible(&runtime, 7, 3).is_err());
        let mut visibility = visibility;
        visibility.shared[0].layers.0.push(make());
        let runtime = Runtime {
            visibility: &visibility,
            civilization_levels: None,
            city_domains: None,
            execution: ExecutionContext::ApplicationThread {
                game_core_executing: true,
            },
            is_evaluating_operation: false,
        };
        assert!(visible(&runtime, 7, 2).is_err());
    }
}
