//! Stock headless turn reports on the persistent action backend.
//! Gameplay query dependencies execute against current canonical state. Native
//! cache objects, telemetry DTOs and client playback are omitted in this scope.
use super::{reporting, Runtime};
use c6_core::{loaded_state::LoadedState, map::UnitReference, unit::ComponentId};

#[derive(Clone, Copy, Debug)]
pub enum Scope {
    StockPlainHeadlessExternalPoliciesNoAddedConsumers,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Record {
    /// Source-ordered report attempt, before its synchronous current queries.
    Report { event: u32, player: i32 },
    RestoreVisibility {
        observer: i32,
        mask: u32,
        first_reveal: bool,
    },
}
#[derive(Debug)]
pub enum Error {
    Binding(super::processing::Error),
    World(c6_core::world::MissingInput),
    Reporting(crate::loaded_actions::Error),
    Routes(reporting::yield_monitor::Error),
    UnsupportedEvent(u32),
    OwnerRange(i32),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stock turn report: {self:?}")
    }
}
impl std::error::Error for Error {}
const INACTIVE_LOCAL: u32 = 0x6e63a083;
const INACTIVE_REMOTE: u32 = 0xf16721cf;
const DEACTIVATED: u32 = 0x6c4dc5c4;
const ACTIVATED_LOCAL: u32 = 0x11c07b90;
const ACTIVATED_REMOTE: u32 = 0xd7b3572b;
const LOCAL_CHANGED: u32 = 0xc0372152;

impl<P> Runtime<'_, P> {
    /// Fixed stock cache/host report families. This never updates Stats, runs
    /// Emergency selection or chooses a player policy. Unknown consumers are
    /// outside the explicit stock scope, rather than silently supported.
    pub fn deliver_turn_report(
        &mut self,
        state: &LoadedState,
        event: u32,
        player: i32,
        _scope: Scope,
    ) -> Result<(), Error> {
        self.check(state).map_err(Error::Binding)?;
        if !matches!(
            event,
            INACTIVE_LOCAL
                | INACTIVE_REMOTE
                | DEACTIVATED
                | ACTIVATED_LOCAL
                | ACTIVATED_REMOTE
                | LOCAL_CHANGED
        ) {
            return Err(Error::UnsupportedEvent(event));
        }
        self.turn_report_records
            .push(Record::Report { event, player });
        if event == DEACTIVATED {
            // Source order is Victory progress snapshot (stored-state reads,
            // output only), followed by current Unit snapshot queries. Reuse
            // the existing canonical collection/Provider path, no copied DTO.
            let owner = i16::try_from(player).map_err(|_| Error::OwnerRange(player))?;
            let units = state
                .world()
                .player(player)
                .map_err(Error::World)?
                .unit_records()
                .map_err(Error::World)?;
            for unit in units.iter() {
                self.reporting
                    .observe_unit_snapshot(
                        state,
                        UnitReference::from_component_id(ComponentId {
                            owner,
                            component_type: 1,
                            id: unit.value.id,
                        }),
                    )
                    .map_err(Error::Reporting)?;
            }
        } else {
            // Current Game/Player/Stats/Emergency cache families share the
            // established warm route domain. No captured score/path answers
            // or native cache layout are installed by this read.
            reporting::yield_monitor::warm_route_domain(state).map_err(Error::Routes)?;
        }
        self.reached
            .push("stock current turn report consumers complete");
        Ok(())
    }
    pub fn deliver_inactive_turn_report(
        &mut self,
        state: &LoadedState,
        event: u32,
        player: i32,
        scope: Scope,
    ) -> Result<(), Error> {
        if !matches!(event, INACTIVE_LOCAL | INACTIVE_REMOTE) {
            return Err(Error::UnsupportedEvent(event));
        }
        self.deliver_turn_report(state, event, player, scope)
    }
    pub fn deliver_deactivated_turn_report(
        &mut self,
        state: &LoadedState,
        player: i32,
        scope: Scope,
    ) -> Result<(), Error> {
        self.deliver_turn_report(state, DEACTIVATED, player, scope)
    }
    pub fn deliver_activated_turn_report(
        &mut self,
        state: &LoadedState,
        event: u32,
        player: i32,
        scope: Scope,
    ) -> Result<(), Error> {
        if !matches!(event, ACTIVATED_LOCAL | ACTIVATED_REMOTE) {
            return Err(Error::UnsupportedEvent(event));
        }
        self.deliver_turn_report(state, event, player, scope)
    }
    pub fn deliver_local_player_change(
        &mut self,
        state: &LoadedState,
        player: i32,
        scope: Scope,
    ) -> Result<(), Error> {
        self.deliver_turn_report(state, LOCAL_CHANGED, player, scope)
    }
    /// RestoreVisibility's fixed mask0xfb/firstReveal=false refresh. Content
    /// discovery is excluded by mask4; natural-wonder CompleteMovement requires
    /// firstReveal. The remaining stock work updates renderer/cache output and
    /// leaves gameplay sight/reveal counts and game RNG untouched.
    pub fn restore_turn_visibility(
        &mut self,
        state: &LoadedState,
        observer: i32,
        _scope: Scope,
    ) -> Result<(), Error> {
        self.check(state).map_err(Error::Binding)?;
        state.world().player(observer).map_err(Error::World)?;
        self.turn_report_records.push(Record::RestoreVisibility {
            observer,
            mask: 0xfb,
            first_reveal: false,
        });
        self.reached
            .push("stock observer renderer visibility refresh");
        Ok(())
    }
}
