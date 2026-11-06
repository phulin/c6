//! Current-state major first contact on the persistent stock action backend.
//!
//! This bounded adapter runs the Linux Meet kernel. Its required consumers own
//! the observed synchronous report/signal registrations; no callback or player
//! choice is replaced with a native policy. City hospitality remains explicit.
use crate::loaded_actions::stock_notifications::{movement::Error, Runtime};
use crate::{loaded_upkeep::OperationReports, research};
#[path = "stock_first_contact/boosts.rs"]
pub mod boosts;
#[path = "stock_first_contact/diplomacy.rs"]
pub mod diplomacy;
#[path = "stock_first_contact/history.rs"]
pub mod history;
#[path = "stock_first_contact/requirements.rs"]
pub mod requirements;
#[path = "stock_first_contact/targets.rs"]
pub mod targets;
#[path = "stock_first_contact/trade.rs"]
pub mod trade;
use c6_core::{
    diplomacy_manager::StatementHandle,
    diplomacy_meeting::{self as meeting, FirstMeetingRequest, Report, VisibilitySource},
    diplomacy_request,
    diplomacy_statement::Logging,
    game_capabilities::Membership,
    game_diplomatic_relations::GameDiplomaticRelations,
    loaded_state::LoadedState,
    player_diplomacy::GossipReports,
    unit::{ComponentId, PlotCoord, Unit},
    unit_placement_transition::PlacementReport,
};

