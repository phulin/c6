//! Linux City.Growth.AddGrowth over the authoritative current city owner.
//! Threshold rules and all ChangePopulation subscribers are required services;
//! this leaf is not City.DoTurn or a complete city economy implementation.

/// Static current rules inputs; no captured threshold answers are stored.
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub base: i32,
    pub multiplier: i32,
    pub exponent: f32,
    pub speed_cost_multiplier: i32,
}
fn native_cvtt(value: f32) -> i32 {
    if value.is_finite() && (-2147483648.0..2147483648.0).contains(&value) {
        value as i32
    } else {
        i32::MIN
    }
}
fn float_q8(value: f32) -> i32 {
    // Native separates floor/integer and the low fractional byte. Rust's
    // saturating float cast alone differs for NaNs and large values.
    let whole = value.floor();
    native_cvtt(whole).wrapping_shl(8) | i32::from(native_cvtt((value - whole) * 256.0) as u8)
}
impl Parameters {
    /// 2be3230: Q8 arithmetic precedes clamped game-speed scaling and AsInt.
    /// powf follows the target float dependency; native boundary comparison is
    /// required before extending the supported input/domain claim.
    pub fn threshold(self, population: i32) -> i32 {
        let population = population.wrapping_sub(1);
        let base = self.base.wrapping_shl(8);
        let linear = self.multiplier.wrapping_mul(population).wrapping_shl(8);
        let power = float_q8((population as f32).powf(self.exponent));
        let value = base.wrapping_add(linear).wrapping_add(power) as i64;
        let speed = self.speed_cost_multiplier.wrapping_shl(8) as i64;
        ((value * speed / 25_600).clamp(i32::MIN as i64, i32::MAX as i64) as i32) >> 8
    }
}

pub trait World {
    type Error;
    fn food_raw(&mut self) -> Result<i32, Self::Error>;
    fn set_food_raw(&mut self, value: i32) -> Result<(), Self::Error>;
    fn population(&mut self) -> Result<i32, Self::Error>;
    /// Rules.Economic.GetGrowthThreshold for the current rules/game speed.
    fn growth_threshold(&mut self, population: i32) -> Result<i32, Self::Error>;
    /// Original ChangePopulation, including Citizens, Religion, GameSignals and
    /// ordered reporting. Implementations must not supply a scalar-only stub.
    fn change_population(&mut self, delta: i32) -> Result<(), Self::Error>;
}
fn threshold<W: World>(world: &mut W) -> Result<i32, W::Error> {
    let population = world.population()?;
    Ok(world.growth_threshold(population)?.wrapping_shl(8))
}

