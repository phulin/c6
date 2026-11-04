//! Linux XP2 Formation movement callback arithmetic and node-cache encoding.
//!
//! These functions consume resolved world queries; they do not establish that a
//! complete path or MoveTo action is supported. In particular callers must resolve
//! placement, visibility, ZOC and movement costs against the actual live world.
use crate::movement::MovementPoints;
use crate::unit::PlotCoord;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitializationMember {
    pub embarked: bool,
    pub automated: bool,
    pub can_attack: bool,
    pub max_moves: i32,
    pub base_moves: [i32; 5],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitializedMembers {
    pub embarked: bool,
    pub automated: bool,
    pub can_attack: bool,
    pub max_moves: i32,
    pub base_moves: [i32; 5],
    pub domain: i32,
}

/// PathInitialize's ordered member reduction (0x2947210). Missing unit IDs are
/// skipped. MaxMoves starts at zero; base moves start at INT_MAX, as in native.
/// The old domain survives when no domain predicate succeeds.
pub fn initialize_members(
    members: impl IntoIterator<Item = Option<InitializationMember>>,
    includes_domains: [bool; 5],
    previous_domain: i32,
) -> InitializedMembers {
    let mut result = InitializedMembers {
        embarked: false,
        automated: false,
        can_attack: false,
        max_moves: 0,
        base_moves: [i32::MAX; 5],
        domain: previous_domain,
    };
    for member in members.into_iter().flatten() {
        result.embarked |= member.embarked;
        result.automated |= member.automated;
        result.can_attack |= member.can_attack;
        result.max_moves = result.max_moves.min(member.max_moves);
        for (base, member_base) in result.base_moves.iter_mut().zip(member.base_moves) {
            *base = (*base).min(member_base);
        }
    }
    if includes_domains[0] || result.embarked {
        result.domain = 0;
    } else if includes_domains[2] {
        result.domain = 2;
    } else if includes_domains[1] {
        result.domain = 1;
    } else if includes_domains[4] {
        result.domain = 4;
    }
    result
}

/// PathDestValid (0x2946440), using IsValidLocation with movement placement flags.
pub fn destination_valid(
    formation_present: bool,
    path_flags: u32,
    visible: bool,
    placement_result: i32,
) -> bool {
    !formation_present || path_flags & 0x4000 != 0 && !visible || placement_result == 1
}

/// PathHeuristic (0x2948fc0). Distance must be the target's wrapped plot distance.
pub fn heuristic(plot_distance: i32, edge_cost: MovementPoints) -> i32 {
    let factor = if edge_cost.0 < 0 {
        1000
    } else {
        (edge_cost.0 >> 8).min(1000)
    };
    plot_distance.wrapping_mul(factor)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MovementResult {
    pub cost: MovementPoints,
    pub remaining: MovementPoints,
    pub exit_cost: MovementPoints,
    pub turns: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParentMovement {
    pub location: PlotCoord,
    pub remaining: MovementPoints,
    pub exit_cost: MovementPoints,
    pub turns: i32,
    pub cache_flags: u32,
}

/// Inputs to CalculateMovementPoints after native world queries have resolved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MovementStep {
    pub formation_location: PlotCoord,
    pub path_flags: u32,
    pub child_cache_flags: u32,
    pub child_is_water: bool,
    pub starting_remaining: MovementPoints,
    /// Base moves for the domain selected by the callback (land = 2, sea = 0).
    pub selected_base_moves: i32,
    /// Base moves in the parent's actual water/land domain, used for ZOC renewal.
    pub parent_domain_base_moves: i32,
    /// Native IsSlowedByZOC result, queried when the parent has no moves left.
    pub slowed_by_zoc: bool,
    /// MovementCost::GetFinalCost, required when either node is revealed.
    pub movement_cost: MovementPoints,
    pub to_plot_exit_cost: MovementPoints,
}

/// CalculateMovementPoints (0x2945790), excluding its last-node-pair memoization.
/// Memoization uses node identity, not coordinates or equal scalar contents.
pub fn calculate_movement_points(
    parent: Option<ParentMovement>,
    step: MovementStep,
) -> MovementResult {
    let Some(parent) = parent else {
        return MovementResult {
            remaining: step.starting_remaining,
            turns: 1,
            ..MovementResult::default()
        };
    };
    let revealed = (parent.cache_flags | step.child_cache_flags) & 0x100 != 0;
    let mut turns = parent.turns;
    let mut available = parent.remaining.0;
    if available <= 0 {
        turns = turns.wrapping_add(1);
        available = MovementPoints::integer(if step.slowed_by_zoc {
            step.parent_domain_base_moves
        } else {
            step.selected_base_moves
        })
        .0;
    }
    let (mut cost, mut exit_cost) = if revealed {
        (
            step.movement_cost.0.wrapping_add(parent.exit_cost.0),
            step.to_plot_exit_cost.0,
        )
    } else {
        (
            if parent.exit_cost.0 == 0 {
                256
            } else {
                parent.exit_cost.0
            },
            0,
        )
    };
    let mut remaining = available.wrapping_sub(cost);
    if remaining < 0 {
        remaining = 0;
        exit_cost = 0;
        if !(parent.location == step.formation_location && step.path_flags & 0x100 != 0)
            && parent.turns == turns
        {
            turns = turns.wrapping_add(1);
        } else if available > 0 {
            cost = available;
        } else if cost <= 0 {
            cost = 256;
        }
    }
    MovementResult {
        cost: MovementPoints(cost),
        remaining: MovementPoints(remaining),
        exit_cost: MovementPoints(exit_cost),
        turns,
    }
}

/// Domain selection in CalculateMovementPoints. When neither node is revealed,
/// the parent's remembered water bit controls the base-move query.
pub fn movement_domain(
    parent_cache_flags: u32,
    child_cache_flags: u32,
    child_is_water: bool,
) -> i32 {
    let water = if (parent_cache_flags | child_cache_flags) & 0x100 != 0 {
        child_is_water
    } else {
        parent_cache_flags & 0x20 != 0
    };
    if water {
        0
    } else {
        2
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PathCostFacts {
    pub path_flags: u32,
    pub child_cache_flags: u32,
    pub includes_land_unit: bool,
    pub parent_is_water: bool,
    pub child_is_water: bool,
    pub can_embark_to: bool,
    pub can_disembark_to: bool,
    pub child_is_hills: bool,
    pub plot_unit_limit: u32,
    pub friendly_units_of_type: u32,
    pub child_is_mountain: bool,
    pub child_is_canyon: bool,
    pub is_human: bool,
    pub is_automated: bool,
    pub child_owned_by_formation_owner: bool,
    pub adjacent_non_revealed_count: i32,
    pub child_is_revealed: bool,
}

/// PathCost (0x2946520). This score is distinct from movement points consumed.
pub fn path_cost(movement: MovementResult, f: PathCostFacts) -> MovementPoints {
    let mut cost = movement.cost.0.wrapping_mul(1000);
    let automatic = !f.is_human || f.is_automated;
    let revealed = f.child_cache_flags & 0x100 != 0;
    if movement.remaining.0 <= 0 {
        cost = cost.wrapping_add(4 << 8);
        if revealed {
            let mut add = |integer: i32| {
                cost = cost.wrapping_add(integer.wrapping_shl(8));
            };
            if f.includes_land_unit && !f.parent_is_water && f.child_is_water && !f.can_embark_to {
                add(1_000_000);
            }
            if f.includes_land_unit && f.parent_is_water && !f.child_is_water && !f.can_disembark_to
            {
                add(1_000_000);
            }
            if f.path_flags & 0x40 != 0 && !f.child_is_hills {
                add(300);
            }
            if f.plot_unit_limit != 0
                && f.path_flags & 4 == 0
                && f.friendly_units_of_type >= f.plot_unit_limit
            {
                add(1_000_000);
            }
            if f.child_is_mountain && automatic {
                add(1_000_000);
            }
            if f.child_is_canyon && automatic {
                add(1_000_000);
            }
            if !f.child_owned_by_formation_owner {
                add(3);
            }
        }
    }
    if f.path_flags & 0x40 != 0 {
        let visibility = 7i32
            .wrapping_sub(f.adjacent_non_revealed_count)
            .wrapping_sub(i32::from(!f.child_is_revealed))
            .wrapping_mul(10);
        cost = cost.wrapping_add(visibility.wrapping_shl(8));
    }
    if revealed && f.includes_land_unit && f.child_is_water && automatic {
        cost = cost.wrapping_add(1000 << 8);
    }
    cost = cost.wrapping_add(if revealed || f.path_flags & 0x40 != 0 {
        2 << 8
    } else {
        3 << 8
    });
    MovementPoints(cost)
}

/// Whether PathNodeAdd (0x29455e0) adds a second-layer node. Query the native
/// destination predicate before requesting the terrain facts used here.
pub fn needs_partial_node(
    event: i32,
    remaining: MovementPoints,
    is_destination: bool,
    path_flags: u32,
    mountain: bool,
    canyon: bool,
) -> bool {
    (event as u32) <= 1
        && remaining.0 > 0
        && !is_destination
        && (path_flags & 4 == 0 || mountain || canyon)
}

pub fn partial_node_known_cost(known: MovementPoints, remaining: MovementPoints) -> MovementPoints {
    MovementPoints(known.0.saturating_add(remaining.0.wrapping_mul(1000)))
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NodeCache {
    pub flags: u32,
    pub friendly_units_of_type: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FeatureMovement {
    pub allow_passthrough: bool,
    pub allow_destination: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeCacheFacts {
    pub visible: bool,
    pub friendly_units_of_type: u32,
    pub at_formation_location: bool,
    pub mountain: bool,
    pub impassable: bool,
    pub water: bool,
    pub placement_result: i32,
    pub revealed: bool,
    pub plot_unit_limit: u32,
    /// A foreign city / defensible district contributes its respective bit.
    pub foreign_city_at_war: Option<bool>,
    pub foreign_defensible_district_at_war: Option<bool>,
    pub visible_enemy_unit: bool,
    pub visible_enemy_unit_out: bool,
    /// Only the first Feature_UnitMovement entry is queried natively.
    pub first_feature_movement: Option<FeatureMovement>,
    pub feature_danger: i32,
    pub location: PlotCoord,
    pub destination: PlotCoord,
    pub is_human: bool,
    pub is_automated: bool,
}

impl Default for NodeCacheFacts {
    fn default() -> Self {
        Self {
            visible: false,
            friendly_units_of_type: 0,
            at_formation_location: false,
            mountain: false,
            impassable: false,
            water: false,
            placement_result: 0,
            revealed: false,
            plot_unit_limit: 0,
            foreign_city_at_war: None,
            foreign_defensible_district_at_war: None,
            visible_enemy_unit: false,
            visible_enemy_unit_out: false,
            first_feature_movement: None,
            feature_danger: 0,
            location: PlotCoord { x: 0, y: 0 },
            destination: PlotCoord { x: 0, y: 0 },
            is_human: false,
            is_automated: false,
        }
    }
}

/// PathNodeInitializeCache (0x2949230). Existing initialized caches are retained.
/// All facts must be resolved in native query order by the world adapter.
pub fn initialize_node_cache(cache: &mut NodeCache, parent: Option<NodeCache>, f: NodeCacheFacts) {
    if cache.flags & 1 != 0 {
        return;
    }
    fn bit(flags: &mut u32, mask: u32, value: bool) {
        *flags = (*flags & !mask) | if value { mask } else { 0 };
    }
    let flags = &mut cache.flags;
    *flags |= 1;
    bit(flags, 2, f.visible);
    cache.friendly_units_of_type = f
        .friendly_units_of_type
        .wrapping_sub(u32::from(f.at_formation_location)) as u16;
    bit(flags, 4, f.mountain);
    bit(flags, 8, f.impassable && !f.mountain);
    bit(flags, 0x10, f.water);
    bit(flags, 0x40, f.placement_result == 1);
    bit(flags, 0x80, f.placement_result == 1);
    bit(flags, 0x100, f.revealed);
    *flags &= 0xfffe61ff;
    bit(
        flags,
        0x8000,
        u32::from(cache.friendly_units_of_type) >= f.plot_unit_limit,
    );
    if f.visible {
        if let Some(war) = f.foreign_city_at_war {
            *flags |= if war { 0x400 } else { 0x200 };
        }
        if let Some(war) = f.foreign_defensible_district_at_war {
            *flags |= if war { 0x1000 } else { 0x800 };
        }
        bit(flags, 0x2000, f.visible_enemy_unit);
        bit(flags, 0x4000, f.visible_enemy_unit_out);
        let blocked = if let Some(movement) = f.first_feature_movement {
            !movement.allow_passthrough
                && (!movement.allow_destination || f.location != f.destination)
        } else {
            // Native tests X and Y independently here; sharing either axis skips danger.
            f.feature_danger > 0
                && f.location.x != f.destination.x
                && f.location.y != f.destination.y
                && (!f.is_human || f.is_automated)
        };
        if blocked {
            *flags |= 0x10000;
        }
    } else {
        *flags &= 0xfffe81ff;
    }
    let remembered_water = if f.revealed || parent.is_some_and(|p| p.flags & 0x100 != 0) {
        f.water
    } else {
        parent.is_some_and(|p| p.flags & 0x20 != 0)
    };
    bit(flags, 0x20, remembered_water);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidityNode {
    pub location: PlotCoord,
    pub turns: i32,
    pub cache: NodeCache,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ValidityResult {
    pub valid: bool,
    pub abort: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PathValidFacts {
    pub path_flags: u32,
    pub formation_location: PlotCoord,
    pub destination: PlotCoord,
    pub calculated_turns: i32,
    pub is_human: bool,
    pub is_automated: bool,
    pub can_attack: bool,
    pub domain: i32,
    pub plot_unit_limit: u32,
    pub includes_land_unit: bool,
    pub includes_naval_unit: bool,
    pub can_embark_to: bool,
    pub can_disembark_to: bool,
    pub adjacent_to_formation_location: bool,
    pub parent_plot_owner: i32,
    pub formation_owner: i32,
    pub parent_owner_at_war: bool,
    pub barbarian_tribe_index: i32,
    pub barbarian_clans_active: bool,
    pub bribe_turns_remaining: i32,
    pub influence: i32,
    pub parent_owner_enforces_borders: bool,
    pub open_borders_from_parent_owner: bool,
}

impl Default for PathValidFacts {
    fn default() -> Self {
        Self {
            path_flags: 0,
            formation_location: PlotCoord { x: 0, y: 0 },
            destination: PlotCoord { x: 0, y: 0 },
            calculated_turns: 0,
            is_human: false,
            is_automated: false,
            can_attack: false,
            domain: 0,
            plot_unit_limit: 0,
            includes_land_unit: false,
            includes_naval_unit: false,
            can_embark_to: false,
            can_disembark_to: false,
            adjacent_to_formation_location: false,
            parent_plot_owner: 0,
            formation_owner: 0,
            parent_owner_at_war: false,
            barbarian_tribe_index: 0,
            barbarian_clans_active: false,
            bribe_turns_remaining: 0,
            influence: 0,
            parent_owner_enforces_borders: false,
            open_borders_from_parent_owner: false,
        }
    }
}

/// PathValid (0x2946a80) after cache initialization and movement calculation.
/// `ancestors` starts with the proposed parent, followed by its parent chain.
/// The child uses calculated turns, while ancestors use stored turns. This is
/// significant while a better parent is considered for an already-linked node.
pub fn path_valid(
    child: ValidityNode,
    ancestors: &[ValidityNode],
    f: PathValidFacts,
) -> ValidityResult {
    let rejected = ValidityResult::default();
    let accepted = ValidityResult {
        valid: true,
        abort: false,
    };
    let Some(parent) = ancestors.first() else {
        return accepted;
    };
    if child.location == f.formation_location || child.cache.flags & 0x10000 != 0 {
        return rejected;
    }
    let automatic = !f.is_human || f.is_automated;
    let mut previous_turn = -1;
    let mut previous_obstruction = false;
    for (index, node) in std::iter::once(&child).chain(ancestors.iter()).enumerate() {
        let turn = if index == 0 {
            f.calculated_turns
        } else {
            node.turns
        };
        let destination = node.location == f.destination;
        let flags = node.cache.flags;
        if (previous_turn == -1 && destination)
            || (previous_turn != -1 && (turn != previous_turn || previous_obstruction))
        {
            if flags & 2 != 0 {
                if (flags & 0x400 == 0 || !f.can_attack)
                    && u32::from(node.cache.friendly_units_of_type) >= f.plot_unit_limit
                    && f.path_flags & 4 == 0
                    && (f.path_flags & 0x1000 == 0 || turn < 2)
                {
                    return ValidityResult {
                        valid: false,
                        abort: destination,
                    };
                }
                if flags & 0x84 == 4 || flags & 4 != 0 && f.path_flags & 4 == 0 && automatic {
                    return rejected;
                }
                if flags & 0x88 == 8 || flags & 8 != 0 && f.path_flags & 4 == 0 && automatic {
                    return rejected;
                }
                if f.path_flags & 0x80 != 0 && flags & 0x10 != 0 {
                    return rejected;
                }
            }
            if f.path_flags & 4 == 0 && flags & 0x200 != 0 {
                return rejected;
            }
        }
        previous_turn = turn;
        previous_obstruction = flags & 0x6000 == 0x6000 || flags & 0x400 != 0;
    }
    let flags = child.cache.flags;
    if f.includes_land_unit {
        if flags & 0x100 != 0 {
            if flags & 0x10 != 0 && parent.cache.flags & 0x10 == 0 {
                if !f.can_embark_to {
                    return rejected;
                }
            } else if parent.cache.flags & 0x10 != 0 && flags & 0x10 == 0 {
                if !f.can_disembark_to {
                    return rejected;
                }
            } else if flags & 0x40 == 0 {
                return rejected;
            }
        }
    } else if f.includes_naval_unit && flags & 0x140 == 0x100 {
        return rejected;
    }
    let attack_allowed = f.can_attack
        && (if f.path_flags & 0x10 != 0 {
            f.destination.x == -9999
                || f.destination.y == -9999
                || child.location.x == f.destination.x
                    && (f.domain == 1 || child.location.y == f.destination.y)
        } else {
            f.path_flags & 0x1000 != 0 && f.calculated_turns > 1
        });
    if !attack_allowed && f.domain != 1 && flags & 0x3400 != 0 {
        return rejected;
    }
    if f.domain != 1
        && f.can_attack
        && f.path_flags & 4 == 0
        && flags & 0x2000 != 0
        && f.adjacent_to_formation_location
        && flags & 0x4000 != 0
        && parent.cache.flags & 0x8000 != 0
    {
        return rejected;
    }
    if f.path_flags & 2 != 0 && f.parent_plot_owner != -1 && f.parent_owner_at_war {
        return rejected;
    }
    if f.parent_plot_owner != -1
        && f.barbarian_tribe_index >= 0
        && f.barbarian_clans_active
        && f.bribe_turns_remaining > 0
    {
        return rejected;
    }
    if (f.path_flags & 8 != 0 || f.is_automated) && parent.turns <= 1 && f.influence > 0 {
        return rejected;
    }
    if f.path_flags & 0x8000 != 0 && parent.turns <= 1 && flags & 0x10 != 0 && f.influence > 0 {
        return rejected;
    }
    if f.path_flags & 1 != 0 && parent.location != f.formation_location {
        if parent.cache.flags & 0x100 == 0 {
            return rejected;
        }
        if f.parent_plot_owner != f.formation_owner
            && f.parent_plot_owner != -1
            && f.parent_owner_enforces_borders
            && !f.parent_owner_at_war
            && !f.open_borders_from_parent_owner
        {
            return rejected;
        }
    }
    accepted
}

/// Per-search memoization in UnitPathCacheData. Native checks parent and child
/// identities only; edits to their scalar fields do not invalidate this entry.
#[derive(Clone, Debug, Default)]
pub struct MovementMemo {
    last_pair: Option<(Option<crate::pathfinder::NodeId>, crate::pathfinder::NodeId)>,
    pub result: MovementResult,
}

impl MovementMemo {
    pub fn calculate(
        &mut self,
        parent: Option<crate::pathfinder::NodeId>,
        child: crate::pathfinder::NodeId,
        parent_movement: Option<ParentMovement>,
        step: MovementStep,
    ) -> MovementResult {
        if self.last_pair != Some((parent, child)) {
            self.result = calculate_movement_points(parent_movement, step);
            self.last_pair = Some((parent, child));
        }
        self.result
    }

    /// PathAdd skips event 5; all other events copy the calculated scalar fields.
    pub fn add(&self, child: &mut crate::pathfinder::WorkingNode, event: i32) {
        if event == 5 {
            return;
        }
        child.moves_remaining_raw = self.result.remaining.0;
        child.turn = self.result.turns;
        child.exit_cost_raw = self.result.exit_cost.0;
    }
}

/// Copy PathNodeAdd's fields into the second layer. The original total score and
/// parent are retained even though the known score increases. Child lists and
/// the upper half of the native cache's second word are not copied.
pub fn prepare_partial_node(
    search: &mut crate::pathfinder::TwoLayer,
    child: crate::pathfinder::NodeId,
) -> Option<crate::pathfinder::NodeId> {
    let source = search.node(child);
    let partial = search.partial_node(i32::from(source.x), i32::from(source.y))?;
    let parent = source.parent;
    let total = source.total_cost_raw;
    let known = partial_node_known_cost(
        MovementPoints(source.known_cost_raw),
        MovementPoints(source.moves_remaining_raw),
    )
    .0;
    let exit = source.exit_cost_raw;
    let heuristic = source.heuristic;
    let turn = source.turn;
    let flags = source.cache_flags;
    let other = source.cache_other;
    let target = search.node_mut(partial);
    target.parent = parent;
    target.total_cost_raw = total;
    target.known_cost_raw = known;
    target.exit_cost_raw = exit;
    target.heuristic = heuristic;
    target.moves_remaining_raw = 0;
    target.turn = turn;
    target.status = 0;
    target.cache_flags = flags;
    target.cache_other = (target.cache_other & 0xffff0000) | (other & 0xffff);
    Some(partial)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn at(x: i32, y: i32) -> PlotCoord {
        PlotCoord { x, y }
    }
    fn step() -> MovementStep {
        MovementStep {
            formation_location: at(0, 0),
            path_flags: 0,
            child_cache_flags: 0x100,
            child_is_water: false,
            starting_remaining: MovementPoints(512),
            selected_base_moves: 2,
            parent_domain_base_moves: 3,
            slowed_by_zoc: false,
            movement_cost: MovementPoints(768),
            to_plot_exit_cost: MovementPoints(256),
        }
    }
    #[test]
    fn overspend_start_flag_retains_turn_and_consumes_available() {
        let parent = ParentMovement {
            location: at(0, 0),
            remaining: MovementPoints(512),
            exit_cost: MovementPoints(0),
            turns: 1,
            cache_flags: 0x100,
        };
        let regular = calculate_movement_points(Some(parent), step());
        assert_eq!(
            regular,
            MovementResult {
                cost: MovementPoints(768),
                remaining: MovementPoints(0),
                exit_cost: MovementPoints(0),
                turns: 2
            }
        );
        let flagged = calculate_movement_points(
            Some(parent),
            MovementStep {
                path_flags: 0x100,
                ..step()
            },
        );
        assert_eq!(flagged.cost, MovementPoints(512));
        assert_eq!(flagged.turns, 1);
    }
    #[test]
    fn cache_feature_danger_retains_native_same_axis_exception() {
        let facts = NodeCacheFacts {
            visible: true,
            revealed: true,
            water: true,
            location: at(3, 4),
            destination: at(3, 8),
            feature_danger: 10,
            ..NodeCacheFacts::default()
        };
        let mut cache = NodeCache::default();
        initialize_node_cache(&mut cache, None, facts);
        assert_eq!(cache.flags & 0x10000, 0);
        assert_ne!(cache.flags & 0x20, 0);
        initialize_node_cache(
            &mut cache,
            None,
            NodeCacheFacts {
                destination: at(7, 8),
                ..facts
            },
        );
        assert_eq!(cache.flags & 0x10000, 0); // Initialized nodes are not recomputed.
        cache.flags &= !1;
        initialize_node_cache(
            &mut cache,
            None,
            NodeCacheFacts {
                destination: at(7, 8),
                ..facts
            },
        );
        assert_ne!(cache.flags & 0x10000, 0);
    }
    #[test]
    fn blocked_destination_aborts_but_blocked_ancestor_only_rejects() {
        let child = ValidityNode {
            location: at(2, 0),
            turns: 0,
            cache: NodeCache {
                flags: 3,
                friendly_units_of_type: 1,
            },
        };
        let parent = ValidityNode {
            location: at(1, 0),
            turns: 1,
            cache: NodeCache {
                flags: 3,
                friendly_units_of_type: 1,
            },
        };
        let facts = PathValidFacts {
            destination: at(2, 0),
            formation_location: at(0, 0),
            calculated_turns: 2,
            plot_unit_limit: 1,
            ..PathValidFacts::default()
        };
        assert_eq!(
            path_valid(child, &[parent], facts),
            ValidityResult {
                valid: false,
                abort: true
            }
        );
        let child = ValidityNode {
            cache: NodeCache {
                flags: 1,
                friendly_units_of_type: 0,
            },
            ..child
        };
        assert_eq!(
            path_valid(child, &[parent], facts),
            ValidityResult {
                valid: false,
                abort: false
            }
        );
    }
    #[test]
    fn cost_distinguishes_consumed_budget_and_hidden_score() {
        let movement = MovementResult {
            cost: MovementPoints(256),
            remaining: MovementPoints(0),
            ..MovementResult::default()
        };
        assert_eq!(
            path_cost(movement, PathCostFacts::default()),
            MovementPoints(256_000 + 7 * 256)
        );
        assert_eq!(
            partial_node_known_cost(MovementPoints(i32::MAX - 10), MovementPoints(1)),
            MovementPoints(i32::MAX)
        );
    }

    #[test]
    fn zoc_renewal_uses_parent_domain_budget() {
        let parent = ParentMovement {
            location: at(1, 0),
            remaining: MovementPoints(0),
            exit_cost: MovementPoints(0),
            turns: 1,
            cache_flags: 0x100,
        };
        let renewed = calculate_movement_points(
            Some(parent),
            MovementStep {
                slowed_by_zoc: true,
                ..step()
            },
        );
        assert_eq!(renewed.remaining, MovementPoints(0));
        assert_eq!(renewed.turns, 2);
        assert_eq!(renewed.exit_cost, MovementPoints(256));
        let ordinary = calculate_movement_points(Some(parent), step());
        assert_eq!(ordinary.remaining, MovementPoints(0));
        assert_eq!(ordinary.exit_cost, MovementPoints(0));
    }

    #[test]
    fn memo_uses_node_identity_and_partial_retains_total_and_parent() {
        use crate::pathfinder::{NodeId, TwoLayer};
        let mut memo = MovementMemo::default();
        let first = memo.calculate(None, NodeId(1), None, step());
        let changed = MovementStep {
            starting_remaining: MovementPoints(7),
            ..step()
        };
        assert_eq!(memo.calculate(None, NodeId(1), None, changed), first);
        assert_eq!(
            memo.calculate(None, NodeId(2), None, changed).remaining,
            MovementPoints(7)
        );
        let mut search = TwoLayer::new(2, 2, false, false).unwrap();
        let child = search.normal_node(1, 0).unwrap();
        let parent = search.normal_node(0, 0).unwrap();
        let node = search.node_mut(child);
        node.parent = Some(parent);
        node.total_cost_raw = 600;
        node.known_cost_raw = 300;
        node.moves_remaining_raw = 256;
        node.cache_other = 0xabcd1234;
        let partial = prepare_partial_node(&mut search, child).unwrap();
        let node = search.node(partial);
        assert_eq!(node.parent, Some(parent));
        assert_eq!(node.total_cost_raw, 600);
        assert_eq!(node.known_cost_raw, 256300);
        assert_eq!(node.moves_remaining_raw, 0);
        assert_eq!(node.cache_other, 0x1234);
    }
}
