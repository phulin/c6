//! Concrete ordinary Farm Repair callbacks on canonical stock owners.
use super::{Definitions, Report};
use crate::{
    loaded_actions::{self as actions, move_to, stock_notifications::Runtime},
    unit_sight_state,
};
use c6_core::{
    loaded_state::LoadedState,
    map::UnitReference,
    unit::{ComponentId, OperationNode, PlotCoord, Unit},
};
#[derive(Debug)]
pub struct Error(pub &'static str, pub String);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Repair {}: {}", self.0, self.1)
    }
}
impl std::error::Error for Error {}
fn err(stage: &'static str, e: impl std::fmt::Debug) -> Error {
    Error(stage, format!("{e:?}"))
}
/// Ordered actual registrations, bound from the admitted initialization epoch.
#[derive(Clone, Copy, Debug)]
pub enum Subscriber {
    AttachModifiers,
    Economic,
    Resources { owner: i32 },
    Lua,
    ImprovementConstructed { boost_row: i64 },
}
pub struct Callbacks {
    binding: c6_core::core_player::initialization::WireBinding,
    order: Vec<Subscriber>,
}
impl Callbacks {
    pub fn from_initial(state: &LoadedState, order: Vec<Subscriber>) -> Self {
        Self {
            binding: state.binding().clone(),
            order,
        }
    }
}
pub struct Dispatcher<'a, 'd> {
    pub definitions: &'a Definitions,
    pub callbacks: &'a Callbacks,
    pub capabilities: &'a c6_core::game_capabilities::Membership,
    pub drought_ids: &'a [i32],
    pub upkeep: &'a crate::unit_movement_state::Inputs<'d>,
    pub appeal: &'a crate::city_production_turn::appeal::Definitions,
    pub appeal_subscribers: Option<&'a [u64]>,
    pub presentation_reports_qualified: bool,
    pub fortify_turn_max: i32,
    pub reports: &'a mut Vec<Output>,
}
#[derive(Clone, Copy, Debug)]
pub enum Output {
    Operation(Report),
    Appeal { plot: u32, value: i32 },
    AppealReport(u32),
}
struct Current<'a, 's, 'r, 'd, P> {
    dispatcher: &'a mut Dispatcher<'s, 'd>,
    backend: &'a mut Runtime<'r, P>,
    sight: &'a mut unit_sight_state::Runtime,
}
fn reference(actor: (i32, u32)) -> UnitReference {
    UnitReference::from_component_id(ComponentId {
        owner: actor.0 as i16,
        component_type: 1,
        id: actor.1,
    })
}
impl<P> super::Effects for Current<'_, '_, '_, '_, P> {
    type Error = Error;
    fn observer_can_see_owner(&self, state: &LoadedState, owner: i32) -> Result<bool, Error> {
        let observer = self
            .sight
            .local_observer
            .ok_or_else(|| err("observer", "missing"))?;
        if observer != state.retained().core.local_player && observer != 1000 {
            return Err(err("observer", "changed current observer"));
        }
        Ok(crate::melee_attack::experience::reporting::observer_can_see_player(observer, owner))
    }
    fn report(&mut self, state: &mut LoadedState, report: Report) -> Result<(), Error> {
        match report {
            Report::Operation { unit, improvement } => {
                crate::loaded_upkeep::OperationReports::operation_report(
                    self.backend,
                    state,
                    actions::IdentityReport {
                        event: 0x3d4006f2,
                        subtype: super::OPERATION_HASH,
                        identity: unit,
                        first: Some(improvement),
                        second: None,
                    },
                )
                .map_err(|e| err("operation report", e))?;
            }
            Report::ImprovementChanged { plot } => {
                let query = self
                    .backend
                    .city_query_inputs()
                    .ok_or_else(|| err("improvement report", "canonical city query owner"))?;
                let mut territory = query
                    .current
                    .territory
                    .write()
                    .map_err(|e| err("mutable territory", e))?;
                actions::pillage::reporting::improvement_changed(
                    state,
                    &mut territory,
                    &self.dispatcher.definitions.reporting,
                    plot,
                )
                .map_err(|e| err("improvement yield reporting", e))?;
            }
            _ => {}
        }
        if !self.dispatcher.presentation_reports_qualified {
            return Err(err("reports", "unqualified recipients"));
        }
        self.dispatcher.reports.push(Output::Operation(report));
        Ok(())
    }
    fn repaired(
        &mut self,
        state: &mut LoadedState,
        plot: usize,
        improvement: u32,
    ) -> Result<(), Error> {
        if state.binding() != &self.dispatcher.callbacks.binding {
            return Err(err("callbacks", "binding"));
        }
        for subscriber in &self.dispatcher.callbacks.order {
            let p = state
                .world()
                .map()
                .plots
                .get(plot)
                .ok_or_else(|| err("callback plot", "missing"))?;
            if p.improvement_hash != improvement
                || p.flags.get(c6_core::map::PlotFlag::ImprovementPillaged)
            {
                return Err(err("callback plot", "changed improvement state"));
            }
            let owner = crate::movement_placement_state::plot_owner(p)
                .map_err(|e| err("callback owner", e))?;
            match *subscriber {
                //23e1ab0 queries SQL ImprovementModifiers before constructing
                //the temporary modifier wrapper. Farm's empty rows return first.
                Subscriber::AttachModifiers => {
                    if self
                        .dispatcher
                        .definitions
                        .modifier_improvements
                        .contains(&improvement)
                    {
                        return Err(err(
                            "attach modifiers",
                            "positive improvement modifier rows",
                        ));
                    }
                }
                //2648290 recalculates Economic values first; disabled game mode
                //returns before summaries. Ordinary Farm is no Industry/Corporation.
                Subscriber::Economic => {
                    if self.dispatcher.capabilities.has(0x10c40410) {
                        return Err(err("Economic", "active recalculation"));
                    }
                }
                //2b709c0 own-owner resource extraction. Resource-free Farm's
                //old/new both NO_RESOURCE, HandleResourceChange returns early.
                Subscriber::Resources { owner: receiver } => {
                    if receiver == owner && p.resource_hash != u32::MAX {
                        return Err(err("Resources", "positive extraction change"));
                    }
                }
                Subscriber::ImprovementConstructed { boost_row } => {
                    if owner == -1 {
                        continue;
                    }
                    let boost = self
                        .dispatcher
                        .definitions
                        .boosts
                        .iter()
                        .find(|b| b.row == boost_row)
                        .ok_or_else(|| err("boost", "missing rule"))?;
                    if boost.improvement == Some(improvement) {
                        let mut count = 0i32;
                        for current in &state.world().map().plots {
                            let current_owner =
                                crate::movement_placement_state::plot_owner(current)
                                    .map_err(|e| err("boost current plot owner", e))?;
                            if current_owner == owner
                                && !current
                                    .flags
                                    .get(c6_core::map::PlotFlag::ImprovementPillaged)
                                && current.improvement_hash == improvement
                            {
                                count = count.wrapping_add(1);
                            }
                        }
                        if count >= boost.items
                            && (!boost.requires_resource || p.resource_hash != u32::MAX)
                            && (!boost.resource_specific || boost.resource == Some(p.resource_hash))
                        {
                            return Err(err("boost", "positive TriggerBoost continuation"));
                        }
                    }
                }
                Subscriber::Lua => self
                    .backend
                    .script
                    .call_empty(
                        state,
                        "OnImprovementRepaired",
                        &[plot as i32, self.dispatcher.definitions.improvement_index],
                    )
                    .map_err(|e| err("Lua", e))?,
            }
        }
        Ok(())
    }
    fn messages_disabled(&self) -> bool {
        self.sight.messages_disabled
    }
    fn local_plot_visible(&self, state: &LoadedState, actor: UnitReference) -> Result<bool, Error> {
        let u = state
            .world()
            .unit(actor)
            .map_err(|e| err("visible unit", e))?
            .ok_or_else(|| err("visible unit", "missing"))?
            .record;
        let observer = self
            .sight
            .local_observer
            .ok_or_else(|| err("visible observer", "missing"))?;
        let shared = self
            .sight
            .shared(observer)
            .map_err(|e| err("shared sight", e))?;
        let layer = shared
            .0
            .iter()
            .find(|l| l.type_hash == c6_core::visibility_change::BASE_LAYER_HASH)
            .ok_or_else(|| err("base sight", "missing"))?;
        let index = layer
            .grid_width
            .wrapping_mul(u.k_location.y as u32)
            .wrapping_add(u.k_location.x as u32) as usize;
        Ok(layer.values.get(index).copied().unwrap_or(0) > 0)
    }
    fn update_appeal(&mut self, state: &mut LoadedState, plot: usize) -> Result<(), Error> {
        let (x, y) = state
            .world()
            .map()
            .coordinates(plot as u32)
            .ok_or_else(|| err("appeal plot", "coordinates"))?;
        crate::city_production_turn::appeal::update(state, [x, y], self.dispatcher.appeal, self)
            .map(|_| ())
            .map_err(|e| err("appeal", e))
    }
    fn finish_moves(&mut self, state: &mut LoadedState, actor: UnitReference) -> Result<(), Error> {
        crate::loaded_upkeep::finish_unit_moves(
            state,
            actor.player(),
            actor.id,
            self.dispatcher.upkeep,
            self.backend,
        )
        .map_err(|e| err("FinishMoves", e))
    }
}
impl<P> crate::city_production_turn::appeal::Effects for Current<'_, '_, '_, '_, P> {
    type Error = Error;
    fn appeal_changed(&mut self, _: &mut LoadedState, plot: u32, value: i32) -> Result<(), Error> {
        if self.dispatcher.appeal_subscribers != Some(&[]) {
            return Err(err("appeal signal", "unknown/positive subscribers"));
        }
        self.dispatcher.reports.push(Output::Appeal { plot, value });
        Ok(())
    }
    fn appeal_report(&mut self, _: &mut LoadedState, plot: u32) -> Result<(), Error> {
        if !self.dispatcher.presentation_reports_qualified {
            return Err(err("appeal report", "unqualified recipients"));
        }
        self.dispatcher.reports.push(Output::AppealReport(plot));
        Ok(())
    }
}
impl<P> move_to::Movement<Runtime<'_, P>> for Dispatcher<'_, '_> {
    type Error = Error;
    fn repair_can_start(
        &mut self,
        state: &LoadedState,
        unit: &Unit,
        visible: bool,
        _: &Runtime<'_, P>,
    ) -> Result<Option<bool>, Error> {
        super::can_start(
            state,
            self.definitions,
            &super::Inputs {
                drought_ids: self.drought_ids,
            },
            unit,
            visible,
        )
        .map(Some)
        .map_err(|e| err("CanStart", e))
    }
    fn start_repair(
        &mut self,
        state: &mut LoadedState,
        sight: &mut unit_sight_state::Runtime,
        actor: (i32, u32),
        node: &OperationNode,
        backend: &mut Runtime<'_, P>,
    ) -> Result<Option<c6_core::skip_turn::StartData>, Error> {
        let defs = self.definitions;
        super::start(
            state,
            &mut Current {
                dispatcher: self,
                backend,
                sight,
            },
            defs,
            reference(actor),
            node,
        )
        .map(Some)
        .map_err(|e| err("Start", e))
    }
    fn fortify_turn_max(&self) -> i32 {
        self.fortify_turn_max
    }
    fn is_fortifyable(&self, _: &LoadedState, _: (i32, u32)) -> Result<bool, Error> {
        Err(err("other handler", "Fortify"))
    }
    fn alert_enemy_visible(
        &self,
        _: &LoadedState,
        _: &unit_sight_state::Runtime,
        _: (i32, u32),
    ) -> Result<bool, Error> {
        Err(err("other handler", "Alert"))
    }
    fn city_at(
        &self,
        _: &LoadedState,
        _: PlotCoord,
    ) -> Result<Option<c6_core::trade::ComponentId>, Error> {
        Err(err("other handler", "Trade"))
    }
    fn preflight(&self, _: &LoadedState, _: (i32, u32), _: move_to::Request) -> Result<(), Error> {
        Err(err("other handler", "MoveTo"))
    }
    fn test_move_to(
        &mut self,
        _: &mut LoadedState,
        _: &unit_sight_state::Runtime,
        _: (i32, u32),
        _: PlotCoord,
        _: u32,
    ) -> Result<i32, Error> {
        Err(err("other handler", "MoveTo"))
    }
    fn move_to(
        &mut self,
        _: &mut LoadedState,
        _: &mut unit_sight_state::Runtime,
        _: (i32, u32),
        _: PlotCoord,
        _: u32,
        _: &mut Runtime<'_, P>,
    ) -> Result<i32, Error> {
        Err(err("other handler", "MoveTo"))
    }
    fn valid_location(
        &mut self,
        _: &LoadedState,
        _: &unit_sight_state::Runtime,
        _: (i32, u32),
        _: PlotCoord,
    ) -> Result<i32, Error> {
        Err(err("other handler", "MoveTo"))
    }
}
