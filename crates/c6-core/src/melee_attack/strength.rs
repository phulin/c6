//! Linux unit-versus-unit strength arithmetic and ordered ordinary attack terms.
//! Query errors identify unsupported branches; this does not resolve combat.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Side {
    Attack,
    Defense,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArithmeticError {
    DivisionByZero,
    DivisionOverflow,
}
#[derive(Debug, Eq, PartialEq)]
pub enum Error<E> {
    Query(E),
    Arithmetic(ArithmeticError),
}
/// Every getter is lazy: unsupported later inputs cannot mask an earlier native
/// return, and defense deliberately queries maximum HP twice when positive.
pub trait Context {
    type Error;
    fn no_damage_reduction(&mut self) -> Result<bool, Self::Error>;
    fn damage(&mut self) -> Result<i32, Self::Error>;
    fn max_hit_points(&mut self) -> Result<i32, Self::Error>;
    fn damage_reduction_modifier(&mut self) -> Result<i32, Self::Error>;
    fn wounded_multiplier(&mut self) -> Result<i32, Self::Error>;
    fn owner_is_major(&mut self) -> Result<bool, Self::Error>;
    fn fuel_reduction(&mut self) -> Result<i32, Self::Error>;
}
fn cvtt(value: f32) -> i32 {
    if !(-2147483648.0..2147483648.0).contains(&value) {
        i32::MIN
    } else {
        value as i32
    }
}
fn fixed_float(value: f32) -> i32 {
    let whole = value.floor();
    cvtt(whole).wrapping_shl(8) | (cvtt((value - whole) * 256.0) & 255)
}
fn multiply(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 8) as i32
}
/// 2bda490: two float-to-Q8 conversions and two signed wide products, retaining
/// the intermediate quantization. This is not integer percentage algebra.
pub fn reduction_for_damage(damage_percent: i32, modifier: i32, multiplier: i32) -> i32 {
    let base = multiplier.wrapping_shl(8);
    let adjustment = fixed_float(modifier as f32 * f32::from_bits(0x3c23d70a));
    let adjusted = base.wrapping_sub(multiply(base, adjustment));
    let fraction = fixed_float(damage_percent as f32 * f32::from_bits(0x3c23d70a));
    multiply(fraction, adjusted)
}
/// Opening signed Q8 contribution only. Native dead-object/formation-class
/// gates belong to the enclosing routine; this value is NOT a total modifier.
pub fn opening_terms<C: Context>(c: &mut C, side: Side) -> Result<i32, Error<C::Error>> {
    let mut total: i32 = 0;
    if !c.no_damage_reduction().map_err(Error::Query)? {
        let apply = side == Side::Attack || c.max_hit_points().map_err(Error::Query)? > 0;
        if apply {
            let damage = c.damage().map_err(Error::Query)?.wrapping_mul(100);
            let max = c.max_hit_points().map_err(Error::Query)?;
            let percent = damage
                .checked_div(max)
                .ok_or(Error::Arithmetic(if max == 0 {
                    ArithmeticError::DivisionByZero
                } else {
                    ArithmeticError::DivisionOverflow
                }))?;
            let modifier = c.damage_reduction_modifier().map_err(Error::Query)?;
            let multiplier = c.wounded_multiplier().map_err(Error::Query)?;
            total = total.wrapping_sub(reduction_for_damage(percent, modifier, multiplier));
        }
    }
    if c.owner_is_major().map_err(Error::Query)? {
        let fuel = c.fuel_reduction().map_err(Error::Query)?;
        if fuel > 0 {
            total = total.wrapping_sub(fuel.wrapping_shl(8));
        }
    }
    Ok(total)
}
/// 2bdda90: count times the current global flanking value, followed by a
/// separately quantized percentage adjustment. Nonpositive counts return before
/// either rule or Unit coefficient is read by the enclosing current-state query.
pub fn flanking_from_count(count: i32, global: i32, modifier: i32) -> i32 {
    if count <= 0 {
        return 0;
    }
    let base = count.wrapping_mul(global).wrapping_shl(8);
    let ratio = ((i64::from(modifier.wrapping_shl(8)) << 8) / (100 << 8)) as i32;
    base.wrapping_add(multiply(base, ratio))
}
/// 2bdc460: the coefficient's signed positivity gate precedes the movement
/// query. This retains fractional moves and the wrapping integer constructor.
pub fn unused_movement_bonus(moves_raw: i32, modifier: i32) -> i32 {
    if modifier <= 0 {
        0
    } else {
        multiply(moves_raw, modifier.wrapping_shl(8))
    }
}
/// 2aaf660: ordinary/religious records are disjoint; expiry equality is
/// inclusive. Unknown source kinds and nonpositive source timestamps do not add.
pub fn diplomatic_combat_bonus(
    modifiers: &[crate::player_diplomacy::CombatModifier],
    turns: &[i32; 10],
    current: i32,
    religious: bool,
) -> i32 {
    modifiers.iter().fold(0_i32, |sum, m| {
        if m.religious_only != religious {
            return sum;
        }
        let index = match m.source {
            1 => 2,
            2 => 0,
            3 => 1,
            4 => 3,
            5 => 4,
            6 => 6,
            8 => 5,
            _ => return sum,
        };
        let last = turns[index];
        if last > 0 && last.wrapping_add(m.turns_active) >= current {
            sum.wrapping_add(m.combat_bonus)
        } else {
            sum
        }
    })
}
/// Native ordinary Unit/Unit attack bonus order. Each stage is a concrete
/// Rules.Combat helper, not a precomputed total or implicit zero capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttackBonus {
    Amphibious,
    VersusOpponent,
    Territory,
    Flanking,
    Miscellaneous,
    HolyCities,
    Neighbors,
    MilitaryPolicies,
    Diplomatic,
    Espionage,
    City,
    AdjacentLevy,
    Allies,
    UnusedMovement,
    FriendlyCity,
    SpecificReligion,
}
pub const ORDINARY_ATTACK_BONUSES: [AttackBonus; 16] = [
    AttackBonus::Amphibious,
    AttackBonus::VersusOpponent,
    AttackBonus::Territory,
    AttackBonus::Flanking,
    AttackBonus::Miscellaneous,
    AttackBonus::HolyCities,
    AttackBonus::Neighbors,
    AttackBonus::MilitaryPolicies,
    AttackBonus::Diplomatic,
    AttackBonus::Espionage,
    AttackBonus::City,
    AttackBonus::AdjacentLevy,
    AttackBonus::Allies,
    AttackBonus::UnusedMovement,
    AttackBonus::FriendlyCity,
    AttackBonus::SpecificReligion,
];
pub trait AttackContext: Context {
    fn defender_exists(&mut self) -> Result<bool, Self::Error>;
    fn defender_delayed_death(&mut self) -> Result<bool, Self::Error>;
    fn attacker_exists(&mut self) -> Result<bool, Self::Error>;
    fn attacker_delayed_death(&mut self) -> Result<bool, Self::Error>;
    fn bonus(&mut self, bonus: AttackBonus) -> Result<i32, Self::Error>;
    /// Resolve the CURRENT registry by native proposal ID after all prior reads.
    /// None means verified absent proposal; an unavailable registry is an error.
    fn proposal_modifier(&mut self) -> Result<Option<i32>, Self::Error>;
}
#[derive(Debug, Eq, PartialEq)]
pub enum AttackError<E> {
    Opening(Error<E>),
    Bonus {
        stage: AttackBonus,
        completed_raw: i32,
        error: E,
    },
    Proposal {
        completed_raw: i32,
        error: E,
    },
}
/// Ordinary attack aggregation2bde530 for the native NULL-Results branch.
/// Live-Results preview callbacks are not implemented here. Unknown
/// bonus queries propagate at their native position, with the computed prefix;
/// callers cannot substitute that prefix for a successful modifier result.
pub fn ordinary_attack_modifier<C: AttackContext>(c: &mut C) -> Result<i32, AttackError<C::Error>> {
    let query = |e| AttackError::Opening(Error::Query(e));
    if !c.defender_exists().map_err(query)?
        || c.defender_delayed_death().map_err(query)?
        || !c.attacker_exists().map_err(query)?
        || c.attacker_delayed_death().map_err(query)?
    {
        return Ok(0);
    }
    let mut total = opening_terms(c, Side::Attack).map_err(AttackError::Opening)?;
    for stage in ORDINARY_ATTACK_BONUSES {
        let value = c.bonus(stage).map_err(|error| AttackError::Bonus {
            stage,
            completed_raw: total,
            error,
        })?;
        total = total.wrapping_add(value);
    }
    if let Some(modifier) = c
        .proposal_modifier()
        .map_err(|error| AttackError::Proposal {
            completed_raw: total,
            error,
        })?
    {
        total = total.wrapping_add(modifier.wrapping_shl(8));
    }
    Ok(total)
}
#[cfg(test)]
mod tests {
    use super::*;
    struct C {
        max: i32,
        damage: i32,
        no: bool,
        major: bool,
        fuel: i32,
        calls: Vec<&'static str>,
    }
    impl Context for C {
        type Error = &'static str;
        fn no_damage_reduction(&mut self) -> Result<bool, Self::Error> {
            self.calls.push("immune");
            Ok(self.no)
        }
        fn damage(&mut self) -> Result<i32, Self::Error> {
            self.calls.push("damage");
            Ok(self.damage)
        }
        fn max_hit_points(&mut self) -> Result<i32, Self::Error> {
            self.calls.push("max");
            Ok(self.max)
        }
        fn damage_reduction_modifier(&mut self) -> Result<i32, Self::Error> {
            self.calls.push("modifier");
            Ok(0)
        }
        fn wounded_multiplier(&mut self) -> Result<i32, Self::Error> {
            self.calls.push("multiplier");
            Ok(10)
        }
        fn owner_is_major(&mut self) -> Result<bool, Self::Error> {
            self.calls.push("major");
            Ok(self.major)
        }
        fn fuel_reduction(&mut self) -> Result<i32, Self::Error> {
            self.calls.push("fuel");
            Ok(self.fuel)
        }
    }
    fn c() -> C {
        C {
            max: 100,
            damage: 50,
            no: false,
            major: true,
            fuel: 3,
            calls: vec![],
        }
    }
    #[test]
    fn side_order_and_nonpositive_maximum_are_distinct() {
        let mut a = c();
        assert_eq!(opening_terms(&mut a, Side::Attack), Ok(-8 * 256));
        assert_eq!(
            a.calls,
            [
                "immune",
                "damage",
                "max",
                "modifier",
                "multiplier",
                "major",
                "fuel"
            ]
        );
        let mut d = c();
        d.max = 0;
        assert_eq!(opening_terms(&mut d, Side::Defense), Ok(-3 * 256));
        assert_eq!(d.calls, ["immune", "max", "major", "fuel"]);
        let mut a = c();
        a.max = 0;
        assert_eq!(
            opening_terms(&mut a, Side::Attack),
            Err(Error::Arithmetic(ArithmeticError::DivisionByZero))
        );
        assert_eq!(a.calls, ["immune", "damage", "max"]);
    }
    #[test]
    fn immunity_and_nonmajor_skip_unread_dependencies() {
        let mut x = c();
        x.no = true;
        x.major = false;
        assert_eq!(opening_terms(&mut x, Side::Defense), Ok(0));
        assert_eq!(x.calls, ["immune", "major"]);
    }
    struct A {
        opening: C,
        stages: Vec<AttackBonus>,
        fail: Option<AttackBonus>,
        proposal_read: bool,
        dead: bool,
    }
    impl Context for A {
        type Error = &'static str;
        fn no_damage_reduction(&mut self) -> Result<bool, Self::Error> {
            self.opening.no_damage_reduction()
        }
        fn damage(&mut self) -> Result<i32, Self::Error> {
            self.opening.damage()
        }
        fn max_hit_points(&mut self) -> Result<i32, Self::Error> {
            self.opening.max_hit_points()
        }
        fn damage_reduction_modifier(&mut self) -> Result<i32, Self::Error> {
            self.opening.damage_reduction_modifier()
        }
        fn wounded_multiplier(&mut self) -> Result<i32, Self::Error> {
            self.opening.wounded_multiplier()
        }
        fn owner_is_major(&mut self) -> Result<bool, Self::Error> {
            self.opening.owner_is_major()
        }
        fn fuel_reduction(&mut self) -> Result<i32, Self::Error> {
            self.opening.fuel_reduction()
        }
    }
    impl AttackContext for A {
        fn defender_exists(&mut self) -> Result<bool, Self::Error> {
            Ok(true)
        }
        fn defender_delayed_death(&mut self) -> Result<bool, Self::Error> {
            Ok(self.dead)
        }
        fn attacker_exists(&mut self) -> Result<bool, Self::Error> {
            Ok(true)
        }
        fn attacker_delayed_death(&mut self) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn bonus(&mut self, b: AttackBonus) -> Result<i32, Self::Error> {
            self.stages.push(b);
            if self.fail == Some(b) {
                Err("missing current bonus")
            } else {
                Ok(match b {
                    AttackBonus::Amphibious => 128,
                    AttackBonus::SpecificReligion => 64,
                    _ => 0,
                })
            }
        }
        fn proposal_modifier(&mut self) -> Result<Option<i32>, Self::Error> {
            assert_eq!(self.stages.last(), Some(&AttackBonus::SpecificReligion));
            self.proposal_read = true;
            Ok(Some(2))
        }
    }
    fn a() -> A {
        A {
            opening: c(),
            stages: vec![],
            fail: None,
            proposal_read: false,
            dead: false,
        }
    }
    #[test]
    fn diplomatic_scope_expiry_and_wrapping_match_native_timing() {
        use crate::player_diplomacy::CombatModifier as M;
        let record = |source, turns_active, combat_bonus, religious_only| M {
            source,
            turns_active,
            combat_bonus,
            religious_only,
        };
        let mut times = [0; 10];
        times[2] = 10;
        times[0] = i32::MAX;
        let values = [
            record(1, 5, 7, false),
            record(1, 5, 99, true),
            record(7, i32::MAX, 999, false),
            record(2, 1, 88, false),
        ];
        assert_eq!(diplomatic_combat_bonus(&values, &times, 15, false), 7);
        assert_eq!(diplomatic_combat_bonus(&values, &times, 16, false), 0);
        assert_eq!(diplomatic_combat_bonus(&values, &times, 15, true), 99);
        times[2] = 0;
        assert_eq!(diplomatic_combat_bonus(&values, &times, 0, false), 0);
    }
    #[test]
    fn attack_order_stops_at_unknown_bonus_and_reads_current_proposal_last() {
        use AttackBonus::*;
        let mut x = a();
        assert_eq!(ordinary_attack_modifier(&mut x), Ok(-1344));
        assert_eq!(
            x.stages,
            [
                Amphibious,
                VersusOpponent,
                Territory,
                Flanking,
                Miscellaneous,
                HolyCities,
                Neighbors,
                MilitaryPolicies,
                Diplomatic,
                Espionage,
                City,
                AdjacentLevy,
                Allies,
                UnusedMovement,
                FriendlyCity,
                SpecificReligion
            ]
        );
        assert!(x.proposal_read);
        let mut x = a();
        x.fail = Some(HolyCities);
        assert_eq!(
            ordinary_attack_modifier(&mut x),
            Err(AttackError::Bonus {
                stage: HolyCities,
                completed_raw: -1920,
                error: "missing current bonus"
            })
        );
        assert_eq!(
            x.stages,
            [
                Amphibious,
                VersusOpponent,
                Territory,
                Flanking,
                Miscellaneous,
                HolyCities
            ]
        );
        assert!(!x.proposal_read);
        let mut x = a();
        x.dead = true;
        assert_eq!(ordinary_attack_modifier(&mut x), Ok(0));
        assert!(x.opening.calls.is_empty());
        assert!(x.stages.is_empty());
        assert!(!x.proposal_read);
    }
    #[test]
    fn flanking_percent_keeps_q8_division_and_unused_moves_keep_fraction() {
        assert_eq!(flanking_from_count(2, 2, 33), 1360);
        assert_eq!(flanking_from_count(1, 2, -50), 256);
        assert_eq!(flanking_from_count(0, i32::MAX, i32::MAX), 0);
        assert_eq!(unused_movement_bonus(128, 3), 384);
        assert_eq!(unused_movement_bonus(-1, 3), -3);
        assert_eq!(unused_movement_bonus(i32::MAX, -1), 0);
    }
    #[test]
    fn quantization_negative_and_wrapping_are_preserved() {
        assert_eq!(reduction_for_damage(33, 0, 10), 840); // .33 quantizes to84/256
        assert_eq!(reduction_for_damage(100, 50, 10), 1280);
        assert_eq!(reduction_for_damage(100, 150, 10), -1280);
        assert_eq!(reduction_for_damage(-1, 0, 10), -30);
        assert_eq!(reduction_for_damage(100, 0, i32::MAX), -256);
    }
}
