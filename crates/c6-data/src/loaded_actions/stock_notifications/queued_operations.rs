//! Persistent queued-operation continuation inside Player.DoAutomation.
//! This resumes the existing manager on the current linked Unit; it never
//! constructs another request, replaces the world, or selects a policy.
use super::{movement, Runtime};
use crate::{game_update_state::automation, loaded_actions::{self as actions, move_to}};
use c6_core::{loaded_state::{game_update::automation::Frame, LoadedState}, unit::OperationParameters};

pub trait Services<P> {
    fn advance(&mut self, state: &mut LoadedState, backend: &mut Runtime<'_, P>,
        frame: &mut Frame, readiness: &actions::reporting::Provider<'_>) -> Result<(), String>;
}
/// Existing rules, current visibility and path-search owners retained across
/// requests and turns. No captured handler answer or AFTER value is supplied.
pub struct Current<'s, 'a, 'r> {
    pub inputs: &'s actions::Inputs<'s, 'r>,
    pub handlers: &'s actions::handlers::BoundHandlers<'s>,
    pub path: &'s crate::formation_path_state::Definitions<'a, 'r>,
    pub placement: &'s crate::unit_placement_state::Definitions<'a, 'r>,
    pub queries: &'s crate::unit_placement_state::Queries<'a>,
    pub search: &'s std::cell::RefCell<c6_core::pathfinder::TwoLayer>,
    pub sight: &'s std::cell::RefCell<crate::unit_sight_state::Runtime>,
    pub manager_width: u32,
}
impl Services<OperationParameters> for Current<'_, '_, '_> {
    fn advance(&mut self, state: &mut LoadedState, backend: &mut Runtime<'_, OperationParameters>,
        frame: &mut Frame, readiness: &actions::reporting::Provider<'_>) -> Result<(), String> {
        let mut sight=self.sight.try_borrow_mut().map_err(|e|format!("queued sight: {e}"))?;
        let mut search=self.search.try_borrow_mut().map_err(|e|format!("queued search: {e}"))?;
        let mut movement=move_to::ComputedMovement {path:self.path,placement:self.placement,
            queries:self.queries,search:&mut search,execution:crate::formation_path_state::ExecutionContext::Exclusive};
        let result=automation::advance_player_unit(state,frame,readiness,self.inputs,self.handlers,
            &mut move_to::Runtime{visibility:&mut sight,manager_width:self.manager_width},backend,
            &mut movement,automation::Scope::StockLinuxXp2RegisteredHandlers,
            Some(c6_core::loaded_state::game_update::deactivation::LoggingScope::StockFileSink))
            .map_err(|e|format!("queued unit: {e:?}"))?;
        match result {automation::UnitProgress::Advanced=>Ok(()),
            automation::UnitProgress::NoUnit=>Err("queued current Unit disappeared before continuation".into())}
    }
}
impl<'a,P> Runtime<'a,P> {
    pub fn bind_queued_operations(&mut self,state:&LoadedState,services:Box<dyn Services<P>+'a>) -> Result<(),movement::Error> {
        self.check(state).map_err(|_|movement::Error::Missing("queued operation binding"))?;
        if self.queued_operations.is_some(){return Err(movement::Error::Missing("queued operations already bound"));}
        self.queued_operations=Some(services);Ok(())
    }
    pub(crate) fn continue_queued_operation(&mut self,state:&mut LoadedState,frame:&mut Frame,
        readiness:&actions::reporting::Provider<'_>) -> Result<(),String> {
        let mut services=self.queued_operations.take().ok_or("current queued-operation services")?;
        let result=services.advance(state,self,frame,readiness);
        self.queued_operations=Some(services);
        result
    }
}
