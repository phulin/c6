//! Bounded Linux planning forecast inputs, without selecting or issuing orders.
//!
//! This preserves Planning::Instance's two-turn Warrior/Scout search and ordered
//! District combat positions. The caller executes every reached forecast's
//! proposal creation/cleanup callbacks; these inputs are not completed combat.
use super::*;

/// DoCombatSimulations first forecasts Unit -> District at `attack_location`.
/// When the current District.GetAttackStrength(false) is nonzero, it then
/// forecasts District -> Unit at each `retaliation_locations` entry in order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DistrictForecast {
    pub attacker: (i32, u32),
    pub defender: (i32, u32),
    pub attack_location: PlotCoord,
    pub retaliation_locations: Vec<PlotCoord>,
}

struct Search<'s, 'a, 'r> {
    context: Context<'s, 'a, 'r>,
    target: PlotCoord,
    unit_location: PlotCoord,
    max_moves: i32,
    positions: Vec<PlotCoord>,
}
impl Search<'_, '_, '_> {
    fn at_war(&self, other: i32) -> Result<bool, Error> {
        let c = &self.context;
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
        .at_war_with(c.record().owner, other)
        .map_err(Error::PlacementQuery)
    }

    /// AddReachablePosition21b8200's combat-only mode, for land range0 and
    /// TARGET_ENEMY_COMBAT_DISTRICT. No cached planning answers are inputs.
    fn record_combat_position(
        &mut self,
        at: PlotCoord,
        parent: PlotCoord,
        remaining: i32,
    ) -> Result<(), Error> {
        let c = &self.context;
        if c.placement()?
            .formation_placement(
                c.record().owner,
                c.record().id,
                at,
                PlacementRequest::from_validate_options(1),
            )
            .map_err(Error::Placement)?
            == 0
        {
            return Ok(());
        }
        let map = c.state.world().map();
        let distance = c6_core::geometry::plot_distance(
            map.header.width,
            map.header.wrap_x,
            at.x,
            at.y,
            self.target.x,
            self.target.y,
        );
        let candidate = if distance == 0 {
            // For a land unit entering the target tile, combat starts at the
            // proposed parent, including the source for a root callback.
            if c.water(at)? {
                return Ok(());
            }
            Some(parent)
        } else if distance == 1 && (remaining == 0 || at == self.unit_location) {
            let (index, _) = c.require_plot(self.target)?;
            let target_units = c
                .state
                .world()
                .plot_units(c.layer, index)
                .map_err(Error::World)?;
            target_units
                .iter()
                .all(|unit| unit.player() == c.record().owner)
                .then_some(at)
        } else {
            None
        };
        if let Some(candidate) = candidate {
            // Source21b8757..8841: target water is excluded for land range0;
            // first six unique positions survive. Later better paths do not
            // replace an already recorded position or alter its order.
            if !c.water(self.target)?
                && self.positions.len() < 6
                && !self.positions.contains(&candidate)
            {
                self.positions.push(candidate);
            }
        }
        Ok(())
    }
}
impl PathCallbacks for Search<'_, '_, '_> {
    type Error = Error;
    fn initialize(&mut self, s: &mut TwoLayer) -> Result<(), Error> {
        self.context.initialize(s)?;
        // CustomMovesRemainingPathInitialize uses full Formation.MaxMoves.
        // A single member makes the native min/escort reduction equal the
        // canonical current Unit.MaxMoves admitted before this search.
        self.context.starting_remaining_override = Some(self.max_moves.wrapping_shl(8));
        Ok(())
    }
    fn uninitialize(&mut self, s: &mut TwoLayer) -> Result<(), Error> {
        self.context.uninitialize(s)
    }
    fn destination_valid(&mut self, _: &mut TwoLayer) -> Result<bool, Error> {
        Ok(true)
    }
    fn is_destination(&mut self, _: &mut TwoLayer, _: PlotCoord) -> Result<bool, Error> {
        Ok(false)
    }
    fn heuristic(&mut self, _: &mut TwoLayer, _: PlotCoord, _: i32) -> Result<i32, Error> {
        Ok(0)
    }
    fn valid(
        &mut self,
        s: &mut TwoLayer,
        parent: Option<NodeId>,
        child: NodeId,
    ) -> Result<i32, Error> {
        self.context.cache(s, parent, child)?;
        let mut movement = self.context.movement(s, parent, child)?;
        if parent.is_some_and(|id| s.node(id).cache_flags & 0x4000 != 0) {
            return Ok(0);
        }
        let mut obstructed = false;
        if let Some((index, plot)) = self.context.plot(s.node(child).location())? {
            if let Some(district) = self
                .context
                .definitions
                .movement
                .district_at(self.context.state, plot)
                .map_err(Error::Cost)?
            {
                if self.at_war(placement::plot_owner(plot).map_err(Error::Placement)?)? {
                    obstructed = self
                        .context
                        .definitions
                        .movement
                        .district_definition_hit_points(district)
                        .map_err(Error::Cost)?
                        .unwrap_or(0)
                        > 0;
                }
            }
            if !obstructed && s.flags & 0x200 == 0 {
                // Native scans current layer membership, without visibility or
                // delayed-death filtering, and tests IsCombat for enemy units.
                for reference in self
                    .context
                    .state
                    .world()
                    .plot_units(self.context.layer, index)
                    .map_err(Error::World)?
                {
                    if self.at_war(reference.player())? {
                        if let Some(unit) = self
                            .context
                            .state
                            .world()
                            .unit(*reference)
                            .map_err(Error::World)?
                        {
                            obstructed |= crate::combat_defender::is_combat(
                                self.context.state,
                                &self.context.definitions.formation,
                                unit.record,
                            )
                            .map_err(|_| Error::Missing("planning enemy combat query"))?;
                        }
                    }
                }
            }
        }
        if obstructed {
            movement.cost.0 = movement.cost.0.wrapping_add(movement.remaining.0);
            movement.remaining = MovementPoints(0);
            movement.exit_cost = MovementPoints(0);
            self.context.memo.override_result(movement);
        } else if let Some(parent) = parent {
            let at = s.node(parent).location();
            let c = &self.context;
            if s.flags & 0x200 == 0 && at != c.record().location {
                let slowed = cost::slowed_by_zoc_cached(
                    c.state,
                    &c.definitions.movement,
                    &c.placement()?,
                    c.record(),
                    at,
                    None,
                    cost::ZocIdentity {
                        owner: c.record().owner,
                        domain: c.init()?.domain,
                        religion: c.religion,
                        layer: c.layer,
                        ignores_zoc: c.record().ignores_zoc,
                    },
                    Some(c.runtime.visibility),
                )
                .map_err(Error::Cost)?;
                if slowed {
                    return Ok(0);
                }
            }
        }
        if movement.turns > 2 || movement.turns == 2 && movement.remaining.0 < 0 {
            return Ok(0);
        }
        self.context.valid(s, parent, child)
    }
    fn cost(&mut self, s: &mut TwoLayer, parent: NodeId, child: NodeId) -> Result<i32, Error> {
        self.context.cost(s, parent, child)
    }
    fn add(
        &mut self,
        s: &mut TwoLayer,
        parent: Option<NodeId>,
        child: NodeId,
        event: i32,
    ) -> Result<(), Error> {
        // PlanningChildNodeAdd records combat candidates even for event5,
        // before an existing node is tested for a better parent.
        let movement = self.context.movement(s, parent, child)?;
        let at = s.node(child).location();
        let from = parent.map_or(at, |id| s.node(id).location());
        self.record_combat_position(at, from, movement.remaining.0)?;
        self.context.add(s, parent, child, event)
    }
    fn node_add(
        &mut self,
        s: &mut TwoLayer,
        _: Option<NodeId>,
        child: NodeId,
        event: i32,
    ) -> Result<(), Error> {
        // The separate ordinary move-result vector has no consumer here.
        // Preserve PathNodeAdd's partial-node insertion using this callback
        // owner, so later add callbacks still record planning candidates.
        if event as u32 > 1 || s.node(child).moves_remaining_raw <= 0 {
            return Ok(());
        }
        let at = s.node(child).location();
        if s.flags & 4 != 0 && !self.context.mountain(at)? {
            return Ok(());
        }
        if let Some(partial) = logic::prepare_partial_node(s, child) {
            s.add_to_open(self, partial)?;
        }
        Ok(())
    }
    fn extra_children_count(&mut self, s: &mut TwoLayer, parent: NodeId) -> Result<i32, Error> {
        self.context.extra_children_count(s, parent)
    }
    fn extra_child(
        &mut self,
        s: &mut TwoLayer,
        parent: NodeId,
        index: i32,
    ) -> Result<PlotCoord, Error> {
        self.context.extra_child(s, parent, index)
    }
}

