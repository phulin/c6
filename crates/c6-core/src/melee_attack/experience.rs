//! Ordinary melee Unit/Unit experience calculation (Linux 2bd93b0).
//! Computes awards only; ChangeExperience and its signals are separate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Side {
    Attacker,
    Defender,
}
#[derive(Clone, Copy, Debug)]
pub struct Rules {
    pub maximum_barbarian_level: i32,
    pub barbarian_soft_cap: i32,
    pub base: i32,
    pub kill_bonus: i32,
    pub attacker_bonus: i32,
    pub maximum: i32,
}
pub trait Context {
    type Error;
    fn can_earn(&mut self, side: Side) -> Result<bool, Self::Error>;
    fn defender_is_combat(&mut self) -> Result<bool, Self::Error>;
    fn opponent_barbarian_or_free(&mut self, side: Side) -> Result<bool, Self::Error>;
    fn level(&mut self, side: Side) -> Result<i32, Self::Error>;
    fn modifier(&mut self, side: Side) -> Result<i32, Self::Error>;
    fn combat(&mut self, side: Side) -> Result<i32, Self::Error>;
}
fn mul(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 8) as i32
}
fn ceil(raw: i32) -> i32 {
    (raw >> 8).wrapping_add(i32::from(raw & 255 != 0))
}
/// Both combat strengths are positive before division. The wide numerator
/// preserves native Q8 division, followed by narrowing to the low 32 bits.
pub fn melee<C: Context>(
    c: &mut C,
    rules: Rules,
    side: Side,
    defender_killed: bool,
) -> Result<i32, C::Error> {
    if !c.can_earn(side)? {
        return Ok(0);
    }
    if side == Side::Defender && !c.defender_is_combat()? {
        return Ok(0);
    }
    if c.opponent_barbarian_or_free(side)? && c.level(side)? >= rules.maximum_barbarian_level {
        return Ok(rules.barbarian_soft_cap);
    }
    let attacker_modifier = c.modifier(Side::Attacker)?;
    let attacker = c.combat(Side::Attacker)?.wrapping_shl(8);
    let defender = c.combat(Side::Defender)?.wrapping_shl(8);
    let defender_modifier = c.modifier(Side::Defender)?;
    let kill = if defender_killed { rules.kill_bonus } else { 1 }.wrapping_shl(8);
    if attacker <= 0 || defender <= 0 {
        return Ok(0);
    }
    let (numerator, denominator, modifier) = match side {
        Side::Attacker => (defender, attacker, attacker_modifier),
        Side::Defender => (attacker, defender, defender_modifier),
    };
    let ratio = ((i64::from(mul(kill, numerator)) << 8) / i64::from(denominator)) as i32;
    let bonus = if side == Side::Attacker {
        rules.attacker_bonus
    } else {
        0
    };
    let first = ceil(
        rules
            .base
            .wrapping_add(bonus)
            .wrapping_shl(8)
            .wrapping_add(ratio),
    );
    Ok(ceil(mul(first.wrapping_shl(8), modifier)).min(rules.maximum))
}
#[cfg(test)]
mod tests {
    use super::*;
    struct C {
        can: bool,
        barb: bool,
        level: i32,
        calls: Vec<&'static str>,
    }
    impl Context for C {
        type Error = ();
        fn can_earn(&mut self, _: Side) -> Result<bool, ()> {
            self.calls.push("can");
            Ok(self.can)
        }
        fn defender_is_combat(&mut self) -> Result<bool, ()> {
            self.calls.push("combat gate");
            Ok(true)
        }
        fn opponent_barbarian_or_free(&mut self, _: Side) -> Result<bool, ()> {
            self.calls.push("opponent");
            Ok(self.barb)
        }
        fn level(&mut self, _: Side) -> Result<i32, ()> {
            self.calls.push("level");
            Ok(self.level)
        }
        fn modifier(&mut self, s: Side) -> Result<i32, ()> {
            self.calls.push("modifier");
            Ok(if s == Side::Attacker { 281 } else { 256 })
        }
        fn combat(&mut self, s: Side) -> Result<i32, ()> {
            self.calls.push("strength");
            Ok(if s == Side::Attacker { 85 } else { 130 })
        }
    }
    fn rules() -> Rules {
        Rules {
            maximum_barbarian_level: 2,
            barbarian_soft_cap: 1,
            base: 2,
            kill_bonus: 2,
            attacker_bonus: 1,
            maximum: 8,
        }
    }
    #[test]
    fn two_ceilings_and_asymmetric_strengths() {
        let mut c = C {
            can: true,
            barb: false,
            level: 1,
            calls: vec![],
        };
        assert_eq!(melee(&mut c, rules(), Side::Attacker, false), Ok(6));
        assert_eq!(
            c.calls,
            ["can", "opponent", "modifier", "strength", "strength", "modifier"]
        );
        assert_eq!(melee(&mut c, rules(), Side::Defender, false), Ok(3));
    }
    #[test]
    fn disabled_and_soft_capped_units_do_not_read_modifiers() {
        let mut c = C {
            can: false,
            barb: true,
            level: 2,
            calls: vec![],
        };
        assert_eq!(melee(&mut c, rules(), Side::Defender, true), Ok(0));
        assert_eq!(c.calls, ["can"]);
        c.can = true;
        c.calls.clear();
        assert_eq!(melee(&mut c, rules(), Side::Attacker, true), Ok(1));
        assert_eq!(c.calls, ["can", "opponent", "level"]);
    }
}
