//! Member preparation before the mandatory generic combat generator.
//! This is a logical member projection, not generated strengths/damage or an
//! applied attack. No operations, fortification, RNG or world state are changed.
use crate::{
    geometry::plot_valid,
    unit::{ComponentId, PlotCoord},
};

pub const MELEE: u32 = 0x2ca3_edd1;

/// The identity and location copied by Results.SetUnit/SetDistrict into Member.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Member {
    pub identity: ComponentId,
    pub location: PlotCoord,
}

/// Only the member portion of the caller-owned combat results. Other combat
/// result fields must still be computed by the generic generator.
#[derive(Debug, Eq, PartialEq)]
pub struct Members {
    pub attacker: Option<Member>,
    pub defender: Option<Member>,
    pub defender_is_district: bool,
    pub location: PlotCoord,
    pub combat_type: u32,
    pub interceptor: Option<Member>,
    pub anti_air: Option<Member>,
}
impl Members {
    pub fn new(location: PlotCoord) -> Self {
        Self {
            attacker: None,
            defender: None,
            defender_is_district: false,
            location,
            combat_type: u32::MAX,
            interceptor: None,
            anti_air: None,
        }
    }
}

/// A mandatory unfinished continuation, never a successful Generate/AttackInto.
#[must_use]
#[derive(Debug, Eq, PartialEq)]
pub struct RequiresGenerate {
    // Native overloads invoke GenerateCombatResults(results, false, true).
    _private: (),
}

/// First ordinary Unit/Unit generator reads (28cf960..28cfa5a). Values are
/// signed FixedPointT<8> storage, before modifiers, damage or any RNG draw.
#[derive(Debug, Eq, PartialEq)]
pub struct BaseStrengths {
    pub attacker_raw: i32,
    pub defender_raw: i32,
}
#[must_use]
#[derive(Debug, Eq, PartialEq)]
pub enum UnitGeneration {
    /// The native support formation-class branch skips ProposeCombat.
    RequiresSupportResolution(BaseStrengths),
    RequiresProposeCombat(BaseStrengths),
}
pub trait StrengthContext {
    type Error;
    fn plot_exists(&mut self) -> Result<bool, Self::Error>;
    fn attacker_embarked(&mut self) -> Result<bool, Self::Error>;
    fn plot_is_coastal_land(&mut self) -> Result<bool, Self::Error>;
    fn attacker_combat(&mut self, coastal_embarked: bool) -> Result<i32, Self::Error>;
    fn defender_combat(&mut self) -> Result<i32, Self::Error>;
    fn defender_formation_class(&mut self) -> Result<i32, Self::Error>;
}
pub fn prepare_unit_strengths<C: StrengthContext>(c: &mut C) -> Result<UnitGeneration, C::Error> {
    let plot = c.plot_exists()?;
    let embarked = c.attacker_embarked()?;
    let coastal = if plot && embarked {
        c.plot_is_coastal_land()?
    } else {
        false
    };
    let attacker_raw = c.attacker_combat(coastal)?.wrapping_shl(8);
    let defender_raw = c.defender_combat()?.wrapping_shl(8);
    let strengths = BaseStrengths {
        attacker_raw,
        defender_raw,
    };
    Ok(if c.defender_formation_class()? == 2 {
        UnitGeneration::RequiresSupportResolution(strengths)
    } else {
        UnitGeneration::RequiresProposeCombat(strengths)
    })
}

/// GenerateCombatResults(Unit,Unit), 28ca120: preserve a valid preexisting
/// results location; otherwise use the defender's copied current location.
pub fn prepare_units(
    results: &mut Members,
    attacker: Member,
    defender: Member,
    flags: u32,
) -> RequiresGenerate {
    prepare(results, attacker, defender, false, flags)
}

/// GenerateCombatResults(Unit,District), 28c9dd0: always replace the results
/// location with the current district location, even when already valid.
pub fn prepare_district(
    results: &mut Members,
    attacker: Member,
    defender: Member,
    flags: u32,
) -> RequiresGenerate {
    prepare(results, attacker, defender, true, flags)
}

