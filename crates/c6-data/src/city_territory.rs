//! Live city territory transitions over canonical Map and City records.
//! Unsaved plot attributes and PlotYields objects remain explicit current state.
use crate::{
    loaded_actions::city_range_attack, registry::linux_type_hash, unit_sight_state, Rules,
};
use c6_core::{
    city_territory::{self as kernel, plot_yields, CityId},
    loaded_state::{self, LoadedState},
    map::{CityAttributes, Plot, PlotFlag},
    unit::{ComponentId, PlotCoord},
};

#[derive(Debug)]
pub enum Error<E> {
    State(loaded_state::Error),
    World(c6_core::world::MissingInput),
    Missing(&'static str),
    Rules(crate::Error),
    Sight(c6_core::visibility_change::ChangeError<unit_sight_state::Error<E>>),
    Effect(E),
    PlotYields(plot_yields::Error<E>),
    Random(loaded_state::game_sync::AccessError),
}
/// Only fields actually present in current native CityAttributes; existing
/// attributes require captured values, newly initialized attributes start at 0.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlotRuntime {
    pub plot: u32,
    pub valid_domains: u16,
    pub yield_effect: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct PlayerInput {
    pub owner: i32,
    pub civilization_level: Option<u32>,
    pub modified_city_tiles: Option<i32>,
}
pub struct Runtime {
    pub attributes: Vec<PlotRuntime>,
    pub plot_yields: plot_yields::Runtime,
    pub players: Vec<PlayerInput>,
    pub start_era: Option<u32>,
    /// Actual Manager.GetAliveMajorIDs cached order, not an alive-player filter.
    pub alive_major_ids: Option<Vec<i32>>,
}
pub struct Definitions {
    binding: c6_core::core_player::initialization::WireBinding,
    levels: Vec<(u32, i32)>,
    eras: Vec<(u32, i32)>,
    city_centers: Vec<u32>,
    improvements: crate::registry::LinuxDefinitionRegistry,
    districts: crate::registry::LinuxDefinitionRegistry,
    capturable: Vec<(u32, bool)>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &crate::resource_binding::ResourceAuthority<'_>,
    ) -> Result<Self, Error<std::convert::Infallible>> {
        if state.binding() != bound.destination_binding() {
            return Err(Error::Missing("territory resource binding"));
        }
        crate::unit_initialization_definitions::Definitions::validate_resources(rules, bound)
            .map_err(|_| Error::Missing("territory initialized definitions"))?;
        let table = |name| {
            rules
                .gameplay()
                .table(name)
                .ok_or(Error::Missing("territory rules table"))
        };
        let mut levels = vec![];
        for row in table("CivilizationLevels")?.rows() {
            levels.push((
                linux_type_hash(
                    row.text("CivilizationLevelType")
                        .map_err(Error::Rules)?
                        .as_bytes(),
                ),
                row.integer("StartingTilesForCity").map_err(Error::Rules)? as i32,
            ));
        }
        let mut eras = vec![];
        for row in table("StartEras")?.rows() {
            eras.push((
                linux_type_hash(row.text("EraType").map_err(Error::Rules)?.as_bytes()),
                row.integer("Tiles").map_err(Error::Rules)? as i32,
            ));
        }
        let mut city_centers = vec![];
        for row in table("Districts")?.rows() {
            if row.integer("CityCenter").map_err(Error::Rules)? != 0 {
                city_centers.push(linux_type_hash(
                    row.text("DistrictType").map_err(Error::Rules)?.as_bytes(),
                ));
            }
        }
        let mut capturable = vec![];
        for row in table("Improvements")?.rows() {
            capturable.push((
                linux_type_hash(
                    row.text("ImprovementType")
                        .map_err(Error::Rules)?
                        .as_bytes(),
                ),
                row.integer("Capturable").map_err(Error::Rules)? != 0,
            ));
        }
        Ok(Self {
            binding: state.binding().clone(),
            levels,
            eras,
            city_centers,
            capturable,
            districts: crate::unit_initialization_definitions::collection(
                rules,
                "Districts",
                "DistrictType",
            )
            .map_err(|_| Error::Missing("territory district registry"))?,
            improvements: crate::unit_initialization_definitions::collection(
                rules,
                "Improvements",
                "ImprovementType",
            )
            .map_err(|_| Error::Missing("territory improvement registry"))?,
        })
    }
}
/// Required synchronous leaves. Canonical ownership, attributes, yield object
/// storage, visibility and Culture counters are performed by this adapter.
pub trait Effects: unit_sight_state::Effects {
    fn influence_token_received(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        city: CityId,
        player: i32,
        tokens: i32,
    ) -> Result<(), Self::Error>;
    fn owner_report(
        &mut self,
        state: &mut LoadedState,
        plot: u32,
        old_owner: i32,
    ) -> Result<(), Self::Error>;
    fn owner_signal(
        &mut self,
        state: &mut LoadedState,
        plot: u32,
        new_city: CityId,
        old_city: CityId,
    ) -> Result<(), Self::Error>;
    fn plot_yields_created(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        receiver: u64,
        id: u32,
    ) -> Result<(), Self::Error>;
    fn plot_yields_destroyed(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        object: plot_yields::Object,
    ) -> Result<(), Self::Error>;
    fn coastal_land_one(&mut self, state: &LoadedState, plot: u32) -> Result<bool, Self::Error>;
    fn set_improvement(
        &mut self,
        state: &mut LoadedState,
        plot: u32,
        kind: i32,
        owner: i32,
    ) -> Result<(), Self::Error>;
    fn expel_from_plot(
        &mut self,
        state: &mut LoadedState,
        location: PlotCoord,
    ) -> Result<(), Self::Error>;
    fn activate_improvement_without_unit(
        &mut self,
        state: &mut LoadedState,
        plot: u32,
    ) -> Result<(), Self::Error>;
    fn reallocate_citizens(
        &mut self,
        state: &mut LoadedState,
        city: CityId,
    ) -> Result<(), Self::Error>;
    fn annex_report(
        &mut self,
        state: &mut LoadedState,
        city: CityId,
        location: PlotCoord,
    ) -> Result<(), Self::Error>;
    fn annex_signal(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: CityId,
        plot: u32,
        old_city: CityId,
    ) -> Result<(), Self::Error>;
    /// Exact ordered GetBuyablePlotList, before this adapter's one sync draw.
    fn buyable_plots(&mut self, state: &LoadedState, city: CityId)
        -> Result<Vec<u32>, Self::Error>;
}
pub struct Context<'a, E> {
    pub state: &'a mut LoadedState,
    pub runtime: &'a mut Runtime,
    pub sight: &'a mut unit_sight_state::Runtime,
    pub sight_definitions: &'a city_range_attack::Definitions,
    pub definitions: &'a Definitions,
    pub effects: &'a mut E,
    pub city: CityId,
    /// Real current actor for the shared visibility adapter. Visibility change
    /// does not query this actor; the native source argument remains NULL CID.
    pub actor: (i32, u32),
}
impl<E: Effects> Context<'_, E> {
    fn validate(&self) -> Result<(), Error<E::Error>> {
        if &self.definitions.binding != self.state.binding() {
            return Err(Error::Missing("territory definitions binding"));
        }
        Ok(())
    }
    fn plot(&self, index: u32) -> Result<&Plot, Error<E::Error>> {
        let p = self
            .state
            .world()
            .map()
            .plot_by_index(index)
            .ok_or(Error::Missing("territory plot"))?;
        if p.flags.get(PlotFlag::HasCityAttributes) != p.city_attributes.is_some() {
            return Err(Error::Missing("territory city attribute presence"));
        }
        Ok(p)
    }
    fn plot_mut(&mut self, index: u32) -> Result<&mut Plot, Error<E::Error>> {
        self.state
            .world_mut()
            .map_err(Error::State)?
            .plot_mut(index as usize)
            .ok_or(Error::Missing("territory plot"))
    }
    fn index(&self, location: PlotCoord) -> Result<Option<u32>, Error<E::Error>> {
        crate::unit_placement_services::plot_index_at::<E::Error>(self.state, location)
            .map_err(|_| Error::Missing("territory map geometry"))
    }
    fn location(&self, index: u32) -> Result<PlotCoord, Error<E::Error>> {
        self.plot(index)?;
        let width = self.state.world().map().header.width;
        if width <= 0 {
            return Err(Error::Missing("territory map width"));
        }
        Ok(PlotCoord {
            x: index as i32 % width,
            y: index as i32 / width,
        })
    }
    fn attributes(&mut self, index: u32) -> Result<(), Error<E::Error>> {
        if self.plot(index)?.city_attributes.is_none() {
            let p = self.plot_mut(index)?;
            p.city_attributes = Some(CityAttributes {
                working_city: u32::MAX,
                owning_city: u32::MAX,
                district: u32::MAX,
                player: -1,
                wonder_building_hash: u32::MAX,
            });
            p.flags.set(PlotFlag::HasCityAttributes, true);
            self.runtime.attributes.retain(|r| r.plot != index);
            self.runtime.attributes.push(PlotRuntime {
                plot: index,
                valid_domains: 0,
                yield_effect: 0,
            });
        }
        Ok(())
    }
    fn live(&mut self, index: u32) -> Result<&mut PlotRuntime, Error<E::Error>> {
        let mut rows = self
            .runtime
            .attributes
            .iter_mut()
            .filter(|r| r.plot == index);
        let row = rows
            .next()
            .ok_or(Error::Missing("current CityAttributes runtime"))?;
        if rows.next().is_some() {
            return Err(Error::Missing("duplicate CityAttributes runtime"));
        }
        Ok(row)
    }
    /// Complete source DoFoundCity order: two starting-count reads on the
    /// nonnegative path, acquisition, then actual cached-major influence reads.
    pub fn do_found_city(&mut self) -> Result<(), Error<E::Error>> {
        self.validate()?;
        let count = if self.starting_plots()? < 0 {
            0
        } else {
            self.starting_plots()?
        };
        self.acquire_plots(count, true)?;
        let owner = self
            .state
            .city(self.city.owner, self.city.id)
            .map_err(Error::State)?
            .header
            .owner;
        let count = self
            .runtime
            .alive_major_ids
            .as_ref()
            .ok_or(Error::Missing("current GetAliveMajorIDs"))?
            .len();
        for index in 0..count {
            let player = *self
                .runtime
                .alive_major_ids
                .as_ref()
                .and_then(|v| v.get(index))
                .ok_or(Error::Missing("retained GetAliveMajorIDs cursor"))?;
            let tokens = |state: &LoadedState| -> Result<i32, Error<E::Error>> {
                let influence = &state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == owner)
                    .ok_or(Error::Missing("current owning player Influence"))?
                    .influence;
                usize::try_from(player)
                    .ok()
                    .and_then(|i| influence.tokens_received.get(i))
                    .copied()
                    .ok_or(Error::Missing("current Influence.GetTokensReceived"))
            };
            if tokens(self.state)? > 0 {
                let amount = tokens(self.state)?;
                self.effects
                    .influence_token_received(self.state, self.runtime, self.city, player, amount)
                    .map_err(Error::Effect)?;
            }
        }
        Ok(())
    }
    pub fn starting_plots(&mut self) -> Result<i32, Error<E::Error>> {
        self.validate()?;
        kernel::starting_plots(self)
    }
    pub fn acquire_plots(&mut self, count: i32, initial_ring: bool) -> Result<(), Error<E::Error>> {
        self.validate()?;
        kernel::acquire_plots(self, count, initial_ring)
    }
    pub fn set_purchased_by_city(
        &mut self,
        location: PlotCoord,
        city: CityId,
    ) -> Result<(), Error<E::Error>> {
        self.validate()?;
        kernel::set_purchased_by_city(self, location, city.owner, city.id)
    }
}
impl<E: Effects> kernel::StartingContext for Context<'_, E> {
    type Error = Error<E::Error>;
    fn city_present(&mut self) -> Result<bool, Self::Error> {
        self.city_exists(self.city)
    }
    fn owner(&mut self) -> Result<i32, Self::Error> {
        Ok(self
            .state
            .city(self.city.owner, self.city.id)
            .map_err(Error::State)?
            .header
            .owner)
    }
    fn level_starting_tiles(&mut self, owner: i32) -> Result<Option<i32>, Self::Error> {
        let level = self
            .runtime
            .players
            .iter()
            .find(|r| r.owner == owner)
            .and_then(|r| r.civilization_level)
            .ok_or(Error::Missing("current civilization level"))?;
        Ok(self
            .definitions
            .levels
            .iter()
            .find(|r| r.0 == level)
            .map(|r| r.1))
    }
    fn modified_city_tiles(&mut self, owner: i32) -> Result<i32, Self::Error> {
        self.runtime
            .players
            .iter()
            .find(|r| r.owner == owner)
            .and_then(|r| r.modified_city_tiles)
            .ok_or(Error::Missing("current modified city tiles"))
    }
    fn is_major(&mut self, owner: i32) -> Result<bool, Self::Error> {
        Ok(self
            .runtime
            .players
            .iter()
            .find(|r| r.owner == owner)
            .and_then(|r| r.civilization_level)
            .ok_or(Error::Missing("current civilization level"))?
            == 0x253718b0)
    }
    fn era_starting_tiles(&mut self) -> Result<i32, Self::Error> {
        let era = self
            .runtime
            .start_era
            .ok_or(Error::Missing("current start era"))?;
        Ok(self
            .definitions
            .eras
            .iter()
            .find(|r| r.0 == era)
            .map_or(0, |r| r.1))
    }
}
impl<E: Effects> kernel::OwnerContext for Context<'_, E> {
    type Error = Error<E::Error>;
    type Plot = u32;
    fn valid_player(&mut self, owner: i32) -> Result<bool, Self::Error> {
        self.state
            .world()
            .player_is_valid(owner)
            .map_err(Error::World)
    }
    fn plot_owner(&mut self, plot: u32) -> Result<i32, Self::Error> {
        Ok(self
            .plot(plot)?
            .city_attributes
            .as_ref()
            .map_or(-1, |a| i32::from(a.player)))
    }
    fn owning_city(&mut self, plot: u32) -> Result<CityId, Self::Error> {
        Ok(self
            .plot(plot)?
            .city_attributes
            .as_ref()
            .map_or(CityId::NONE, |a| CityId {
                owner: i32::from(a.player),
                id: a.owning_city,
            }))
    }
    fn set_owning_city(&mut self, plot: u32, city: CityId) -> Result<(), Self::Error> {
        self.attributes(plot)?;
        let a = self
            .plot_mut(plot)?
            .city_attributes
            .as_mut()
            .expect("initialized");
        a.player = city.owner as i8;
        a.owning_city = city.id;
        Ok(())
    }
    fn remove_city_attributes(&mut self, plot: u32) -> Result<(), Self::Error> {
        let p = self.plot_mut(plot)?;
        p.city_attributes = None;
        p.flags.set(PlotFlag::HasCityAttributes, false);
        self.runtime.attributes.retain(|r| r.plot != plot);
        Ok(())
    }
    fn adjacent_plot(&mut self, plot: u32, direction: u8) -> Result<Option<u32>, Self::Error> {
        let from = self.location(plot)?;
        const XY: [(i32, i32); 6] = [(0, 1), (1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1)];
        let (dx, dy) = XY
            .get(direction as usize)
            .copied()
            .ok_or(Error::Missing("Map direction"))?;
        let half = |y: i32| {
            if y >= 0 {
                y >> 1
            } else {
                let n = y.wrapping_sub(1);
                n.wrapping_add(((n as u32) >> 31) as i32) >> 1
            }
        };
        let y = from.y.wrapping_add(dy);
        let x = from
            .x
            .wrapping_sub(half(from.y))
            .wrapping_add(dx)
            .wrapping_add(half(y));
        self.index(PlotCoord { x, y })
    }
    fn change_owner_visibility(
        &mut self,
        owner: i32,
        plot: u32,
        delta: i32,
    ) -> Result<(), Self::Error> {
        let mut context = unit_sight_state::Context {
            state: self.state,
            runtime: self.sight,
            effects: self.effects,
            sight_definitions: self.sight_definitions,
            actor: self.actor,
        };
        c6_core::visibility_change::change_visibility_count(
            &mut context,
            c6_core::unit_sight::VisibilityChange {
                owner,
                plot_index: plot,
                delta,
                hidden_type: -1,
                source_flag: true,
                near_origin: false,
                origin_index: -1,
                sight_range: 1,
                source: ComponentId {
                    owner: -1,
                    component_type: 0,
                    id: u32::MAX,
                },
            },
        )
        .map(|_| ())
        .map_err(Error::Sight)
    }
    fn report_owner_changed(&mut self, plot: u32, old_owner: i32) -> Result<(), Self::Error> {
        self.effects
            .owner_report(self.state, plot, old_owner)
            .map_err(Error::Effect)
    }
    fn signal_owner_changed(
        &mut self,
        plot: u32,
        new_city: CityId,
        old_city: CityId,
    ) -> Result<(), Self::Error> {
        self.effects
            .owner_signal(self.state, plot, new_city, old_city)
            .map_err(Error::Effect)
    }
}
use kernel::{OwnerContext, PurchasedContext};
impl<E: Effects> kernel::PurchasedContext for Context<'_, E> {
    fn plot_at(&mut self, location: PlotCoord) -> Result<Option<u32>, Self::Error> {
        self.index(location)
    }
    fn city_exists(&mut self, city: CityId) -> Result<bool, Self::Error> {
        match self.state.city(city.owner, city.id) {
            Ok(_) => Ok(true),
            Err(loaded_state::Error::Missing(loaded_state::MissingInput::City { .. })) => Ok(false),
            Err(e) => Err(Error::State(e)),
        }
    }
    fn add_plot_yields(
        &mut self,
        plot: u32,
        city: CityId,
        improvement: i32,
    ) -> Result<u32, Self::Error> {
        plot_yields::add(
            self,
            plot_yields::Object::new(plot, Some(city), improvement),
        )
        .map_err(Error::PlotYields)
    }
    fn plot_yield_effect(&mut self, plot: u32) -> Result<u32, Self::Error> {
        self.attributes(plot)?;
        Ok(self.live(plot)?.yield_effect)
    }
    fn remove_plot_yields(&mut self, id: u32) -> Result<(), Self::Error> {
        plot_yields::remove(self, id).map_err(Error::PlotYields)
    }
    fn set_plot_yield_effect(&mut self, plot: u32, id: u32) -> Result<(), Self::Error> {
        self.live(plot)?.yield_effect = id;
        Ok(())
    }
    fn set_valid_domains(&mut self, plot: u32) -> Result<(), Self::Error> {
        self.attributes(plot)?;
        self.live(plot)?.valid_domains = 0;
        let water = self
            .effects
            .water(self.state, plot)
            .map_err(Error::Effect)?;
        let city = if water {
            false
        } else if let Some(a) = self.plot(plot)?.city_attributes.as_ref() {
            if a.district == u32::MAX {
                false
            } else {
                let d = self
                    .state
                    .district(i32::from(a.player), a.district)
                    .map_err(|_| Error::Missing("current plot District"))?;
                self.definitions.city_centers.contains(&d.district_hash)
            }
        } else {
            false
        };
        if water
            || (city
                && self
                    .effects
                    .coastal_land_one(self.state, plot)
                    .map_err(Error::Effect)?)
        {
            self.live(plot)?.valid_domains |= 1;
        }
        if !self
            .effects
            .water(self.state, plot)
            .map_err(Error::Effect)?
        {
            self.live(plot)?.valid_domains |= 4;
        }
        self.live(plot)?.valid_domains |= 16;
        self.live(plot)?.valid_domains |= 2;
        Ok(())
    }
}
impl<E: Effects> plot_yields::Context for Context<'_, E> {
    type Error = E::Error;
    fn runtime(&self) -> &plot_yields::Runtime {
        &self.runtime.plot_yields
    }
    fn runtime_mut(&mut self) -> &mut plot_yields::Runtime {
        &mut self.runtime.plot_yields
    }
    fn created(&mut self, receiver: u64, id: u32) -> Result<(), E::Error> {
        self.effects
            .plot_yields_created(self.state, self.runtime, receiver, id)
    }
    fn destroyed(&mut self, object: plot_yields::Object) -> Result<(), E::Error> {
        self.effects
            .plot_yields_destroyed(self.state, self.runtime, object)
    }
}
impl<E: Effects> kernel::AcquireContext for Context<'_, E> {
    type Error = Error<E::Error>;
    type Plot = u32;
    fn city_owner(&mut self) -> Result<i32, Self::Error> {
        Ok(self
            .state
            .city(self.city.owner, self.city.id)
            .map_err(Error::State)?
            .header
            .owner)
    }
    fn city_plot_location(&mut self) -> Result<PlotCoord, Self::Error> {
        let c = self
            .state
            .city(self.city.owner, self.city.id)
            .map_err(Error::State)?;
        Ok(PlotCoord {
            x: c.header.location_x,
            y: c.header.location_y,
        })
    }
    fn plot_at(&mut self, location: PlotCoord) -> Result<Option<u32>, Self::Error> {
        self.index(location)
    }
    fn plot_owner(&mut self, plot: u32) -> Result<i32, Self::Error> {
        OwnerContext::plot_owner(self, plot)
    }
    fn district_type(&mut self, plot: u32) -> Result<i32, Self::Error> {
        let Some(a) = self.plot(plot)?.city_attributes.as_ref() else {
            return Ok(-1);
        };
        if a.district == u32::MAX {
            return Ok(-1);
        };
        self.state
            .district(i32::from(a.player), a.district)
            .and_then(|d| self.definitions.districts.find_index(d.district_hash).map(|i| i as i32).ok_or(c6_core::loaded_state::district_initialization::DistrictError::MissingDistrict {owner:i32::from(a.player), id:a.district}))
            .map_err(|_| Error::Missing("current plot District"))
    }
    fn plot_location(&mut self, plot: u32) -> Result<PlotCoord, Self::Error> {
        self.location(plot)
    }
    fn annex_plot(&mut self, location: PlotCoord) -> Result<(), Self::Error> {
        kernel::annex_plot(self, location)
    }
    fn next_buyable_plot(&mut self) -> Result<Option<u32>, Self::Error> {
        let plots = self
            .effects
            .buyable_plots(self.state, self.city)
            .map_err(Error::Effect)?;
        if plots.is_empty() {
            return Ok(None);
        }
        let at = self
            .state
            .game_sync_rand(plots.len() as u16)
            .map_err(Error::Random)?;
        let plot = *plots
            .get(at as usize)
            .ok_or(Error::Missing("buyable plot draw index"))?;
        self.plot(plot)?;
        Ok(Some(plot))
    }
}
impl<E: Effects> kernel::AnnexContext for Context<'_, E> {
    fn current_city(&mut self) -> Result<CityId, Self::Error> {
        let c = self
            .state
            .city(self.city.owner, self.city.id)
            .map_err(Error::State)?;
        Ok(CityId {
            owner: c.header.owner,
            id: c.header.id,
        })
    }
    fn set_working_city(&mut self, location: PlotCoord, city: CityId) -> Result<(), Self::Error> {
        if let Some(p) = self.index(location)? {
            self.attributes(p)?;
            self.plot_mut(p)?
                .city_attributes
                .as_mut()
                .expect("initialized")
                .working_city = city.id;
        }
        Ok(())
    }
    fn improvement_type(&mut self, plot: u32) -> Result<i32, Self::Error> {
        let hash = self.plot(plot)?.improvement_hash;
        if hash == u32::MAX {
            return Ok(-1);
        }
        self.definitions
            .improvements
            .find_index(hash)
            .map(|i| i as i32)
            .ok_or(Error::Missing("current improvement definition"))
    }
    fn improvement_owner(&mut self, plot: u32) -> Result<i32, Self::Error> {
        Ok(i32::from(self.plot(plot)?.improvement_owner))
    }
    fn set_improvement(&mut self, plot: u32, kind: i32, owner: i32) -> Result<(), Self::Error> {
        self.effects
            .set_improvement(self.state, plot, kind, owner)
            .map_err(Error::Effect)
    }
    fn capturable_improvement(&mut self, kind: i32) -> Result<Option<bool>, Self::Error> {
        let Some(d) = self.definitions.improvements.at(kind as u32) else {
            return Ok(None);
        };
        let hash = linux_type_hash(&d.type_name);
        Ok(self
            .definitions
            .capturable
            .iter()
            .find(|r| r.0 == hash)
            .map(|r| r.1))
    }
    fn unit_count(&mut self, plot: u32) -> Result<i32, Self::Error> {
        Ok(i32::from(self.plot(plot)?.unit_count))
    }
    fn expel_from_plot(&mut self, location: PlotCoord) -> Result<(), Self::Error> {
        self.effects
            .expel_from_plot(self.state, location)
            .map_err(Error::Effect)
    }
    fn activate_improvement_without_unit(&mut self, plot: u32) -> Result<(), Self::Error> {
        // UnitActivateImprovement2bea9e0's NULL-Unit branch still resolves the
        // current Plot owner and civilization level before the improvement.
        let owner = OwnerContext::plot_owner(self, plot)?;
        if owner == -1 {
            return Ok(());
        }
        let owner = OwnerContext::plot_owner(self, plot)?;
        self.state.world().player(owner).map_err(Error::World)?;
        let level = self
            .runtime
            .players
            .iter()
            .find(|p| p.owner == owner)
            .and_then(|p| p.civilization_level)
            .ok_or(Error::Missing("improvement activation civilization level"))?;
        if level == 0xd959ec24 {
            return Ok(());
        }
        if kernel::AnnexContext::improvement_type(self, plot)? == -1 {
            return Ok(());
        }
        self.effects
            .activate_improvement_without_unit(self.state, plot)
            .map_err(Error::Effect)
    }
    fn acquired(&mut self, city: CityId, delta: i32, reset_next: bool) -> Result<(), Self::Error> {
        let culture = &mut self
            .state
            .city_mut(city.owner, city.id)
            .map_err(Error::State)?
            .culture;
        if reset_next {
            culture.next_buyable_plot = -1;
        }
        culture.plots_acquired = culture.plots_acquired.wrapping_add(delta);
        Ok(())
    }
    fn reallocate_citizens(&mut self, city: CityId) -> Result<(), Self::Error> {
        self.effects
            .reallocate_citizens(self.state, city)
            .map_err(Error::Effect)
    }
    fn report_annexed(&mut self, city: CityId, location: PlotCoord) -> Result<(), Self::Error> {
        self.effects
            .annex_report(self.state, city, location)
            .map_err(Error::Effect)
    }
    fn signal_annexed(
        &mut self,
        owner: i32,
        city: CityId,
        plot: u32,
        old_city: CityId,
    ) -> Result<(), Self::Error> {
        self.effects
            .annex_signal(self.state, owner, city, plot, old_city)
            .map_err(Error::Effect)
    }
}

