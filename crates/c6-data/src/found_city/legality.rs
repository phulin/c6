//! Current canonical FoundCity handler and stock placement queries. Outer
//! Manager/script gates remain the ordinary loaded-actions query machinery.
use super::Definitions;
use crate::{registry::linux_type_hash, Rules, SqlValue};
use c6_core::{
    found_city::{self, placement},
    loaded_state::LoadedState,
    map::{Plot, PlotFlag},
    unit::{ComponentId, PlotCoord, Unit},
};
#[derive(Debug)]
pub enum Error {
    Definitions(super::Error),
    Rules(crate::Error),
    Missing(&'static str),
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    ProbeLimit,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FoundCity legality: {self:?}")
    }
}
impl std::error::Error for Error {}
pub struct Context<'a> {
    pub state: &'a LoadedState,
    pub definitions: &'a Definitions,
    pub rules: &'a Rules,
    pub unit: &'a Unit,
    pub plot: Option<usize>,
    pub max_probes: usize,
}
impl Context<'_> {
    fn at(&self, p: usize) -> Result<&Plot, Error> {
        self.state
            .world()
            .map()
            .plots
            .get(p)
            .ok_or(Error::Missing("current plot"))
    }
    fn coord(&self, p: usize) -> Result<PlotCoord, Error> {
        let (x, y) = self
            .state
            .world()
            .map()
            .coordinates(p as u32)
            .ok_or(Error::Missing("current plot coordinate"))?;
        Ok(PlotCoord { x, y })
    }
    fn feature(&self, p: usize) -> Result<i32, Error> {
        let hash = self.at(p)?.feature_hash;
        if hash == u32::MAX {
            return Ok(-1);
        }
        self.definitions
            .features
            .find_index(hash)
            .and_then(|v| i32::try_from(v).ok())
            .ok_or(Error::Missing("current feature definition"))
    }
    fn city_at(&self, p: usize) -> Result<Option<ComponentId>, Error> {
        let Some(attrs) = &self.at(p)?.city_attributes else {
            return Ok(None);
        };
        if attrs.player == -1 || attrs.district == u32::MAX {
            return Ok(None);
        }
        let owner = i32::from(attrs.player);
        self.state.world().player(owner).map_err(Error::World)?;
        let player = self
            .state
            .retained()
            .players
            .iter()
            .find(|v| v.header.owner == owner)
            .ok_or(Error::Missing("current District owner"))?;
        let Some(d) = player
            .districts
            .records
            .entries
            .iter()
            .find(|e| e.value.id == attrs.district)
            .map(|e| &e.value)
        else {
            return Ok(None);
        };
        if d.district_hash != c6_core::city_district::CITY_CENTER_HASH {
            return Ok(None);
        }
        Ok(player
            .cities
            .records
            .entries
            .iter()
            .find(|e| e.value.header.id == d.city_id)
            .map(|e| ComponentId {
                owner: e.value.header.owner as i16,
                component_type: 2,
                id: e.value.header.id,
            }))
    }
}
impl found_city::CanStartContext for Context<'_> {
    type Error = Error;
    type Plot = usize;
    fn settling_disabled(&mut self) -> Result<bool, Error> {
        self.definitions
            .settling_disabled(self.state, self.unit.e_owner)
            .map_err(Error::Definitions)
    }
    fn unit_definition_found_city(&mut self) -> Result<Option<bool>, Error> {
        let r = self
            .state
            .unit_runtime(self.unit.e_owner, self.unit.id)
            .map_err(Error::State)?;
        self.definitions
            .unit_found_city(self.state, r.definition_index as u32)
            .map_err(Error::Definitions)
    }
    fn unit_moves_remaining_q8(&mut self) -> Result<i32, Error> {
        Ok(self.unit.x_moves_remaining)
    }
    fn plot(&mut self) -> Result<Option<usize>, Error> {
        Ok(self
            .plot
            .filter(|p| *p < self.state.world().map().plots.len()))
    }
    fn owner(&mut self) -> Result<i32, Error> {
        Ok(self.unit.e_owner)
    }
    fn valid_found_location(&mut self, owner: i32, p: usize) -> Result<bool, Error> {
        let location = self.coord(p)?;
        placement::is_valid(self, owner, location, self.max_probes).map_err(|e| match e {
            placement::Error::Context(e) => e,
            placement::Error::ProbeLimit => Error::ProbeLimit,
        })
    }
    fn feature_type(&mut self, p: usize) -> Result<i32, Error> {
        self.feature(p)
    }
    fn feature_definition(&mut self, kind: i32) -> Result<Option<found_city::Feature>, Error> {
        self.definitions
            .feature_find(self.state, kind as u32)
            .map(|x| {
                x.map(|f| found_city::Feature {
                    removable: f.removable,
                })
            })
            .map_err(Error::Definitions)
    }
    fn removable_feature_output(&mut self, _: i32) -> Result<(), Error> {
        Err(Error::Missing(
            "optional UI output map is not requested by this entry",
        ))
    }
}
impl placement::Context for Context<'_> {
    type Error = Error;
    type Plot = usize;
    fn map_exists(&mut self) -> Result<bool, Error> {
        Ok(true)
    }
    fn plot(&mut self, location: PlotCoord) -> Result<Option<usize>, Error> {
        Ok(
            c6_core::movement_world::plot_at(self.state.world().map(), location)
                .ok()
                .map(|v| v.0),
        )
    }
    fn impassable(&mut self, p: usize) -> Result<bool, Error> {
        Ok(self.at(p)?.flags.get(PlotFlag::Impassable))
    }
    fn feature_type(&mut self, p: usize) -> Result<i32, Error> {
        self.feature(p)
    }
    fn feature_allows_settlement(&mut self, kind: i32) -> Result<bool, Error> {
        self.definitions
            .feature_at(self.state, kind as u32)
            .map_err(Error::Definitions)?
            .map(|v| v.settlement)
            .ok_or(Error::Missing("direct feature definition"))
    }
    fn district(&mut self, p: usize) -> Result<bool, Error> {
        let Some(a) = &self.at(p)?.city_attributes else {
            return Ok(false);
        };
        if a.player == -1 || a.district == u32::MAX {
            return Ok(false);
        }
        let owner = i32::from(a.player);
        self.state.world().player(owner).map_err(Error::World)?;
        let player = self
            .state
            .retained()
            .players
            .iter()
            .find(|v| v.header.owner == owner)
            .ok_or(Error::Missing("current District owner"))?;
        Ok(player
            .districts
            .records
            .entries
            .iter()
            .any(|d| d.value.id == a.district))
    }
    fn player_id(&mut self, owner: i32) -> Result<i32, Error> {
        Ok(self
            .state
            .world()
            .player(owner)
            .map_err(Error::World)?
            .owner())
    }
    fn owned(&mut self, p: usize) -> Result<bool, Error> {
        Ok(self
            .at(p)?
            .city_attributes
            .as_ref()
            .is_some_and(|a| a.player != -1))
    }
    fn owner(&mut self, p: usize) -> Result<i32, Error> {
        Ok(self
            .at(p)?
            .city_attributes
            .as_ref()
            .map_or(-1, |a| i32::from(a.player)))
    }
    fn water(&mut self, p: usize) -> Result<bool, Error> {
        let hash = self.at(p)?.terrain_hash;
        let table = self
            .rules
            .gameplay()
            .table("Terrains")
            .ok_or(Error::Missing("Terrains"))?;
        let row = table
            .rows()
            .find(|r| {
                r.text("TerrainType")
                    .is_ok_and(|s| linux_type_hash(s.as_bytes()) == hash)
            })
            .ok_or(Error::Missing("current terrain definition"))?;
        match row.get("Water").map_err(Error::Rules)? {
            SqlValue::Integer(0) => Ok(false),
            SqlValue::Integer(1) => Ok(true),
            _ => Err(Error::Missing("Terrain Water boolean")),
        }
    }
    fn minimum_city_range(&mut self) -> Result<i32, Error> {
        let table = self
            .rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or(Error::Missing("GlobalParameters"))?;
        let mut rows = table
            .rows()
            .filter(|r| r.text("Name").ok() == Some("CITY_MIN_RANGE"));
        let row = rows.next().ok_or(Error::Missing(
            "source absent CITY_MIN_RANGE fallback not supplied",
        ))?;
        if rows.next().is_some() {
            return Err(Error::Missing("ambiguous CITY_MIN_RANGE"));
        }
        crate::game_parameters::sql_integer(row.get("Value").map_err(Error::Rules)?)
            .ok_or(Error::Missing("CITY_MIN_RANGE integer"))
    }
    fn location(&mut self, p: usize) -> Result<PlotCoord, Error> {
        self.coord(p)
    }
    fn same_landmass(&mut self, a: usize, b: usize) -> Result<bool, Error> {
        Ok(self.at(a)?.landmass_id == self.at(b)?.landmass_id)
    }
    fn plot_from_delta(
        &mut self,
        center: PlotCoord,
        dx: i32,
        dy: i32,
        range: i32,
    ) -> Result<Option<usize>, Error> {
        placement::plot_from_delta(self.state.world().map(), center, dx, dy, range)
            .map_err(|_| Error::Missing("initialized map geometry"))
    }
    fn city(&mut self, p: usize) -> Result<bool, Error> {
        Ok(self.city_at(p)?.is_some())
    }
    fn city_identity(&mut self, p: usize) -> Result<ComponentId, Error> {
        self.city_at(p)?
            .ok_or(Error::Missing("current City identity"))
    }
    fn plot_distance(&mut self, a: PlotCoord, b: PlotCoord) -> Result<i32, Error> {
        let m = self.state.world().map();
        Ok(c6_core::geometry::plot_distance(
            m.header.width,
            m.header.wrap_x,
            a.x,
            a.y,
            b.x,
            b.y,
        ))
    }
}
