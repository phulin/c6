//! Ordered nonhuman control deactivation over the existing canonical owners.
//! A false readiness result retains the current priority and yields an explicit
//! unfinished frame. No native policy chooses player actions here.
use super::{human_activation, npc_activation::Kind, Error, LoadedState, Runtime};
use crate::loaded_state::ActionStamp;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Progress {
    Pending,
    Complete,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Stage {
    Drain,
    End,
    Cleanup,
    Depth,
    Timer,
    Complete,
    Failed,
}
pub trait Effects {
    fn ready(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        owner: i32,
        kind: Kind,
    ) -> Result<bool, Error>;
    fn priority_end(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        owner: i32,
        kind: Kind,
    ) -> Result<(), Error>;
    fn end(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        owner: i32,
        kind: Kind,
        priority: i32,
    ) -> Result<(), Error>;
    fn cleanup(&mut self, state: &mut LoadedState, owner: i32) -> Result<(), Error>;
    fn turn_complete(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        owner: i32,
    ) -> Result<(), Error>;
}
/// The three initialized sequences have distinct order and signatures. Keep
/// this frame through a pending readiness result; completion is idempotent.
pub struct Frame {
    identity: ActionStamp,
    owner: i32,
    full: bool,
    readiness: Vec<Kind>,
    priorities: Vec<Kind>,
    end: Vec<Kind>,
    stage: Stage,
}
impl Frame {
    pub fn new(
        state: &LoadedState,
        owner: i32,
        full: bool,
        readiness: Vec<Kind>,
        priorities: Vec<Kind>,
        end: Vec<Kind>,
    ) -> Result<Self, Error> {
        if state.player_slot_status(owner).map_err(super::missing)? == 3 {
            return Err(Error::Unsupported(
                "NPC deactivation requires nonhuman owner",
            ));
        }
        human_activation::current_priority(state, owner)?;
        Ok(Self {
            identity: state.action_stamp(),
            owner,
            full,
            readiness,
            priorities,
            end,
            stage: Stage::Drain,
        })
    }
    pub fn owner(&self) -> i32 {
        self.owner
    }
    pub fn is_complete(&self) -> bool {
        self.stage == Stage::Complete
    }
    pub fn advance(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        effects: &mut impl Effects,
    ) -> Result<Progress, Error> {
        if !self.identity.same_instance(state.action_stamp())
            || runtime.binding != *state.binding()
            || runtime.failed
            || self.stage == Stage::Failed
        {
            return Err(Error::Unsupported("NPC deactivation frame identity/state"));
        }
        if self.stage == Stage::Complete
            && human_activation::current_priority(state, self.owner)? != -1
        {
            return Err(Error::Unsupported(
                "new NPC activation requires a new deactivation frame",
            ));
        }
        let result = self.advance_inner(state, runtime, effects);
        if result.is_err() {
            self.stage = Stage::Failed;
            runtime.failed = true;
        }
        result
    }
    fn advance_inner(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut Runtime,
        effects: &mut impl Effects,
    ) -> Result<Progress, Error> {
        loop {
            match self.stage {
                Stage::Drain => {
                    if !self.full || human_activation::current_priority(state, self.owner)? < 0 {
                        self.stage = Stage::End;
                        continue;
                    }
                    let mut ready = true;
                    for &kind in &self.readiness {
                        // Native AND does not short-circuit later callbacks.
                        ready &= effects.ready(state, runtime, self.owner, kind)?;
                    }
                    if !ready {
                        return Ok(Progress::Pending);
                    }
                    if human_activation::current_priority(state, self.owner)? >= 0 {
                        for &kind in &self.priorities {
                            effects.priority_end(state, runtime, self.owner, kind)?;
                        }
                        let next =
                            human_activation::current_priority(state, self.owner)?.wrapping_sub(1);
                        human_activation::set_priority(state, self.owner, next)?;
                    }
                }
                Stage::End => {
                    for &kind in &self.end {
                        effects.end(state, runtime, self.owner, kind, -1)?;
                    }
                    self.stage = Stage::Cleanup;
                }
                Stage::Cleanup => {
                    if self.full {
                        effects.cleanup(state, self.owner)?;
                    }
                    self.stage = Stage::Depth;
                }
                Stage::Depth => {
                    let running_as_human = state
                        .retained
                        .players
                        .iter()
                        .find(|p| p.header.owner == self.owner)
                        .and_then(|p| p.ai_control.as_ref())
                        .ok_or(Error::Unsupported("NPC current Control Manager"))?
                        .running_as_human;
                    if !running_as_human
                        && state
                            .player_slot_status(self.owner)
                            .map_err(super::missing)?
                            != 3
                    {
                        let depth = runtime
                            .global_ai_processing_depth
                            .as_mut()
                            .ok_or(Error::Unsupported("current GlobalAi processing depth"))?;
                        *depth = depth.wrapping_sub(1);
                    }
                    self.stage = Stage::Timer;
                }
                Stage::Timer => {
                    effects.turn_complete(state, runtime, self.owner)?;
                    self.stage = Stage::Complete;
                }
                Stage::Complete => return Ok(Progress::Complete),
                Stage::Failed => return Err(Error::Unsupported("failed NPC deactivation")),
            }
        }
    }
}

/// Shared source DoTurnEnd ownership release; this performs no role coercion.
pub fn release_controller(
    state: &mut LoadedState,
    owner: i32,
    controller: u32,
) -> Result<(), Error> {
    super::human_control::release_controller(state, owner, controller)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, Eq, PartialEq)]
    enum Call {
        Ready(Kind, i32),
        Priority(Kind, i32),
        End(Kind, i32),
        Cleanup,
        Timer(i32),
    }
    struct Calls {
        seen: Vec<Call>,
        block_once: bool,
        change_priority: bool,
        fail_cleanup: bool,
    }
    impl Effects for Calls {
        fn ready(
            &mut self,
            s: &mut LoadedState,
            _: &mut Runtime,
            o: i32,
            k: Kind,
        ) -> Result<bool, Error> {
            self.seen
                .push(Call::Ready(k, human_activation::current_priority(s, o)?));
            if self.block_once {
                self.block_once = false;
                Ok(false)
            } else {
                Ok(true)
            }
        }
        fn priority_end(
            &mut self,
            s: &mut LoadedState,
            _: &mut Runtime,
            o: i32,
            k: Kind,
        ) -> Result<(), Error> {
            self.seen
                .push(Call::Priority(k, human_activation::current_priority(s, o)?));
            if self.change_priority && k == Kind::Homeland {
                human_activation::set_priority(s, o, 0)?;
            }
            Ok(())
        }
        fn end(
            &mut self,
            _: &mut LoadedState,
            _: &mut Runtime,
            _: i32,
            k: Kind,
            p: i32,
        ) -> Result<(), Error> {
            self.seen.push(Call::End(k, p));
            Ok(())
        }
        fn cleanup(&mut self, _: &mut LoadedState, _: i32) -> Result<(), Error> {
            self.seen.push(Call::Cleanup);
            if self.fail_cleanup {
                Err(Error::Unsupported("destructor"))
            } else {
                Ok(())
            }
        }
        fn turn_complete(
            &mut self,
            _: &mut LoadedState,
            runtime: &mut Runtime,
            _: i32,
        ) -> Result<(), Error> {
            self.seen
                .push(Call::Timer(runtime.global_ai_processing_depth.unwrap()));
            Ok(())
        }
    }
    fn fixture(priority: i32) -> (LoadedState, Runtime, i32) {
        let (mut s, mut r, o) = crate::loaded_state::tests::current_update_player_fixture();
        s.controls.as_mut().unwrap().players[0].slot_status = 2;
        let c = s.retained.players[0].ai_control.as_mut().unwrap();
        c.player_id = o;
        c.priority = priority;
        c.running_as_human = false;
        r.global_ai_processing_depth = Some(8);
        (s, r, o)
    }
    fn calls() -> Calls {
        Calls {
            seen: vec![],
            block_once: false,
            change_priority: false,
            fail_cleanup: false,
        }
    }
    #[test]
    fn pending_readiness_visits_all_consumers_and_retains_priority_and_depth() {
        let (mut s, mut r, o) = fixture(1);
        let mut f = Frame::new(
            &s,
            o,
            true,
            vec![Kind::Targets, Kind::BehaviorTree],
            vec![Kind::Homeland],
            vec![Kind::Planning, Kind::City],
        )
        .unwrap();
        let mut c = calls();
        c.block_once = true;
        assert_eq!(
            f.advance(&mut s, &mut r, &mut c).unwrap(),
            Progress::Pending
        );
        assert_eq!(
            c.seen,
            [
                Call::Ready(Kind::Targets, 1),
                Call::Ready(Kind::BehaviorTree, 1)
            ]
        );
        assert_eq!(human_activation::current_priority(&s, o).unwrap(), 1);
        assert_eq!(r.global_ai_processing_depth, Some(8));
        assert_eq!(
            f.advance(&mut s, &mut r, &mut c).unwrap(),
            Progress::Complete
        );
        assert!(human_activation::is_done_turn(&s, o).unwrap());
        assert_eq!(r.global_ai_processing_depth, Some(7));
        let end = c
            .seen
            .iter()
            .position(|x| *x == Call::End(Kind::Planning, -1))
            .unwrap();
        assert_eq!(
            &c.seen[end..],
            [
                Call::End(Kind::Planning, -1),
                Call::End(Kind::City, -1),
                Call::Cleanup,
                Call::Timer(7)
            ]
        );
        let n = c.seen.len();
        assert_eq!(
            f.advance(&mut s, &mut r, &mut c).unwrap(),
            Progress::Complete
        );
        assert_eq!(c.seen.len(), n);
        assert_eq!(r.global_ai_processing_depth, Some(7));
    }
    #[test]
    fn callbacks_reread_current_priority_and_cleanup_failure_precedes_depth_release() {
        let (mut s, mut r, o) = fixture(2);
        let mut f = Frame::new(
            &s,
            o,
            true,
            vec![],
            vec![Kind::Homeland, Kind::Tactical],
            vec![Kind::City],
        )
        .unwrap();
        let mut c = calls();
        c.change_priority = true;
        c.fail_cleanup = true;
        assert!(f.advance(&mut s, &mut r, &mut c).is_err());
        assert_eq!(
            c.seen,
            [
                Call::Priority(Kind::Homeland, 2),
                Call::Priority(Kind::Tactical, 0),
                Call::End(Kind::City, -1),
                Call::Cleanup
            ]
        );
        assert_eq!(human_activation::current_priority(&s, o).unwrap(), -1);
        assert_eq!(r.global_ai_processing_depth, Some(8));
        assert!(r.failed);
        assert!(f.advance(&mut s, &mut r, &mut c).is_err());
    }
}