/// Current Unit.IsCombat gate for appending District combat positions. Source
/// AddReachablePosition21b8832..885f requires Unit.IsCombat for a District;
/// religious strength only substitutes for that gate on religious Unit targets.
/// This does not admit those other targets or complete any planning transition.
pub fn has_district_combat_forecasts(
    state: &LoadedState,
    definitions: &crate::formation_rules::Definitions,
    unit: &c6_core::unit::Unit,
) -> Result<bool, Error> {
    crate::combat_defender::is_combat(state, definitions, unit)
        .map_err(|_| Error::Missing("planning Unit.IsCombat query"))
}

/// Compute positions for one live, unembarked range0 planner: Warrior/max2
/// or Scout/max3, with searchTurns2 and one current enemy combat District. The caller admits the
/// complete planner/target roster and rejects other nearby combat targets.
/// Unsupported reached movement/placement inputs remain errors.
pub fn district_forecast(
    context: Context<'_, '_, '_>,
    planner_unit: u32,
    district: (i32, u32),
    search_turns: i32,
) -> Result<Option<DistrictForecast>, Error> {
    let owner = context.record().owner;
    let player = context.state.world().player(owner).map_err(Error::World)?;
    let unit = player
        .unit(planner_unit)
        .map_err(Error::World)?
        .ok_or(Error::Missing("planning Unit"))?
        .record;
    let runtime = context
        .state
        .unit_runtime(owner, planner_unit)
        .map_err(Error::State)?;
    let max_moves = unit_moves::unit_max_moves(
        context.state,
        unit,
        &unit_moves::Inputs {
            definitions: &context.definitions.units,
        },
    )
    .map_err(Error::UnitMoves)?;
    let supported = unit.e_type_index == crate::registry::linux_type_hash(b"UNIT_WARRIOR")
        && max_moves == 2
        || unit.e_type_index == crate::registry::linux_type_hash(b"UNIT_SCOUT") && max_moves == 3;
    if search_turns != 2
        || context.record().unit_ids.as_slice() != [planner_unit]
        || !supported
        || unit.b_delayed_death
        || unit_moves::unit_is_embarked(unit)
        || runtime.domain != 2
        || runtime.formation.is_none_or(|formation| {
            formation.owner != owner || formation.resolved_id != context.record().id
        })
        || unit.k_location != context.record().location
        || !has_district_combat_forecasts(context.state, &context.definitions.formation, unit)?
        || unit.k_attack_range_modifiers.i_attack_range != 0
    {
        return Err(Error::Missing(
            "planning Warrior/max2 or Scout/max3, searchTurns2/range0 scope",
        ));
    }
    let unit_location = unit.k_location;
    context.require_plot(unit_location)?;
    let target = context
        .state
        .district(district.0, district.1)
        .map_err(|_| Error::Missing("planning target District"))?;
    let target_location = PlotCoord {
        x: target.location.0,
        y: target.location.1,
    };
    if context
        .definitions
        .movement
        .district_definition_hit_points(target)
        .map_err(Error::Cost)?
        .unwrap_or(0)
        <= 0
    {
        return Err(Error::Missing("planning target combat District"));
    }
    context.require_plot(target_location)?;
    let map = context.state.world().map();
    let distance = c6_core::geometry::plot_distance(
        map.header.width,
        map.header.wrap_x,
        unit_location.x,
        unit_location.y,
        target_location.x,
        target_location.y,
    );
    let mut engine = TwoLayer::new(
        map.header.width,
        map.header.height,
        map.header.wrap_x,
        map.header.wrap_y,
    )
    .ok_or(Error::Missing("planning map geometry"))?;
    let mut search = Search {
        context,
        target: target_location,
        unit_location,
        max_moves,
        positions: Vec::new(),
    };
    if !search.at_war(district.0)? {
        return Err(Error::Missing("planning target enemy District"));
    }
    if distance > max_moves.wrapping_mul(search_turns) {
        return Ok(None);
    }
    engine.generate(
        &mut search,
        unit_location,
        PlotCoord { x: -9999, y: -9999 },
        0x915,
        false,
    )?;
    Ok((!search.positions.is_empty()).then_some(DistrictForecast {
        attacker: (owner, planner_unit),
        defender: district,
        attack_location: target_location,
        retaliation_locations: search.positions,
    }))
}