/// The reached AnnexPlot expulsion prefix, using current parents, current Unit
/// domains and the real Formation placement query. This closes the stationary
/// case; relocation/air/paired/failure continuations remain explicit effects.
#[derive(Debug)]
pub enum ExpelError {
    Missing(&'static str),
    State(loaded_state::Error),
    World(c6_core::world::MissingInput),
    Placement(crate::movement_placement_state::Error),
    Continuation {
        unit: c6_core::map::UnitReference,
        placement: Option<i32>,
    },
}
pub fn expel_stationary(
    placement: &crate::movement_placement_state::Context<'_, '_, '_>,
    location: PlotCoord,
) -> Result<(), ExpelError> {
    let state = placement.current_state();
    let Some(plot) = crate::unit_placement_services::plot_index_at::<()>(state, location)
        .map_err(|_| ExpelError::Missing("Expel plot geometry"))?
    else {
        return Ok(());
    };
    // Native copies the selected layer's current ID vector before callbacks.
    let units = state
        .world()
        .plot_units(0, plot as usize)
        .map_err(ExpelError::World)?
        .to_vec();
    let mut deferred = Vec::new();
    for reference in units {
        let Some(found) = state.world().unit(reference).map_err(ExpelError::World)? else {
            continue;
        };
        let unit = found.record;
        let parent = state
            .object_pairing()
            .map_err(ExpelError::State)?
            .get_parent(ComponentId {
                owner: unit.e_owner as i16,
                component_type: 1,
                id: unit.id,
            });
        if parent != c6_core::object_pairing::NULL_COMPONENT {
            deferred.push(reference);
            continue;
        }
        if state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(ExpelError::State)?
            .domain
            == 1
        {
            deferred.push(reference);
            continue;
        }
        let result = placement
            .formation_placement(
                unit.e_owner,
                unit.formation_id,
                location,
                c6_core::movement_shore::PlacementRequest::from_validate_options(0x10),
            )
            .map_err(ExpelError::Placement)?;
        if result != 1 {
            return Err(ExpelError::Continuation {
                unit: reference,
                placement: Some(result),
            });
        }
    }
    if let Some(unit) = deferred.first() {
        return Err(ExpelError::Continuation {
            unit: *unit,
            placement: None,
        });
    }
    Ok(())
}

#[derive(Debug)]
pub enum OwnerChangedError {
    State(loaded_state::Error),
    Missing(&'static str),
    /// Current nonempty coastal protection requires its actual building/range
    /// and reporting continuations; it must never become an inland no-op.
    CoastalProtection {
        plot: u32,
        protected: bool,
        submerged: bool,
    },
}
/// Rules.Cities.OnPlotCityOwnershipChanged2bd5360, reached by the stock
/// GameSignals+1128 receiver. No coastal record proves all three lookups miss:
/// IsProtected=false, HasFloodProtectionAt=false, SetProtected does not write.
/// This does not classify an unknown Terrain component as an empty collection.
pub fn rules_cities_owner_changed(
    state: &LoadedState,
    location: PlotCoord,
    new_city: CityId,
    _old_city: CityId,
) -> Result<(), OwnerChangedError> {
    if !new_city.is_valid() {
        return Ok(());
    }
    match state.city(new_city.owner, new_city.id) {
        Ok(_) => {}
        Err(loaded_state::Error::Missing(loaded_state::MissingInput::City { .. })) => return Ok(()),
        Err(e) => return Err(OwnerChangedError::State(e)),
    }
    let Some(plot) = crate::unit_placement_services::plot_index_at::<()>(state, location)
        .map_err(|_| OwnerChangedError::Missing("owner change map geometry"))?
    else {
        return Ok(());
    };
    let mut rows = state
        .world()
        .map()
        .components
        .iter()
        .filter_map(|c| match c {
            c6_core::map::Component::Terrain(t) => Some(t),
            _ => None,
        });
    let terrain = rows.next().ok_or(OwnerChangedError::Missing(
        "current Terrain coastal lowlands",
    ))?;
    if rows.next().is_some() {
        return Err(OwnerChangedError::Missing("duplicate current Terrain"));
    }
    if let Some(row) = terrain
        .coastal_lowlands
        .iter()
        .find(|r| r.plot_index == plot as i32)
    {
        return Err(OwnerChangedError::CoastalProtection {
            plot,
            protected: row.protected,
            submerged: row.submerged,
        });
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImprovementReapply {
    pub plot: u32,
    pub improvement_type: i32,
    pub old_city: CityId,
}
/// Actual GameEventHandler.OnPlotAnnexed_ReapplyGameEffects23e72c0 first
/// queries the current improvement. NO_IMPROVEMENT returns at23e769c before
/// constructing an object, retrieving ModifierSystem, logging or callbacks.
/// A present improvement returns the real reached continuation, not success.
pub fn annexed_improvement_reapply(
    state: &LoadedState,
    definitions: &Definitions,
    plot: u32,
    old_city: CityId,
) -> Result<Option<ImprovementReapply>, Error<std::convert::Infallible>> {
    if state.binding() != &definitions.binding {
        return Err(Error::Missing("territory definitions binding"));
    }
    let current = state
        .world()
        .map()
        .plot_by_index(plot)
        .ok_or(Error::Missing("annexed improvement Plot"))?;
    if current.improvement_hash == u32::MAX {
        return Ok(None);
    }
    let improvement_type = definitions
        .improvements
        .find_index(current.improvement_hash)
        .ok_or(Error::Missing("annexed current improvement definition"))?
        as i32;
    Ok(Some(ImprovementReapply {
        plot,
        improvement_type,
        old_city,
    }))
}
