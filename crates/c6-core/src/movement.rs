//! Linux XP2 movement-cost arithmetic over explicitly resolved native query facts.
//!
//! This does not resolve world/rules/modifier queries, test movement legality, move
//! units, or advance turns. See `docs/linux-movement-cost.md`. Static target-byte
//! contracts are checked; native differential acceptance remains outstanding.

/// Signed native FixedPointT<8> storage. Integer conversion and sums wrap at 32 bits.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct MovementPoints(pub i32);

impl MovementPoints {
    pub const fn integer(value: i32) -> Self {
        Self(value.wrapping_shl(8))
    }

    /// Native floorf, integer shift, fractional truncation and low-byte assembly.
    /// Assumes x86_64 binary32, round-to-nearest, masked exceptions, DAZ/FTZ off.
    /// Models returned bits, not MXCSR flags or floating exception side effects.
    pub fn route_float_bits(bits: u32) -> Self {
        let value = f32::from_bits(bits);
        let floor = value.floor();
        let whole = cvttss2si(floor).wrapping_shl(8);
        let fraction = cvttss2si((value - floor) * 256.0) as u8;
        Self(whole | i32::from(fraction))
    }

    fn add(self, other: Self) -> Self {
        Self(self.0.wrapping_add(other.0))
    }

    fn subtract(self, other: Self) -> Self {
        Self(self.0.wrapping_sub(other.0))
    }
}

// x86 CVTTSS2SI returns the integer-indefinite bit pattern on invalid conversion;
// Rust float-to-int casts saturate instead. The in-range cast truncates correctly.
fn cvttss2si(value: f32) -> i32 {
    if !value.is_finite() || !(-2147483648.0..2147483648.0).contains(&value) {
        i32::MIN
    } else {
        value as i32
    }
}

/// The four mutable scalar fields of native MovementCost::Values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CostAccumulator {
    pub hard_minimum: MovementPoints,
    pub soft_maximum: MovementPoints,
    pub regular: MovementPoints,
    pub to_plot_exit: MovementPoints,
}

impl Default for CostAccumulator {
    fn default() -> Self {
        Self {
            hard_minimum: MovementPoints(0),
            soft_maximum: MovementPoints::integer(0x7f_ffff),
            regular: MovementPoints(0),
            to_plot_exit: MovementPoints(0),
        }
    }
}

impl CostAccumulator {
    pub fn change_regular(&mut self, value: MovementPoints) {
        self.regular = self.regular.add(value);
    }

    pub fn update_hard_minimum(&mut self, value: MovementPoints) {
        self.hard_minimum = self.hard_minimum.max(value);
    }

    pub fn update_soft_maximum(&mut self, value: MovementPoints) {
        self.soft_maximum = self.soft_maximum.min(value);
    }

    pub fn change_to_plot_exit(&mut self, value: MovementPoints) {
        self.to_plot_exit = self.to_plot_exit.add(value);
    }

    pub fn final_cost(&self) -> MovementPoints {
        let cost = self.regular.min(self.soft_maximum).max(self.hard_minimum);
        if cost.0 > 0 {
            cost
        } else {
            MovementPoints::integer(1)
        }
    }
}

/// Native ZOCValue discriminants, named here by their observed branch behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZocValue {
    Query,
    NotSlowed,
    Slowed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MoveBudget {
    /// The original constructor accepts an integer, then converts to Q8.
    pub base_moves: i32,
    pub max_moves: MovementPoints,
    pub remaining: MovementPoints,
    pub exit_moves: MovementPoints,
}

/// Resolved per-member predicate from the actual Unit getter, plus escort mobility.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemberIgnore {
    pub ignores: bool,
    pub escort_mobility: bool,
}

/// Native formation aggregation: missing IDs are skipped; an empty set returns true.
/// An ignoring member with escort mobility returns true immediately, otherwise all
/// resolved members must ignore. Used independently for terrain, feature and shores/rivers.
pub fn formation_ignores(members: impl IntoIterator<Item = Option<MemberIgnore>>) -> bool {
    let mut all_ignore = true;
    for member in members.into_iter().flatten() {
        if member.ignores {
            if member.escort_mobility {
                return true;
            }
        } else {
            all_ignore = false;
        }
    }
    all_ignore
}

/// Formation query outputs, each supplied by its distinct native-equivalent resolver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormationFacts {
    pub ignore_terrain: bool,
    pub ignore_feature: bool,
    pub ignore_rivers: bool,
    pub ignore_shores: bool,
    pub can_embark: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RouteFacts {
    pub pillaged: bool,
    pub bridged: bool,
    /// None means route definition lookup returned null, distinct from no route.
    pub movement_cost_float_bits: Option<u32>,
}

