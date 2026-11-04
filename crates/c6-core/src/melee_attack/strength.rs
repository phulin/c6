//! Linux unit-versus-unit strength opening terms. Remaining situational bonuses
//! and current proposal modifiers must still be evaluated before damage.
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
    #[test]
    fn quantization_negative_and_wrapping_are_preserved() {
        assert_eq!(reduction_for_damage(33, 0, 10), 840); // .33 quantizes to84/256
        assert_eq!(reduction_for_damage(100, 50, 10), 1280);
        assert_eq!(reduction_for_damage(100, 150, 10), -1280);
        assert_eq!(reduction_for_damage(-1, 0, 10), -30);
        assert_eq!(reduction_for_damage(100, 0, i32::MAX), -256);
    }
}
