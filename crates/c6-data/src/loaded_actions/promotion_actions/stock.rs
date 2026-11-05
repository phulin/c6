//! Promotion continuations over persistent stock modifier, movement and sight owners.
use super::{Definitions, Effects};
use crate::loaded_actions::{
    stock_notifications::{self, city_production::unit::Factory},
    upgrade_actions::transfer,
};
use c6_core::{loaded_state::LoadedState, unit::ComponentId};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Report {
    Started {
        unit: ComponentId,
        promotion_index: usize,
    },
    Earned {
        unit: ComponentId,
        promotion_index: usize,
    },
    Level {
        unit: ComponentId,
        level: i32,
    },
    CommandComplete {
        unit: ComponentId,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LevelConsumer {
    HistoricMoment,
    EraScore,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompletionConsumer {
    Lua,
}
/// Actual current subscriber qualifications, never inferred from Save equality.
pub struct Inputs<'a, 'd> {
    pub transfer: &'a transfer::Inputs<'d>,
    pub level_consumers: Option<Vec<LevelConsumer>>,
    pub completed_consumers: Option<Vec<CompletionConsumer>>,
    pub reports: Vec<Report>,
}
pub struct Current<'s, 'a, 'd, 'r, 't, 'q, P> {
    pub backend: &'s mut stock_notifications::Runtime<'a, P>,
    pub factory: Factory<'s, 'd, 'r>,
    pub inputs: &'s mut Inputs<'t, 'q>,
    pub definitions: &'s Definitions,
}
fn fail(e: impl std::fmt::Debug) -> String {
    format!("promotion: {e:?}")
}
impl<P> Effects for Current<'_, '_, '_, '_, '_, '_, P> {
    type Error = String;
    fn common_guards(&self, state: &LoadedState, id: (i32, u32)) -> Result<bool, String> {
        self.backend.check(state).map_err(fail)?;
        if !self
            .factory
            .resources
            .phases()
            .from_processor(&state.retained().core.phase_processor, id.0)
            .map_err(fail)?
        {
            return Ok(false);
        }
        if !state.turn_active(id.0).map_err(fail)? {
            return Ok(false);
        }
        if state.player_slot_status(id.0).map_err(fail)? == 3
            && state.pause_storage().map_err(fail)? != 0
        {
            return Ok(false);
        }
        let u = super::actor::<()>(state, id).map_err(fail)?;
        if !u.k_operations.nodes.is_empty() {
            return Err("promotion Manager IsInterruptible for queued operation".into());
        }
        Ok(u.i_damage
            < self
                .factory
                .placement
                .movement
                .maximum_damage(u)
                .map_err(fail)?
            && !u.b_delayed_death)
    }
    fn preflight(&self, state: &LoadedState, id: (i32, u32), promotion: u32) -> Result<(), String> {
        self.backend.check(state).map_err(fail)?;
        self.definitions.check::<()>(state).map_err(fail)?;
        self.factory
            .resources
            .validate_rules(self.inputs.transfer.rules)
            .map_err(fail)?;
        if !self.inputs.transfer.presentation_reports_qualified
            || self.inputs.transfer.promotion_consumers.is_none()
            || self.inputs.level_consumers.is_none()
            || self.inputs.completed_consumers.is_none()
        {
            return Err("unqualified promotion signal/report consumers".into());
        }
        let u = super::actor::<()>(state, id).map_err(fail)?;
        if u.k_experience.i_stored_promotions > 0 {
            return Err("stored-promotion XP continuation".into());
        }
        let entry = self
            .definitions
            .promotions
            .iter()
            .find(|v| v.hash == promotion)
            .ok_or("promotion definition")?;
        let registry = crate::unit_initialization_definitions::collection(
            self.inputs.transfer.rules,
            "UnitPromotions",
            "UnitPromotionType",
        )
        .map_err(fail)?;
        let name = &registry
            .find(entry.hash)
            .ok_or("promotion registry entry")?
            .type_name;
        crate::unit_creation::collections::ability::preflight_promotion(
            self.inputs.transfer.rules,
            name,
        )
        .map_err(fail)?;
        Ok(())
    }
    fn started(
        &mut self,
        _: &mut LoadedState,
        unit: ComponentId,
        index: usize,
    ) -> Result<(), String> {
        self.inputs.reports.push(Report::Started {
            unit,
            promotion_index: index,
        });
        Ok(())
    }
    fn earned(
        &mut self,
        state: &mut LoadedState,
        unit: ComponentId,
        promotion: u32,
        index: usize,
    ) -> Result<(), String> {
        let registry = crate::unit_initialization_definitions::collection(
            self.inputs.transfer.rules,
            "UnitPromotions",
            "UnitPromotionType",
        )
        .map_err(fail)?;
        let name = &registry
            .find(promotion)
            .ok_or("promotion registry entry")?
            .type_name;
        for consumer in self
            .inputs
            .transfer
            .promotion_consumers
            .as_ref()
            .ok_or("promotion consumers")?
        {
            if *consumer == transfer::PromotionConsumer::AttachGameEffects {
                self.backend.upgrade_promotion_earned(
                    state,
                    self.inputs.transfer.rules,
                    unit,
                    name,
                )?
            } else {
                let u =
                    super::actor::<()>(state, (i32::from(unit.owner), unit.id)).map_err(fail)?;
                let domain = state.unit_runtime(u.e_owner, u.id).map_err(fail)?.domain;
                transfer::promotion_leaf(
                    self.inputs.transfer,
                    *consumer,
                    u.e_type_index,
                    domain,
                    u.k_experience.i_level,
                )?;
            }
        }
        self.inputs.reports.push(Report::Earned {
            unit,
            promotion_index: index,
        });
        Ok(())
    }
    fn is_combat(&self, state: &LoadedState, id: (i32, u32)) -> Result<bool, String> {
        let u = super::actor::<()>(state, id).map_err(fail)?;
        Ok(self.factory.combat.combat(state, u).map_err(fail)? > 0
            || self.factory.combat.ranged(state, u).map_err(fail)? > 0
            || self.factory.combat.bombard(state, u).map_err(fail)? > 0)
    }
    fn change_damage(
        &mut self,
        state: &mut LoadedState,
        id: (i32, u32),
        delta: i32,
    ) -> Result<(), String> {
        crate::unit_placement_state::change_damage(
            state,
            self.factory.placement,
            self.factory.queries,
            self.factory.sight,
            self.backend,
            id,
            delta,
        )
        .map_err(fail)?;
        Ok(())
    }
    fn remove_notification(
        &mut self,
        state: &mut LoadedState,
        unit: ComponentId,
    ) -> Result<(), String> {
        let owner = i32::from(unit.owner);
        if state.player_slot_status(owner).map_err(fail)? != 3 {
            return Ok(());
        }
        let records = &state
            .player_notifications()
            .map_err(fail)?
            .collection(owner)
            .map_err(fail)?
            .records;
        if records.iter().any(|r| r.type_hash == 0x2ba6e111) {
            return Err("existing PromoteUnit notification payload/removal continuation".into());
        }
        Ok(())
    }
    fn level_changed(
        &mut self,
        state: &mut LoadedState,
        unit: ComponentId,
        level: i32,
    ) -> Result<(), String> {
        for consumer in self
            .inputs
            .level_consumers
            .as_ref()
            .ok_or("level consumers")?
        {
            match consumer {
                LevelConsumer::HistoricMoment => {
                    if level == 4 || level == 5 {
                        return Err("positive unit promotion historic moment".into());
                    }
                }
                LevelConsumer::EraScore => {
                    // 299d270 queries owner, then returns before score/state
                    // lookups when the new level is below three.
                    super::actor::<()>(state, (i32::from(unit.owner), unit.id)).map_err(fail)?;
                    if level >= 3 {
                        return Err("positive promotion EraScore continuation".into());
                    }
                }
            }
        }
        self.backend.check(state).map_err(fail)?;
        self.inputs.reports.push(Report::Level { unit, level });
        Ok(())
    }
    fn completed(
        &mut self,
        state: &mut LoadedState,
        unit: ComponentId,
        promotion: u32,
    ) -> Result<(), String> {
        self.backend.check(state).map_err(fail)?;
        for consumer in self
            .inputs
            .completed_consumers
            .as_ref()
            .ok_or("promotion completion consumers")?
        {
            match consumer {
                CompletionConsumer::Lua => {
                    let current = super::actor::<()>(state, (i32::from(unit.owner), unit.id))
                        .map_err(fail)?;
                    let index = self
                        .definitions
                        .promotions
                        .iter()
                        .position(|p| p.hash == promotion)
                        .ok_or("promotion definition")?;
                    let arguments = [current.e_owner, current.id as i32, index as i32];
                    self.backend
                        .script
                        .call_empty(state, "PostUnitPromotionEarned", &arguments)
                        .map_err(fail)?;
                }
            }
        }
        Ok(())
    }
    fn command_complete(
        &mut self,
        state: &mut LoadedState,
        unit: ComponentId,
    ) -> Result<(), String> {
        self.backend.check(state).map_err(fail)?;
        let local = self
            .backend
            .local_player()
            .ok_or("promotion local observer")?;
        if i32::from(unit.owner) == local {
            self.inputs.reports.push(Report::CommandComplete { unit });
        }
        Ok(())
    }
    fn finish_moves(&mut self, state: &mut LoadedState, id: (i32, u32)) -> Result<(), String> {
        crate::loaded_upkeep::finish_unit_moves(
            state,
            id.0,
            id.1,
            self.factory.movement,
            self.backend,
        )
        .map_err(fail)
    }
}
