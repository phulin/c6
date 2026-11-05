//! Current normal-city activation services over canonical state and rule caches.
//! Production effects remain a borrowed service, allowing the caller to retain
//! the same script, modifier and report owners across actions and turns.
use super::{growth, housing, religion, upkeep, yield_calculation, Error as CityError};
use crate::{
    city_production_turn::{self as production, appeal},
    loaded_actions::configuration::Configuration,
};
use c6_core::{loaded_state::LoadedState, unit::PlotCoord};
#[path = "territory_view.rs"]
pub mod territory_view;
/// The fixed stock output consumers publish derived snapshots only. Added
/// synchronous consumers require a separate implementation of upkeep::Effects.
pub enum Scope {
    StockPlainHeadlessExternalPolicies,
}
/// Borrowed current owners. With `trade: None`, each query reads the canonical
/// graph from LoadedState rather than retaining a route snapshot across turns.
pub struct Current<'a> {
    pub territory: &'a dyn territory_view::Territory,
    pub drought_ids: Option<&'a [i32]>,
    pub trade: Option<&'a c6_core::trade_graph_state::GraphState>,
    pub capabilities: &'a c6_core::game_capabilities::Membership,
    pub current_era_index: Option<i32>,
}
impl Current<'_> {
    pub fn calculate(
        &self,
        state: &LoadedState,
        owner: i32,
        city: u32,
        yield_index: i32,
        kind: u32,
        definitions: &yield_calculation::Definitions,
    ) -> Result<i32, CityError> {
        let trade = match self.trade {
            Some(graph) => graph,
            None => state
                .trade_graph()
                .map_err(|_| CityError::Unsupported("current canonical trade graph"))?,
        };
        let current_era_index = match self.current_era_index {
            Some(era) => era,
            None => state
                .game_eras()
                .map_err(|_| CityError::Unsupported("current canonical game era"))?
                .current_era()
                .ok_or(CityError::Unsupported("current canonical era index"))?,
        };
        let territory = self.territory.read()?;
        yield_calculation::calculate(
            state,
            owner,
            city,
            yield_index,
            kind,
            definitions,
            &yield_calculation::Current {
                territory: &territory,
                drought_ids: self.drought_ids,
                trade,
                capabilities: self.capabilities,
                current_era_index,
            },
        )
    }
}
pub struct Context<'a, E> {
    pub production: &'a mut E,
    pub definitions: &'a yield_calculation::Definitions,
    pub current: &'a Current<'a>,
    pub housing: &'a housing::Definitions,
    pub configuration: &'a Configuration<'a>,
    pub culture: &'a upkeep::culture::Definitions,
    pub loyalty: &'a upkeep::loyalty::Definitions,
    pub power: &'a crate::city_power::Definitions,
    pub combat: &'a crate::loaded_actions::city_range_attack::Definitions,
    pub workers: &'a upkeep::citizens::Definitions,
    pub buyable: &'a upkeep::buyable_plots::Definitions,
    pub appeal: &'a appeal::Definitions,
    pub player_properties: &'a [(i32, Vec<(u32, i32)>)],
    pub loyalty_changed_subscribers: Option<usize>,
    pub alive: Vec<i32>,
    pub levels: Vec<(i32, u32)>,
    pub fallout: Vec<i32>,
    pub calls: Vec<&'static str>,
    pub scope: Scope,
}
impl<E: production::Effects> Context<'_, E> {
    pub fn activate(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<production::Outcome, CityError> {
        let definitions = self.definitions;
        upkeep::do_turn_activate(state, owner, city, &definitions.growth, self)
    }

    fn missing<T>(&mut self, name: &'static str) -> std::result::Result<T, CityError> {
        self.calls.push(name);
        Err(CityError::Unsupported(name))
    }
    fn current_amenities(
        &self,
        state: &LoadedState,
        owner: i32,
        city: u32,
    ) -> std::result::Result<i32, CityError> {
        self.definitions.amenities.amenities(
            state,
            owner,
            city,
            self.current.capabilities,
            &self.definitions.growth,
        )
    }
    fn happiness(
        &self,
        state: &LoadedState,
        owner: i32,
        city: u32,
    ) -> std::result::Result<i32, CityError> {
        let amenities = self.current_amenities(state, owner, city)?;
        let needed = self.definitions.economy.amenities_needed(
            state.city(owner, city)?.header.population,
            self.current.capabilities,
        );
        Ok(self
            .definitions
            .economy
            .happiness_from_surplus(amenities.wrapping_sub(needed)))
    }
    fn citizen_current<'q>(
        &'q self,
        territory: &'q crate::city_territory::Runtime,
    ) -> upkeep::citizens::Current<'q> {
        upkeep::citizens::Current {
            definitions: self.definitions,
            territory,
            drought_ids: self.current.drought_ids,
            fallout_turns: &self.fallout,
        }
    }
    fn qualify_report(&self, event: u64, expected: &[u64]) -> std::result::Result<(), CityError> {
        // Fixed stock cache consumers below only publish derived city/player
        // snapshots. Scope excludes added synchronous consumers, not gameplay.
        match (event, expected) {
            (0x0242e68c, [0x2717400])
            | (0x89de2474, [0x27173c0])
            | (0x67001fe1, [0x2710a10, 0x27bcd80]) => Ok(()),
            _ => Err(CityError::Unsupported("unknown city upkeep report")),
        }
    }
}
impl<E: production::Effects> production::Effects for Context<'_, E> {
    fn calculate_yield_q8(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        y: i32,
        k: u32,
    ) -> std::result::Result<i32, production::Error> {
        production::Effects::calculate_yield_q8(&mut *self.production, s, o, c, y, k)
    }
    fn current_build(
        &mut self,
        s: &LoadedState,
        o: i32,
        c: u32,
        e: &c6_core::city_production::QueueEntry,
    ) -> std::result::Result<production::CurrentBuild, production::Error> {
        production::Effects::current_build(&mut *self.production, s, o, c, e)
    }
    fn start_progress(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        e: &c6_core::city_production::QueueEntry,
    ) -> std::result::Result<(), production::Error> {
        production::Effects::start_progress(&mut *self.production, s, o, c, e)
    }
    fn progress_report(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        e: &c6_core::city_production::QueueEntry,
    ) -> std::result::Result<(), production::Error> {
        production::Effects::progress_report(&mut *self.production, s, o, c, e)
    }
    fn finish(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        e: &c6_core::city_production::QueueEntry,
    ) -> std::result::Result<bool, production::Error> {
        production::Effects::finish(&mut *self.production, s, o, c, e)
    }
    fn new_front(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
    ) -> std::result::Result<(), production::Error> {
        production::Effects::new_front(&mut *self.production, s, o, c)
    }
}
impl<E: production::Effects> crate::found_city::cultural_identity::Context for Context<'_, E> {
    fn alive_ids(&self) -> &[i32] {
        &self.alive
    }
    fn civilization_level(&self, o: i32) -> std::result::Result<u32, CityError> {
        self.levels
            .iter()
            .find(|p| p.0 == o)
            .map(|p| p.1)
            .ok_or(CityError::Unsupported("current civilization level"))
    }
    fn food_surplus_q8(
        &mut self,
        s: &LoadedState,
        o: i32,
        c: u32,
    ) -> std::result::Result<i32, CityError> {
        let y = self
            .definitions
            .yield_index(b"YIELD_FOOD")
            .ok_or(CityError::Unsupported("Food definition"))?;
        let amount = self.current.calculate(s, o, c, y, 0, self.definitions)?;
        Ok(amount.wrapping_sub(
            s.city(o, c)?
                .header
                .population
                .wrapping_mul(self.definitions.growth.food_consumption_per_population)
                .wrapping_shl(8),
        ))
    }
    fn happiness(
        &mut self,
        s: &LoadedState,
        o: i32,
        c: u32,
    ) -> std::result::Result<i32, CityError> {
        Context::happiness(self, s, o, c)
    }
    fn player_property_int(
        &mut self,
        _: &LoadedState,
        o: i32,
        _key: u32,
    ) -> std::result::Result<Option<i32>, CityError> {
        let properties = self
            .player_properties
            .iter()
            .find(|p| p.0 == o)
            .ok_or(CityError::Unsupported("current Player properties"))?;
        Ok(properties.1.iter().rev().find(|p| p.0 == _key).map(|p| p.1))
    }

    fn trend_changed(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
        _: i32,
        _: Option<bool>,
    ) -> std::result::Result<(), CityError> {
        self.qualify_report(0x0242e68c, &[0x2717400])?;
        self.calls.push("conversion trend output");
        Ok(())
    }
}
impl<E: production::Effects> upkeep::loyalty::Effects for Context<'_, E> {
    fn loyalty_changed(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
    ) -> std::result::Result<(), CityError> {
        if self.loyalty_changed_subscribers != Some(0) {
            return self.missing("positive current loyalty signal subscribers");
        }
        self.calls.push("empty actual loyalty signal");
        Ok(())
    }
    fn loyalty_report(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
    ) -> std::result::Result<(), CityError> {
        self.qualify_report(0x89de2474, &[0x27173c0])?;
        self.calls.push("loyalty output");
        Ok(())
    }
    fn losing_loyalty_notification(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
    ) -> std::result::Result<(), CityError> {
        self.missing("losing loyalty notification")
    }
}
impl<E: production::Effects> upkeep::culture::Effects for Context<'_, E> {
    fn enabled(&self) -> bool {
        self.current.capabilities.has(0xc7331068)
    }
    fn may_annex_with_culture(&self, o: i32) -> std::result::Result<bool, CityError> {
        let level = self
            .levels
            .iter()
            .find(|p| p.0 == o)
            .ok_or(CityError::Unsupported("border current level"))?
            .1;
        Ok(self.culture.may_annex_with_culture(level))
    }
    fn culture_yield_q8(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
    ) -> std::result::Result<i32, CityError> {
        let y = self
            .definitions
            .yield_index(b"YIELD_CULTURE")
            .ok_or(CityError::Unsupported("Culture definition"))?;
        self.current.calculate(s, o, c, y, 0, self.definitions)
    }
    fn next_buyable_plot(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
    ) -> std::result::Result<Option<u32>, CityError> {
        let territory = self.current.territory.read()?;
        upkeep::buyable_plots::next_plot(
            s,
            o,
            c,
            self.buyable,
            &self.definitions.tiles,
            &crate::city_turn::tiles::Current {
                territory: &territory,
                drought_ids: self.current.drought_ids,
                resource_visibility: &self.definitions.resources,
            },
        )
        .map_err(|e| {
            let _ = e;
            CityError::Unsupported("current buyable-plot selection")
        })
    }
    fn plot_is_unowned(&self, s: &LoadedState, p: u32) -> std::result::Result<bool, CityError> {
        Ok(s.world()
            .map()
            .plots
            .get(p as usize)
            .ok_or(CityError::Unsupported("cached expansion plot"))?
            .city_attributes
            .as_ref()
            .is_none_or(|a| a.player == -1))
    }
    fn annex_plot(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
        _: u32,
    ) -> std::result::Result<(), CityError> {
        self.missing("positive culture annexation continuation")
    }
}
impl<E: production::Effects> appeal::Effects for Context<'_, E> {
    type Error = CityError;
    fn appeal_changed(
        &mut self,
        _: &mut LoadedState,
        _: u32,
        _: i32,
    ) -> std::result::Result<(), CityError> {
        self.missing("positive worker appeal signal")
    }
    fn appeal_report(&mut self, _: &mut LoadedState, _: u32) -> std::result::Result<(), CityError> {
        self.missing("positive worker appeal report")
    }
}
impl<E: production::Effects> upkeep::citizens::Effects for Context<'_, E> {
    fn food_surplus_q8(
        &mut self,
        s: &LoadedState,
        o: i32,
        c: u32,
    ) -> std::result::Result<i32, CityError> {
        crate::found_city::cultural_identity::Context::food_surplus_q8(self, s, o, c)
    }
    fn visit_plot(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        p: usize,
    ) -> std::result::Result<(), CityError> {
        let defs = self.appeal;
        // Release the immutable field borrow across the reentrant service call.
        let result = appeal::update_and_test_adjacent(
            s,
            Some(p),
            defs,
            &mut WorkerAppeal,
            &mut |s: &mut LoadedState, _: &mut WorkerAppeal, a: u32| {
                let p = s
                    .world()
                    .map()
                    .plots
                    .get(a as usize)
                    .ok_or(CityError::Unsupported("adjacent worker plot"))?;
                Ok(!p
                    .city_attributes
                    .as_ref()
                    .is_some_and(|a| i32::from(a.player) == o && a.owning_city == c))
            },
        );
        result.map(|_| ()).map_err(|e| {
            let _ = e;
            CityError::Unsupported("worker appeal continuation")
        })
    }
    fn capacity(
        &mut self,
        s: &LoadedState,
        o: i32,
        c: u32,
        p: PlotCoord,
    ) -> std::result::Result<i32, CityError> {
        let territory = self.current.territory.read()?;
        upkeep::citizens::capacity(s, o, c, p, self.workers, &self.citizen_current(&territory))
    }
    fn plot_yields(
        &mut self,
        s: &LoadedState,
        o: i32,
        c: u32,
        p: usize,
    ) -> std::result::Result<Vec<c6_core::city_citizen_assignment::Yield>, CityError> {
        let territory = self.current.territory.read()?;
        upkeep::citizens::plot_yields(s, o, c, p, &self.citizen_current(&territory))
    }
    fn food_and_gold_indices(&self) -> std::result::Result<(i32, i32), CityError> {
        Ok((
            self.definitions
                .yield_index(b"YIELD_FOOD")
                .ok_or(CityError::Unsupported("Food index"))?,
            self.definitions
                .yield_index(b"YIELD_GOLD")
                .ok_or(CityError::Unsupported("Gold index"))?,
        ))
    }
    fn workers_changed(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        i: i32,
        old: i32,
    ) -> std::result::Result<(), CityError> {
        let city = s.city(o, c)?;
        let at = i as usize;
        let current = city
            .citizens
            .working_plot
            .get(at)
            .ok_or(CityError::Unsupported("worker index"))?
            .wrapping_add(
                *city
                    .citizens
                    .forced_working_plot
                    .get(at)
                    .ok_or(CityError::Unsupported("forced worker index"))?,
            );
        if current == old {
            return Ok(());
        }
        self.qualify_report(0x67001fe1, &[0x2710a10, 0x27bcd80])?;
        let center = PlotCoord {
            x: city.header.location_x,
            y: city.header.location_y,
        };
        if let Some(p) = c6_core::city_citizen_assignment::plot_location(s.world().map(), center, i)
            .map_err(|_| CityError::Unsupported("worker report location"))?
        {
            let (_, p) = c6_core::movement_world::plot_at(s.world().map(), p)
                .map_err(|_| CityError::Unsupported("worker report plot"))?;
            if p.improvement_hash != u32::MAX {
                return self.missing("worker improvement activation");
            }
        }
        self.calls.push("worker output report");
        Ok(())
    }
}
struct PowerGossip;
impl c6_core::city_power_activation::stock_signals::GossipContext for PowerGossip {
    type Error = crate::city_power::Error;
    fn city_owner(&mut self) -> std::result::Result<i32, Self::Error> {
        Err(Self::Error::Missing("first power gossip owner"))
    }
    fn city_id(&mut self) -> std::result::Result<u32, Self::Error> {
        Err(Self::Error::Missing("first power gossip city"))
    }
    fn turn_visibility_count(&mut self) -> std::result::Result<usize, Self::Error> {
        Err(Self::Error::Missing("first power gossip visibility"))
    }
    fn current_turn(&mut self) -> std::result::Result<i32, Self::Error> {
        Err(Self::Error::Missing("first power gossip turn"))
    }
    fn append_powered_city(
        &mut self,
        _: c6_core::city_power_activation::stock_signals::Occurrence,
    ) -> std::result::Result<(), Self::Error> {
        Err(Self::Error::Missing("first power gossip append"))
    }
    fn report_powered_city(
        &mut self,
        _: c6_core::city_power_activation::stock_signals::Occurrence,
    ) -> std::result::Result<(), Self::Error> {
        Err(Self::Error::Missing("first power gossip report"))
    }
}
struct WorkerAppeal;
impl appeal::Effects for WorkerAppeal {
    type Error = CityError;
    fn appeal_changed(
        &mut self,
        _: &mut LoadedState,
        _: u32,
        _: i32,
    ) -> std::result::Result<(), CityError> {
        Err(CityError::Unsupported("positive worker appeal signal"))
    }
    fn appeal_report(&mut self, _: &mut LoadedState, _: u32) -> std::result::Result<(), CityError> {
        Err(CityError::Unsupported("positive worker appeal report"))
    }
}
impl<E: production::Effects> crate::city_power::loaded::Effects for Context<'_, E> {
    fn power_changed(
        &mut self,
        _: &mut LoadedState,
        _: (i32, u32),
        first: bool,
    ) -> std::result::Result<(), crate::city_power::Error> {
        c6_core::city_power_activation::stock_signals::powered_city_changed(
            &mut PowerGossip,
            first,
        )?;
        self.calls
            .push("PowerGenerated first=false -> actual gossip early return");
        Ok(())
    }
    fn shortage_changed(
        &mut self,
        _: &mut LoadedState,
        _: (i32, u32),
    ) -> std::result::Result<(), crate::city_power::Error> {
        Err(crate::city_power::Error::Missing("power shortage report"))
    }
}
impl<E: production::Effects> upkeep::Effects for Context<'_, E> {
    fn border_expansion(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
    ) -> std::result::Result<(), CityError> {
        upkeep::culture::do_border_expansion(s, o, c, self.culture, self)
    }
    fn loyalty_upkeep(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
    ) -> std::result::Result<(), CityError> {
        upkeep::loyalty::do_turn_activate(s, o, c, self.loyalty, self)
    }
    fn assign_citizens(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
    ) -> std::result::Result<(), CityError> {
        upkeep::citizens::reallocate(s, o, c, self)
    }
    fn restore_combat(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
    ) -> std::result::Result<(), CityError> {
        upkeep::combat::restore(s, o, c, self.combat, &self.levels)
    }
    fn activate_power(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
    ) -> std::result::Result<(), CityError> {
        let binding = s.binding().clone();
        let levels = self.levels.clone();
        crate::city_power::loaded::recalculate(
            s,
            self.power,
            (o, c),
            crate::movement_placement_state::CurrentCivilizationLevels {
                binding: &binding,
                values: &levels,
            },
            self,
        )
        .map_err(CityError::Power)
    }
}
impl<E: production::Effects> growth::Effects for Context<'_, E> {
    fn food_yield_q8(
        &mut self,
        state: &mut c6_core::loaded_state::LoadedState,
        owner: i32,
        city: u32,
    ) -> std::result::Result<i32, CityError> {
        self.calls.push("current food yield");
        let index = self
            .definitions
            .yield_index(b"YIELD_FOOD")
            .ok_or(CityError::Unsupported("Food definition"))?;
        self.current
            .calculate(state, owner, city, index, 0, self.definitions)
    }
    fn other_growth_modifier_q8(
        &mut self,
        state: &mut c6_core::loaded_state::LoadedState,
        owner: i32,
        city: u32,
    ) -> std::result::Result<i32, CityError> {
        self.calls.push("current other growth");
        self.definitions
            .economy
            .other_growth_modifier(state, owner, city, self.configuration)
    }
    fn happiness_growth_modifier_q8(
        &mut self,
        state: &mut c6_core::loaded_state::LoadedState,
        owner: i32,
        city: u32,
    ) -> std::result::Result<i32, CityError> {
        self.calls.push("current happiness growth");
        let amenities = self.current_amenities(state, owner, city)?;
        let needed = self.definitions.economy.amenities_needed(
            state.city(owner, city)?.header.population,
            self.current.capabilities,
        );
        Ok(self.definitions.economy.happiness_growth_modifier(
            self.definitions
                .economy
                .happiness_from_surplus(amenities.wrapping_sub(needed)),
        ))
    }
    fn housing_growth_modifier_q8(
        &mut self,
        state: &mut c6_core::loaded_state::LoadedState,
        owner: i32,
        city: u32,
    ) -> std::result::Result<i32, CityError> {
        self.calls.push("current housing growth");
        let amount = self
            .housing
            .housing(state, owner, city, &self.definitions.buildings)?;
        Ok(self.definitions.economy.housing_surplus_modifier(
            amount.wrapping_sub(state.city(owner, city)?.header.population),
        ))
    }
    fn loyalty_growth_modifier_q8(
        &mut self,
        state: &mut c6_core::loaded_state::LoadedState,
        owner: i32,
        city: u32,
    ) -> std::result::Result<i32, CityError> {
        self.calls.push("current loyalty growth");
        self.definitions
            .economy
            .loyalty_growth_modifier(state, owner, city)
    }
    fn amenities(
        &mut self,
        state: &mut c6_core::loaded_state::LoadedState,
        owner: i32,
        city: u32,
    ) -> std::result::Result<i32, CityError> {
        self.calls.push("current amenities");
        self.current_amenities(state, owner, city)
    }
    fn amenities_needed(&mut self, population: i32) -> std::result::Result<i32, CityError> {
        self.calls.push("current amenities needed");
        Ok(self
            .definitions
            .economy
            .amenities_needed(population, self.current.capabilities))
    }
    fn growth_notification(
        &mut self,
        _: &mut c6_core::loaded_state::LoadedState,
        _: i32,
        _: u32,
        _: u32,
    ) -> std::result::Result<(), CityError> {
        self.missing("actual growth notification effects")
    }
}
impl<E: production::Effects> crate::city_turn::PopulationEffects for Context<'_, E> {
    fn assign_citizens(
        &mut self,
        state: &mut c6_core::loaded_state::LoadedState,
        owner: i32,
        city: u32,
        reallocate: bool,
    ) -> std::result::Result<(), CityError> {
        if reallocate {
            upkeep::citizens::reallocate(state, owner, city, self)
        } else {
            upkeep::citizens::try_assign_all(state, owner, city, true, self)
        }
    }
    fn population_changed_signal(
        &mut self,
        state: &mut c6_core::loaded_state::LoadedState,
        owner: i32,
        city: u32,
        delta: i32,
    ) -> std::result::Result<(), CityError> {
        self.production
            .population_services()
            .ok_or(CityError::Unsupported(
                "current population callback services",
            ))?
            .population_changed_signal(state, owner, city, delta)
    }
    fn report_population(
        &mut self,
        state: &mut c6_core::loaded_state::LoadedState,
        owner: i32,
        city: u32,
        event: u32,
        population: i32,
    ) -> std::result::Result<(), CityError> {
        self.production
            .population_services()
            .ok_or(CityError::Unsupported(
                "current population callback services",
            ))?
            .report_population(state, owner, city, event, population)
    }
}
impl<E: production::Effects> religion::Effects for Context<'_, E> {
    fn civilization_level(
        &mut self,
        _: &c6_core::loaded_state::LoadedState,
        owner: i32,
    ) -> std::result::Result<u32, CityError> {
        self.levels
            .iter()
            .find(|v| v.0 == owner)
            .map(|v| v.1)
            .ok_or(CityError::Unsupported("current growth civilization level"))
    }
    fn give_free_envoy(
        &mut self,
        state: &mut c6_core::loaded_state::LoadedState,
        player: i32,
        minor: i32,
    ) -> std::result::Result<(), CityError> {
        self.production
            .population_services()
            .ok_or(CityError::Unsupported(
                "current population callback services",
            ))?
            .give_free_envoy(state, player, minor)
    }
    fn city_religion_changed(
        &mut self,
        state: &mut c6_core::loaded_state::LoadedState,
        owner: i32,
        city: u32,
        old: religion::ReligionIdentity,
        new: religion::ReligionIdentity,
    ) -> std::result::Result<(), CityError> {
        self.production
            .population_services()
            .ok_or(CityError::Unsupported(
                "current population callback services",
            ))?
            .city_religion_changed(state, owner, city, old, new)
    }
    fn report_city_religion(
        &mut self,
        state: &mut c6_core::loaded_state::LoadedState,
        owner: i32,
        city: u32,
        event: u32,
        identity: Option<religion::ReligionIdentity>,
    ) -> std::result::Result<(), CityError> {
        self.production
            .population_services()
            .ok_or(CityError::Unsupported(
                "current population callback services",
            ))?
            .report_city_religion(state, owner, city, event, identity)
    }
    fn player_religion_changed(
        &mut self,
        state: &mut c6_core::loaded_state::LoadedState,
        observer: i32,
        owner: i32,
        new: religion::ReligionIdentity,
    ) -> std::result::Result<(), CityError> {
        self.production
            .population_services()
            .ok_or(CityError::Unsupported(
                "current population callback services",
            ))?
            .player_religion_changed(state, observer, owner, new)
    }
    fn city_converted_by(
        &mut self,
        state: &mut c6_core::loaded_state::LoadedState,
        converter: i32,
        owner: i32,
        city: u32,
    ) -> std::result::Result<(), CityError> {
        self.production
            .population_services()
            .ok_or(CityError::Unsupported(
                "current population callback services",
            ))?
            .city_converted_by(state, converter, owner, city)
    }
    fn alive_players(
        &mut self,
        _: &c6_core::loaded_state::LoadedState,
    ) -> std::result::Result<Vec<i32>, CityError> {
        Ok(self.alive.clone())
    }
}
