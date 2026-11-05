//! External Train choices and ordinary production on the SAME stock owners.
use super::super::{movement, Runtime};
use crate::{
    city_production_turn::{self as production, unit_selection as selection, units},
    unit_creation as creation,
};
use c6_core::{
    city_production::{Order, QueueEntry},
    loaded_state::LoadedState,
    unit::ComponentId,
};
#[derive(Clone, Copy, Debug)]
pub enum CreatedConsumer {
    Achievement,
    Unsupported(u64),
}
#[derive(Clone, Copy, Debug)]
pub enum TrainedConsumer {
    Targets(i32),
    Unsupported(u64),
}
pub struct Signals {
    pub created: Vec<CreatedConsumer>,
    pub trained: Vec<(i32, Vec<TrainedConsumer>)>,
}
#[derive(Clone, Copy, Debug)]
pub enum Report {
    Front {
        request: u32,
        unit_index: i32,
        formation: i32,
    },
    QueueEntry {
        index: usize,
    },
    Progress,
    Completed {
        unit_index: i32,
        formation: i32,
    },
}
#[derive(Clone, Copy, Debug)]
pub struct Record {
    pub owner: i32,
    pub city: u32,
    pub report: Report,
}
impl<P> Runtime<'_, P> {
    pub fn bind_unit_production_signals(
        &mut self,
        state: &LoadedState,
        signals: Signals,
    ) -> Result<(), production::Error> {
        self.check(state).map_err(|_| production::Error::Binding)?;
        if self.unit_production_signals.is_some() {
            return Err(production::Error::Unsupported(
                "Unit production signals already bound",
            ));
        }
        self.unit_production_signals = Some(signals);
        Ok(())
    }
}
/// Selection needs only current legality/query owners; no factory is allocated.
pub struct Selection<'a, 'inputs, P = c6_core::unit::OperationParameters> {
    pub runtime: &'a mut Runtime<'inputs, P>,
    pub definitions: &'a selection::Definitions,
    pub current: selection::Current<'a>,
    pub reports: Vec<Record>,
}
impl<P> Selection<'_, '_, P> {
    pub fn choose(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        hash: u32,
    ) -> Result<(), production::Error> {
        self.runtime
            .check(state)
            .map_err(|_| production::Error::Binding)?;
        if self.runtime.city_query_inputs().is_none() {
            return Err(production::Error::Unsupported(
                "current Train report queries",
            ));
        }
        let current = selection::Current {
            build_maps: self.current.build_maps,
            fallout_turns: self.current.fallout_turns,
            civilization_levels: self.current.civilization_levels,
        };
        selection::append_unit(state, owner, city, hash, self.definitions, &current, self)?;
        super::resolve_city_production(&mut self.runtime.decisions, owner, city);
        Ok(())
    }
    fn report_queries(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
    ) -> Result<(), production::Error> {
        production::run_build_queue_report_queries(state, owner, city, self)
    }
}
impl<P> production::QueueReportQueries for Selection<'_, '_, P> {
    fn get_yield_q8(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        index: i32,
        kind: Option<u32>,
    ) -> Result<i32, production::Error> {
        self.runtime
            .current_city_yield(state, owner, city, index, kind.unwrap_or(u32::MAX))
            .map_err(|_| production::Error::Unsupported("current Train report yield"))
    }
}
impl<P> selection::FrontEffects for Selection<'_, '_, P> {
    fn unit_front_report(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        city: u32,
        request: u32,
        unit_index: i32,
        formation: i32,
    ) -> Result<(), production::Error> {
        self.report_queries(state, owner, city)?;
        self.reports.push(Record {
            owner,
            city,
            report: Report::Front {
                request,
                unit_index,
                formation,
            },
        });
        Ok(())
    }
}
impl<P> production::Effects for Selection<'_, '_, P> {
    fn calculate_yield_q8(
        &mut self,
        state: &mut LoadedState,
        o: i32,
        c: u32,
        index: i32,
        kind: u32,
    ) -> Result<i32, production::Error> {
        self.runtime
            .current_city_yield(state, o, c, index, kind)
            .map_err(|_| production::Error::Unsupported("current Train yield"))
    }
    fn current_build(
        &mut self,
        s: &LoadedState,
        o: i32,
        c: u32,
        e: &QueueEntry,
    ) -> Result<production::CurrentBuild, production::Error> {
        selection::current_build(s, o, c, e, self.definitions)
    }
    fn start_progress(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        e: &QueueEntry,
    ) -> Result<(), production::Error> {
        selection::start_progress(s, o, c, e, self.definitions)
    }
    fn progress_report(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        _: &QueueEntry,
    ) -> Result<(), production::Error> {
        self.report_queries(s, o, c)?;
        self.reports.push(Record {
            owner: o,
            city: c,
            report: Report::Progress,
        });
        Ok(())
    }
    fn finish(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
        _: &QueueEntry,
    ) -> Result<bool, production::Error> {
        Err(production::Error::Unsupported("Train completion factory"))
    }
    fn new_front(&mut self, s: &mut LoadedState, o: i32, c: u32) -> Result<(), production::Error> {
        let current = selection::Current {
            build_maps: self.current.build_maps,
            fallout_turns: self.current.fallout_turns,
            civilization_levels: self.current.civilization_levels,
        };
        selection::new_front(s, o, c, self.definitions, &current, self)
    }
}
impl<P> production::QueueEffects for Selection<'_, '_, P> {
    fn queue_changed(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        index: usize,
    ) -> Result<(), production::Error> {
        self.report_queries(s, o, c)?;
        self.reports.push(Record {
            owner: o,
            city: c,
            report: Report::QueueEntry { index },
        });
        Ok(())
    }
}
/// Short-lived factory inputs borrow current placement/sight and the ongoing
/// stock runtime. No state, effect store or requirement counter is copied.
pub struct Factory<'a, 'd, 'r> {
    pub definitions: &'a creation::Definitions,
    pub resources: &'a crate::resource_binding::ResourceAuthority<'r>,
    pub movement: &'a crate::unit_movement_state::Inputs<'d>,
    pub combat: &'a crate::formation_rules::Definitions,
    pub costs: &'a crate::research_unit_costs::Definitions,
    pub placement: &'a crate::unit_placement_state::Definitions<'d, 'r>,
    pub queries: &'a crate::unit_placement_state::Queries<'d>,
    pub runtime: &'a creation::Runtime,
    pub tags: &'a crate::loaded_actions::reporting::hold_cycling::commands::Tags,
    pub sight: &'a mut crate::unit_sight_state::Runtime,
    pub finish: &'a units::Definitions,
}
pub struct Effects<'a, 'inputs, 'd, 'r, P = c6_core::unit::OperationParameters> {
    pub selection: Selection<'a, 'inputs, P>,
    pub factory: Factory<'a, 'd, 'r>,
}
impl<P> production::Effects for Effects<'_, '_, '_, '_, P> {
    fn calculate_yield_q8(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        i: i32,
        k: u32,
    ) -> Result<i32, production::Error> {
        production::Effects::calculate_yield_q8(&mut self.selection, s, o, c, i, k)
    }
    fn current_build(
        &mut self,
        s: &LoadedState,
        o: i32,
        c: u32,
        e: &QueueEntry,
    ) -> Result<production::CurrentBuild, production::Error> {
        production::Effects::current_build(&mut self.selection, s, o, c, e)
    }
    fn start_progress(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        e: &QueueEntry,
    ) -> Result<(), production::Error> {
        production::Effects::start_progress(&mut self.selection, s, o, c, e)
    }
    fn progress_report(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        e: &QueueEntry,
    ) -> Result<(), production::Error> {
        production::Effects::progress_report(&mut self.selection, s, o, c, e)
    }
    fn finish(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        c: u32,
        e: &QueueEntry,
    ) -> Result<bool, production::Error> {
        if !matches!(e.order, Order::Train { .. }) {
            return Err(production::Error::Unsupported(
                "non-Train stock unit completion",
            ));
        }
        let cost = selection::current_build(s, o, c, e, self.selection.definitions)?.cost;
        let f = &mut self.factory;
        let mut factory = creation::Context {
            state: s,
            definitions: f.definitions,
            resources: f.resources,
            movement: f.movement,
            combat: f.combat,
            costs: f.costs,
            placement: f.placement,
            queries: f.queries,
            runtime: f.runtime,
            tags: f.tags,
            sight: f.sight,
            effects: self.selection.runtime,
        };
        let result = units::finish_current(
            &mut factory,
            f.finish,
            e,
            cost,
            ComponentId {
                owner: o as i16,
                component_type: 2,
                id: c,
            },
        );
        if let Err(error) = &result {
            self.selection.runtime.last_error = Some(format!("Train completion: {error:?}"));
        }
        result.map_err(|error| production::Error::UnitProduction(Box::new(error)))
    }
    fn new_front(&mut self, s: &mut LoadedState, o: i32, c: u32) -> Result<(), production::Error> {
        production::Effects::new_front(&mut self.selection, s, o, c)
    }
}
impl<P> units::Effects for Runtime<'_, P> {
    fn production_unit_created(
        &mut self,
        s: &mut LoadedState,
        u: ComponentId,
        c: ComponentId,
    ) -> Result<(), units::Error<movement::Error>> {
        let signals = self
            .unit_production_signals
            .as_ref()
            .ok_or(units::Error::Missing(
                "current production UnitCreated signals",
            ))?;
        let input = self
            .unit_creation
            .as_ref()
            .ok_or(units::Error::Missing("current Unit factory metadata"))?;
        for listener in &signals.created {
            match listener {
                CreatedConsumer::Achievement => {
                    creation::production_effects::stock_created_in_city(
                        s,
                        input.definitions,
                        (i32::from(u.owner), u.id),
                        c,
                    )
                    .map_err(|e| units::Error::Creation(Box::new(e)))?
                }
                CreatedConsumer::Unsupported(_) => {
                    return Err(units::Error::Missing(
                        "unsupported production UnitCreated listener",
                    ))
                }
            }
        }
        Ok(())
    }
    fn player_production_unit_created(
        &mut self,
        s: &mut LoadedState,
        o: i32,
        u: ComponentId,
        _: u32,
    ) -> Result<(), units::Error<movement::Error>> {
        let listeners = self
            .unit_production_signals
            .as_ref()
            .ok_or(units::Error::Missing("current Player trained signals"))?
            .trained
            .iter()
            .find(|v| v.0 == o)
            .ok_or(units::Error::Missing("current Player trained owner"))?;
        let input = self
            .unit_creation
            .as_ref()
            .ok_or(units::Error::Missing("current Unit factory metadata"))?;
        for listener in &listeners.1 {
            match *listener {
                TrainedConsumer::Targets(observer) => {
                    creation::production_effects::player_unit_created(
                        s,
                        input.definitions,
                        observer,
                        (i32::from(u.owner), u.id),
                    )
                    .map_err(|e| units::Error::Creation(Box::new(e)))?
                }
                TrainedConsumer::Unsupported(_) => {
                    return Err(units::Error::Missing("unsupported Player trained listener"))
                }
            }
        }
        Ok(())
    }
    fn production_completed_report(
        &mut self,
        s: &mut LoadedState,
        c: ComponentId,
        _: i32,
        _: i32,
    ) -> Result<(), units::Error<movement::Error>> {
        self.check(s)
            .map_err(|_| units::Error::Missing("Train report runtime binding"))?;
        s.city(i32::from(c.owner), c.id)
            .map_err(units::Error::State)?;
        // Source-qualified stock OnProductionCompleted consumers publish recomputable
        // queue/production caches. The live empty queue yields to external choice.
        Ok(())
    }
}
