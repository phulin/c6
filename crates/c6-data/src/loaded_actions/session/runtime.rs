//! Reusable stock runtime composition for explicitly initialized typed inputs.
//!
//! This builds services once and moves their canonical owners into Session.
//! Definitions and live query providers stay borrowed from the embedding arena;
//! nothing is leaked, cloned into a second mutable world, or rebuilt per request.
//! It is neither new-game initialization nor Civ VI save loading. Missing
//! optional feature services keep their existing unsupported reached boundaries.
use super::{initialized, stock_notifications as stock, Session};
use crate::loaded_actions::{notifications::processing, reporting, stock_script};
use c6_core::{
    loaded_state::{game_update, LoadedState},
    player_notifications_owner::OccurrenceLimits,
};
use std::ops::{Deref, DerefMut};

/// Common research wiring used by action callbacks, factory boosts and upkeep.
/// Every consumer sees these same immutable definitions and live query owners.
#[derive(Clone, Copy)]
pub struct Research<'a> {
    pub inputs: stock::unit_creation::ResearchInputs<'a>,
    pub queries: stock::research::Queries<'a>,
    pub units: &'a crate::unit_movement_state::Definitions,
}
/// Explicit initialized common services; no defaults infer gameplay state.
pub struct Inputs<'a, P = c6_core::unit::OperationParameters> {
    pub script: stock_script::Runtime<P>,
    pub reporting: reporting::Provider<'a>,
    pub facts: stock::Facts,
    pub players: crate::unit_placement_services::Runtime,
    pub research: Research<'a>,
}
/// Rejected construction returns all supplied owners, including script/RNG.
pub struct Rejected<'a, P> {
    pub error: processing::Error,
    pub inputs: Inputs<'a, P>,
}
/// Owns the stock backend while explicit feature inputs are attached. Deref
/// exposes the existing typed extension bindings for features outside this
/// common composition. Consume `finish` exactly once, then retain its Session.
pub struct Builder<'a, P = c6_core::unit::OperationParameters> {
    backend: stock::Runtime<'a, P>,
    research: Research<'a>,
}
impl<'a, P> Builder<'a, P> {
    pub fn new(state: &LoadedState, inputs: Inputs<'a, P>) -> Result<Self, Box<Rejected<'a, P>>> {
        let invalid = !inputs.script.script().matches(state)
            || inputs.players.binding != *state.binding()
            || inputs.research.inputs.definitions.binding() != state.binding()
            || inputs
                .research
                .queries
                .graph
                .is_some_and(|g| g.binding() != state.binding());
        if invalid {
            return Err(Box::new(Rejected {
                error: processing::Error::Unsupported("runtime common service identity"),
                inputs,
            }));
        }
        let Inputs {
            script,
            reporting,
            facts,
            players,
            research,
        } = inputs;
        // All fallible identity predicates were checked above; these are fresh
        // slots, so duplicate binding cannot occur during common construction.
        let mut backend = stock::Runtime::new(
            state,
            script,
            reporting,
            facts,
            stock::Scope::StockPlainHeadlessNoAddedConsumers,
        )
        .unwrap_or_else(|_| unreachable!("checked script identity"));
        backend
            .bind_player_services(state, players)
            .unwrap_or_else(|_| unreachable!("checked player identity and fresh slot"));
        backend
            .bind_research_queries(state, research.queries)
            .unwrap_or_else(|_| unreachable!("checked graph identity and fresh slot"));
        backend
            .bind_research_unit_updates(state, research.units)
            .unwrap_or_else(|_| unreachable!("fresh unit update slot"));
        Ok(Self { backend, research })
    }
    /// Join the canonical modifier/requirement/collection owners to fresh-unit
    /// lifecycle callbacks and the SAME research definitions as ordinary actions.
    /// On a binding error previously attached owners remain in this builder.
    pub fn factory(
        &mut self,
        state: &LoadedState,
        mut input: stock::unit_creation::State<'a>,
        owners: stock::unit_creation::ModifierOwners,
    ) -> Result<(), Box<stock::unit_creation::RejectedFactory<'a>>> {
        if input.research.is_some() {
            return Err(Box::new(stock::unit_creation::RejectedFactory {
                error: stock::movement::Error::Missing("runtime factory research already supplied"),
                input,
                owners,
            }));
        }
        input.research = Some(self.research.inputs);
        self.backend.bind_factory(state, input, owners)
    }
    /// Attach research to a factory installed by an existing typed extension.
    pub fn factory_research(&mut self, state: &LoadedState) -> Result<(), stock::movement::Error> {
        self.backend
            .bind_unit_creation_research(state, self.research.inputs)
    }
    pub fn finish<'s>(
        self,
        state: &'s mut LoadedState,
        owners: Owners<'s>,
    ) -> Result<Session<'s, 'a, P>, Box<initialized::Rejected<'s, 'a, P>>> {
        Session::from_initialized(
            state,
            initialized::Initialized {
                backend: self.backend,
                notifications: owners.notifications,
                occurrence_limits: owners.occurrence_limits,
                update: owners.update,
                research: owners.research,
                visibility: owners.visibility,
            },
        )
    }
}
/// Mutable owners moved once into the retained Session. In particular research
/// is never reconstructed between a kill callback and the following round.
pub struct Owners<'s> {
    pub notifications: processing::Inputs<'s>,
    pub occurrence_limits: OccurrenceLimits,
    pub update: game_update::Runtime,
    pub research: crate::research::Runtime,
    pub visibility: crate::game_update_state::deactivation::VisibilityHistory,
}
impl<'a, P> Deref for Builder<'a, P> {
    type Target = stock::Runtime<'a, P>;
    fn deref(&self) -> &Self::Target {
        &self.backend
    }
}
impl<P> DerefMut for Builder<'_, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.backend
    }
}

