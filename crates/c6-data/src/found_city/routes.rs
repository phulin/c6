//! City.Manager.SetComplete's road selection and initial road creation.
//! Source: 28b9400, 2bcdc40, 2bcdb70, 2bcd460 and 2bcd6c0.
use crate::{
    registry::linux_type_hash,
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{collection, Definitions as Init},
    Rules, SqlValue,
};
use c6_core::{
    loaded_state::LoadedState,
    map::{Plot, PlotFlag},
};
#[derive(Debug)]
pub enum Error<E> {
    Rules(crate::Error),
    Definitions(crate::unit_initialization_definitions::Error),
    State(c6_core::loaded_state::Error),
    Missing(&'static str),
    Effect(E),
}
#[derive(Clone, Debug)]
struct Route {
    value: i32,
    present: bool,
    owned: bool,
    bridges: bool,
    era: Option<(i32, i32)>,
    unit_only: bool,
    tech: Option<u32>,
}
pub struct Definitions {
    binding: c6_core::core_player::initialization::WireBinding,
    routes: Vec<Route>,
    eras: Vec<(u32, i32)>,
    first_era: i32,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        authority: &ResourceAuthority<'_>,
    ) -> Result<Self, Error<std::convert::Infallible>> {
        Init::validate_resources(rules, authority).map_err(Error::Definitions)?;
        if authority.destination_binding() != state.binding() {
            return Err(Error::Missing("route rules binding"));
        }
        let registry = collection(rules, "Routes", "RouteType").map_err(Error::Definitions)?;
        let era_defs = collection(rules, "Eras", "EraType").map_err(Error::Definitions)?;
        let era_table = rules
            .gameplay()
            .table("Eras")
            .ok_or(Error::Missing("Eras"))?;
        let eras = era_defs
            .entries()
            .iter()
            .map(|d| {
                Ok((
                    linux_type_hash(&d.type_name),
                    era_table
                        .row_by_id(d.database_rowid)
                        .ok_or(Error::Missing("Era row"))?
                        .integer("ChronologyIndex")
                        .map_err(Error::Rules)? as i32,
                ))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let first_era = eras
            .iter()
            .enumerate()
            .min_by_key(|(_, (_, chronology))| *chronology)
            .map(|(i, _)| i as i32)
            .unwrap_or(-1);
        let table = rules
            .gameplay()
            .table("Routes")
            .ok_or(Error::Missing("Routes"))?;
        let xp = rules
            .gameplay()
            .table("Routes_XP2")
            .ok_or(Error::Missing("Routes_XP2"))?;
        let mut routes = vec![];
        for d in registry.entries() {
            let r = table
                .row_by_id(d.database_rowid)
                .ok_or(Error::Missing("Route row"))?;
            let extension = xp.rows().find(|r| {
                r.text("RouteType")
                    .ok()
                    .is_some_and(|n| n.as_bytes() == d.type_name)
            });
            let reference = |v: &SqlValue| match v {
                SqlValue::Null => Ok(None),
                SqlValue::Text(n) => Ok(Some(linux_type_hash(n))),
                _ => Err(Error::Missing("route reference")),
            };
            let era = reference(r.get("PrereqEra").map_err(Error::Rules)?)?.and_then(|h| {
                eras.iter()
                    .enumerate()
                    .find(|(_, e)| e.0 == h)
                    .map(|(i, e)| (i as i32, e.1))
            });
            routes.push(Route {
                value: r.integer("PlacementValue").map_err(Error::Rules)? as i32,
                present: r
                    .integer("PlacementRequiresRoutePresent")
                    .map_err(Error::Rules)?
                    != 0,
                owned: r
                    .integer("PlacementRequiresOwnedTile")
                    .map_err(Error::Rules)?
                    != 0,
                bridges: r.integer("SupportsBridges").map_err(Error::Rules)? != 0,
                era,
                unit_only: extension
                    .map(|r| r.integer("BuildOnlyWithUnit"))
                    .transpose()
                    .map_err(Error::Rules)?
                    .unwrap_or(0)
                    != 0,
                tech: extension
                    .map(|r| r.get("PrereqTech"))
                    .transpose()
                    .map_err(Error::Rules)?
                    .map(reference)
                    .transpose()?
                    .flatten(),
            });
        }
        Ok(Self {
            binding: state.binding().clone(),
            routes,
            eras,
            first_era,
        })
    }
    fn can_have(&self, plot: &Plot, water: bool, index: usize) -> bool {
        let r = &self.routes[index];
        allows_route(
            water,
            i32::from(plot.route_type),
            r,
            self.routes.get(plot.route_type as u8 as usize),
        )
    }
    pub fn best(
        &self,
        state: &LoadedState,
        plot: u32,
        owner: i32,
        water: bool,
    ) -> Result<i32, Error<std::convert::Infallible>> {
        if state.binding() != &self.binding {
            return Err(Error::Missing("route binding"));
        }
        if owner == -1 {
            return Ok(-1);
        }
        let Some(plot) = state.world().map().plots.get(plot as usize) else {
            return Ok(-1);
        };
        let p = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("route Player"))?;
        let current = self
            .eras
            .iter()
            .position(|e| e.0 == p.eras.era_hash)
            .ok_or(Error::Missing("current Player Era"))?;
        let plot_owner = plot
            .city_attributes
            .as_ref()
            .map_or(-1, |a| i32::from(a.player));
        let mut best = -1;
        let mut value = -1;
        for (i, r) in self.routes.iter().enumerate() {
            if r.value <= value
                || !self.can_have(plot, water, i)
                || (r.owned && plot_owner != owner)
            {
                continue;
            }
            if let Some((required, chronology)) = r.era {
                if current as i32 != required && self.eras[current].1 < chronology {
                    continue;
                }
            }
            // This entry point is the source UnitTypes=-1 path.
            if r.unit_only {
                continue;
            }
            if let Some(hash) = r.tech {
                if !p
                    .techs
                    .has_tech
                    .pairs
                    .iter()
                    .rfind(|p| p.definition_hash == hash)
                    .ok_or(Error::Missing("current route prerequisite Tech"))?
                    .value
                {
                    continue;
                }
            }
            value = r.value;
            best = i as i32;
        }
        if plot_owner != -1 {
            let level = state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == plot_owner)
                .ok_or(Error::Missing("current route plot owner"))?
                .trade
                .improved_route_level;
            if value >= 0 && level > 0 {
                let upgraded = value.wrapping_add(level);
                for (i, r) in self.routes.iter().enumerate() {
                    if !r.unit_only && r.value == upgraded {
                        best = i as i32;
                    }
                }
            }
        }
        Ok(best)
    }
}
pub trait Effects {
    type Error;
    fn water(&self, state: &LoadedState, plot: u32) -> Result<bool, Self::Error>;
    fn alive_ids(&self) -> Result<Vec<i32>, Self::Error>;
    fn visible(
        &self,
        state: &LoadedState,
        sight: &crate::unit_sight_state::Runtime,
        owner: i32,
        plot: u32,
    ) -> Result<bool, Self::Error>;
    /// Balanced output-only connectivity publication; keep synchronous consumers.
    fn report(&mut self, state: &LoadedState, plot: u32, event: u32) -> Result<(), Self::Error>;
    fn route_changed(
        &mut self,
        state: &mut LoadedState,
        plot: u32,
        route: i32,
        owner: i32,
    ) -> Result<(), Self::Error>;
}
/// Source Manager wrapper, called AFTER District.SetComplete and its signals.
/// Replacement/pillaged route continuations are explicit until supported.
pub fn complete<E: Effects>(
    state: &mut LoadedState,
    sight: &crate::unit_sight_state::Runtime,
    defs: &Definitions,
    plot: u32,
    owner: i32,
    effects: &mut E,
) -> Result<i32, Error<E::Error>> {
    let water = effects.water(state, plot).map_err(Error::Effect)?;
    let route = defs.best(state, plot, owner, water).map_err(|e| match e {
        Error::Rules(e) => Error::Rules(e),
        Error::Definitions(e) => Error::Definitions(e),
        Error::State(e) => Error::State(e),
        Error::Missing(e) => Error::Missing(e),
        Error::Effect(e) => match e {},
    })?;
    if route == -1 {
        return Ok(-1);
    }
    let old = state
        .world()
        .map()
        .plots
        .get(plot as usize)
        .ok_or(Error::Missing("route plot"))?;
    if !defs.can_have(old, water, route as usize) {
        return Ok(-1);
    }
    if i32::from(old.route_type) == route {
        return Ok(route);
    }
    if old.route_type != -1 || old.flags.get(PlotFlag::RoutePillaged) {
        return Err(Error::Missing("road replacement/pillage continuation"));
    }
    let definition = &defs.routes[route as usize];
    {
        let p = state
            .world_mut()
            .map_err(Error::State)?
            .plot_mut(plot as usize)
            .ok_or(Error::Missing("route plot"))?;
        p.route_type = route as i8;
        p.route_era = definition.era.map_or(defs.first_era, |e| e.0) as i8;
        p.flags.set(PlotFlag::RouteBridgey, definition.bridges);
    }
    if !sight.messages_disabled {
        effects
            .report(state, plot, 0x5d0f4014)
            .map_err(Error::Effect)?;
    }
    for player in effects.alive_ids().map_err(Error::Effect)? {
        if effects
            .visible(state, sight, player, plot)
            .map_err(Error::Effect)?
        {
            let visibility = state
                .world_mut()
                .map_err(Error::State)?
                .visibility_mut(player)
                .ok_or(Error::Missing("route visibility"))?;
            if let Some(layer) = visibility
                .layers
                .iter_mut()
                .find(|l| l.type_hash == 0xa8a6f95c)
            {
                if let Some(value) = layer.values.get_mut(plot as usize) {
                    *value = route as i16;
                }
            }
        }
    }
    // Native separately writes the special1000 omniscient presentation instance.
    // It has no Player identity and is not an arena gameplay visibility owner.
    if !sight.messages_disabled {
        let observer = sight
            .local_observer
            .ok_or(Error::Missing("route local observer"))?;
        let v = state
            .world()
            .visibility(observer)
            .ok_or(Error::Missing("route observer visibility"))?;
        let known = v
            .layers
            .iter()
            .find(|l| l.type_hash == 0xa8a6f95c)
            .and_then(|l| l.values.get(plot as usize))
            .copied();
        if known == Some(route as i16) {
            effects
                .report(state, plot, 0x33d8725a)
                .map_err(Error::Effect)?;
        }
    }
    effects
        .route_changed(state, plot, route, owner)
        .map_err(Error::Effect)?;
    Ok(route)
}

fn allows_route(water: bool, current: i32, route: &Route, old: Option<&Route>) -> bool {
    !water && (!route.present || current != -1) && old.is_none_or(|old| old.value <= route.value)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn route(value: i32, present: bool) -> Route {
        Route {
            value,
            present,
            owned: false,
            bridges: false,
            era: None,
            unit_only: false,
            tech: None,
        }
    }
    #[test]
    fn cannot_put_positive_route_on_water_or_skip_required_existing_road() {
        assert!(!allows_route(true, -1, &route(1, false), None));
        assert!(!allows_route(false, -1, &route(1, true), None));
        assert!(allows_route(false, -1, &route(1, false), None));
        assert!(allows_route(
            false,
            0,
            &route(2, true),
            Some(&route(1, false))
        ));
    }
    #[test]
    fn equal_placement_is_allowed_but_downgrade_is_not() {
        assert!(allows_route(
            false,
            0,
            &route(2, false),
            Some(&route(2, false))
        ));
        assert!(!allows_route(
            false,
            0,
            &route(1, false),
            Some(&route(2, false))
        ));
        // A genuine absent old definition has no placement value to reject against.
        assert!(allows_route(false, 40, &route(1, false), None));
    }
}
