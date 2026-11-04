//! Stock headless joins for current-player deactivation and next activation.
//! These are internal phases; no function certifies a complete turn by itself.
use super::{turn_reports::Scope, Runtime};
use crate::loaded_actions::configuration::Configuration;
use crate::{game_update_state as data, Rules};
use c6_core::{
    game_update::Error,
    loaded_state::{game_update as loaded, LoadedState},
};

struct ResourceEffects {
    speed: u32,
    uninitializing: bool,
}
impl data::deactivation::resources::Effects for ResourceEffects {
    type Error = &'static str;
    fn game_speed(&mut self, _: &LoadedState) -> Result<u32, Self::Error> {
        Ok(self.speed)
    }
    fn game_uninitializing(&mut self, _: &LoadedState) -> Result<bool, Self::Error> {
        Ok(self.uninitializing)
    }
    fn allocate_luxuries(&mut self, _: &mut LoadedState, _: i32) -> Result<(), Self::Error> {
        Err("current luxury allocation")
    }
    fn resource_changed(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: i32,
        _: i32,
        _: i32,
    ) -> Result<(), Self::Error> {
        Err("current ResourceChanged signal")
    }
    fn report_resource(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: i32,
        _: u32,
    ) -> Result<(), Self::Error> {
        Err("current resource report consumers")
    }
}
impl<P> Runtime<'_, P> {
    pub fn select_next_player(
        &self,
        state: &LoadedState,
        configuration: &Configuration<'_>,
        phases: &data::activation::Phases,
    ) -> Result<loaded::activation::Selection, Error> {
        let alive = self
            .player_services
            .as_ref()
            .and_then(|s| s.alive_players.as_ref())
            .ok_or(Error::Unsupported("current next-player alive order"))?;
        let network = configuration
            .is_network_multiplayer(state.binding())
            .map_err(|_| Error::Unsupported("current multiplayer mode"))?;
        loaded::activation::select(state, network, alive.values(), |state| phases.mode(state))
    }

    /// After automation, diplomacy, visibility recording and controller cleanup.
    pub fn finish_deactivation_resources(
        &mut self,
        state: &mut LoadedState,
        rules: &Rules,
        owner: i32,
        speed: u32,
        uninitializing: bool,
        scope: Scope,
    ) -> Result<usize, Error> {
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Unsupported("current deactivation player"))?;
        if !player.cities.records.entries.is_empty() {
            return Err(Error::Unsupported("current city deactivation choices"));
        }
        let order = player
            .resources
            .resource_ownership
            .iter()
            .map(|e| e.resource_hash)
            .collect::<Vec<_>>();
        let definitions =
            data::deactivation::resources::Definitions::from_rules::<&'static str>(rules, &order)
                .map_err(|_| Error::Unsupported("current deactivation resource definitions"))?;
        let changed = data::deactivation::resources::deactivate(
            state,
            owner,
            &definitions,
            &mut ResourceEffects {
                speed,
                uninitializing,
            },
        )
        .map_err(|_| Error::Unsupported("current resource deactivation effect"))?;
        self.deliver_deactivated_turn_report(state, owner, scope)
            .map_err(|_| Error::Unsupported("current deactivated turn report"))?;
        Ok(changed)
    }
    /// Source unit suffix followed by Formation.RestoreMovement, after the
    /// earlier activation subsystems and resource notifications have returned.
    pub fn restore_player_movement(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        units: &[u32],
        formations: &[u32],
        movement: &crate::unit_movement_state::Inputs<'_>,
        formation_definitions: &crate::formation_rules::Definitions,
    ) -> Result<(), Error> {
        for &id in units {
            crate::loaded_upkeep::activate_unit(
                state,
                owner,
                id,
                movement,
                formation_definitions,
                self,
            )
            .map_err(|_| Error::Unsupported("activation current Unit restore suffix"))?;
        }
        for &id in formations {
            crate::loaded_upkeep::restore_formation_movement(state, owner, id, movement, self)
                .map_err(|_| Error::Unsupported("activation current Formation restore suffix"))?;
        }
        Ok(())
    }
    /// Set active, apply the source hotseat local-player/observer change, then
    /// emit activation reporting. First-activation upkeep still follows.
    pub fn activate_selected_player(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut loaded::Runtime,
        configuration: &Configuration<'_>,
        owner: i32,
        visibility_messages_disabled: bool,
        scope: Scope,
    ) -> Result<(), Error> {
        let Some(active) = loaded::activation::begin_active(state, owner)? else {
            return Ok(());
        };
        let output = loaded::activation::after_stock_log(
            state,
            runtime,
            active,
            loaded::deactivation::LoggingScope::StockFileSink,
            |state| {
                configuration
                    .is_hotseat(state.binding())
                    .map_err(|_| Error::Unsupported("current hotseat mode"))
            },
            |state, runtime, owner| {
                let change = loaded::activation::local_player::set(state, runtime, owner)?;
                let backend = std::cell::RefCell::new(&mut *self);
                change
                    .deliver(
                        state,
                        runtime,
                        |_| Ok(visibility_messages_disabled),
                        |state, event, owner| {
                            backend
                                .borrow_mut()
                                .deliver_turn_report(state, event, owner, scope)
                        },
                        |state, observer| {
                            backend
                                .borrow_mut()
                                .restore_turn_visibility(state, observer, scope)
                        },
                    )
                    .map_err(|_| Error::Unsupported("hotseat observer refresh"))
            },
        )?;
        output
            .deliver(state, |state, event, owner| {
                self.deliver_turn_report(state, event, owner, scope)
            })
            .map_err(|_| Error::Unsupported("current activation report"))
    }
}
