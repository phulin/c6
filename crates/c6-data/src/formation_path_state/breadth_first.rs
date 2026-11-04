//! Source PlotsOnly BFS specialization used by UnitInfluenceJob.
use super::*;

struct Search<'s, 'a, 'r> {
    context: Context<'s, 'a, 'r>,
    positions: Vec<PlotCoord>,
    requires_moves: bool,
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
    fn can_record(&self, s: &TwoLayer, child: NodeId) -> Result<bool, Error> {
        let node = s.node(child);
        if node.cache_flags & 2 == 0 {
            return Ok(true);
        }
        if node.cache_flags & 0x4c != 0x40 {
            return Ok(false);
        }
        if node.cache_other as u16 == 0 {
            return Ok(true);
        }
        let c = &self.context;
        let (index, _) = c.require_plot(node.location())?;
        let units = c
            .state
            .world()
            .plot_units(c.layer, index)
            .map_err(Error::World)?;
        Ok(units
            .first()
            .is_none_or(|unit| unit.player() == c.record().owner))
    }
}
impl PathCallbacks for Search<'_, '_, '_> {
    type Error = Error;
    fn initialize(&mut self, s: &mut TwoLayer) -> Result<(), Error> {
        self.context.initialize(s)?;
        let c = &self.context;
        let player = c
            .state
            .world()
            .player(c.record().owner)
            .map_err(Error::World)?;
        let inputs = unit_moves::Inputs {
            definitions: &c.definitions.units,
        };
        let mut minimum = 9001;
        let mut escort = 0;
        for &id in &c.record().unit_ids {
            if let Some(found) = player.unit(id).map_err(Error::World)? {
                let unit = found.record;
                let moves =
                    unit_moves::unit_max_moves(c.state, unit, &inputs).map_err(Error::UnitMoves)?;
                minimum = minimum.min(moves);
                if unit.k_movement_modifiers.b_escort_mobility {
                    escort = escort.max(moves);
                }
            }
        }
        self.context.starting_remaining_override = Some(minimum.max(escort).wrapping_shl(8));
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
        if let Some((_, plot)) = self.context.plot(s.node(child).location())? {
            if let Some(district) = self
                .context
                .definitions
                .movement
                .district_at(self.context.state, plot)
                .map_err(Error::Cost)?
            {
                let owner = placement::plot_owner(plot).map_err(Error::Placement)?;
                if self.at_war(owner)? {
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
        }
        // This specialization always includes mask0x200, so native skips its
        // enemy-unit scan and parent ZOC test. No visibility answer replaces it.
        if obstructed {
            movement.cost.0 = movement.cost.0.wrapping_add(movement.remaining.0);
            movement.remaining = MovementPoints(0);
            movement.exit_cost = MovementPoints(0);
            self.context.memo.override_result(movement);
        }
        if movement.turns > 1 || movement.turns == 1 && movement.remaining.0 < 0 {
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
        // PlotsOnly::GetsCombatInfo is literal false. The child delegate takes
        // its ordinary PathAdd tail without an AddReachablePosition call.
        self.context.add(s, parent, child, event)
    }
    fn node_add(
        &mut self,
        s: &mut TwoLayer,
        parent: Option<NodeId>,
        child: NodeId,
        event: i32,
    ) -> Result<(), Error> {
        if (event == 5 || parent.is_none() && event == 3) && self.can_record(s, child)? {
            let at = s.node(child).location();
            let flags = ((s.flags & 4) << 1) | ((s.flags >> 4) & 3) | ((s.flags >> 6) & 0x80);
            let c = &self.context;
            let result = c
                .placement()?
                .formation_placement(
                    c.record().owner,
                    c.record().id,
                    at,
                    PlacementRequest::from_validate_options(flags),
                )
                .map_err(Error::Placement)?;
            if result != 0
                && (!self.requires_moves || s.node(child).moves_remaining_raw != 0)
                && !self.positions.contains(&at)
            {
                self.positions.push(at);
            }
        }
        self.context.node_add(s, parent, child, event)
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

/// Native UnitInfluenceJob uses custom full movement, one turn and flags0x311.
/// The search exhausts the open list; false GeneratePath does not mean failure.
pub fn influence_positions(
    context: Context<'_, '_, '_>,
    requires_moves: bool,
) -> Result<Vec<PlotCoord>, Error> {
    let map = context.state.world().map();
    let mut engine = TwoLayer::new(
        map.header.width,
        map.header.height,
        map.header.wrap_x,
        map.header.wrap_y,
    )
    .ok_or(Error::Missing("BFS map dimensions"))?;
    let source = context.record().location;
    let mut search = Search {
        context,
        positions: Vec::new(),
        requires_moves,
    };
    engine.generate(
        &mut search,
        source,
        PlotCoord { x: -9999, y: -9999 },
        0xb11,
        false,
    )?;
    Ok(search.positions)
}
