//! City.Placement.IsValidFoundLocation (28bda20) and the stock Rules.Cities
//! minimum-distance scan (2bd57d0). Map topology, current definitions and current
//! plot state must come from the same world; no cached legal-answer input.
use crate::unit::{ComponentId, PlotCoord};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error<E> {
    Context(E),
    ProbeLimit,
}
impl<E: std::fmt::Debug> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FoundCity placement: {self:?}")
    }
}
impl<E: std::fmt::Debug> std::error::Error for Error<E> {}

/// Map.IInstance.GetPlotFromDeltaWithRangeCheck2103f20 + GetPlotFromDelta
/// 2104a50 + stock Map.Instance.GetPlot(x,y)2a48a30. Range precedes wrapping.
pub fn plot_from_delta(
    map: &crate::map::Map,
    center: PlotCoord,
    dx: i32,
    dy: i32,
    range: i32,
) -> Result<Option<usize>, crate::movement_world::MissingInput> {
    let ax = dx.wrapping_abs();
    let ay = dy.wrapping_abs();
    let distance = if (dx ^ dy) >= 0 {
        ax.wrapping_add(ay)
    } else {
        ax.max(ay)
    };
    if distance > range {
        return Ok(None);
    }
    fn half(y: i32) -> i32 {
        if y >= 0 {
            y >> 1
        } else {
            let n = y.wrapping_sub(1);
            n.wrapping_sub(n >> 31) >> 1
        }
    }
    let y = center.y.wrapping_add(dy);
    let x = center
        .x
        .wrapping_sub(half(center.y))
        .wrapping_add(dx)
        .wrapping_add(half(y));
    if !crate::geometry::plot_valid(x, y) {
        return Ok(None);
    }
    if map.header.width <= 0 || map.header.height <= 0 {
        return Err(crate::movement_world::MissingInput::InvalidMapDimensions);
    }
    let x = crate::movement_world::coord_range(x, map.header.width, map.header.wrap_x);
    let y = crate::movement_world::coord_range(y, map.header.height, map.header.wrap_y);
    if x < 0 || y < 0 || x >= map.header.width || y >= map.header.height {
        return Ok(None);
    }
    Ok(Some(
        crate::movement_world::plot_at(map, PlotCoord { x, y })?.0,
    ))
}

