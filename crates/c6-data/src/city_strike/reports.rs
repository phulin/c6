//! Typed city-strike publications. These describe report output, not completion
//! of command gameplay or the separate end-turn-blocking update.
use super::Generated;
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::{LoadedState, ObservedSnapshotWitness},
    map::UnitReference,
    unit::ComponentId,
};

/// Explicit supported report environment; added consumers need their effects.
#[derive(Clone, Copy, Debug)]
pub enum Profile {
    StockPlainHeadlessExternalPoliciesNoAddedConsumers,
}
/// Report qualification belongs to the same canonical state origin.
#[derive(Clone, Debug)]
pub struct Scope {
    binding: WireBinding,
    witness: ObservedSnapshotWitness,
}
impl Scope {
    /// The caller must qualify the stock city and district report registrations,
    /// host subscribers and all reached source leaves for this observed origin.
    /// A profile assertion alone does not perform that private qualification.
    pub fn from_qualified_origin(state: &LoadedState, _: Profile) -> Option<Self> {
        Some(Self {
            binding: state.binding().clone(),
            witness: state.observed_snapshot()?.clone(),
        })
    }
    pub fn matches(&self, state: &LoadedState) -> bool {
        state.binding() == &self.binding && state.observed_snapshot() == Some(&self.witness)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Output {
    /// Ordered attacker/defender status messages with native localization omitted.
    SurvivingStatuses {
        attacker: ComponentId,
        defender: UnitReference,
        damage: i32,
        status: u32,
    },
    DistrictCombatChanged {
        district: ComponentId,
        subtype: u32,
    },
    CityCommandStarted {
        city: ComponentId,
        district: ComponentId,
        command_hash: u32,
        value: i32,
    },
}
pub trait Effects {
    type Error;
    fn city_surviving_statuses(
        &mut self,
        state: &mut LoadedState,
        generated: &Generated,
        status: u32,
    ) -> Result<(), Self::Error>;
    fn district_combat_changed(
        &mut self,
        state: &mut LoadedState,
        district: ComponentId,
    ) -> Result<(), Self::Error>;
    fn city_command_started(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        district: ComponentId,
        command_hash: u32,
    ) -> Result<(), Self::Error>;
}

impl<P> crate::loaded_actions::stock_notifications::Runtime<'_, P> {
    pub fn bind_city_strike_reports(
        &mut self,
        state: &LoadedState,
        scope: Scope,
    ) -> Result<(), crate::loaded_actions::stock_notifications::movement::Error> {
        use crate::loaded_actions::stock_notifications::movement::Error;
        self.check(state)
            .map_err(|e| Error::Requirement(format!("{e:?}")))?;
        if !scope.matches(state) {
            return Err(Error::Missing("city strike report origin"));
        }
        self.combat_mut()?.city_strike_report_scope = Some(scope);
        Ok(())
    }
    fn city_strike_report_origin(
        &self,
        state: &LoadedState,
    ) -> Result<(), crate::loaded_actions::stock_notifications::movement::Error> {
        use crate::loaded_actions::stock_notifications::movement::Error;
        self.check(state)
            .map_err(|e| Error::Requirement(format!("{e:?}")))?;
        if !self
            .combat()?
            .city_strike_report_scope
            .as_ref()
            .is_some_and(|s| s.matches(state))
        {
            return Err(Error::Missing("qualified city strike report scope"));
        }
        Ok(())
    }
}
impl<P> Effects for crate::loaded_actions::stock_notifications::Runtime<'_, P> {
    type Error = crate::loaded_actions::stock_notifications::movement::Error;
    fn city_surviving_statuses(
        &mut self,
        state: &mut LoadedState,
        generated: &Generated,
        status: u32,
    ) -> Result<(), Self::Error> {
        use crate::loaded_actions::stock_notifications::movement::Error;
        self.city_strike_report_origin(state)?;
        self.combat_mut()?
            .city_strike_reports
            .push(Output::SurvivingStatuses {
                attacker: generated.attacker,
                defender: generated.defender,
                damage: generated.damage,
                status,
            });
        if self.city_strike_notification_required(state, generated.defender)? {
            return Err(Error::Missing(
                "human multiplayer city combat notification creation",
            ));
        }
        Ok(())
    }
    fn district_combat_changed(
        &mut self,
        state: &mut LoadedState,
        district: ComponentId,
    ) -> Result<(), Self::Error> {
        use crate::loaded_actions::stock_notifications::movement::Error;
        self.city_strike_report_origin(state)?;
        if district.component_type != 3 {
            return Err(Error::Missing("city strike District report identity"));
        }
        state
            .district(i32::from(district.owner), district.id)
            .map_err(|e| Error::Requirement(format!("{e:?}")))?;
        // Reflection Fill reads current District fields and existing air-unit
        // pairings into a distinct DTO. Its cache/host writes are presentation.
        self.combat_mut()?
            .city_strike_reports
            .push(Output::DistrictCombatChanged {
                district,
                subtype: 0x599feed4,
            });
        Ok(())
    }
    fn city_command_started(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
        district: ComponentId,
        command_hash: u32,
    ) -> Result<(), Self::Error> {
        use crate::loaded_actions::stock_notifications::movement::Error;
        self.city_strike_report_origin(state)?;
        let current = state
            .city(i32::from(city.owner), city.id)
            .map_err(|e| Error::Requirement(format!("{e:?}")))?;
        if current.header.owner != state.retained().core.local_player {
            return Ok(());
        }
        if city.component_type != 2 || district.component_type != 3 || command_hash != 0x8abcccc1 {
            return Err(Error::Missing("city strike command report identity"));
        }
        // The reviewed City Add(update,false) queries have a warm-path domain;
        // recheck it against current state, rather than inheriting old answers.
        crate::loaded_actions::reporting::yield_monitor::warm_route_domain(state)
            .map_err(|e| Error::Requirement(format!("city report routes {e:?}")))?;
        self.combat_mut()?
            .city_strike_reports
            .push(Output::CityCommandStarted {
                city,
                district,
                command_hash,
                value: 0,
            });
        Ok(())
    }
}
