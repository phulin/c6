//! Source-ordered placement of a freshly allocated City using canonical services.
//! No fixture values or independent runtime are owned by this composition.
use c6_core::{
    loaded_state::LoadedState,
    unit::{ComponentId, PlotCoord, Unit},
};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Report {
    MinimumDistancePlot(u32),
    Visibility(i32),
    PositionChanged,
}
pub trait Effects:
    crate::city_territory::Effects
    + crate::unit_sight_state::Effects<Error = String>
    + crate::district_initialization::Effects<Error = String>
    + crate::city_production_turn::appeal::Effects<Error = String>
    + crate::city_turn::PopulationEffects
    + super::routes::Effects<Error = String>
{
    fn city_added_to_map(
        &mut self,
        state: &LoadedState,
        city: ComponentId,
        location: PlotCoord,
    ) -> Result<(), String>;
    fn district_added_to_map(
        &mut self,
        state: &LoadedState,
        d: crate::district_initialization::DistrictReference,
        location: PlotCoord,
        sight: &crate::unit_sight_state::Runtime,
    ) -> Result<(), String>;
    fn founding_workers(
        &mut self,
        state: &mut LoadedState,
        territory: &mut crate::city_territory::Runtime,
        owner: i32,
        city: u32,
    ) -> Result<(), String>;
    fn reallocate_workers(
        &mut self,
        state: &mut LoadedState,
        territory: &mut crate::city_territory::Runtime,
        owner: i32,
        city: u32,
    ) -> Result<(), String>;
    fn starting_buildings(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        location: PlotCoord,
        district_index: i32,
        start_era: Option<u32>,
        levels: &[(i32, u32)],
    ) -> Result<(), String>;
    fn validate_stationary(
        &mut self,
        state: &mut LoadedState,
        territory: &mut crate::city_territory::Runtime,
        location: PlotCoord,
        all_layers: bool,
    ) -> Result<(), String>;
    fn report(
        &mut self,
        state: &LoadedState,
        city: ComponentId,
        report: Report,
    ) -> Result<(), String>;
}
pub struct Context<'a, E> {
    pub state: &'a mut LoadedState,
    pub actor: (i32, u32),
    pub effects: &'a mut E,
    pub definitions: &'a super::Definitions,
    pub rules: &'a crate::Rules,
    pub visibility: &'a mut crate::unit_sight_state::Runtime,
    pub territory: &'a mut crate::city_territory::Runtime,
    pub territory_definitions: &'a crate::city_territory::Definitions,
    pub sight_definitions: &'a crate::loaded_actions::city_range_attack::Definitions,
    pub district_definitions: &'a crate::district_initialization::Definitions,
    pub appeal_definitions: &'a crate::city_production_turn::appeal::Definitions,
    pub phases: &'a crate::unit_operation_manager::PhaseDefinitions,
    pub levels: &'a [(i32, u32)],
    pub route_definitions: &'a super::routes::Definitions,
    pub growth_rules: &'a crate::city_turn::GrowthRules,
    pub district_cost_definitions: &'a crate::research::districts::Definitions,
    pub civic_graph: &'a crate::research::CivicGraph,
    pub strategic_parameters: &'a [(i32, i32, i32)],
}
impl<E: Effects> Context<'_, E> {
    fn unit(&self) -> Result<&Unit, String> {
        self.state
            .world()
            .player(self.actor.0)
            .map_err(|e| format!("{e:?}"))?
            .unit(self.actor.1)
            .map_err(|e| format!("{e:?}"))?
            .map(|u| u.record)
            .ok_or("current founding Unit missing".into())
    }
    pub fn place(&mut self, city: ComponentId, location: PlotCoord) -> Result<i32, String> {
        self.place_with_initializing(city, location, false)
    }
    /// Transfer uses the same placement path with `initializing=true`, which
    /// suppresses starting buildings while retaining construction and upkeep.
    pub fn place_with_initializing(
        &mut self,
        city: ComponentId,
        location: PlotCoord,
        initializing: bool,
    ) -> Result<i32, String> {
        let unit = self.unit()?;
        let mut legality = crate::found_city::legality::Context {
            state: self.state,
            unit,
            definitions: self.definitions,
            rules: self.rules,
            plot: None,
            max_probes: 100000,
        };
        let valid = c6_core::found_city::placement::is_valid(
            &mut legality,
            i32::from(city.owner),
            location,
            100000,
        )
        .map_err(|e| format!("{e:?}"))?;
        if !valid {
            return Ok(0);
        }
        let c = self
            .state
            .city_mut(i32::from(city.owner), city.id)
            .map_err(|e| format!("{e:?}"))?;
        c.header.location_x = location.x;
        c.header.location_y = location.y;
        {
            let city_id = c6_core::city_territory::CityId {
                owner: i32::from(city.owner),
                id: city.id,
            };
            let mut territory = crate::city_territory::Context {
                state: self.state,
                runtime: self.territory,
                sight: self.visibility,
                sight_definitions: self.sight_definitions,
                definitions: self.territory_definitions,
                effects: self.effects,
                city: city_id,
                actor: self.actor,
            };
            territory
                .set_purchased_by_city(location, city_id)
                .map_err(|e| format!("center purchase: {e:?}"))?;
        }
        {
            let plot = c6_core::movement_world::plot_at(self.state.world().map(), location)
                .map_err(|e| format!("{e:?}"))?
                .0;
            let attrs = self
                .state
                .world_mut()
                .map_err(|e| format!("{e:?}"))?
                .plot_mut(plot)
                .ok_or("current center plot")?
                .city_attributes
                .as_mut()
                .ok_or("purchased center attributes")?;
            attrs.working_city = city.id;
            let feature = self.state.world().map().plots[plot].feature_hash;
            if feature != u32::MAX
                && self
                    .definitions
                    .feature_find(self.state, feature)
                    .map_err(|e| format!("{e:?}"))?
                    .is_some_and(|d| d.removable)
            {
                return Err("reached center removable feature builder".into());
            }
            self.effects.city_added_to_map(self.state, city, location)?;
            crate::city_production_turn::appeal::update(
                self.state,
                [location.x, location.y],
                self.appeal_definitions,
                self.effects,
            )
            .map_err(|e| format!("FoundCity appeal: {e:?}"))?;
            let population = crate::found_city::starting_era::apply(
                self.state,
                self.rules,
                i32::from(city.owner),
                city.id,
                self.territory.start_era.ok_or("current StartEra")?,
            )
            .map_err(|e| format!("{e:?}"))?;
            let home = self
                .state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == i32::from(city.owner))
                .ok_or("current city collection")?
                .cities
                .home_continent_hash;
            let continent = self.state.world().map().plots[plot].continent_hash;
            if home != u32::MAX && continent != u32::MAX && home != continent {
                return Err("off-home-continent starting population modifier".into());
            }
            let kind = self
                .district_definitions
                .type_index(c6_core::city_district::CITY_CENTER_HASH)
                .ok_or("current native CityCenter definition")?;
            if self.state.world().map().plots[plot].improvement_hash != u32::MAX {
                return Err("reached center replacement improvement".into());
            }
            let queries=crate::loaded_actions::city_range_attack::Queries{definitions:self.sight_definitions,phases:self.phases,scope:crate::loaded_actions::city_range_attack::CommandScope::SourceInitializedStockXp2,civilization_level_types:self.levels,combined_visibility:None};
            let district = crate::district_initialization::create_in_city(
                self.state,
                self.district_definitions,
                &queries,
                self.effects,
                crate::district_initialization::Creation {
                    owner: i32::from(city.owner),
                    city_id: city.id,
                    location,
                    district_type: kind,
                    complete: false,
                    bought: false,
                },
            )
            .map_err(|e| format!("center District create: {e:?}"))?;
            let p = self
                .state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == i32::from(city.owner))
                .ok_or("current district owner")?;
            if !p.districts.creation_yields.is_empty() {
                return Err("current district-create yield rewards".into());
            }
            if let Some(district) = district {
                crate::district_initialization::set_complete(
                    self.state,
                    self.district_definitions,
                    self.effects,
                    district,
                    true,
                    false,
                )
                .map_err(|e| format!("center District completion: {e:?}"))?;
            }
            if district.is_some() {
                let defs = self.route_definitions;
                crate::found_city::routes::complete(
                    self.state,
                    self.visibility,
                    defs,
                    plot as u32,
                    i32::from(city.owner),
                    self.effects,
                )
                .map_err(|e| format!("CityManager.SetComplete road: {e:?}"))?;
            }
            if let Some(district) = district {
                self.effects.district_added_to_map(
                    self.state,
                    district,
                    location,
                    self.visibility,
                )?;
            }
            {
                let city_id = c6_core::city_territory::CityId {
                    owner: i32::from(city.owner),
                    id: city.id,
                };
                let mut territory = crate::city_territory::Context {
                    state: self.state,
                    runtime: self.territory,
                    sight: self.visibility,
                    sight_definitions: self.sight_definitions,
                    definitions: self.territory_definitions,
                    effects: self.effects,
                    city: city_id,
                    actor: self.actor,
                };
                c6_core::city_territory::PurchasedContext::set_valid_domains(
                    &mut territory,
                    plot as u32,
                )
                .map_err(|e| format!("post-center valid domains {e:?}"))?;
            }
            let growth = self.growth_rules;
            crate::city_turn::change_population(
                self.state,
                i32::from(city.owner),
                city.id,
                population,
                growth,
                self.effects,
            )
            .map_err(|e| format!("FoundCity population: {e:?}"))?;
            {
                let city_id = c6_core::city_territory::CityId {
                    owner: i32::from(city.owner),
                    id: city.id,
                };
                let mut territory = crate::city_territory::Context {
                    state: self.state,
                    runtime: self.territory,
                    sight: self.visibility,
                    sight_definitions: self.sight_definitions,
                    definitions: self.territory_definitions,
                    effects: self.effects,
                    city: city_id,
                    actor: self.actor,
                };
                territory
                    .do_found_city()
                    .map_err(|e| format!("starting territory: {e:?}"))?;
            }
            self.effects.founding_workers(
                self.state,
                self.territory,
                i32::from(city.owner),
                city.id,
            )?;
            // Growth/Buildings/BuildQueue/TurnManager founding hooks are literal RET
            // in the target; the NULL-Unit improvement activation remains a real query.
            {
                let city_id = c6_core::city_territory::CityId {
                    owner: i32::from(city.owner),
                    id: city.id,
                };
                let mut territory = crate::city_territory::Context {
                    state: self.state,
                    runtime: self.territory,
                    sight: self.visibility,
                    sight_definitions: self.sight_definitions,
                    definitions: self.territory_definitions,
                    effects: self.effects,
                    city: city_id,
                    actor: self.actor,
                };
                c6_core::city_territory::AnnexContext::activate_improvement_without_unit(
                    &mut territory,
                    plot as u32,
                )
                .map_err(|e| format!("center activation: {e:?}"))?;
            }
            let percent = self
                .district_cost_definitions
                .completion_percent(self.state, i32::from(city.owner), self.civic_graph)
                .map_err(|e| format!("current research progression {e:?}"))?;
            self.district_cost_definitions
                .recompute_no_specialty(
                    self.state,
                    i32::from(city.owner),
                    percent,
                    self.territory.start_era,
                )
                .map_err(|e| format!("current district costs {e:?}"))?;

            // AI.IsActive is true: the actual WorldBuilder key is absent, so its
            // TypedVariant empty value converts to false. The checked wrapper preserves
            // strategic writes and rejects nonhuman/native-policy CityBuild data.
            let (_, minimum, additional) = *self
                .strategic_parameters
                .iter()
                .find(|r| r.0 == i32::from(city.owner))
                .ok_or("current strategic parameters")?;
            let parameters = c6_core::city_build_initialization::founding::Parameters::bind(
                self.state,
                i32::from(city.owner),
                minimum,
                additional,
            );
            c6_core::city_build_initialization::founding::on_found_city_external_policy(
                self.state,
                i32::from(city.owner),
                city.id,
                &parameters,
            )
            .map_err(|e| format!("CityBuild.DoFoundCity {e:?}"))?;

            if !initializing {
                self.effects.starting_buildings(
                    self.state,
                    city,
                    location,
                    kind,
                    self.territory.start_era,
                    self.levels,
                )?;
            }
            // RulesCities+50 EnumerateCityMinimumDistancePlots2bd5680 preserves native
            // dx/dy order, wrapped deltas and the landmass/distance predicate.
            let range = {
                let unit = self.unit()?;
                let mut query = crate::found_city::legality::Context {
                    state: self.state,
                    unit,
                    definitions: self.definitions,
                    rules: self.rules,
                    plot: None,
                    max_probes: 100000,
                };
                c6_core::found_city::placement::Context::minimum_city_range(&mut query)
                    .map_err(|e| format!("{e:?}"))?
            };
            let mut notifications = vec![];
            if range >= range.wrapping_neg() {
                let mut dx = range.wrapping_neg();
                let mut probes = 0usize;
                loop {
                    let mut dy = range.wrapping_neg();
                    loop {
                        probes += 1;
                        if probes > 100000 {
                            return Err("minimum-distance source enumeration bound".into());
                        }
                        if let Some(index) = c6_core::found_city::placement::plot_from_delta(
                            self.state.world().map(),
                            location,
                            dx,
                            dy,
                            range,
                        )
                        .map_err(|e| format!("{e:?}"))?
                        {
                            let map = self.state.world().map();
                            let p = &map.plots[index];
                            let (x, y) = map
                                .coordinates(index as u32)
                                .ok_or("current minimum-distance plot coordinate")?;
                            if (p.landmass_id == map.plots[plot].landmass_id
                                || c6_core::geometry::plot_distance(
                                    map.header.width,
                                    map.header.wrap_x,
                                    x,
                                    y,
                                    location.x,
                                    location.y,
                                ) < range)
                                && !p.city_attributes.as_ref().is_some_and(|a| {
                                    i32::from(a.player) == i32::from(city.owner)
                                        && a.owning_city == city.id
                                })
                            {
                                notifications.push(index);
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
            }
            for index in notifications {
                Effects::report(
                    self.effects,
                    self.state,
                    city,
                    Report::MinimumDistancePlot(index as u32),
                )?;
            }
            self.effects.reallocate_workers(
                self.state,
                self.territory,
                i32::from(city.owner),
                city.id,
            )?;
            if !self.visibility.messages_disabled {
                let observer = self
                    .visibility
                    .local_observer
                    .ok_or("current City visibility observer")?;
                let layer = self
                    .visibility
                    .shared(observer)
                    .map_err(str::to_owned)?
                    .0
                    .iter()
                    .find(|r| r.type_hash == c6_core::visibility_change::BASE_LAYER_HASH)
                    .ok_or("current City normal visibility")?;
                let visible = *layer
                    .values
                    .get(plot)
                    .ok_or("current City visibility cell")?
                    > 0;
                let revealed = if visible {
                    2
                } else {
                    i32::from(
                        *self
                            .state
                            .world()
                            .visibility(observer)
                            .ok_or("current City revealed grid")?
                            .revealed
                            .get(plot)
                            .ok_or("current City revealed cell")?
                            != 0,
                    )
                };
                if revealed != 0 {
                    Effects::report(self.effects, self.state, city, Report::Visibility(revealed))?;
                }
            }
            self.effects
                .validate_stationary(self.state, self.territory, location, true)?;
            Effects::report(self.effects, self.state, city, Report::PositionChanged)?;
            Ok(1)
        }
    }
}
