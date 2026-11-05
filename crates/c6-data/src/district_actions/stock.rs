//! Ordinary Campus selection and production on the persistent stock owners.
pub mod completion;
pub mod roads;
use super::{construction, Current, Definitions};
use crate::{
    city_production_turn::{self as production, appeal},
    district_initialization as districts,
    loaded_actions::{
        choose_tech, city_range_attack,
        stock_notifications::{research as stock_research, Runtime},
    },
    research,
};
use c6_core::{
    city_production::{Order, QueueEntry},
    loaded_state::LoadedState,
    unit::PlotCoord,
};

pub struct Inputs<'a> {
    pub definitions: &'a Definitions,
    pub current: &'a Current<'a>,
    pub queries: &'a dyn production::selection::Queries,
    pub combat: &'a city_range_attack::Queries<'a>,
    pub workers: &'a crate::city_turn::upkeep::citizens::Definitions,
    pub appeal: &'a appeal::Definitions,
    pub appeal_subscribers: Option<&'a [u64]>,
    pub research: &'a research::Definitions,
    pub choices: &'a choose_tech::Definitions,
    pub tech_graph: &'a choose_tech::GameTechs,
    pub culture: Option<&'a stock_research::CultureInputs<'a>>,
    pub completion: &'a completion::Services<'a>,
    pub roads: &'a roads::Inputs<'a>,
}
#[derive(Clone, Copy, Debug)]
pub enum Report {
    Selected,
    QueueEntry(usize),
    Progress,
    Completed,
}
pub struct Effects<'s, 'runtime, 'a, P> {
    pub backend: &'s mut Runtime<'runtime, P>,
    pub research_runtime: &'s mut research::Runtime,
    pub inputs: &'a Inputs<'a>,
    pub gossip_history: &'s [c6_core::city_power_activation::gossip_delivery::TurnVisibility],
    pub reports: Vec<Report>,
    pub workers: Vec<crate::builder_improvement::workers::Report>,
}
pub(super) fn error(stage: &'static str, reason: impl std::fmt::Debug) -> production::Error {
    production::Error::DistrictService(stage, format!("{reason:?}"))
}
impl<P> Effects<'_, '_, '_, P> {
    pub fn choose(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        hash: u32,
        location: PlotCoord,
    ) -> Result<(), production::Error> {
        super::choose(
            state,
            owner,
            city,
            hash,
            location,
            self.inputs.definitions,
            self.inputs.current,
            self,
        )?;
        self.backend.decisions.retain(|d| {
            d.type_hash != 0x40fb81ed
                || d.owner != owner
                || !d.component.is_some_and(|c| {
                    c.component_type == 2 && i32::from(c.owner) == owner && c.id == city
                })
        });
        Ok(())
    }
    fn hash(&self, entry: &QueueEntry) -> Result<u32, production::Error> {
        match entry.order {
            Order::Zone { district_hash }
                if district_hash == self.inputs.definitions.campus_hash =>
            {
                Ok(district_hash)
            }
            _ => Err(production::Error::Unsupported(
                "non-Campus district production",
            )),
        }
    }
}
impl<P> production::Effects for Effects<'_, '_, '_, P> {
    fn population_services(&mut self) -> Option<&mut dyn crate::city_turn::PopulationEffects> {
        Some(self.backend)
    }
    fn calculate_yield_q8(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        y: i32,
        k: u32,
    ) -> Result<i32, production::Error> {
        self.backend
            .current_city_yield(s, o, c, y, k)
            .map_err(|e| error("district current yield", e))
    }
    fn current_build(
        &mut self,
        s: &LoadedState,
        o: i32,
        c: u32,
        e: &QueueEntry,
    ) -> Result<production::CurrentBuild, production::Error> {
        let hash = self.hash(e)?;
        let index = e.location[1]
            .wrapping_mul(s.world().map().header.width)
            .wrapping_add(e.location[0]);
        let mut build = self
            .inputs
            .definitions
            .production_parameters(s, o, c, hash)?;
        if self.inputs.queries.occupied_by_enemy(s, o, index)? {
            build.factors_q8 = vec![0];
        }
        Ok(build)
    }
    fn start_progress(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
        e: &QueueEntry,
    ) -> Result<(), production::Error> {
        self.hash(e)?;
        Ok(()) // ordinary Campus has no strategic consumption here
    }
    fn progress_report(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
        e: &QueueEntry,
    ) -> Result<(), production::Error> {
        self.hash(e)?;
        self.reports.push(Report::Progress);
        Ok(())
    }
    fn finish(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        e: &QueueEntry,
    ) -> Result<bool, production::Error> {
        let hash = self.hash(e)?;
        construction::complete(s, o, c, self.inputs.definitions, self)?;
        let queue = &mut s.city_mut(o, c)?.build_queue;
        queue.entries.remove(0);
        queue.previous_production_type_hash = hash;
        self.reports.push(Report::Completed);
        Ok(true)
    }
    fn new_front(&mut self, s: &mut LoadedState, o: i32, c: u32) -> Result<(), production::Error> {
        let Some(entry) = s.city(o, c)?.build_queue.entries.first().cloned() else {
            return Ok(());
        };
        let hash = self.hash(&entry)?;
        let location = PlotCoord {
            x: entry.location[0],
            y: entry.location[1],
        };
        if !super::can_choose(
            s,
            o,
            c,
            hash,
            location,
            self.inputs.definitions,
            self.inputs.current,
        )? {
            return Err(production::Error::IllegalSelection(entry.order));
        }
        construction::place(
            s,
            o,
            c,
            location,
            self.inputs.definitions,
            self.inputs.combat,
            self,
        )?;
        self.reports.push(Report::Selected);
        Ok(())
    }
}
impl<P> production::QueueEffects for Effects<'_, '_, '_, P> {
    fn queue_changed(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
        index: usize,
    ) -> Result<(), production::Error> {
        self.reports.push(Report::QueueEntry(index));
        Ok(())
    }
}
impl<P> districts::Effects for Effects<'_, '_, '_, P> {
    type Error = production::Error;
    fn player_district_completed(
        &mut self,
        _: &mut LoadedState,
        o: i32,
        _: i32,
        _: u32,
    ) -> Result<(), production::Error> {
        let count = self
            .inputs
            .completion
            .player_listeners
            .iter()
            .find(|r| r.0 == o)
            .ok_or(production::Error::Unsupported(
                "current Player.DistrictCompleted listeners",
            ))?
            .1;
        if count != 0 {
            return Err(production::Error::Unsupported(
                "Player.DistrictCompleted subscribers",
            ));
        }
        Ok(())
    }
    fn game_district_completed(
        &mut self,
        s: &mut LoadedState,
        d: districts::DistrictReference,
        b: bool,
    ) -> Result<(), production::Error> {
        completion::dispatch(self, s, d, b)
    }
}
impl<P> construction::Effects for Effects<'_, '_, '_, P> {
    fn complete_route(
        &mut self,
        state: &mut LoadedState,
        d: districts::DistrictReference,
    ) -> Result<(), production::Error> {
        roads::complete(self, state, d)
    }
    fn award_creation_yields(
        &mut self,
        s: &mut LoadedState,
        d: districts::DistrictReference,
    ) -> Result<(), production::Error> {
        let p = s
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == d.owner)
            .ok_or(production::Error::Unsupported("district creation owner"))?;
        if p.districts
            .creation_yields
            .iter()
            .any(|y| y.district_hash == self.inputs.definitions.campus_hash)
        {
            return Err(production::Error::Unsupported(
                "positive district creation yield",
            ));
        }
        Ok(())
    }
    fn recompute_cost(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        h: u32,
    ) -> Result<(), production::Error> {
        let map = self
            .inputs
            .current
            .district_maps
            .iter()
            .find(|m| m.owner == o)
            .ok_or(production::Error::Unsupported(
                "current district replacement owner",
            ))?;
        let mut effects = stock_research::Effects::new(
            self.backend,
            self.research_runtime,
            self.inputs.choices,
            self.inputs.culture,
        );
        effects.choice_graph = Some(self.inputs.tech_graph);
        research::Context {
            state: s,
            definitions: self.inputs.research,
            effects: &mut effects,
        }
        .recompute_placed_district_cost(o, h, map)
        .map_err(|e| error("placed district cost", e))
    }
    fn district_zoned(
        &mut self,
        _: &mut LoadedState,
        o: i32,
        _: i32,
    ) -> Result<(), production::Error> {
        self.inputs.completion.check_zoned(o)
    }
    fn reallocate_citizens(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
    ) -> Result<(), production::Error> {
        crate::city_turn::upkeep::citizens::reallocate(
            s,
            o,
            c,
            &mut crate::builder_improvement::workers::Current {
                backend: self.backend,
                definitions: self.inputs.workers,
                appeal: self.inputs.appeal,
                fallout: self.inputs.current.fallout_turns,
                appeal_subscribers: self.inputs.appeal_subscribers,
                reports: &mut self.workers,
            },
        )
        .map_err(|e| error("district citizen allocation", e))
    }
    fn completed_aftermath(
        &mut self,
        s: &mut LoadedState,
        d: districts::DistrictReference,
    ) -> Result<(), production::Error> {
        completion::aftermath(self, s, d)
    }
    fn starting_buildings(
        &mut self,
        s: &mut LoadedState,
        d: districts::DistrictReference,
    ) -> Result<(), production::Error> {
        self.inputs.completion.starting_buildings(
            s,
            d,
            self.research_runtime.start_era_hash,
            self.inputs.current.district_maps,
            self.inputs.definitions.campus_hash,
        )
    }
}
