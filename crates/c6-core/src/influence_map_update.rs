//! Eager InfluenceMap rebuild at the current callback boundary.
//!
//! Unit influence jobs and ranged district neighborhoods remain explicit
//! unsupported branches. The supported path still reads current war, unit,
//! district and feature state; an empty threat set is never supplied as an answer.
use crate::player_ai_influence_map::InfluenceMap;

#[derive(Debug, PartialEq, Eq)]
pub enum Error<E> {
    Backend(E),
    MissingReligiousStorage,
    PlotIndex(usize),
    EnemyUnitJobs,
    EnemyDistrict,
    FriendlyRangedDistrict,
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "influence rebuild: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}
pub trait Context {
    type Error;
    type Unit: Copy;
    type District: Copy;
    fn is_human(&mut self) -> Result<bool, Self::Error>;
    fn turn_active(&mut self) -> Result<bool, Self::Error>;
    fn plot_count(&mut self) -> Result<usize, Self::Error>;
    fn influence(&mut self) -> Result<&mut InfluenceMap, Self::Error>;
    fn alive_count(&mut self) -> Result<usize, Self::Error>;
    fn alive_at(&mut self, index: usize) -> Result<i32, Self::Error>;
    /// Direction is other.IsAtWarWith(the InfluenceMap's parent owner).
    fn at_war(&mut self, other: i32) -> Result<bool, Self::Error>;
    fn unit_count(&mut self, owner: i32) -> Result<usize, Self::Error>;
    fn unit_at(&mut self, owner: i32, index: usize) -> Result<Self::Unit, Self::Error>;
    /// GetPlayerVisibility precedes every Unit predicate, even delayed death.
    fn visibility(&mut self) -> Result<(), Self::Error>;
    fn delayed_death(&mut self, unit: Self::Unit) -> Result<bool, Self::Error>;
    fn is_combat(&mut self, unit: Self::Unit) -> Result<bool, Self::Error>;
    fn valid_location(&mut self, unit: Self::Unit) -> Result<bool, Self::Error>;
    fn unit_visible(&mut self, unit: Self::Unit) -> Result<bool, Self::Error>;
    fn district_count(&mut self, owner: Option<i32>) -> Result<usize, Self::Error>;
    fn district_at(
        &mut self,
        owner: Option<i32>,
        index: usize,
    ) -> Result<Self::District, Self::Error>;
    fn district_visible(&mut self, district: Self::District) -> Result<bool, Self::Error>;
    fn district_plot(&mut self, district: Self::District) -> Result<usize, Self::Error>;
    fn district_defense_q8(&mut self, district: Self::District) -> Result<i32, Self::Error>;
    fn district_can_attack(&mut self, district: Self::District) -> Result<bool, Self::Error>;
    fn feature_danger(&mut self, plot: usize) -> Result<Option<i32>, Self::Error>;
}

/// UnitRemoved's captured InfluenceMap lambda. False branches do not inspect
/// storage, diplomacy or visibility. True executes the rebuild synchronously.
pub fn on_unit_removed<C: Context>(c: &mut C) -> Result<bool, Error<C::Error>> {
    if !c.is_human().map_err(Error::Backend)? || !c.turn_active().map_err(Error::Backend)? {
        return Ok(false);
    }
    update(c)?;
    Ok(true)
}

pub fn update<C: Context>(c: &mut C) -> Result<(), Error<C::Error>> {
    let count = c.plot_count().map_err(Error::Backend)?;
    {
        let map = c.influence().map_err(Error::Backend)?;
        let religious = map
            .religious_influence
            .as_mut()
            .ok_or(Error::MissingReligiousStorage)?;
        // Allocate is called if either native vector is empty. Existing nonempty
        // vectors retain their actual size; native clear visits that whole size.
        if map.influence.is_empty() || religious.is_empty() {
            map.influence.resize(count, 0);
            religious.resize(count, 0);
        }
        map.influence.fill(0);
        religious.fill(0);
    }
    for i in 0..c.alive_count().map_err(Error::Backend)? {
        let other = c.alive_at(i).map_err(Error::Backend)?;
        if !c.at_war(other).map_err(Error::Backend)? {
            continue;
        }
        let mut selected = false;
        for j in 0..c.unit_count(other).map_err(Error::Backend)? {
            let unit = c.unit_at(other, j).map_err(Error::Backend)?;
            c.visibility().map_err(Error::Backend)?;
            if !c.delayed_death(unit).map_err(Error::Backend)?
                && c.is_combat(unit).map_err(Error::Backend)?
                && c.valid_location(unit).map_err(Error::Backend)?
                && c.unit_visible(unit).map_err(Error::Backend)?
            {
                selected = true;
            }
        }
        if selected {
            return Err(Error::EnemyUnitJobs);
        }
        for j in 0..c.district_count(Some(other)).map_err(Error::Backend)? {
            let district = c.district_at(Some(other), j).map_err(Error::Backend)?;
            if c.district_visible(district).map_err(Error::Backend)? {
                return Err(Error::EnemyDistrict);
            }
        }
    }
    for i in 0..c.district_count(None).map_err(Error::Backend)? {
        let district = c.district_at(None, i).map_err(Error::Backend)?;
        let plot = c.district_plot(district).map_err(Error::Backend)?;
        if c.district_defense_q8(district).map_err(Error::Backend)? > 0 {
            *c.influence()
                .map_err(Error::Backend)?
                .influence
                .get_mut(plot)
                .ok_or(Error::PlotIndex(plot))? = 0;
        }
        if c.district_can_attack(district).map_err(Error::Backend)? {
            return Err(Error::FriendlyRangedDistrict);
        }
    }
    for plot in 0..count {
        if let Some(danger) = c.feature_danger(plot).map_err(Error::Backend)? {
            let value = c
                .influence()
                .map_err(Error::Backend)?
                .influence
                .get_mut(plot)
                .ok_or(Error::PlotIndex(plot))?;
            *value = value.wrapping_add(danger as i16);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        human: bool,
        active: bool,
        war: bool,
        units: usize,
        delayed: bool,
        combat: bool,
        visible: bool,
        map: InfluenceMap,
        log: Vec<&'static str>,
    }
    impl Default for Fixture {
        fn default() -> Self {
            Self {
                human: true,
                active: true,
                war: false,
                units: 0,
                delayed: false,
                combat: true,
                visible: true,
                map: InfluenceMap {
                    wire_version: 2,
                    influence: vec![77, 88],
                    religious_influence: Some(vec![99, 100]),
                },
                log: vec![],
            }
        }
    }
    impl Context for Fixture {
        type Error = &'static str;
        type Unit = usize;
        type District = usize;
        fn is_human(&mut self) -> Result<bool, Self::Error> {
            self.log.push("human");
            Ok(self.human)
        }
        fn turn_active(&mut self) -> Result<bool, Self::Error> {
            self.log.push("active");
            Ok(self.active)
        }
        fn plot_count(&mut self) -> Result<usize, Self::Error> {
            self.log.push("plots");
            Ok(2)
        }
        fn influence(&mut self) -> Result<&mut InfluenceMap, Self::Error> {
            Ok(&mut self.map)
        }
        fn alive_count(&mut self) -> Result<usize, Self::Error> {
            Ok(1)
        }
        fn alive_at(&mut self, _: usize) -> Result<i32, Self::Error> {
            Ok(17)
        }
        fn at_war(&mut self, other: i32) -> Result<bool, Self::Error> {
            assert_eq!(other, 17);
            self.log.push("war");
            Ok(self.war)
        }
        fn unit_count(&mut self, _: i32) -> Result<usize, Self::Error> {
            self.log.push("units");
            Ok(self.units)
        }
        fn unit_at(&mut self, _: i32, index: usize) -> Result<usize, Self::Error> {
            Ok(index)
        }
        fn visibility(&mut self) -> Result<(), Self::Error> {
            self.log.push("visibility");
            Ok(())
        }
        fn delayed_death(&mut self, _: usize) -> Result<bool, Self::Error> {
            self.log.push("delayed");
            Ok(self.delayed)
        }
        fn is_combat(&mut self, _: usize) -> Result<bool, Self::Error> {
            self.log.push("combat");
            Ok(self.combat)
        }
        fn valid_location(&mut self, _: usize) -> Result<bool, Self::Error> {
            self.log.push("location");
            Ok(true)
        }
        fn unit_visible(&mut self, _: usize) -> Result<bool, Self::Error> {
            self.log.push("visible");
            Ok(self.visible)
        }
        fn district_count(&mut self, owner: Option<i32>) -> Result<usize, Self::Error> {
            Ok(usize::from(owner.is_none()))
        }
        fn district_at(&mut self, _: Option<i32>, index: usize) -> Result<usize, Self::Error> {
            Ok(index)
        }
        fn district_visible(&mut self, _: usize) -> Result<bool, Self::Error> {
            unreachable!()
        }
        fn district_plot(&mut self, _: usize) -> Result<usize, Self::Error> {
            self.log.push("district plot");
            Ok(0)
        }
        fn district_defense_q8(&mut self, _: usize) -> Result<i32, Self::Error> {
            self.log.push("defense");
            Ok(1)
        }
        fn district_can_attack(&mut self, _: usize) -> Result<bool, Self::Error> {
            self.log.push("can attack");
            Ok(false)
        }
        fn feature_danger(&mut self, index: usize) -> Result<Option<i32>, Self::Error> {
            self.log.push("feature");
            Ok(Some(if index == 0 { 65535 } else { 32768 }))
        }
    }
    #[test]
    fn human_active_short_circuit_preserves_both_maps() {
        let mut c = Fixture {
            human: false,
            ..Fixture::default()
        };
        let before = c.map.clone();
        assert_eq!(on_unit_removed(&mut c), Ok(false));
        assert_eq!(c.log, ["human"]);
        assert_eq!(c.map, before);
        c.human = true;
        c.active = false;
        c.log.clear();
        assert_eq!(on_unit_removed(&mut c), Ok(false));
        assert_eq!(c.log, ["human", "active"]);
        assert_eq!(c.map, before);
    }
    #[test]
    fn rebuild_clears_both_then_friendly_defense_then_wrapping_feature_danger() {
        let mut c = Fixture::default();
        assert_eq!(on_unit_removed(&mut c), Ok(true));
        assert_eq!(c.map.influence, [-1, i16::MIN]);
        assert_eq!(
            c.map.religious_influence.as_deref(),
            Some([0, 0].as_slice())
        );
        assert_eq!(
            c.log,
            [
                "human",
                "active",
                "plots",
                "war",
                "district plot",
                "defense",
                "can attack",
                "feature",
                "feature"
            ]
        );
    }
    #[test]
    fn selected_enemy_jobs_refuse_with_cleared_prefix_after_all_unit_filters() {
        let mut c = Fixture {
            war: true,
            units: 2,
            ..Fixture::default()
        };
        assert_eq!(on_unit_removed(&mut c), Err(Error::EnemyUnitJobs));
        assert_eq!(c.map.influence, [0, 0]);
        assert_eq!(c.map.religious_influence, Some(vec![0, 0]));
        assert_eq!(c.log.iter().filter(|s| **s == "visible").count(), 2);
        assert!(!c.log.contains(&"district plot"));
    }
    #[test]
    fn delayed_enemy_gets_visibility_pointer_before_lazy_filter_and_empty_vectors_allocate() {
        let mut c = Fixture {
            war: true,
            units: 1,
            delayed: true,
            ..Fixture::default()
        };
        c.map.influence.clear();
        assert_eq!(on_unit_removed(&mut c), Ok(true));
        assert!(!c.log.contains(&"combat"));
        let position = c.log.iter().position(|s| *s == "visibility").unwrap();
        assert_eq!(c.log[position + 1], "delayed");
        assert_eq!(c.map.influence.len(), 2);
        assert_eq!(c.map.religious_influence, Some(vec![0, 0]));
    }
}