/// Query facts for one non-null native plot. Definition absence means lookup null.
/// All integer costs come from the active original rules/configuration, not defaults.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlotFacts {
    pub is_city: bool,
    pub is_water: bool,
    pub has_city_attributes: bool,
    pub city_domain_valid: bool,
    pub terrain_cost: Option<i32>,
    pub feature_change: Option<i32>,
    pub improvement_change: Option<i32>,
    pub route: Option<RouteFacts>,
    /// None means NO_DISTRICT. Some(false) means an actual non-free district definition.
    pub district_free_embark: Option<bool>,
    pub district_complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuleParameters {
    pub city_movement_cost: i32,
    pub movement_river_cost: i32,
    pub movement_portal_cost: i32,
    pub movement_embark_cost: i32,
}

/// Spatial query results. `river_crossing` is Rules::Movement::IsRiverCrossing,
/// including its water/city/map checks, not just a raw saved river-edge bit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EdgeFacts {
    pub river_crossing: bool,
    pub adjacent: bool,
    pub source_portal_has_exit_at_destination: bool,
}

/// Apply the nine native rule phases to an existing accumulator. This deliberately
/// excludes the ZOC prelude; MovementCostNoZOC calls the phases directly.
pub fn apply_rules(
    values: &mut CostAccumulator,
    budget: MoveBudget,
    formation: FormationFacts,
    from: PlotFacts,
    to: PlotFacts,
    edge: EdgeFacts,
    parameters: RuleParameters,
) {
    if to.is_city {
        values.update_soft_maximum(MovementPoints::integer(parameters.city_movement_cost));
    }
    if let Some(cost) = to.terrain_cost {
        values.change_regular(MovementPoints::integer(if formation.ignore_terrain {
            1
        } else {
            cost
        }));
    }
    if !formation.ignore_feature {
        if let Some(change) = to.feature_change {
            values.change_regular(MovementPoints::integer(change));
        }
    }
    if let Some(change) = to.improvement_change {
        values.change_regular(MovementPoints::integer(change));
    }
    if let (Some(from_route), Some(to_route)) = (from.route, to.route) {
        if !from_route.pillaged
            && !to_route.pillaged
            && (!edge.river_crossing || (from_route.bridged && to_route.bridged))
        {
            if let (Some(from_bits), Some(to_bits)) = (
                from_route.movement_cost_float_bits,
                to_route.movement_cost_float_bits,
            ) {
                // UCOMISS/CMOVA selects destination only if ordered and greater.
                let bits = if f32::from_bits(to_bits) > f32::from_bits(from_bits) {
                    to_bits
                } else {
                    from_bits
                };
                values.update_soft_maximum(MovementPoints::route_float_bits(bits));
            }
        }
    }
    if edge.river_crossing && !formation.ignore_rivers {
        values.change_regular(MovementPoints::integer(parameters.movement_river_cost));
    }
    if !edge.adjacent && edge.source_portal_has_exit_at_destination {
        values.change_regular(MovementPoints::integer(parameters.movement_portal_cost));
    }
    // Both shore phases use the source water/domain predicate. Embark uses the
    // destination equivalent, and tests source raw IsWater separately.
    let from_requires_embark =
        from.is_water && !(from.has_city_attributes && from.city_domain_valid);
    let to_requires_embark = to.is_water && !(to.has_city_attributes && to.city_domain_valid);
    let embarking = formation.can_embark && to_requires_embark && !from.is_water;
    let disembarking = formation.can_embark && from_requires_embark && !to.is_water;
    if embarking || disembarking {
        // Preserve the supplied target's source-completion test and assignment,
        // including replacement by the destination flag rather than logical OR.
        let mut free = from.district_complete && from.district_free_embark == Some(true);
        if from.district_complete {
            if let Some(destination_free) = to.district_free_embark {
                free = destination_free;
            }
        }
        if !free && !formation.ignore_shores {
            values.change_regular(MovementPoints::integer(parameters.movement_embark_cost));
        }
        let base = MovementPoints::integer(budget.base_moves);
        if base < budget.remaining {
            values.change_to_plot_exit(budget.remaining.subtract(base));
        }
    }
}