fn prepare(
    results: &mut Members,
    attacker: Member,
    defender: Member,
    district: bool,
    flags: u32,
) -> RequiresGenerate {
    results.attacker = Some(attacker);
    results.defender = Some(defender);
    results.defender_is_district = district;
    if district || !plot_valid(results.location.x, results.location.y) {
        results.location = defender.location;
    }
    results.combat_type = MELEE;
    if flags & 0x80 == 0 {
        // AddSupportMembers28cd150 first clears slot3, then slot2. MELEE is
        // neither WMD61c414a2 nor AIR46a0d8c5, so it returns before all member
        // queries, interceptor/anti-air selection and operation mutation.
        results.anti_air = None;
        results.interceptor = None;
    }
    RequiresGenerate { _private: () }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn member(id: u32) -> Member {
        Member {
            identity: ComponentId {
                owner: 2,
                component_type: 1,
                id,
            },
            location: PlotCoord { x: id as i32, y: 7 },
        }
    }
    #[test]
    fn melee_clears_stale_support_and_preserves_valid_unit_location() {
        let mut r = Members::new(PlotCoord { x: 30, y: 20 });
        r.anti_air = Some(member(3));
        r.interceptor = Some(member(4));
        let _next = prepare_units(&mut r, member(1), member(2), 0);
        assert_eq!(r.location, PlotCoord { x: 30, y: 20 });
        assert_eq!(r.combat_type, MELEE);
        assert_eq!(r.anti_air, None);
        assert_eq!(r.interceptor, None);
        assert_eq!(r.attacker, Some(member(1)));
        assert_eq!(r.defender, Some(member(2)));
    }
    #[test]
    fn district_overwrites_location_but_unit_only_falls_back_when_invalid() {
        let mut r = Members::new(PlotCoord { x: -9999, y: 0 });
        let _next = prepare_units(&mut r, member(1), member(2), 0);
        assert_eq!(r.location, member(2).location);
        let _next = prepare_district(&mut r, member(1), member(5), 0);
        assert_eq!(r.location, member(5).location);
        assert!(r.defender_is_district);
    }
    #[test]
    fn low_byte_bit7_alone_skips_support_clear() {
        let mut r = Members::new(member(1).location);
        r.interceptor = Some(member(9));
        let _next = prepare_units(&mut r, member(1), member(2), 0x80);
        assert_eq!(r.interceptor, Some(member(9)));
        let _next = prepare_units(&mut r, member(1), member(2), 0x8000_0000);
        assert_eq!(r.interceptor, None);
    }

    struct Strengths {
        calls: Vec<&'static str>,
        embarked: bool,
        plot: bool,
        class: i32,
        fail: Option<&'static str>,
    }
    impl Strengths {
        fn read<T>(&mut self, name: &'static str, value: T) -> Result<T, &'static str> {
            self.calls.push(name);
            if self.fail == Some(name) {
                Err(name)
            } else {
                Ok(value)
            }
        }
    }
    impl StrengthContext for Strengths {
        type Error = &'static str;
        fn plot_exists(&mut self) -> Result<bool, Self::Error> {
            self.read("plot", self.plot)
        }
        fn attacker_embarked(&mut self) -> Result<bool, Self::Error> {
            self.read("embarked", self.embarked)
        }
        fn plot_is_coastal_land(&mut self) -> Result<bool, Self::Error> {
            self.read("coastal", true)
        }
        fn attacker_combat(&mut self, coastal: bool) -> Result<i32, Self::Error> {
            self.read(
                if coastal {
                    "attack_true"
                } else {
                    "attack_false"
                },
                i32::MAX,
            )
        }
        fn defender_combat(&mut self) -> Result<i32, Self::Error> {
            self.read("defend_false", 85)
        }
        fn defender_formation_class(&mut self) -> Result<i32, Self::Error> {
            self.read("class", self.class)
        }
    }
    #[test]
    fn ordinary_strengths_read_before_proposal_and_keep_wrapping_q8() {
        let mut c = Strengths {
            calls: vec![],
            embarked: false,
            plot: true,
            class: 0,
            fail: None,
        };
        assert_eq!(
            prepare_unit_strengths(&mut c),
            Ok(UnitGeneration::RequiresProposeCombat(BaseStrengths {
                attacker_raw: -256,
                defender_raw: 85 << 8
            }))
        );
        assert_eq!(
            c.calls,
            ["plot", "embarked", "attack_false", "defend_false", "class"]
        );
    }
    #[test]
    fn coastal_error_stops_before_strengths_and_missing_plot_skips_coastal() {
        let mut c = Strengths {
            calls: vec![],
            embarked: true,
            plot: true,
            class: 2,
            fail: Some("coastal"),
        };
        assert_eq!(prepare_unit_strengths(&mut c), Err("coastal"));
        assert_eq!(c.calls, ["plot", "embarked", "coastal"]);
        c.plot = false;
        c.calls.clear();
        assert!(matches!(
            prepare_unit_strengths(&mut c),
            Ok(UnitGeneration::RequiresSupportResolution(_))
        ));
        assert_eq!(
            c.calls,
            ["plot", "embarked", "attack_false", "defend_false", "class"]
        );
    }
}