/// Qualified stock attack services. Capture/address decoding belongs to the
/// importer; these are production gameplay inputs and ordered callback owners.
pub struct Combat<'a> {
    pub inputs: crate::melee_attack::backend::Inputs,
    pub completion: crate::melee_attack::resolve::completion::stock::Bindings,
    pub observer: Option<i32>,
    pub promotion: &'a dyn stock::combat::PromotionQuery,
    pub movement: &'a dyn crate::melee_attack::stock::MovementQueries,
    pub scope: stock::combat::Scope,
    pub notice: stock::combat::CombatNoticeScope,
    pub reports: CombatReports,
}
/// Exact supported damage, attacks and formation report consumers. Selecting
/// this scope requires the importer to qualify absence of added consumers.
pub enum CombatReports {
    StockHeadlessNoAddedConsumers,
}
pub struct RejectedCombat<'a> {
    pub error: String,
    pub inputs: Combat<'a>,
}
/// Compose lifecycle/formation reports with the existing factory's modifier
/// owners and attack completion callbacks. No second modifier store is created.
/// Failure retains any attached report prefix in `backend` and returns attack
/// inputs. A caller must inspect that prefix before retrying assembly.
pub fn combat<'a, P>(
    backend: &mut stock::Runtime<'a, P>,
    state: &LoadedState,
    inputs: Combat<'a>,
) -> Result<(), Box<RejectedCombat<'a>>> {
    let reports = (|| {
        backend
            .check_combat_slot(state)
            .map_err(|e| e.to_string())?;
        match inputs.reports {
            CombatReports::StockHeadlessNoAddedConsumers => {
                backend
                    .reporting
                    .bind_damage_changed_route(
                        state,
                        &[
                            reporting::damage::Consumer::CacheUnitUpdate,
                            reporting::damage::Consumer::HostUnitDamageChanged,
                        ],
                        0,
                        0,
                    )
                    .map_err(|e| e.to_string())?;
                backend
                    .reporting
                    .bind_attacks_changed_route(
                        state,
                        &[
                            reporting::attacks::Consumer::CacheUnitUpdate,
                            reporting::attacks::Consumer::HostUnitAttacksRemainingChanged,
                        ],
                        0,
                        0,
                    )
                    .map_err(|e| e.to_string())?;
                backend
                    .reporting
                    .bind_formation_movement_changed_route(
                        state,
                        &[
                            reporting::formation::Consumer::FormationMemberSnapshots,
                            reporting::formation::Consumer::CacheUpdated,
                        ],
                        0,
                    )
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok::<_, String>(())
    })();
    if let Err(error) = reports {
        return Err(Box::new(RejectedCombat { error, inputs }));
    }
    let Combat {
        inputs,
        completion,
        observer,
        promotion,
        movement,
        scope,
        notice,
        ..
    } = inputs;
    let mut combat =
        stock::combat::State::for_attack(state, inputs, completion, observer, promotion, scope)
            .with_combat_notice_scope(notice);
    combat.melee_queries = Some(movement);
    backend
        .bind_combat(state, combat)
        .unwrap_or_else(|_| unreachable!("checked stock combat slot and same World"));
    Ok(())
}
