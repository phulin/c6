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
    /// Native nonpositive BASE defending-strength branch, which never draws.
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
pub trait ExchangeContext: Context {
    fn calculate_damage(
        &mut self,
        parameters: &DamageParameters,
        average: bool,
    ) -> Result<i32, Self::Error>;
    fn defender_embarked(&mut self) -> Result<bool, Self::Error>;
    fn defender_can_fight_while_embarked(&mut self) -> Result<bool, Self::Error>;
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Exchange {
    pub attacker_received: i32,
    pub defender_received: i32,
}
fn execute<C: ExchangeContext>(c: &mut C, plan: Next) -> Result<i32, C::Error> {
    match plan {
        Next::FixedDamage(damage) => Ok(damage),
        Next::RequiresCalculateDamage {
            parameters,
            average,
        } => c.calculate_damage(&parameters, average),
    }
}
/// Forward damage first, then the CURRENT defender's embark/fight guard, then
/// retaliation. Each random CalculateDamage consumes its own same-stream draw.
pub fn exchange<C: ExchangeContext>(
    c: &mut C,
    attacker: Strength,
    defender: Strength,
    average: bool,
) -> Result<Exchange, C::Error> {
    let forward = prepare(c, defender, attacker, average)?;
    let defender_received = execute(c, forward)?;
    let attacker_received = if !c.defender_embarked()? || c.defender_can_fight_while_embarked()? {
        let reverse = prepare(c, attacker, defender, average)?;
        execute(c, reverse)?
    } else {
        0
    };
    Ok(Exchange {
        attacker_received,
        defender_received,
    })
}
fn cvtt(value: f32) -> i32 {
    if !(-2_147_483_648.0..2_147_483_648.0).contains(&value) {
        i32::MIN
    } else {
        value as i32
    }
}
/// CalculateUnitVsUnitDamage2bd8b80: the positive test uses the original defending base,
/// before either modifier addition. Rule reads retain their native order.
pub fn prepare<C: Context>(
    c: &mut C,
    defender: Strength,
    attacker: Strength,
    average: bool,
) -> Result<Next, C::Error> {
    if defender.base_raw <= 0 {
        let maximum = c.maximum_hit_points()? as f32;
        let percentage = c.minimum_civilian_damage_percent()? as f32;
        return Ok(Next::FixedDamage(cvtt(
            (percentage * f32::from_bits(0x3c23_d70a)) * maximum,
        )));
    }
    // Exact DamageParameters constructor defaults; fields overwritten below
    // still perform their real getters even when the value equals the default.
    let mut parameters = DamageParameters {
        attacker_strength_raw: attacker.base_raw.wrapping_add(attacker.modifier_raw),
        defender_strength_raw: defender.base_raw.wrapping_add(defender.modifier_raw),
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

/// Linux CalculateDamage(District, Unit): both strengths are already combined.
/// Unlike Unit/Unit damage, this path always reaches the damage kernel, even
/// for nonpositive defending strength, and never reads the civilian percentage.
pub fn prepare_district_unit<C: Context>(
    c: &mut C,
    defender_raw: i32,
    attacker_raw: i32,
    average: bool,
) -> Result<Next, C::Error> {
    let mut parameters = DamageParameters {
        attacker_strength_raw: attacker_raw,
        defender_strength_raw: defender_raw,
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
    impl ExchangeContext for Rules {
        fn calculate_damage(
            &mut self,
            p: &DamageParameters,
            average: bool,
        ) -> Result<i32, Self::Error> {
            assert!(average);
            self.reads
                .push(if p.attacker_strength_raw < p.defender_strength_raw {
                    "forward"
                } else {
                    "retaliation"
                });
            crate::combat::calculate_average_damage(p, 12).map_err(|_| "math")
        }
        fn defender_embarked(&mut self) -> Result<bool, Self::Error> {
            self.reads.push("embarked");
            Ok(false)
        }
        fn defender_can_fight_while_embarked(&mut self) -> Result<bool, Self::Error> {
            panic!("unembarked short circuit")
        }
    }
    #[test]
    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
    fn asymmetric_exchange_keeps_forward_then_retaliation_and_parameter_sides() {
        let mut c = Rules {
            reads: vec![],
            fail_base: false,
        };
        let result = exchange(
            &mut c,
            Strength {
                base_raw: 85 << 8,
                modifier_raw: 5 << 8,
            },
            Strength {
                base_raw: 130 << 8,
                modifier_raw: 0,
            },
            true,
        )
        .unwrap();
        assert_eq!(
            result,
            Exchange {
                attacker_received: 100,
                defender_received: 6
            }
        );
        assert_eq!(
            c.reads,
            [
                "maximum",
                "minimum",
                "scaling",
                "base",
                "dampener",
                "forward",
                "embarked",
                "maximum",
                "minimum",
                "scaling",
                "base",
                "dampener",
                "retaliation"
            ]
        );
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
    fn district_nonpositive_defense_still_assembles_random_damage_parameters() {
        let mut c = Rules {
            reads: vec![],
            fail_base: false,
        };
        let Next::RequiresCalculateDamage {
            parameters,
            average,
        } = prepare_district_unit(&mut c, -256, 20 << 8, false).unwrap()
        else {
            panic!("district strike must reach damage kernel")
        };
        assert_eq!(parameters.defender_strength_raw, -256);
        assert_eq!(parameters.attacker_strength_raw, 20 << 8);
        assert!(!average);
        assert_eq!(
            c.reads,
            ["maximum", "minimum", "scaling", "base", "dampener"]
        );
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
        assert_eq!(p.defender_strength_raw, i32::MIN);
        assert_eq!(p.attacker_strength_raw, 5);
        assert_eq!(p.power_scaling_raw, 10);
        assert_eq!(p.defense_type, u32::MAX);
        assert!(!average);
        assert_eq!(
            c.reads,
            ["maximum", "minimum", "scaling", "base", "dampener"]
        );
    }
    #[test]
    fn nonpositive_defender_base_ignores_positive_modifier_and_reads_no_random_rules() {
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