pub trait Context {
    type Error;
    type Plot: Copy;
    fn map_exists(&mut self) -> Result<bool, Self::Error>;
    fn plot(&mut self, location: PlotCoord) -> Result<Option<Self::Plot>, Self::Error>;
    fn impassable(&mut self, plot: Self::Plot) -> Result<bool, Self::Error>;
    fn feature_type(&mut self, plot: Self::Plot) -> Result<i32, Self::Error>;
    /// Native direct-index Feature.IsSettlement. Missing reached definitions
    /// must be an error, not an invented false or absent feature.
    fn feature_allows_settlement(&mut self, kind: i32) -> Result<bool, Self::Error>;
    fn district(&mut self, plot: Self::Plot) -> Result<bool, Self::Error>;
    fn player_id(&mut self, owner: i32) -> Result<i32, Self::Error>;
    fn owned(&mut self, plot: Self::Plot) -> Result<bool, Self::Error>;
    fn owner(&mut self, plot: Self::Plot) -> Result<i32, Self::Error>;
    fn water(&mut self, plot: Self::Plot) -> Result<bool, Self::Error>;
    fn minimum_city_range(&mut self) -> Result<i32, Self::Error>;
    fn location(&mut self, plot: Self::Plot) -> Result<PlotCoord, Self::Error>;
    /// Equality is actual landmass identity, including the absent identity.
    fn same_landmass(&mut self, a: Self::Plot, b: Self::Plot) -> Result<bool, Self::Error>;
    /// Compute native GetPlotFromDeltaWithRangeCheck using current map topology.
    fn plot_from_delta(
        &mut self,
        center: PlotCoord,
        dx: i32,
        dy: i32,
        range: i32,
    ) -> Result<Option<Self::Plot>, Self::Error>;
    fn city(&mut self, plot: Self::Plot) -> Result<bool, Self::Error>;
    fn city_identity(&mut self, plot: Self::Plot) -> Result<ComponentId, Self::Error>;
    fn plot_distance(&mut self, a: PlotCoord, b: PlotCoord) -> Result<i32, Self::Error>;
}
struct Budget(usize);
impl Budget {
    fn charge<E>(&mut self) -> Result<(), Error<E>> {
        self.0 = self.0.checked_sub(1).ok_or(Error::ProbeLimit)?;
        Ok(())
    }
}
fn scan<C: Context>(
    c: &mut C,
    center: C::Plot,
    range: i32,
    budget: &mut Budget,
) -> Result<Option<C::Plot>, Error<C::Error>> {
    let start = range.wrapping_neg();
    if range < start {
        return Ok(None);
    }
    let mut dx = start;
    loop {
        let mut dy = start;
        loop {
            budget.charge()?;
            let location = c.location(center).map_err(Error::Context)?;
            if let Some(plot) = c
                .plot_from_delta(location, dx, dy, range)
                .map_err(Error::Context)?
            {
                if c.city(plot).map_err(Error::Context)? {
                    if c.same_landmass(plot, center).map_err(Error::Context)? {
                        return Ok(Some(plot));
                    }
                    let a = c.location(plot).map_err(Error::Context)?;
                    let b = c.location(center).map_err(Error::Context)?;
                    if c.plot_distance(a, b).map_err(Error::Context)? < range {
                        return Ok(Some(plot));
                    }
                }
            }
            if dy >= range {
                break;
            }
            dy = dy.wrapping_add(1);
        }
        if dx >= range {
            break;
        }
        dx = dx.wrapping_add(1);
    }
    Ok(None)
}
/// Exact stock minimum-distance query. The first matching city's ComponentID
/// may itself be invalid; retain it for the caller's actual IsValid check.
pub fn find_city_within_minimum_distance<C: Context>(
    c: &mut C,
    center: C::Plot,
    max_probes: usize,
) -> Result<Option<ComponentId>, Error<C::Error>> {
    let range = c.minimum_city_range().map_err(Error::Context)?;
    match scan(c, center, range, &mut Budget(max_probes))? {
        Some(plot) => Ok(Some(c.city_identity(plot).map_err(Error::Context)?)),
        None => Ok(None),
    }
}
/// `max_probes` bounds host work only; exhausting it is Unsupported/error,
/// never legal/illegal gameplay. This preserves extreme native range values.
/// The optional stock Rules.Cities implementation is computed here, not a
/// supplied predicate. Callers with a different live Rules.Cities vtable need
/// that implementation instead of this stock profile.
pub fn is_valid<C: Context>(
    c: &mut C,
    owner: i32,
    location: PlotCoord,
    max_probes: usize,
) -> Result<bool, Error<C::Error>> {
    if !c.map_exists().map_err(Error::Context)? {
        return Ok(false);
    }
    let Some(plot) = c.plot(location).map_err(Error::Context)? else {
        return Ok(false);
    };
    if c.impassable(plot).map_err(Error::Context)? {
        return Ok(false);
    }
    let feature = c.feature_type(plot).map_err(Error::Context)?;
    if feature != -1
        && !c
            .feature_allows_settlement(feature)
            .map_err(Error::Context)?
    {
        return Ok(false);
    }
    if c.district(plot).map_err(Error::Context)? {
        return Ok(false);
    }
    if owner != -1 {
        // Native obtains the player even for an unowned candidate plot.
        let id = c.player_id(owner).map_err(Error::Context)?;
        if c.owned(plot).map_err(Error::Context)? && c.owner(plot).map_err(Error::Context)? != id {
            return Ok(false);
        }
    } else if c.owned(plot).map_err(Error::Context)? {
        return Ok(false);
    }
    if c.water(plot).map_err(Error::Context)? {
        return Ok(false);
    }
    let mut budget = Budget(max_probes);
    let range = c.minimum_city_range().map_err(Error::Context)?;
    if let Some(found) = scan(c, plot, range, &mut budget)? {
        let identity = c.city_identity(found).map_err(Error::Context)?;
        if identity.owner != -1 && identity.id != u32::MAX {
            return Ok(false);
        }
    }
    // Native placement performs a second scan even when the stock Rules.Cities
    // result was absent/invalid. Do not elide it or reuse its first answer.
    let range = c.minimum_city_range().map_err(Error::Context)?;
    Ok(scan(c, plot, range, &mut budget)?.is_none())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Map {
        range: i32,
        same: bool,
        distance: i32,
        owned: bool,
        owner: i32,
        player_calls: usize,
        scans: usize,
        identity: ComponentId,
        feature: i32,
        settlement: bool,
        probes: Vec<(i32, i32)>,
    }
    impl Default for Map {
        fn default() -> Self {
            Self {
                range: 1,
                same: true,
                distance: 1,
                owned: false,
                owner: 0,
                player_calls: 0,
                scans: 0,
                identity: ComponentId {
                    owner: 0,
                    component_type: 0,
                    id: 1,
                },
                feature: -1,
                settlement: true,
                probes: vec![],
            }
        }
    }
    impl Context for Map {
        type Error = &'static str;
        type Plot = u8;
        fn map_exists(&mut self) -> Result<bool, Self::Error> {
            Ok(true)
        }
        fn plot(&mut self, _: PlotCoord) -> Result<Option<u8>, Self::Error> {
            Ok(Some(0))
        }
        fn impassable(&mut self, _: u8) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn feature_type(&mut self, _: u8) -> Result<i32, Self::Error> {
            Ok(self.feature)
        }
        fn feature_allows_settlement(&mut self, _: i32) -> Result<bool, Self::Error> {
            Ok(self.settlement)
        }
        fn district(&mut self, _: u8) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn player_id(&mut self, v: i32) -> Result<i32, Self::Error> {
            self.player_calls += 1;
            Ok(v)
        }
        fn owned(&mut self, _: u8) -> Result<bool, Self::Error> {
            Ok(self.owned)
        }
        fn owner(&mut self, _: u8) -> Result<i32, Self::Error> {
            Ok(self.owner)
        }
        fn water(&mut self, _: u8) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn minimum_city_range(&mut self) -> Result<i32, Self::Error> {
            self.scans += 1;
            Ok(self.range)
        }
        fn location(&mut self, p: u8) -> Result<PlotCoord, Self::Error> {
            Ok(PlotCoord { x: p as i32, y: 0 })
        }
        fn same_landmass(&mut self, _: u8, _: u8) -> Result<bool, Self::Error> {
            Ok(self.same)
        }
        fn plot_from_delta(
            &mut self,
            _: PlotCoord,
            x: i32,
            y: i32,
            _: i32,
        ) -> Result<Option<u8>, Self::Error> {
            self.probes.push((x, y));
            Ok(Some(if x == self.range && y == self.range {
                1
            } else {
                0
            }))
        }
        fn city(&mut self, p: u8) -> Result<bool, Self::Error> {
            Ok(p == 1)
        }
        fn city_identity(&mut self, _: u8) -> Result<ComponentId, Self::Error> {
            Ok(self.identity)
        }
        fn plot_distance(&mut self, _: PlotCoord, _: PlotCoord) -> Result<i32, Self::Error> {
            Ok(self.distance)
        }
    }
    fn at() -> PlotCoord {
        PlotCoord { x: 4, y: 5 }
    }
    #[test]
    fn distance_boundary_depends_on_landmass_and_preserves_loop_order() {
        let mut map = Map::default();
        assert_eq!(is_valid(&mut map, 0, at(), 100), Ok(false));
        assert_eq!(
            map.probes,
            [
                (-1, -1),
                (-1, 0),
                (-1, 1),
                (0, -1),
                (0, 0),
                (0, 1),
                (1, -1),
                (1, 0),
                (1, 1)
            ]
        );
        let mut map = Map {
            same: false,
            ..Map::default()
        };
        assert_eq!(is_valid(&mut map, 0, at(), 100), Ok(true));
        assert_eq!(map.scans, 2);
        map.distance = 0;
        assert_eq!(is_valid(&mut map, 0, at(), 100), Ok(false));
    }
    #[test]
    fn invalid_first_city_identity_does_not_bypass_second_city_scan() {
        let mut map = Map::default();
        map.identity.id = u32::MAX;
        assert_eq!(is_valid(&mut map, 0, at(), 100), Ok(false));
        assert_eq!(map.scans, 2);
        assert_eq!(map.probes.len(), 18);
    }
    #[test]
    fn owner_minus_one_and_feature_gate_are_lazy() {
        let mut map = Map {
            owned: true,
            ..Map::default()
        };
        assert_eq!(is_valid(&mut map, -1, at(), 100), Ok(false));
        assert_eq!(map.player_calls, 0);
        let mut map = Map {
            feature: 3,
            settlement: false,
            ..Map::default()
        };
        assert_eq!(is_valid(&mut map, 0, at(), 100), Ok(false));
        assert_eq!(map.player_calls, 0);
        assert_eq!(map.scans, 0);
    }
    #[test]
    fn negative_ranges_and_host_budget_are_not_clamped_into_gameplay() {
        let mut map = Map {
            range: -1,
            ..Map::default()
        };
        assert_eq!(is_valid(&mut map, 0, at(), 0), Ok(true));
        assert!(map.probes.is_empty());
        let mut map = Map::default();
        assert_eq!(is_valid(&mut map, 0, at(), 1), Err(Error::ProbeLimit));
        assert_eq!(map.probes.len(), 1);
        let mut map = Map {
            range: i32::MIN,
            ..Map::default()
        };
        assert_eq!(
            find_city_within_minimum_distance(&mut map, 0, 1),
            Ok(Some(map.identity))
        );
        assert_eq!(map.probes, [(i32::MIN, i32::MIN)]);
    }
}