/// ZOC hard-minimum selection before the rule phases. Bit 11 bypasses the ZOC test.
/// Query mode requires the actual IsSlowedByZOC output, not an estimated enemy count.
pub fn apply_zoc_minimum(
    values: &mut CostAccumulator,
    budget: MoveBudget,
    zoc: ZocValue,
    flags: u32,
    is_slowed_by_zoc: bool,
) {
    let slowed = flags & (1 << 11) == 0
        && (zoc == ZocValue::Slowed || (zoc == ZocValue::Query && is_slowed_by_zoc));
    let minimum = if slowed {
        if budget.remaining.0 > 0 {
            budget.remaining
        } else {
            budget.max_moves
        }
    } else {
        budget.exit_moves
    };
    values.update_hard_minimum(minimum);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn land() -> PlotFacts {
        PlotFacts {
            is_city: false,
            is_water: false,
            has_city_attributes: false,
            city_domain_valid: false,
            terrain_cost: Some(1),
            feature_change: None,
            improvement_change: None,
            route: None,
            district_free_embark: None,
            district_complete: false,
        }
    }

    const BUDGET: MoveBudget = MoveBudget {
        base_moves: 2,
        max_moves: MovementPoints(1024),
        remaining: MovementPoints(768),
        exit_moves: MovementPoints(0),
    };
    const FORMATION: FormationFacts = FormationFacts {
        ignore_terrain: false,
        ignore_feature: false,
        ignore_rivers: false,
        ignore_shores: false,
        can_embark: true,
    };
    const EDGE: EdgeFacts = EdgeFacts {
        river_crossing: false,
        adjacent: true,
        source_portal_has_exit_at_destination: false,
    };
    const PARAMETERS: RuleParameters = RuleParameters {
        city_movement_cost: 1,
        movement_river_cost: 3,
        movement_portal_cost: 4,
        movement_embark_cost: 2,
    };

    #[test]
    fn bridged_route_caps_river_and_feature_but_zoc_wins() {
        let mut from = land();
        let mut to = land();
        from.route = Some(RouteFacts {
            pillaged: false,
            bridged: true,
            movement_cost_float_bits: Some(0.5_f32.to_bits()),
        });
        to.route = Some(RouteFacts {
            pillaged: false,
            bridged: true,
            movement_cost_float_bits: Some(0.75_f32.to_bits()),
        });
        to.feature_change = Some(1);
        let mut v = CostAccumulator::default();
        apply_rules(
            &mut v,
            BUDGET,
            FORMATION,
            from,
            to,
            EdgeFacts {
                river_crossing: true,
                ..EDGE
            },
            PARAMETERS,
        );
        assert_eq!(v.regular, MovementPoints::integer(5));
        assert_eq!(v.final_cost(), MovementPoints(192));
        apply_zoc_minimum(&mut v, BUDGET, ZocValue::Slowed, 0, false);
        assert_eq!(v.final_cost(), BUDGET.remaining);
    }

    #[test]
    fn shore_uses_source_completion_and_destination_assignment() {
        let mut from = land();
        let mut to = land();
        to.is_water = true;
        from.district_free_embark = Some(true);
        from.district_complete = true;
        to.district_free_embark = Some(false);
        let mut v = CostAccumulator::default();
        apply_rules(&mut v, BUDGET, FORMATION, from, to, EDGE, PARAMETERS);
        assert_eq!(v.regular, MovementPoints::integer(3));
        assert_eq!(v.to_plot_exit, MovementPoints(256));
        from.district_complete = false;
        to.district_free_embark = Some(true);
        to.district_complete = true;
        let mut v = CostAccumulator::default();
        apply_rules(&mut v, BUDGET, FORMATION, from, to, EDGE, PARAMETERS);
        assert_eq!(v.regular, MovementPoints::integer(3));
        from.district_complete = true;
        let mut v = CostAccumulator::default();
        apply_rules(&mut v, BUDGET, FORMATION, from, to, EDGE, PARAMETERS);
        assert_eq!(v.regular, MovementPoints::integer(1));
    }

    #[test]
    fn zoc_bypass_uses_exit_and_no_remaining_uses_maximum() {
        let budget = MoveBudget {
            remaining: MovementPoints(0),
            exit_moves: MovementPoints(64),
            ..BUDGET
        };
        let mut v = CostAccumulator::default();
        apply_zoc_minimum(&mut v, budget, ZocValue::Slowed, 0, false);
        assert_eq!(v.hard_minimum, budget.max_moves);
        let mut v = CostAccumulator::default();
        apply_zoc_minimum(&mut v, budget, ZocValue::Slowed, 1 << 11, false);
        assert_eq!(v.hard_minimum, budget.exit_moves);
    }

    #[test]
    fn hard_minimum_wins_soft_cap_and_nonpositive_falls_back_to_one() {
        let mut v = CostAccumulator::default();
        assert_eq!(v.final_cost(), MovementPoints(256));
        v.change_regular(MovementPoints(768));
        v.update_soft_maximum(MovementPoints(128));
        v.update_hard_minimum(MovementPoints(512));
        assert_eq!(v.final_cost(), MovementPoints(512));
        v.change_regular(MovementPoints(i32::MAX));
        assert!(v.regular.0 < 0);
        assert_eq!(v.final_cost(), MovementPoints(512));
    }

    #[test]
    fn route_conversion_preserves_negative_fraction_and_integer_indefinite() {
        for (value, expected) in [(0.75_f32, 192), (-0.1, -26), (1.999, 511)] {
            assert_eq!(
                MovementPoints::route_float_bits(value.to_bits()),
                MovementPoints(expected)
            );
        }
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 2147483648.0] {
            assert_eq!(
                MovementPoints::route_float_bits(value.to_bits()),
                MovementPoints(0)
            );
        }
        // Native two-stage rounding is not mathematical floor(value * 256).
        assert_eq!(
            MovementPoints::route_float_bits(0x8000_0001),
            MovementPoints(-256)
        );
    }

    #[test]
    fn escort_can_override_an_earlier_nonignoring_member() {
        let no = Some(MemberIgnore {
            ignores: false,
            escort_mobility: false,
        });
        let escort = Some(MemberIgnore {
            ignores: true,
            escort_mobility: true,
        });
        assert!(formation_ignores([None, no, escort]));
        assert!(!formation_ignores([None, no]));
        assert!(formation_ignores([]));
    }
}
