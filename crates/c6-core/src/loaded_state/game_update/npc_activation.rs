//! Resumable Linux XP2 AI.Control.Manager.DoTurnActivate (201e880).
//! This frame executes control scalars, not native AI policy selection. Each
//! registered subscriber must retain its actual gameplay/RNG effects. Unknown
//! subscribers and incomplete leaves must fail rather than report completion.
use super::{human_activation, Error, LoadedState, Runtime};
use crate::{core_player::initialization::WireBinding, player_ai_control::ControlManager};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    City,
    BehaviorTree,
    FreeCities,
    Barbarian,
    Planning,
    Diplomatic,
    Religious,
    Targets,
    GrandStrategic,
    Governors,
    Research,
    Economic,
    Espionage,
    Homeland,
    Culture,
    Military,
    Tactical,
}
/// CultureAi202f44f clears the pending policy byte before calling the
/// external government/policy selection site. This is canonical state shared
/// with later callbacks; a deferred response must not restore the request.
pub fn take_culture_policy_request(state: &mut LoadedState, owner: i32) -> Result<bool, Error> {
    let player = state
        .retained
        .players
        .iter()
        .position(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("NPC Culture policy owner"))?;
    let culture = state.retained.players[player]
        .ai_culture
        .as_ref()
        .ok_or(Error::Unsupported("NPC Culture policy component"))?;
    if culture.must_change_policies.0 == 0 {
        return Ok(false);
    }
    state.revision = state
        .revision
        .checked_add(1)
        .ok_or(Error::Unsupported("NPC Culture policy revision"))?;
    state.retained.players[player]
        .ai_culture
        .as_mut()
        .unwrap()
        .must_change_policies
        .0 = 0;
    Ok(true)
}
/// Diplomatic205ef60 for an uninitialized evaluator roster and states that
/// cannot select a transition/action. Definition vectors alone are not a call.
pub fn activate_diplomatic_without_actions(
    state: &mut LoadedState,
    owner: i32,
) -> Result<(), Error> {
    let index = state
        .retained
        .players
        .iter()
        .position(|p| p.header.owner == owner)
        .ok_or(Error::Unsupported("NPC Diplomatic owner"))?;
    let d = state.retained.players[index]
        .ai_diplomatic
        .as_ref()
        .ok_or(Error::Unsupported("NPC Diplomatic component"))?;
    if !d.managed_contracts.is_empty() {
        return Err(Error::Unsupported("NPC Diplomatic contract activation"));
    }
    // Execute20863e0 and AdjustAllianceCosts205fd60 use these same guards.
    if d.diplo_action_evaluator
        .handlers
        .iter()
        .any(|h| h.owner != -1 && h.opponent != -1)
    {
        return Err(Error::Unsupported("NPC initialized diplomatic evaluator"));
    }
    let mut activate = Vec::with_capacity(d.diplomatic_states.states.len());
    for s in &d.diplomatic_states.states {
        let update = s.is_active.0 != 0 && s.is_updating.0 == 0 && s.opponent != -1;
        // StateVector.Execute20966b0 evaluates active transitions even when
        // UpdateState's separate recursion guard would return immediately.
        if (s.is_active.0 != 0 && s.opponent != -1 && !s.transitions.is_empty())
            || (update && (!s.allowed_action_hashes.is_empty() || !s.triggers.is_empty()))
        {
            return Err(Error::Unsupported("NPC Diplomatic state update effects"));
        }
        // HasMet2aa65a0 rejects self/invalid before reading contact cells.
        let met = s.is_active.0 == 0
            && s.opponent != -1
            && s.opponent != owner
            && state
                .world
                .player_is_valid(s.opponent)
                .map_err(|_| Error::Unsupported("NPC Diplomatic validity"))?
            && state
                .game_relations()
                .map_err(|_| Error::Unsupported("NPC Diplomatic relations"))?
                .has_met(owner, s.opponent)
                .map_err(|_| Error::Unsupported("NPC Diplomatic contact"))?;
        let changed = !update && s.has_changed_state.0 != 0;
        if (s.is_active.0 != 0 || met) && !changed && !s.allowed_action_hashes.is_empty() {
            return Err(Error::Unsupported("NPC Diplomatic external action choice"));
        }
        activate.push((met, update));
    }
    state.revision = state
        .revision
        .checked_add(1)
        .ok_or(Error::Unsupported("NPC Diplomatic revision"))?;
    let cutoff = state.retained.core.current_turn.wrapping_sub(50);
    let d = state.retained.players[index]
        .ai_diplomatic
        .as_mut()
        .unwrap();
    d.requested_items.retain(|item| item.game_turn > cutoff);
    d.favor_traded_this_turn = 0;
    d.diplo_action_evaluator.is_updated.0 = 1;
    d.diplomatic_states.is_updated.0 = 1;
    for (s, (met, update)) in d.diplomatic_states.states.iter_mut().zip(activate) {
        if update {
            s.desired_action_hashes.clear();
            s.has_changed_state.0 = 0;
            s.is_updating.0 = 0;
        }
        if met {
            s.is_active.0 = 1;
        }
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionKind {
    Pantheon,
    Technology,
    Civic,
    PlayerActions,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decision {
    pub player: i32,
    pub kind: DecisionKind,
    pub legal_indices: Vec<i32>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CallbackProgress {
    Continue,
    ChoicePending(Decision),
}
#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use = "a policy pause or player action boundary is not completed Game.Update"]
pub enum Progress {
    ChoicePending(Decision),
    /// Activation is complete. Player decisions, including EndTurn, remain external.
    PlayerTurn,
}
/// Actual subscriber boundaries. A callback is invoked at every priority in
/// current registration order; the leaf must evaluate its source guards using
/// the CURRENT priority, full-turn flag and canonical player state.
/// No default successful implementation is provided.
pub trait Effects {
    fn wait_for_jobs(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Error>;
    fn off_turn(&mut self, state: &mut LoadedState, owner: i32) -> Result<bool, Error>;
    fn callback(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        owner: i32,
        kind: Kind,
        full_turn: bool,
    ) -> Result<CallbackProgress, Error>;
    /// Complete the paused leaf from its retained policy boundary. Validate the
    /// selected current request and execute its suffix; do not replay its prefix.
    #[allow(clippy::too_many_arguments)]
    fn resume_choice(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        owner: i32,
        kind: Kind,
        full_turn: bool,
        decision: &Decision,
        selected_index: i32,
    ) -> Result<CallbackProgress, Error>;
    fn report(&mut self, state: &mut LoadedState, owner: i32, event: u32) -> Result<(), Error>;
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Stage {
    Start,
    Wait,
    OffTurn,
    OffTurnWait,
    Cleanup,
    Priority,
    Report,
    Complete,
    Failed,
}
/// One activation only. Keep this same frame across semantic policy pauses.
/// `callbacks` is the captured current native ring order, not a sorted list of
/// configured priorities. The supported ring must not change during activation.
pub struct Frame {
    binding: WireBinding,
    owner: i32,
    full_turn: bool,
    callbacks: Vec<Kind>,
    stage: Stage,
    callback_index: usize,
    pending: Option<Decision>,
    off_turn_pending: bool,
    off_turn_retries: u32,
    initial_processing_depth: Option<i32>,
}
fn control(state: &LoadedState, owner: i32) -> Result<&ControlManager, Error> {
    let value = state
        .retained
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .and_then(|p| p.ai_control.as_ref())
        .ok_or(Error::Unsupported("NPC activation Control.Manager"))?;
    if value.player_id != owner {
        return Err(Error::Unsupported("NPC activation Control owner"));
    }
    Ok(value)
}
fn edit(state: &mut LoadedState, owner: i32) -> Result<&mut ControlManager, Error> {
    control(state, owner)?;
    state.revision = state
        .revision
        .checked_add(1)
        .ok_or(Error::Unsupported("NPC activation revision capacity"))?;
    Ok(state
        .retained
        .players
        .iter_mut()
        .find(|p| p.header.owner == owner)
        .and_then(|p| p.ai_control.as_mut())
        .expect("checked"))
}
fn depth(runtime: &mut Runtime, amount: i32) -> Result<(), Error> {
    let value = runtime
        .global_ai_processing_depth
        .as_mut()
        .ok_or(Error::Unsupported("current GlobalAi processing depth"))?;
    *value = value.wrapping_add(amount);
    Ok(())
}
impl Frame {
    /// `_units` is retained for call-site compatibility. Cleanup reads the
    /// current canonical unit roster after all off-turn callbacks have run.
    pub fn new(
        state: &LoadedState,
        owner: i32,
        _units: Vec<u32>,
        full_turn: bool,
        callbacks: Vec<Kind>,
    ) -> Result<Self, Error> {
        if state.player_slot_status(owner).map_err(super::missing)? == 3 {
            return Err(Error::Unsupported("NPC activation requires nonhuman owner"));
        }
        control(state, owner)?;
        Ok(Self {
            binding: state.binding().clone(),
            owner,
            full_turn,
            callbacks,
            stage: Stage::Start,
            callback_index: 0,
            pending: None,
            off_turn_pending: false,
            off_turn_retries: u32::MAX,
            initial_processing_depth: None,
        })
    }
    pub fn owner(&self) -> i32 {
        self.owner
    }
    pub fn pending(&self) -> Option<&Decision> {
        self.pending.as_ref()
    }
    pub fn callback_index(&self) -> usize {
        self.callback_index
    }
    fn check(&self, state: &LoadedState, runtime: &Runtime) -> Result<(), Error> {
        if state.binding() != &self.binding
            || state.binding() != &runtime.binding
            || runtime.failed
            || self.stage == Stage::Failed
        {
            return Err(Error::Unsupported("NPC activation continuation owner"));
        }
        Ok(())
    }
    fn accept(&mut self, progress: CallbackProgress) -> Result<(), Error> {
        match progress {
            CallbackProgress::Continue => {
                self.pending = None;
                self.callback_index += 1;
            }
            CallbackProgress::ChoicePending(decision) => {
                if decision.player != self.owner || decision.legal_indices.is_empty() {
                    return Err(Error::Unsupported("NPC activation invalid policy boundary"));
                }
                self.pending = Some(decision);
            }
        }
        Ok(())
    }
    /// Repeated advance while paused returns the same decision without calling
    /// any effect. Hard errors retain writes and poison the shared runtime.
    pub fn advance(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        effects: &mut impl Effects,
    ) -> Result<Progress, Error> {
        self.check(state, runtime)?;
        let result = self.run(state, runtime, effects);
        if result.is_err() {
            self.stage = Stage::Failed;
            runtime.failed = true;
        }
        result
    }
    pub fn resume_choice(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        selected_index: i32,
        effects: &mut impl Effects,
    ) -> Result<Progress, Error> {
        self.check(state, runtime)?;
        let decision = self
            .pending
            .clone()
            .ok_or(Error::Unsupported("NPC activation has no pending policy"))?;
        // Invalid policy input does not execute or poison an otherwise resumable frame.
        if !decision.legal_indices.contains(&selected_index) {
            return Err(Error::Unsupported("NPC activation illegal policy choice"));
        }
        let result = (|| {
            let kind = *self
                .callbacks
                .get(self.callback_index)
                .ok_or(Error::Unsupported("NPC activation pending callback"))?;
            let progress = effects.resume_choice(
                state,
                runtime,
                self.owner,
                kind,
                self.full_turn,
                &decision,
                selected_index,
            )?;
            self.accept(progress)?;
            self.run(state, runtime, effects)
        })();
        if result.is_err() {
            self.stage = Stage::Failed;
            runtime.failed = true;
        }
        result
    }
    fn run(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        effects: &mut impl Effects,
    ) -> Result<Progress, Error> {
        loop {
            if let Some(decision) = &self.pending {
                return Ok(Progress::ChoicePending(decision.clone()));
            }
            match self.stage {
                Stage::Start => {
                    edit(state, self.owner)?.running_as_human = false;
                    // !runningAsHuman && !IsHuman -> GlobalAi.ProcessAi2020cb0.
                    self.initial_processing_depth = runtime.global_ai_processing_depth;
                    depth(runtime, 1)?;
                    self.stage = Stage::Wait;
                }
                Stage::Wait => {
                    effects.wait_for_jobs(state, self.owner)?;
                    self.stage = Stage::OffTurn;
                }
                Stage::OffTurn => {
                    self.off_turn_pending = effects.off_turn(state, self.owner)?;
                    self.stage = Stage::OffTurnWait;
                }
                Stage::OffTurnWait => {
                    effects.wait_for_jobs(state, self.owner)?;
                    if self.off_turn_pending {
                        self.off_turn_retries = self.off_turn_retries.wrapping_add(1);
                    }
                    self.stage = if self.off_turn_pending && self.off_turn_retries < 25 {
                        Stage::OffTurn
                    } else {
                        Stage::Cleanup
                    };
                }
                Stage::Cleanup => {
                    edit(state, self.owner)?.has_out_of_turn_processing = false;
                    let units = state
                        .world()
                        .player(self.owner)
                        .map_err(super::missing)?
                        .unit_records()
                        .map_err(super::missing)?
                        .iter()
                        .map(|slot| slot.value.id)
                        .collect::<Vec<_>>();
                    for id in units {
                        let assigned = state
                            .world()
                            .player(self.owner)
                            .map_err(super::missing)?
                            .unit(id)
                            .map_err(super::missing)?
                            .ok_or(Error::Unsupported("NPC activation current ordered unit"))?
                            .record
                            .ai_controller;
                        if assigned != u32::MAX
                            && !control(state, self.owner)?
                                .controls
                                .instances
                                .iter()
                                .any(|c| c.id == assigned)
                        {
                            state
                                .unit_mut(self.owner, id)
                                .map_err(super::missing)?
                                .ai_controller = u32::MAX;
                        }
                    }
                    human_activation::set_priority(state, self.owner, 4)?;
                    self.stage = Stage::Priority;
                }
                Stage::Priority => {
                    if self.callback_index == 0
                        && human_activation::current_priority(state, self.owner)? < 0
                    {
                        human_activation::set_priority(state, self.owner, 4)?;
                        self.stage = Stage::Report;
                    } else if let Some(&kind) = self.callbacks.get(self.callback_index) {
                        let progress =
                            effects.callback(state, runtime, self.owner, kind, self.full_turn)?;
                        self.accept(progress)?;
                    } else {
                        let next =
                            human_activation::current_priority(state, self.owner)?.wrapping_sub(1);
                        human_activation::set_priority(state, self.owner, next)?;
                        self.callback_index = 0;
                    }
                }
                Stage::Report => {
                    effects.report(state, self.owner, 0xa235ddf7)?;
                    self.stage = Stage::Complete;
                }
                Stage::Complete => return Ok(Progress::PlayerTurn),
                Stage::Failed => return Err(Error::Unsupported("NPC activation failed frame")),
            }
        }
    }
}

/// Install one controller on the same runtime that owns Game.Update's player cursor.
/// Refuse to overwrite an unfinished activation or external player turn.
pub fn begin_controller(
    state: &LoadedState,
    runtime: &mut Runtime,
    owner: i32,
    units: Vec<u32>,
    full_turn: bool,
    callbacks: Vec<Kind>,
) -> Result<(), Error> {
    if runtime.failed || runtime.binding != *state.binding() || runtime.npc_activation.is_some() {
        return Err(Error::Unsupported(
            "NPC activation already pending or failed",
        ));
    }
    runtime.npc_activation = Some(Frame::new(state, owner, units, full_turn, callbacks)?);
    Ok(())
}
pub fn pending(runtime: &Runtime) -> Option<&Decision> {
    runtime.npc_activation.as_ref().and_then(Frame::pending)
}
pub fn controller(runtime: &Runtime) -> Option<&Frame> {
    runtime.npc_activation.as_ref()
}
pub fn advance(
    state: &mut LoadedState,
    runtime: &mut Runtime,
    effects: &mut impl Effects,
) -> Result<Progress, Error> {
    let mut frame = runtime
        .npc_activation
        .take()
        .ok_or(Error::Unsupported("NPC activation controller missing"))?;
    let result = frame.advance(state, runtime, effects);
    runtime.npc_activation = Some(frame);
    result
}
pub fn resume_choice(
    state: &mut LoadedState,
    runtime: &mut Runtime,
    selected_index: i32,
    effects: &mut impl Effects,
) -> Result<Progress, Error> {
    let mut frame = runtime
        .npc_activation
        .take()
        .ok_or(Error::Unsupported("NPC activation controller missing"))?;
    let result = frame.resume_choice(state, runtime, selected_index, effects);
    runtime.npc_activation = Some(frame);
    result
}
/// Release only after the real ordinary deactivation path has completed.
/// DoTurnDeactivate201edb0 owns its EndAiProcessing201efff; retiring this
/// activation must not decrement processing depth a second time. Complete
/// deactivation gameplay/callbacks remain a separate required implementation.
pub fn retire_after_deactivation(state: &LoadedState, runtime: &mut Runtime) -> Result<(), Error> {
    let frame = runtime
        .npc_activation
        .as_ref()
        .ok_or(Error::Unsupported("NPC activation controller missing"))?;
    frame.check(state, runtime)?;
    let completed = usize::try_from(frame.owner)
        .ok()
        .and_then(|i| state.retained.core.player_processor.player_complete.get(i))
        .copied()
        .unwrap_or(false);
    if frame.stage != Stage::Complete
        || !completed
        || human_activation::current_priority(state, frame.owner)? != -1
        || frame.initial_processing_depth.is_none()
        || runtime.global_ai_processing_depth != frame.initial_processing_depth
    {
        return Err(Error::Unsupported("NPC real deactivation unfinished"));
    }
    runtime.npc_activation = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diplomatic_empty_active_state_retains_prefix_and_update_stores() {
        use crate::player_ai_diplomatic::{BoolByte, TradeItem};
        let (mut state, _, owner) = fixture();
        state.retained.core.current_turn = 100;
        let d = state.retained.players[0].ai_diplomatic.as_mut().unwrap();
        d.managed_contracts.clear();
        d.diplo_action_evaluator.handlers.clear();
        d.diplomatic_states.states.truncate(1);
        let s = &mut d.diplomatic_states.states[0];
        s.opponent = owner;
        s.is_active = BoolByte(1);
        s.is_updating = BoolByte(0);
        s.has_changed_state = BoolByte(1);
        s.allowed_action_hashes.clear();
        s.transitions.clear();
        s.triggers.clear();
        s.desired_action_hashes = vec![123];
        d.requested_items = [49, 50, 51]
            .map(|game_turn| TradeItem {
                game_turn,
                other_player: owner,
                item_type: 1,
                item_sub_type: 2,
                item_value: 3,
            })
            .to_vec();
        d.favor_traded_this_turn = 17;
        activate_diplomatic_without_actions(&mut state, owner).unwrap();
        let d = state.retained.players[0].ai_diplomatic.as_ref().unwrap();
        assert_eq!(
            d.requested_items
                .iter()
                .map(|i| i.game_turn)
                .collect::<Vec<_>>(),
            [51]
        );
        assert_eq!(d.favor_traded_this_turn, 0);
        assert_eq!(d.diplo_action_evaluator.is_updated.0, 1);
        assert_eq!(d.diplomatic_states.is_updated.0, 1);
        let s = &d.diplomatic_states.states[0];
        assert!(s.desired_action_hashes.is_empty());
        assert_eq!(
            (s.is_active.0, s.is_updating.0, s.has_changed_state.0),
            (1, 0, 0)
        );
    }
    #[test]
    fn diplomatic_inactive_definitions_do_not_execute_positive_state_effects() {
        use crate::player_ai_diplomatic::BoolByte;
        let (mut state, _, owner) = fixture();
        let d = state.retained.players[0].ai_diplomatic.as_mut().unwrap();
        d.managed_contracts.clear();
        d.diplo_action_evaluator.handlers.clear();
        d.diplomatic_states.states.truncate(1);
        let s = &mut d.diplomatic_states.states[0];
        s.opponent = owner;
        s.is_active = BoolByte(0);
        s.is_updating = BoolByte(0);
        s.has_changed_state = BoolByte(1);
        assert!(!s.transitions.is_empty());
        let definitions = s.transitions.clone();
        activate_diplomatic_without_actions(&mut state, owner).unwrap();
        let d = state.retained.players[0].ai_diplomatic.as_mut().unwrap();
        assert_eq!(d.diplomatic_states.states[0].transitions, definitions);
        d.diplomatic_states.states[0].is_active = BoolByte(1);
        assert!(matches!(
            activate_diplomatic_without_actions(&mut state, owner),
            Err(Error::Unsupported("NPC Diplomatic state update effects"))
        ));
        state.retained.players[0]
            .ai_diplomatic
            .as_mut()
            .unwrap()
            .diplomatic_states
            .states[0]
            .is_updating = BoolByte(1);
        assert!(matches!(
            activate_diplomatic_without_actions(&mut state, owner),
            Err(Error::Unsupported("NPC Diplomatic state update effects"))
        ));
    }
    fn fixture() -> (LoadedState, Runtime, i32) {
        let (mut state, mut runtime, owner) =
            crate::loaded_state::tests::current_update_player_fixture();
        state.controls.as_mut().unwrap().players[0].slot_status = 2;
        let c = state.retained.players[0].ai_control.as_mut().unwrap();
        c.player_id = owner;
        c.running_as_human = true;
        c.has_out_of_turn_processing = true;
        c.controls.instances.clear();
        runtime.global_ai_processing_depth = Some(i32::MAX);
        (state, runtime, owner)
    }
    struct Probe {
        trace: Vec<(i32, Kind)>,
        resumes: Vec<Kind>,
        waits: usize,
        report: bool,
        fail: bool,
    }
    impl Effects for Probe {
        fn wait_for_jobs(&mut self, _: &mut LoadedState, _: i32) -> Result<(), Error> {
            self.waits += 1;
            Ok(())
        }
        fn off_turn(&mut self, _: &mut LoadedState, _: i32) -> Result<bool, Error> {
            Ok(false)
        }
        fn callback(
            &mut self,
            state: &mut LoadedState,
            runtime: &mut Runtime,
            owner: i32,
            kind: Kind,
            _: bool,
        ) -> Result<CallbackProgress, Error> {
            assert_eq!(runtime.global_ai_processing_depth, Some(i32::MIN));
            let priority = human_activation::current_priority(state, owner)?;
            self.trace.push((priority, kind));
            if self.fail {
                return Err(Error::Unsupported("actual leaf failure"));
            }
            let decision_kind = match (priority, kind) {
                (3, Kind::Research) => Some(DecisionKind::Technology),
                (2, Kind::Culture) => Some(DecisionKind::Civic),
                (1, Kind::Religious) => Some(DecisionKind::Pantheon),
                _ => None,
            };
            Ok(match decision_kind {
                Some(kind) => CallbackProgress::ChoicePending(Decision {
                    player: owner,
                    kind,
                    legal_indices: vec![7, 11],
                }),
                None => CallbackProgress::Continue,
            })
        }
        fn resume_choice(
            &mut self,
            _: &mut LoadedState,
            _: &mut Runtime,
            _: i32,
            kind: Kind,
            _: bool,
            _: &Decision,
            selected: i32,
        ) -> Result<CallbackProgress, Error> {
            assert_eq!(selected, 11);
            self.resumes.push(kind);
            Ok(CallbackProgress::Continue)
        }
        fn report(&mut self, state: &mut LoadedState, owner: i32, event: u32) -> Result<(), Error> {
            assert_eq!(event, 0xa235ddf7);
            assert_eq!(human_activation::current_priority(state, owner)?, 4);
            self.report = true;
            Ok(())
        }
    }
    fn probe() -> Probe {
        Probe {
            trace: vec![],
            resumes: vec![],
            waits: 0,
            report: false,
            fail: false,
        }
    }
    #[test]
    fn pantheon_pause_keeps_religious_cursor_until_external_choice() {
        let (mut state, mut runtime, owner) = fixture();
        begin_controller(
            &state,
            &mut runtime,
            owner,
            vec![],
            true,
            vec![Kind::Religious],
        )
        .unwrap();
        let mut p = probe();
        assert!(matches!(
            advance(&mut state, &mut runtime, &mut p).unwrap(),
            Progress::ChoicePending(Decision {
                kind: DecisionKind::Pantheon,
                ..
            })
        ));
        let stamp = state.action_stamp();
        let trace = p.trace.clone();
        let depth = runtime.global_ai_processing_depth;
        assert!(matches!(
            advance(&mut state, &mut runtime, &mut p).unwrap(),
            Progress::ChoicePending(_)
        ));
        assert!(resume_choice(&mut state, &mut runtime, 99, &mut p).is_err());
        assert_eq!(state.action_stamp(), stamp);
        assert_eq!(p.trace, trace);
        assert_eq!(runtime.global_ai_processing_depth, depth);
        assert_eq!(
            resume_choice(&mut state, &mut runtime, 11, &mut p).unwrap(),
            Progress::PlayerTurn
        );
        assert_eq!(p.resumes, [Kind::Religious]);
        assert_eq!(
            p.trace,
            [
                (4, Kind::Religious),
                (3, Kind::Religious),
                (2, Kind::Religious),
                (1, Kind::Religious),
                (0, Kind::Religious)
            ]
        );
        assert_eq!(
            advance(&mut state, &mut runtime, &mut p).unwrap(),
            Progress::PlayerTurn
        );
        assert_eq!(p.resumes, [Kind::Religious]);
    }
    #[test]
    fn two_policy_pauses_preserve_callback_cursor_and_processing_depth() {
        let (mut state, mut runtime, owner) = fixture();
        let ring = vec![
            Kind::FreeCities,
            Kind::Research,
            Kind::Culture,
            Kind::Economic,
        ];
        begin_controller(&state, &mut runtime, owner, vec![], true, ring.clone()).unwrap();
        let mut p = probe();
        assert!(matches!(
            advance(&mut state, &mut runtime, &mut p).unwrap(),
            Progress::ChoicePending(Decision {
                kind: DecisionKind::Technology,
                ..
            })
        ));
        assert!(!control(&state, owner).unwrap().running_as_human);
        assert!(!control(&state, owner).unwrap().has_out_of_turn_processing);
        assert_eq!(p.trace.len(), 6);
        let stamp = state.action_stamp();
        assert!(matches!(
            advance(&mut state, &mut runtime, &mut p).unwrap(),
            Progress::ChoicePending(_)
        ));
        assert_eq!(p.trace.len(), 6);
        assert_eq!(state.action_stamp(), stamp);
        assert!(resume_choice(&mut state, &mut runtime, 99, &mut p).is_err());
        assert!(!runtime.failed());
        assert!(matches!(
            resume_choice(&mut state, &mut runtime, 11, &mut p).unwrap(),
            Progress::ChoicePending(Decision {
                kind: DecisionKind::Civic,
                ..
            })
        ));
        assert_eq!(p.trace.len(), 11);
        assert_eq!(runtime.global_ai_processing_depth, Some(i32::MIN));
        assert_eq!(
            resume_choice(&mut state, &mut runtime, 11, &mut p).unwrap(),
            Progress::PlayerTurn
        );
        assert_eq!(
            p.trace,
            (0..=4)
                .rev()
                .flat_map(|priority| ring.iter().map(move |&kind| (priority, kind)))
                .collect::<Vec<_>>()
        );
        assert_eq!(p.resumes, [Kind::Research, Kind::Culture]);
        assert_eq!(p.waits, 2);
        assert!(p.report);
        assert_eq!(runtime.global_ai_processing_depth, Some(i32::MIN));
        assert!(retire_after_deactivation(&state, &mut runtime).is_err());
        assert_eq!(
            advance(&mut state, &mut runtime, &mut p).unwrap(),
            Progress::PlayerTurn
        );
        assert_eq!(p.trace.len(), 20);
        state.retained.core.player_processor.player_complete[owner as usize] = true;
        // Merely signaling EndTurn does not establish completed control work.
        assert!(retire_after_deactivation(&state, &mut runtime).is_err());
        assert_eq!(runtime.global_ai_processing_depth, Some(i32::MIN));
        human_activation::set_priority(&mut state, owner, -1).unwrap();
        assert!(retire_after_deactivation(&state, &mut runtime).is_err());
        // Model the separate real deactivation EndAiProcessing tail.
        runtime.global_ai_processing_depth = Some(i32::MAX);
        retire_after_deactivation(&state, &mut runtime).unwrap();
        assert!(controller(&runtime).is_none());
        assert_eq!(runtime.global_ai_processing_depth, Some(i32::MAX));
        assert_eq!(p.waits, 2);
    }
    #[test]
    fn hard_callback_failure_retains_prefix_and_blocks_replay() {
        let (mut state, mut runtime, owner) = fixture();
        begin_controller(
            &state,
            &mut runtime,
            owner,
            vec![],
            true,
            vec![Kind::FreeCities],
        )
        .unwrap();
        let mut p = probe();
        p.fail = true;
        assert!(advance(&mut state, &mut runtime, &mut p).is_err());
        assert!(runtime.failed());
        assert_eq!(runtime.global_ai_processing_depth, Some(i32::MIN));
        assert_eq!(
            human_activation::current_priority(&state, owner).unwrap(),
            4
        );
        assert!(advance(&mut state, &mut runtime, &mut p).is_err());
        assert_eq!(p.trace.len(), 1);
        assert!(!p.report);
    }
}
