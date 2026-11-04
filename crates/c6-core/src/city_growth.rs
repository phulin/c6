//! Linux City.Growth.AddGrowth over the authoritative current city owner.
//! Threshold rules and all ChangePopulation subscribers are required services;
//! this leaf is not City.DoTurn or a complete city economy implementation.

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
