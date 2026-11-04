//! Linux Unit/Unit damage-parameter construction, before CalculateDamage.
//! This does not draw, apply damage, or complete combat generation/resolution.
use crate::{combat::DamageParameters, movement::MovementPoints};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Strength {
    pub base_raw: i32,
    pub modifier_raw: i32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Next {
    /// Native nonpositive BASE attack-strength branch, which never draws.
    FixedDamage(i32),
    RequiresCalculateDamage {
        parameters: DamageParameters,
        average: bool,
    },
}
pub trait Context {
    type Error;
    fn maximum_hit_points(&mut self) -> Result<i32, Self::Error>;
    fn minimum_damage(&mut self) -> Result<i32, Self::Error>;
    fn power_scaling(&mut self) -> Result<f32, Self::Error>;
    fn base_damage(&mut self) -> Result<i32, Self::Error>;
    fn power_dampener(&mut self) -> Result<i32, Self::Error>;
    fn minimum_civilian_damage_percent(&mut self) -> Result<i32, Self::Error>;
}
fn cvtt(value: f32) -> i32 {
    if !(-2_147_483_648.0..2_147_483_648.0).contains(&value) {
        i32::MIN
    } else {
        value as i32
    }
}
/// CalculateUnitVsUnitDamage2bd8b80: the positive test uses the original base,
/// before either modifier addition. Rule reads retain their native order.
pub fn prepare<C: Context>(
    c: &mut C,
    dealing: Strength,
    receiving: Strength,
    average: bool,
) -> Result<Next, C::Error> {
    if dealing.base_raw <= 0 {
        let maximum = c.maximum_hit_points()? as f32;
        let percentage = c.minimum_civilian_damage_percent()? as f32;
        return Ok(Next::FixedDamage(cvtt(
            (percentage * f32::from_bits(0x3c23_d70a)) * maximum,
        )));
    }
    // Exact DamageParameters constructor defaults; fields overwritten below
    // still perform their real getters even when the value equals the default.
    let mut parameters = DamageParameters {
        attacker_strength_raw: dealing.base_raw.wrapping_add(dealing.modifier_raw),
        defender_strength_raw: receiving.base_raw.wrapping_add(receiving.modifier_raw),
        defender_max_hit_points: 0,
        outer_defense_damage: 0,
        outer_defense_max_hit_points: 0,
        defense_type: u32::MAX,
        power_scaling_raw: 256,
        minimum_damage: 0,
        base_damage: 0,
        power_dampener: 0,
        max_extra_damage: 0,
        bypass_defenses: false,
    };
    parameters.defender_max_hit_points = c.maximum_hit_points()?;
    parameters.minimum_damage = c.minimum_damage()?;
    parameters.power_scaling_raw = MovementPoints::route_float_bits(c.power_scaling()?.to_bits()).0;
    parameters.base_damage = c.base_damage()?;
    parameters.power_dampener = c.power_dampener()?;
    Ok(Next::RequiresCalculateDamage {
        parameters,
        average,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Rules {
        reads: Vec<&'static str>,
        fail_base: bool,
    }
    impl Context for Rules {
        type Error = &'static str;
        fn maximum_hit_points(&mut self) -> Result<i32, Self::Error> {
            self.reads.push("maximum");
            Ok(100)
        }
        fn minimum_damage(&mut self) -> Result<i32, Self::Error> {
            self.reads.push("minimum");
            Ok(1)
        }
        fn power_scaling(&mut self) -> Result<f32, Self::Error> {
            self.reads.push("scaling");
            Ok(0.04)
        }
        fn base_damage(&mut self) -> Result<i32, Self::Error> {
            self.reads.push("base");
            if self.fail_base {
                Err("base")
            } else {
                Ok(24)
            }
        }
        fn power_dampener(&mut self) -> Result<i32, Self::Error> {
            self.reads.push("dampener");
            Ok(0)
        }
        fn minimum_civilian_damage_percent(&mut self) -> Result<i32, Self::Error> {
            self.reads.push("civilian");
            Ok(50)
        }
    }
    #[test]
    fn base_guard_precedes_wrapped_modifier_sums_and_preserves_rule_order() {
        let mut c = Rules {
            reads: vec![],
            fail_base: false,
        };
        let Next::RequiresCalculateDamage {
            parameters: p,
            average,
        } = prepare(
            &mut c,
            Strength {
                base_raw: i32::MAX,
                modifier_raw: 1,
            },
            Strength {
                base_raw: 0,
                modifier_raw: 5,
            },
            false,
        )
        .unwrap()
        else {
            panic!()
        };
        assert_eq!(p.attacker_strength_raw, i32::MIN);
        assert_eq!(p.defender_strength_raw, 5);
        assert_eq!(p.power_scaling_raw, 10);
        assert_eq!(p.defense_type, u32::MAX);
        assert!(!average);
        assert_eq!(
            c.reads,
            ["maximum", "minimum", "scaling", "base", "dampener"]
        );
    }
    #[test]
    fn nonpositive_base_ignores_positive_modifier_and_reads_no_random_rules() {
        let mut c = Rules {
            reads: vec![],
            fail_base: true,
        };
        let value = prepare(
            &mut c,
            Strength {
                base_raw: 0,
                modifier_raw: 10000,
            },
            Strength {
                base_raw: 1,
                modifier_raw: 0,
            },
            false,
        )
        .unwrap();
        assert_eq!(value, Next::FixedDamage(50));
        assert_eq!(c.reads, ["maximum", "civilian"]);
    }
    #[test]
    fn getter_failure_retains_prefix_without_reading_later_rules() {
        let mut c = Rules {
            reads: vec![],
            fail_base: true,
        };
        assert_eq!(
            prepare(
                &mut c,
                Strength {
                    base_raw: 1,
                    modifier_raw: 0
                },
                Strength {
                    base_raw: 1,
                    modifier_raw: 0
                },
                true
            ),
            Err("base")
        );
        assert_eq!(c.reads, ["maximum", "minimum", "scaling", "base"]);
    }
}