/// 288a900: at most one population change, even for very large food input.
/// Reads after ChangePopulation observe actual synchronous callback mutations.
/// Errors retain the exact prefix already performed; callers must retain that
/// failed operation rather than count it as completed growth or retry it.
pub fn add_growth<W: World>(world: &mut W, delta_raw_q8: i32) -> Result<(), W::Error> {
    let food = world.food_raw()?.wrapping_add(delta_raw_q8);
    world.set_food_raw(food)?;
    let current_food = world.food_raw()?;
    if current_food >= threshold(world)? {
        // Native deliberately recomputes the old-population threshold.
        let cost = threshold(world)?;
        let food = world.food_raw()?.wrapping_sub(cost);
        world.set_food_raw(food)?;
        world.change_population(1)?;
        let food = world.food_raw()?;
        if food > threshold(world)? {
            let population = world.population()?;
            let cap = world.growth_threshold(population)?.wrapping_sub(1);
            world.set_food_raw(cap.wrapping_shl(8))?;
        }
    } else if world.food_raw()? < 0 {
        if world.population()? >= 2 {
            world.change_population(-1)?;
            // Native uses the original delta, not the old negative food or a
            // callback-mutated food value, with the new population threshold.
            let food = threshold(world)?.wrapping_add(delta_raw_q8).max(0);
            world.set_food_raw(food)?;
        } else {
            world.set_food_raw(0)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct City {
        food: i32,
        population: i32,
        changes: Vec<i32>,
        thresholds: Vec<i32>,
        on_change_food: Option<i32>,
    }
    impl City {
        fn new(food: i32, population: i32) -> Self {
            Self {
                food,
                population,
                changes: vec![],
                thresholds: vec![],
                on_change_food: None,
            }
        }
    }
    impl World for City {
        type Error = &'static str;
        fn food_raw(&mut self) -> Result<i32, Self::Error> {
            Ok(self.food)
        }
        fn set_food_raw(&mut self, v: i32) -> Result<(), Self::Error> {
            self.food = v;
            Ok(())
        }
        fn population(&mut self) -> Result<i32, Self::Error> {
            Ok(self.population)
        }
        fn growth_threshold(&mut self, p: i32) -> Result<i32, Self::Error> {
            self.thresholds.push(p);
            Ok(p * 10)
        }
        fn change_population(&mut self, delta: i32) -> Result<(), Self::Error> {
            self.changes.push(delta);
            self.population += delta;
            if let Some(food) = self.on_change_food {
                self.food = food;
            }
            Ok(())
        }
    }
    #[test]
    fn rules_threshold_keeps_fraction_before_game_speed_and_native_cast_bits() {
        let p = Parameters {
            base: 15,
            multiplier: 8,
            exponent: 1.5,
            speed_cost_multiplier: 100,
        };
        assert_eq!(p.threshold(1), 15);
        assert_eq!(p.threshold(2), 24);
        assert_eq!(
            Parameters {
                speed_cost_multiplier: 150,
                ..p
            }
            .threshold(3),
            50
        );
        assert_eq!(float_q8(-0.1), -26);
        assert_eq!(float_q8(f32::NAN), 0);
        assert_eq!(float_q8(f32::INFINITY), 0);
    }

    #[test]
    fn accumulates_fractional_food_without_population_change() {
        let mut c = City::new(256, 2);
        add_growth(&mut c, 129).unwrap();
        assert_eq!(c.food, 385);
        assert!(c.changes.is_empty());
        assert_eq!(c.thresholds, [2]);
    }
    #[test]
    fn threshold_equality_grows_once_and_subtracts_old_cost() {
        let mut c = City::new(19 * 256, 2);
        add_growth(&mut c, 256).unwrap();
        assert_eq!((c.population, c.food), (3, 0));
        assert_eq!(c.changes, [1]);
        assert_eq!(c.thresholds, [2, 2, 3]);
    }
    #[test]
    fn large_surplus_caps_below_new_threshold_and_never_loops() {
        let mut c = City::new(0, 2);
        add_growth(&mut c, 100 * 256).unwrap();
        assert_eq!((c.population, c.food), (3, 29 * 256));
        assert_eq!(c.changes, [1]);
        assert_eq!(c.thresholds, [2, 2, 3, 3]);
    }
    #[test]
    fn surplus_equal_to_new_threshold_is_preserved() {
        let mut c = City::new(0, 2);
        add_growth(&mut c, 50 * 256).unwrap();
        assert_eq!(c.food, 30 * 256);
        assert_eq!(c.population, 3);
    }
    #[test]
    fn starvation_uses_new_population_threshold_plus_original_delta() {
        let mut c = City::new(256, 3);
        add_growth(&mut c, -3 * 256).unwrap();
        assert_eq!((c.population, c.food), (2, 17 * 256));
        assert_eq!(c.changes, [-1]);
        let mut c = City::new(0, 3);
        add_growth(&mut c, -50 * 256).unwrap();
        assert_eq!(c.food, 0);
    }
    #[test]
    fn population_one_cannot_starve_below_one() {
        let mut c = City::new(0, 1);
        add_growth(&mut c, -1).unwrap();
        assert_eq!(c.food, 0);
        assert!(c.changes.is_empty());
    }
    #[test]
    fn callback_food_change_is_observed_and_addition_wraps() {
        let mut c = City::new(20 * 256, 2);
        c.on_change_food = Some(128);
        add_growth(&mut c, 0).unwrap();
        assert_eq!(c.food, 128);
        let mut c = City::new(i32::MAX, 1);
        add_growth(&mut c, 1).unwrap();
        assert_eq!(c.food, 0);
        assert!(c.changes.is_empty());
    }
}
