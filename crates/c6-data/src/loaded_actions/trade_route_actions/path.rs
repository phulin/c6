//! Trade.Movement callbacks2c0f8c0..2c10bb5 over the exact shared search engine.
//! Portal lookup currently requires a complete empty canonical Route manager;
//! no captured paths, path lengths, validity answers or costs are accepted.
use super::{city, Error};
use crate::{registry::linux_type_hash as hash, Rules};
use c6_core::{
    city_district::DomainMask,
    loaded_state::LoadedState,
    map::{Component, Plot, PlotFlag},
    pathfinder::{NodeId, PathCallbacks, TwoLayer},
    unit::PlotCoord,
};

pub struct Definitions {
    inputs: c6_core::core_player::initialization::InputIdentities,
    terrains: Vec<(u32, bool, bool)>,
    dangers: Vec<(u32, i32)>,
    embark_districts: Vec<u32>,
    range: i32,
    land_refuel: i32,
    water_refuel: i32,
    best_route: i32,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let table = |name| rules.gameplay().table(name).ok_or(Error::Missing(name));
        let mut terrains = vec![];
        for row in table("Terrains")?.rows() {
            terrains.push((
                hash(row.text("TerrainType").map_err(Error::Rules)?.as_bytes()),
                row.integer("Water").map_err(Error::Rules)? != 0,
                row.integer("Mountain").map_err(Error::Rules)? != 0,
            ));
        }
        let mut dangers = vec![];
        for row in table("Features")?.rows() {
            dangers.push((
                hash(row.text("FeatureType").map_err(Error::Rules)?.as_bytes()),
                row.integer("DangerValue").map_err(Error::Rules)? as i32,
            ));
        }
        let mut embark_districts = vec![];
        for row in table("Districts")?.rows() {
            if row.integer("TradeEmbark").map_err(Error::Rules)? != 0 {
                embark_districts.push(hash(
                    row.text("DistrictType").map_err(Error::Rules)?.as_bytes(),
                ));
            }
        }
        let parameter = |name: &'static str| -> Result<i32, Error> {
            for row in table("GlobalParameters")?.rows() {
                if row.text("Name").map_err(Error::Rules)? == name {
                    return crate::game_parameters::sql_integer(
                        row.get("Value").map_err(Error::Rules)?,
                    )
                    .ok_or(Error::Missing(name));
                }
            }
            Err(Error::Missing(name))
        };
        let routes =
            crate::unit_initialization_definitions::collection(rules, "Routes", "RouteType")
                .map_err(|_| Error::Missing("Trade Routes registry"))?;
        let mut best_route = -1;
        let mut best_value = -1i32;
        for (index, entry) in routes.entries().iter().enumerate() {
            let value = table("Routes")?
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("Trade Route row"))?
                .integer("PlacementValue")
                .map_err(Error::Rules)? as i32;
            if value > best_value {
                best_value = value;
                best_route = index as i32;
            }
        }
        Ok(Self {
            inputs: state.binding().inputs.clone(),
            terrains,
            dangers,
            embark_districts,
            range: parameter("TRADE_ROUTE_BASE_RANGE")?,
            land_refuel: parameter("TRADE_ROUTE_LAND_RANGE_REFUEL")?,
            water_refuel: parameter("TRADE_ROUTE_WATER_RANGE_REFUEL")?,
            best_route,
        })
    }
}
/// Current configuration/capability facts, not search answers. Domains must be
/// rebuilt from current City/District/Building state by the existing production
/// placement context. Best route is required only when PathCost observes roads.
pub struct Inputs<'a> {
    pub owner: i32,
    pub major: bool,
    pub water_allowed: bool,
    pub domains: &'a [Option<DomainMask>],
    pub movement: &'a crate::unit_movement_state::Definitions,
}
struct Context<'s, 'i> {
    state: &'s LoadedState,
    definitions: &'s Definitions,
    inputs: Inputs<'i>,
    land_allowed: bool,
    range: i32,
    land_refuel: i32,
    water_refuel: i32,
    last_pair: Option<(Option<NodeId>, NodeId)>,
    moves: i32,
    refueled: bool,
}
impl Context<'_, '_> {
    fn plot(&self, at: PlotCoord) -> Result<(usize, &Plot), Error> {
        c6_core::movement_world::plot_at(self.state.world().map(), at)
            .map_err(|_| Error::Missing("Trade search plot"))
    }
    fn domain_mask(&self, index: usize, plot: &Plot) -> Result<Option<DomainMask>, Error> {
        if plot.city_attributes.is_none() {
            return Ok(None);
        }
        self.inputs
            .domains
            .get(index)
            .copied()
            .flatten()
            .map(Some)
            .ok_or(Error::Missing("current CityAttributes domain mask"))
    }
    fn terrain(&self, plot: &Plot) -> Result<(bool, bool), Error> {
        self.definitions
            .terrains
            .iter()
            .find(|e| e.0 == plot.terrain_hash)
            .map(|e| (e.1, e.2))
            .ok_or(Error::Missing("Trade Terrain definition"))
    }
    fn only_land(&self, at: PlotCoord) -> Result<bool, Error> {
        let (index, plot) = self.plot(at)?;
        if let Some(mask) = self.domain_mask(index, plot)? {
            return Ok(mask.is_valid(2) && !mask.is_valid(0));
        }
        Ok(!self.terrain(plot)?.0)
    }
    fn only_water(&self, at: PlotCoord) -> Result<bool, Error> {
        let (index, plot) = self.plot(at)?;
        if let Some(mask) = self.domain_mask(index, plot)? {
            return Ok(!mask.is_valid(2) && mask.is_valid(0));
        }
        Ok(self.terrain(plot)?.0)
    }
    fn hard_transition(&self, a: PlotCoord, b: PlotCoord) -> Result<bool, Error> {
        Ok((self.only_land(a)? && self.only_water(b)?)
            || (self.only_water(a)? && self.only_land(b)?))
    }
    fn district(&self, plot: &Plot) -> Option<&c6_core::player_districts::District> {
        let a = plot.city_attributes.as_ref()?;
        self.state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == i32::from(a.player))?
            .districts
            .records
            .entries
            .iter()
            .find(|d| d.value.id == a.district)
            .map(|d| &d.value)
    }
    fn calculate(
        &mut self,
        s: &TwoLayer,
        parent: Option<NodeId>,
        child: NodeId,
    ) -> Result<(), Error> {
        if self.last_pair == Some((parent, child)) {
            return Ok(());
        }
        let mut remaining = parent.map_or(self.range.wrapping_shl(8), |p| {
            s.node(p).moves_remaining_raw
        });
        let mut refueled = false;
        if let Some(parent) = parent {
            if remaining >= 0 {
                let a = s.node(parent).location();
                let b = s.node(child).location();
                if (self.hard_transition(a, b)? || (!self.land_allowed && self.only_land(b)?))
                    && remaining > 256
                {
                    remaining = 256;
                }
                let (_, plot) = self.plot(a)?;
                if let Some(district) = self.district(plot) {
                    if self
                        .definitions
                        .embark_districts
                        .contains(&district.district_hash)
                    {
                        let city = self
                            .state
                            .city(
                                i32::from(plot.city_attributes.as_ref().unwrap().player),
                                district.city_id,
                            )
                            .map_err(Error::State)?;
                        let header = &city.header;
                        let is_source =
                            (header.location_x, header.location_y) == (s.source.x, s.source.y);
                        let is_destination = (header.location_x, header.location_y)
                            == (s.destination.x, s.destination.y);
                        let constructed = usize::try_from(self.inputs.owner)
                            .ok()
                            .and_then(|i| city.trade.constructed_trading_posts.get(i))
                            .copied()
                            .unwrap_or(false);
                        let active = if constructed {
                            !self
                                .state
                                .game_relations()
                                .map_err(Error::State)?
                                .is_at_war(header.owner, self.inputs.owner)
                                .map_err(|_| {
                                    Error::Missing("Trade trading-post war relationship")
                                })?
                        } else {
                            false
                        };
                        let refill = if active || is_source {
                            if self.only_land(b)? {
                                self.land_refuel
                            } else if self.only_water(b)? {
                                self.water_refuel
                            } else {
                                self.land_refuel.max(self.water_refuel)
                            }
                        } else if is_destination {
                            3
                        } else {
                            0
                        };
                        if refill >= 1 {
                            remaining = refill.wrapping_shl(8);
                            refueled = true;
                        }
                    }
                }
            }
        }
        self.moves = remaining.wrapping_sub(256);
        self.refueled = refueled;
        self.last_pair = Some((parent, child));
        Ok(())
    }
}
impl PathCallbacks for Context<'_, '_> {
    type Error = Error;
    fn initialize(&mut self, _: &mut TwoLayer) -> Result<(), Error> {
        Ok(())
    }
    fn uninitialize(&mut self, _: &mut TwoLayer) -> Result<(), Error> {
        Ok(())
    }
    fn destination_valid(&mut self, _: &mut TwoLayer) -> Result<bool, Error> {
        Ok(true)
    }
    fn is_destination(&mut self, s: &mut TwoLayer, at: PlotCoord) -> Result<bool, Error> {
        Ok(at == s.destination)
    }
    fn heuristic(&mut self, s: &mut TwoLayer, at: PlotCoord, _: i32) -> Result<i32, Error> {
        let h = &self.state.world().map().header;
        Ok(c6_core::geometry::plot_distance(
            h.width,
            h.wrap_x,
            at.x,
            at.y,
            s.destination.x,
            s.destination.y,
        )
        .wrapping_mul(100))
    }
    fn valid(
        &mut self,
        s: &mut TwoLayer,
        parent: Option<NodeId>,
        child: NodeId,
    ) -> Result<i32, Error> {
        let at = s.node(child).location();
        let (index, plot) = self.plot(at)?;
        let (water, mountain) = self.terrain(plot)?;
        let impassable = plot.flags.get(PlotFlag::Impassable) && !mountain;
        let revealed = match self
            .state
            .world()
            .visibility(self.inputs.owner)
            .and_then(|v| v.revealed.get(index))
        {
            Some(0) => false,
            Some(1) => true,
            _ => return Err(Error::Missing("Trade path revealed byte")),
        };
        let danger = self
            .definitions
            .dangers
            .iter()
            .find(|e| e.0 == plot.feature_hash)
            .map_or(0, |e| e.1);
        let danger = danger > 0 && at.x != s.destination.x && at.y != s.destination.y;
        let mut foreign = 0;
        if let Some(city) = city::<()>(self.state, at)? {
            if i32::from(city.owner) != self.inputs.owner {
                let war = self
                    .state
                    .game_relations()
                    .map_err(Error::State)?
                    .is_at_war(self.inputs.owner, i32::from(city.owner))
                    .map_err(|_| Error::Missing("Trade foreign-city war relationship"))?;
                foreign = if war { 0x400 } else { 0x200 };
            }
        }
        let flags = &mut s.node_mut(child).cache_flags;
        *flags = (*flags & !0x1071e)
            | 2
            | (u32::from(mountain) << 2)
            | (u32::from(impassable) << 3)
            | (u32::from(water) << 4)
            | (u32::from(revealed) << 8)
            | foreign
            | (u32::from(danger) << 16);
        let Some(parent) = parent else {
            return Ok(1);
        };
        if impassable || foreign == 0x400 || danger || mountain || (!revealed && self.inputs.major)
        {
            return Ok(0);
        }
        if !self.inputs.water_allowed && self.only_water(at)? {
            return Ok(0);
        }
        let from = s.node(parent).location();
        if self.hard_transition(from, at)? {
            let (_, parent_plot) = self.plot(from)?;
            let can_embark = self
                .district(parent_plot)
                .is_some_and(|d| self.definitions.embark_districts.contains(&d.district_hash))
                || self
                    .district(plot)
                    .is_some_and(|d| self.definitions.embark_districts.contains(&d.district_hash));
            if !can_embark {
                return Ok(0);
            }
        }
        self.calculate(s, Some(parent), child)?;
        Ok(i32::from(self.moves >= 0))
    }
    fn cost(&mut self, s: &mut TwoLayer, parent: NodeId, child: NodeId) -> Result<i32, Error> {
        let from = s.node(parent).location();
        let to = s.node(child).location();
        self.calculate(s, Some(parent), child)?;
        let mut cost = 100i32;
        if !self.refueled && self.hard_transition(from, to)? {
            cost = cost.wrapping_add(10000);
        }
        let (index, plot) = self.plot(to)?;
        if self.domain_mask(index, plot)?.is_some_and(|m| m.0 & 5 == 5)
            || city::<()>(self.state, to)?.is_some()
        {
            return Ok(cost.wrapping_shl(8));
        }
        if plot.route_type != -1 {
            let best = self.definitions.best_route;
            cost = cost.wrapping_add(if best != -1 && best == i32::from(plot.route_type) {
                10
            } else {
                50
            });
        } else if self.terrain(plot)?.0 {
            cost = cost.wrapping_add(50);
        } else {
            let movement = crate::unit_movement_state::plot_movement_cost(
                self.state,
                plot,
                self.inputs.movement,
            )
            .map_err(|_| Error::Missing("Trade plot movement cost"))?;
            if movement == i32::MAX {
                return Ok(i32::MAX.wrapping_shl(8));
            }
            if movement > 0 {
                cost = cost.wrapping_add(movement.wrapping_mul(100));
            }
        }
        Ok(cost.wrapping_shl(8))
    }
    fn add(
        &mut self,
        s: &mut TwoLayer,
        parent: Option<NodeId>,
        child: NodeId,
        event: i32,
    ) -> Result<(), Error> {
        if event != 5 {
            if let Some(parent) = parent {
                if s.node(parent).location() == s.source {
                    s.node_mut(parent).moves_remaining_raw = self.range.wrapping_shl(8);
                }
            }
            self.calculate(s, parent, child)?;
            s.node_mut(child).moves_remaining_raw = self.moves;
        }
        Ok(())
    }
    fn node_add(
        &mut self,
        _: &mut TwoLayer,
        _: Option<NodeId>,
        _: NodeId,
        _: i32,
    ) -> Result<(), Error> {
        Ok(())
    }
    fn extra_children_count(&mut self, _: &mut TwoLayer, _: NodeId) -> Result<i32, Error> {
        Ok(0)
    }
    fn extra_child(&mut self, _: &mut TwoLayer, _: NodeId, _: i32) -> Result<PlotCoord, Error> {
        Err(Error::Requires("Trade portal exit"))
    }
}