/// Gameplay consumers from the actual current registration order. Implementors
/// must preserve reentrant state and RNG. Admission cannot use AFTER values.
pub trait Effects<P> {
    fn preflight(
        &self,
        state: &LoadedState,
        backend: &Runtime<'_, P>,
        first: i32,
        second: i32,
        plot: Option<u32>,
        source: ComponentId,
    ) -> Result<(), Error>;
    fn visibility_changed(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        owner: i32,
        other: i32,
    ) -> Result<(), Error>;
    fn log_has_met(&mut self, state: &mut LoadedState, owner: i32, other: i32)
        -> Result<(), Error>;
    fn notify_contact(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        recipient: i32,
        other: i32,
        plot: u32,
    ) -> Result<(), Error>;
    fn report(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        report: Report,
    ) -> Result<(), Error>;
    fn preflight_choice(
        &self,
        state: &LoadedState,
        backend: &Runtime<'_, P>,
        decision: c6_core::diplomacy_first_meeting::PendingDecision,
        choice: c6_core::diplomacy_first_meeting::Choice,
    ) -> Result<(), Error>;
    fn session_closed(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        session_id: u32,
    ) -> Result<(), Error>;
    fn first_meeting_report(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        report: c6_core::diplomacy_first_meeting::Report,
    ) -> Result<(), Error>;
    fn signal_meet(
        &mut self,
        state: &mut LoadedState,
        backend: &mut Runtime<'_, P>,
        first: i32,
        second: i32,
    ) -> Result<(), Error>;
}
pub struct Inputs<'a, P> {
    pub visibility: &'a research::diplomacy::Definitions,
    pub civilization_levels: &'a [(i32, u32)],
    pub capabilities: &'a Membership,
    pub any_multiplayer: bool,
    pub generating: bool,
    pub hospitality_distance: i32,
    pub local_player: i32,
    pub logging: Logging,
    pub neutral_choices: &'a [c6_core::diplomacy_first_meeting::Choice],
    pub effects: Box<dyn Effects<P> + 'a>,
}
pub(super) struct State<'a, P> {
    input: Inputs<'a, P>,
    next_statement: u64,
    pending: Vec<c6_core::diplomacy_first_meeting::PendingDecision>,
    unconsumed: Vec<(StatementHandle, FirstMeetingRequest)>,
}
impl<'a, P> Runtime<'a, P> {
    pub fn bind_first_contact(
        &mut self,
        state: &LoadedState,
        input: Inputs<'a, P>,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Missing("first-contact identity"))?;
        if self.first_contact.is_some() {
            return Err(Error::Missing("first-contact already bound"));
        }
        let inventory = state
            .diplomacy_manager()
            .map_err(Error::State)?
            .statement_inventory()
            .ok_or(Error::Missing("first-contact current statement inventory"))?;
        let next_statement = inventory
            .iter()
            .map(|s| s.handle.0)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(Error::Missing("first-contact statement identity exhausted"))?;
        self.first_contact = Some(State {
            input,
            next_statement,
            pending: Vec::new(),
            unconsumed: Vec::new(),
        });
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn trigger_first_contact_boost(
        &mut self,
        state: &mut LoadedState,
        definitions: &boosts::Definitions,
        row: i32,
        levels: &[(i32, u32)],
        ever_alive_minors: &[i32],
        first: i32,
        second: i32,
        research: &research::Definitions,
        choices: &crate::loaded_actions::choose_tech::Definitions,
        culture: Option<&crate::loaded_actions::stock_notifications::research::CultureInputs<'_>>,
    ) -> Result<(), Error> {
        let mut runtime = self
            .action_research
            .take()
            .ok_or(Error::Missing("first-contact canonical action research"))?;
        let result = {
            let mut effects = crate::loaded_actions::stock_notifications::research::Effects::new(
                self,
                &mut runtime,
                choices,
                culture,
            );
            let mut trigger = crate::melee_attack::resolve::completion::boosts::ResearchTrigger {
                definitions: research,
                effects: &mut effects,
            };
            definitions.on_met(
                state,
                row,
                levels,
                ever_alive_minors,
                first,
                second,
                &mut trigger,
            )
        };
        self.action_research = Some(runtime);
        result
    }
    /// The current initialized encounter array, including unsaved sight writes.
    pub fn first_contact_encountered_units(&self, owner: i32) -> Result<&[bool], Error> {
        self.player_services
            .as_ref()
            .ok_or(Error::Missing("first-contact current encounter owner"))?
            .player::<Error>(owner)
            .map_err(|e| Error::Requirement(format!("first-contact encounter owner: {e:?}")))?
            .encountered_units
            .as_deref()
            .ok_or(Error::Missing("first-contact current encounter flags"))
    }
    pub fn pending_first_meetings(&self) -> &[c6_core::diplomacy_first_meeting::PendingDecision] {
        self.first_contact.as_ref().map_or(&[], |s| &s.pending)
    }
    pub fn unconsumed_first_meetings(&self) -> &[(StatementHandle, FirstMeetingRequest)] {
        self.first_contact.as_ref().map_or(&[], |s| &s.unconsumed)
    }
    pub fn respond_first_meeting(
        &mut self,
        state: &mut LoadedState,
        issuer: i32,
        choice: c6_core::diplomacy_first_meeting::Choice,
    ) -> Result<c6_core::diplomacy_first_meeting::ChoiceOutcome, Error> {
        use c6_core::diplomacy_first_meeting::{ChoiceContext, ChoiceEffect, ChoiceOutcome};
        let mut current = self
            .first_contact
            .take()
            .ok_or(Error::Missing("first-contact current response consumers"))?;
        let result = (|| {
            let decision = *current
                .pending
                .first()
                .ok_or(Error::Missing("first-contact pending response"))?;
            if decision.player != issuer {
                return Err(Error::Missing("first-contact response player"));
            }
            current
                .input
                .effects
                .preflight_choice(state, self, decision, choice)?;
            let player = |owner| -> Result<diplomacy_request::Player, Error> {
                let level = current
                    .input
                    .civilization_levels
                    .iter()
                    .find(|p| p.0 == owner)
                    .ok_or(Error::Missing("first-contact response civilization level"))?
                    .1;
                Ok(diplomacy_request::Player {
                    owner,
                    major: level == meeting::MAJOR_LEVEL,
                    human: state.player_slot_status(owner).map_err(Error::State)? == 3,
                })
            };
            let services = self
                .player_services
                .as_ref()
                .ok_or(Error::Missing("first-contact response live players"))?;
            let alive = services
                .alive_players
                .as_ref()
                .ok_or(Error::Missing("first-contact response alive roster"))?;
            let mut active_human_count = 0u32;
            for &owner in alive.values() {
                if state.player_slot_status(owner).map_err(Error::State)? == 3 {
                    let active = state
                        .partial_player_controls(owner)
                        .map_err(Error::State)?
                        .turn_active
                        .ok_or(Error::Missing("first-contact current human turn-active"))?;
                    active_human_count = active_human_count.wrapping_add(u32::from(active));
                }
            }
            let has_capital = |owner| -> Result<bool, Error> {
                Ok(state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == owner)
                    .ok_or(Error::Missing("first-contact response city parent"))?
                    .cities
                    .records
                    .entries
                    .iter()
                    .any(|c| c.value.header.capital))
            };
            let manager = state.diplomacy_manager().map_err(Error::State)?;
            let (game_mode, logging) = manager
                .execution_profile()
                .ok_or(Error::Missing("first-contact response execution profile"))?;
            let statement = c6_core::diplomacy_statement::Context {
                local_player: current.input.local_player,
                game_mode,
                active_session: manager.active_session().map_err(|e| {
                    Error::Requirement(format!("first-contact response active session: {e:?}"))
                })?,
                logging,
            };
            let start = manager
                .statement_inventory()
                .ok_or(Error::Missing("first-contact response inventory"))?
                .iter()
                .map(|s| s.handle.0)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or(Error::Missing("first-contact response identity exhausted"))?
                .max(current.next_statement);
            let handles = [
                StatementHandle(start),
                StatementHandle(
                    start
                        .checked_add(1)
                        .ok_or(Error::Missing("first-contact response identity exhausted"))?,
                ),
                StatementHandle(
                    start
                        .checked_add(2)
                        .ok_or(Error::Missing("first-contact response identity exhausted"))?,
                ),
            ];
            let next = start
                .checked_add(3)
                .ok_or(Error::Missing("first-contact response identity exhausted"))?;
            let context = ChoiceContext {
                statement,
                ai: player(decision.session.from)?,
                human: player(decision.session.to)?,
                ai_has_capital: has_capital(decision.session.from)?,
                human_has_capital: has_capital(decision.session.to)?,
                active_human_count,
            };
            let plan = manager
                .plan_first_meeting_choice(decision, choice, context, handles)
                .map_err(|e| Error::Requirement(format!("first-contact response plan: {e:?}")))?;
            current.pending.remove(0);
            current.next_statement = next;
            let outcome = state
                .apply_first_meeting_choice(plan, |state, effect| match effect {
                    ChoiceEffect::Statement(report) => current
                        .input
                        .effects
                        .first_meeting_report(state, self, report),
                    ChoiceEffect::FriendlyMeeting { player, other } => {
                        let index = usize::try_from(other)
                            .map_err(|_| Error::Missing("first-contact friendly index"))?;
                        *state
                            .war_diplomacy_mut(player)
                            .map_err(Error::State)?
                            .had_friendly_meeting
                            .get_mut(index)
                            .ok_or(Error::Missing("first-contact friendly cell"))? = true;
                        Ok(())
                    }
                    ChoiceEffect::SessionClosed { session_id } => current
                        .input
                        .effects
                        .session_closed(state, self, session_id),
                })
                .map_err(|e| {
                    Error::Requirement(format!("first-contact response continuation: {e:?}"))
                })?;
            if let ChoiceOutcome::Pending(decision) = outcome {
                current.pending.push(decision);
            }
            Ok(outcome)
        })();
        self.first_contact = Some(current);
        result
    }
    pub(super) fn process_initial_first_meeting(
        &mut self,
        state: &mut LoadedState,
        handle: StatementHandle,
    ) -> Result<(), Error> {
        let mut current = self
            .first_contact
            .take()
            .ok_or(Error::Missing("first-contact current statement consumers"))?;
        let result = (|| {
            let manager = state.diplomacy_manager().map_err(Error::State)?;
            let info = manager
                .statement_inventory()
                .and_then(|v| v.iter().find(|s| s.handle == handle))
                .copied()
                .ok_or(Error::Missing("first-contact current statement"))?;
            let player = |owner| -> Result<diplomacy_request::Player, Error> {
                let level = current
                    .input
                    .civilization_levels
                    .iter()
                    .find(|p| p.0 == owner)
                    .ok_or(Error::Missing("first-contact statement civilization level"))?
                    .1;
                Ok(diplomacy_request::Player {
                    owner,
                    major: level == meeting::MAJOR_LEVEL,
                    human: state.player_slot_status(owner).map_err(Error::State)? == 3,
                })
            };
            let (game_mode, logging) = manager.execution_profile().ok_or(Error::Missing(
                "first-contact current statement execution profile",
            ))?;
            let context = c6_core::diplomacy_statement::Context {
                local_player: current.input.local_player,
                game_mode,
                active_session: manager.active_session().map_err(|e| {
                    Error::Requirement(format!("first-contact active session: {e:?}"))
                })?,
                logging,
            };
            let plan = manager
                .plan_first_meeting_statement(
                    handle,
                    context,
                    player(info.from)?,
                    player(info.to)?,
                    current.input.neutral_choices,
                )
                .map_err(|e| Error::Requirement(format!("first-contact statement plan: {e:?}")))?;
            current
                .pending
                .try_reserve(1)
                .map_err(|_| Error::Missing("first-contact decision allocation"))?;
            let decision = state
                .apply_first_meeting_statement(plan, |state, report| {
                    current
                        .input
                        .effects
                        .first_meeting_report(state, self, report)
                })
                .map_err(|e| {
                    Error::Requirement(format!("first-contact statement process: {e:?}"))
                })?;
            current.pending.push(decision);
            Ok(())
        })();
        self.first_contact = Some(current);
        result
    }
    pub(super) fn meet_major(
        &mut self,
        state: &mut LoadedState,
        first: i32,
        second: i32,
        plot: Option<u32>,
        source: ComponentId,
    ) -> Result<(), Error> {
        let mut current = self
            .first_contact
            .take()
            .ok_or(Error::Missing("first-contact current consumers"))?;
        let result = meeting::meet(
            &mut Context {
                state,
                backend: self,
                current: &mut current,
            },
            first,
            second,
            plot,
            source,
        )
        .map(|_| ())
        .map_err(|e| Error::Requirement(format!("first contact: {e:?}")));
        self.first_contact = Some(current);
        result
    }
}
struct Context<'s, 'a, P> {
    state: &'s mut LoadedState,
    backend: &'s mut Runtime<'a, P>,
    current: &'s mut State<'a, P>,
}
impl<P> Context<'_, '_, P> {
    fn level_of(&self, player: i32) -> Result<u32, Error> {
        self.current
            .input
            .civilization_levels
            .iter()
            .find(|p| p.0 == player)
            .map(|p| p.1)
            .ok_or(Error::Missing("first-contact civilization level"))
    }
    fn controls(&self, player: i32) -> Result<bool, Error> {
        Ok(self
            .state
            .player_slot_status(player)
            .map_err(Error::State)?
            == 3)
    }
}
impl<P> meeting::Context for Context<'_, '_, P> {
    type Error = Error;
    type Player = i32;
    type Unit = (i32, u32);
    type Plot = u32;
    type City = (i32, u32);
    fn player(&mut self, owner: i32) -> Result<i32, Error> {
        self.state.world().player(owner).map_err(Error::World)?;
        Ok(owner)
    }
    fn player_valid(&mut self, owner: i32) -> Result<bool, Error> {
        self.state
            .world()
            .player_is_valid(owner)
            .map_err(Error::World)
    }
    fn id(&mut self, player: i32) -> Result<i32, Error> {
        Ok(player)
    }
    fn alive(&mut self, player: i32) -> Result<bool, Error> {
        self.state.player_alive(player).map_err(Error::State)
    }
    fn major(&mut self, player: i32) -> Result<bool, Error> {
        Ok(self.level_of(player)? == meeting::MAJOR_LEVEL)
    }
    fn barbarian(&mut self, player: i32) -> Result<bool, Error> {
        Ok(self.level_of(player)? == 0xd959ec24)
    }
    fn human(&mut self, player: i32) -> Result<bool, Error> {
        self.controls(player)
    }
    fn level(&mut self, player: i32) -> Result<u32, Error> {
        self.level_of(player)
    }
    fn any_multiplayer(&mut self) -> Result<bool, Error> {
        Ok(self.current.input.any_multiplayer)
    }
    fn generating(&mut self) -> Result<bool, Error> {
        Ok(self.current.input.generating)
    }
    fn relations(&self) -> Result<&GameDiplomaticRelations, Error> {
        self.state.game_relations().map_err(Error::State)
    }
    fn relations_mut(&mut self) -> Result<&mut GameDiplomaticRelations, Error> {
        self.state.war_relations_mut().map_err(Error::State)
    }
    fn current_turn(&mut self) -> Result<i32, Error> {
        Ok(self.state.retained().core.current_turn)
    }
    fn preflight_meet(
        &self,
        first: i32,
        second: i32,
        plot: Option<u32>,
        source: ComponentId,
    ) -> Result<(), Error> {
        if self.level_of(first)? != meeting::MAJOR_LEVEL
            || self.level_of(second)? != meeting::MAJOR_LEVEL
        {
            return Err(Error::Missing("first-contact major-pair profile"));
        }
        // This first-contact profile reaches neutral hospitality. City-bearing
        // contacts require the native closest-city ordering and its consumers.
        for owner in [first, second] {
            let player = self
                .state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == owner)
                .ok_or(Error::Missing("first-contact player"))?;
            if !player.cities.records.entries.is_empty() {
                return Err(Error::Missing("first-contact city hospitality"));
            }
        }
        if plot.is_some()
            && !self.current.input.generating
            && (self.controls(first)? ^ self.controls(second)?)
        {
            use c6_core::diplomacy_first_meeting::Choice;
            if self.current.input.neutral_choices != [Choice::Positive, Choice::Exit] {
                return Err(Error::Missing(
                    "first-contact current neutral selection rules",
                ));
            }
            let players = self
                .current
                .input
                .civilization_levels
                .iter()
                .map(|&(owner, level)| {
                    Ok(diplomacy_request::Player {
                        owner,
                        major: level == meeting::MAJOR_LEVEL,
                        human: self.controls(owner)?,
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;
            let (from, to) = if self.controls(first)? {
                (second, first)
            } else {
                (first, second)
            };
            self.state
                .diplomacy_manager()
                .map_err(Error::State)?
                .plan_first_meeting_request(
                    FirstMeetingRequest {
                        statement_type: meeting::NEUTRAL_MEETING,
                        from,
                        to,
                        hospitality_city: PlotCoord { x: -9999, y: -9999 },
                    },
                    StatementHandle(self.current.next_statement),
                    diplomacy_request::Context {
                        capabilities: self.current.input.capabilities.hashes(),
                        players: &players,
                        local_player: self.current.input.local_player,
                        logging: self.current.input.logging,
                    },
                )
                .map_err(|e| Error::Requirement(format!("first-contact request support: {e:?}")))?;
        }
        self.current
            .input
            .effects
            .preflight(self.state, self.backend, first, second, plot, source)
    }
    fn gossip_mut(&mut self, player: i32) -> Result<&mut GossipReports, Error> {
        self.state
            .player_gossip_reports_mut(player)
            .map_err(Error::State)
    }
    fn visibility_mut(&mut self, _: i32) -> Result<&mut [i32], Error> {
        Err(Error::Missing(
            "first-contact enum visibility uses canonical recheck",
        ))
    }
    fn visibility_source_mut(&mut self, _: i32) -> Result<&mut [i32], Error> {
        Err(Error::Missing(
            "first-contact enum sources use canonical recheck",
        ))
    }
    fn visibility_sources(&self) -> &[VisibilitySource] {
        &[]
    }
    fn visibility_source_active(&mut self, _: i32, _: i32, _: i32) -> Result<bool, Error> {
        Err(Error::Missing(
            "first-contact enum sources use canonical recheck",
        ))
    }
    fn dynamic_visibility(&mut self, _: i32, _: i32) -> Result<Vec<i32>, Error> {
        Err(Error::Missing(
            "first-contact enum sources use canonical recheck",
        ))
    }
    fn recheck_contact_visibility(
        &mut self,
        player: i32,
        other: i32,
    ) -> Result<(), meeting::Error<Error>> {
        let input = &mut self.current.input;
        research::diplomacy::recheck(
            self.state,
            input.visibility,
            input.civilization_levels,
            player,
            other,
            |state, owner, other| {
                input
                    .effects
                    .visibility_changed(state, self.backend, owner, other)
            },
        )
        .map_err(|e| {
            meeting::Error::Context(Error::Requirement(format!(
                "first-contact visibility: {e:?}"
            )))
        })
    }
    fn visibility_changed(&mut self, _: u32, owner: i32, other: i32) -> Result<(), Error> {
        self.current
            .input
            .effects
            .visibility_changed(self.state, self.backend, owner, other)
    }
    fn log_has_met(&mut self, owner: i32, other: i32) -> Result<(), Error> {
        self.current
            .input
            .effects
            .log_has_met(self.state, owner, other)
    }
    fn find_unit(&mut self, source: ComponentId) -> Result<Option<(i32, u32)>, Error> {
        Ok(self
            .state
            .world()
            .unit(c6_core::map::UnitReference::from_component_id(source))
            .map_err(Error::World)?
            .map(|u| (u.record.e_owner, u.record.id)))
    }
    fn unit(&self, actor: (i32, u32)) -> Result<&Unit, Error> {
        self.state
            .world()
            .player(actor.0)
            .map_err(Error::World)?
            .unit(actor.1)
            .map_err(Error::World)?
            .map(|u| u.record)
            .ok_or(Error::Missing("first-contact source unit"))
    }
    fn unit_mut(&mut self, actor: (i32, u32)) -> Result<&mut Unit, Error> {
        self.state.unit_mut(actor.0, actor.1).map_err(Error::State)
    }
    fn preflight_movement_report(&self, _: (i32, u32), _: u32) -> Result<(), Error> {
        self.backend.movement()?;
        Ok(())
    }
    fn movement_report(&mut self, report: PlacementReport) -> Result<(), Error> {
        self.backend.placement_report(self.state, report)
    }
    fn notify_contact(&mut self, recipient: i32, other: i32, plot: u32) -> Result<(), Error> {
        self.current
            .input
            .effects
            .notify_contact(self.state, self.backend, recipient, other, plot)
    }
    fn report(&mut self, report: Report) -> Result<(), Error> {
        self.current
            .input
            .effects
            .report(self.state, self.backend, report)
    }
    fn closest_city(&mut self, _: i32, _: u32) -> Result<Option<(i32, u32)>, Error> {
        Ok(None)
    }
    fn city_distance(&mut self, _: (i32, u32), _: u32) -> Result<i32, Error> {
        Err(Error::Missing("first-contact city hospitality"))
    }
    fn city_location(&mut self, _: (i32, u32)) -> Result<PlotCoord, Error> {
        Err(Error::Missing("first-contact city hospitality"))
    }
    fn hospitality_distance(&mut self) -> Result<i32, Error> {
        Ok(self.current.input.hospitality_distance)
    }
    fn request_first_meeting(&mut self, request: FirstMeetingRequest) -> Result<(), Error> {
        let players = self
            .current
            .input
            .civilization_levels
            .iter()
            .map(|&(owner, level)| {
                Ok(diplomacy_request::Player {
                    owner,
                    major: level == meeting::MAJOR_LEVEL,
                    human: self.controls(owner)?,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let context = diplomacy_request::Context {
            capabilities: self.current.input.capabilities.hashes(),
            players: &players,
            local_player: self.current.input.local_player,
            logging: self.current.input.logging,
        };
        let handle = StatementHandle(self.current.next_statement);
        let next = self
            .current
            .next_statement
            .checked_add(1)
            .ok_or(Error::Missing("first-contact statement identity exhausted"))?;
        let plan = self
            .state
            .diplomacy_manager()
            .map_err(Error::State)?
            .plan_first_meeting_request(request, handle, context)
            .map_err(|e| Error::Requirement(format!("first-contact request: {e:?}")))?;
        self.current
            .unconsumed
            .try_reserve(1)
            .map_err(|_| Error::Missing("first-contact request ownership allocation"))?;
        let outcome = self
            .state
            .diplomacy_manager_mut()
            .map_err(Error::State)?
            .apply_first_meeting_request(plan, context)
            .map_err(|e| Error::Requirement(format!("first-contact request commit: {e:?}")))?;
        self.current.next_statement = next;
        if let diplomacy_request::Outcome::Unconsumed { handle, request } = outcome {
            self.current.unconsumed.push((handle, request));
        }
        Ok(())
    }
    fn signal_meet(&mut self, first: i32, second: i32) -> Result<(), Error> {
        self.current
            .input
            .effects
            .signal_meet(self.state, self.backend, first, second)
    }
}