pub fn find(
    state: &LoadedState,
    definitions: &Definitions,
    inputs: Inputs<'_>,
    search: &mut TwoLayer,
    source: PlotCoord,
    destination: PlotCoord,
) -> Result<Option<Vec<i32>>, Error> {
    if definitions.inputs != state.binding().inputs {
        return Err(Error::Binding);
    }
    let mut routes = state.world().map().components.iter().filter_map(|c| {
        if let Component::Route(r) = c {
            Some(r)
        } else {
            None
        }
    });
    let routes = routes
        .next()
        .filter(|_| routes.next().is_none())
        .ok_or(Error::Missing("canonical Route manager"))?;
    if !routes.portals.is_empty() {
        return Err(Error::Requires("Trade portal area lookup"));
    }
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == inputs.owner)
        .ok_or(Error::Missing("Trade path Player"))?;
    let range = definitions
        .range
        .wrapping_add(player.trade.extra_route_range);
    let land_allowed = player.trade.forbid_land_route_domestic <= 0;
    let mut context = Context {
        state,
        definitions,
        range: if land_allowed { range } else { 3 },
        land_refuel: if land_allowed {
            definitions
                .land_refuel
                .wrapping_add(player.trade.extra_land_range_refuel)
        } else {
            3
        },
        water_refuel: definitions
            .water_refuel
            .wrapping_add(player.trade.extra_water_range_refuel),
        inputs,
        land_allowed,
        last_pair: None,
        moves: -256,
        refueled: false,
    };
    let found = search.generate(&mut context, source, destination, 0, false)?;
    if !found {
        return Ok(None);
    }
    let width = state.world().map().header.width;
    Ok(Some(
        search
            .copy_path()
            .into_iter()
            .rev()
            .map(|n| {
                i32::from(n.y)
                    .wrapping_mul(width)
                    .wrapping_add(i32::from(n.x))
            })
            .collect(),
    ))
}
