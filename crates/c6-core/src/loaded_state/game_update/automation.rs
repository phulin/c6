//! Current Player.DoAutomation prefix and logical linked-unit cursor. Unit
//! records must be in actual native live-chain order. No sorted ID snapshot.
use super::{missing, required};
use crate::{game_update::Error, loaded_state::LoadedState, unit::Unit};

#[derive(Debug)]
pub enum Begin {
    Skipped,
    Units(Frame),
}
#[derive(Debug)]
pub struct Frame {
    owner: i32,
    human: bool,
    cursor: Option<i32>,
    current: Option<(i32, u32)>,
    any_attention: bool,
    instance: u64,
    failed: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnitWork {
    pub actor: (i32, u32),
    pub human: bool,
    pub had_pending: bool,
}
#[derive(Debug)]
pub struct RequiresDiplomacy {
    pub owner: i32,
    pub human: bool,
    pub any_attention: bool,
}

fn index(owner: i32) -> Result<usize, Error> {
    usize::try_from(owner)
        .ok()
        .filter(|i| *i < 64)
        .ok_or(Error::Unsupported("automation player slot"))
}
fn revision(state: &mut LoadedState) -> Result<(), Error> {
    state.revision = state
        .revision
        .checked_add(1)
        .ok_or(Error::Unsupported("loaded revision exhausted"))?;
    Ok(())
}
fn processed(state: &mut LoadedState, owner: i32) -> Result<(), Error> {
    let i = index(owner)?;
    revision(state)?;
    state.retained.core.player_processor.automation_processed[i] = true;
    Ok(())
}
fn processing(state: &mut LoadedState, owner: i32) -> Result<(), Error> {
    if state.controls.is_none() {
        return Err(Error::Unsupported("current processingAutomation controls"));
    }
    revision(state)?;
    state
        .controls
        .as_mut()
        .expect("checked")
        .processing_automation = Some(owner);
    Ok(())
}

/// 2b5b1a0 through actual Units.Begin/End. Tactical permission is queried only
/// after the processed flag, and IsAlive only after tactical permission passes.
pub fn begin(
    state: &mut LoadedState,
    owner: i32,
    mut allow_tactical: impl FnMut(&LoadedState, i32) -> Result<bool, Error>,
) -> Result<Begin, Error> {
    state.world.player(owner).map_err(missing)?;
    if state.retained.core.player_processor.automation_processed[index(owner)?] {
        return Ok(Begin::Skipped);
    }
    if !allow_tactical(state, owner)? || !state.player_alive(owner).map_err(missing)? {
        processed(state, owner)?;
        return Ok(Begin::Skipped);
    }
    let human = state.player_slot_status(owner).map_err(missing)? == 3;
    processing(state, owner)?;
    let cursor = state
        .world
        .player(owner)
        .map_err(missing)?
        .unit_records()
        .map_err(missing)?
        .first()
        .map(|s| s.index);
    Ok(Begin::Units(Frame {
        owner,
        human,
        cursor,
        current: None,
        any_attention: false,
        instance: state.instance_identity,
        failed: false,
    }))
}
impl Frame {
    fn check(&self, state: &LoadedState) -> Result<(), Error> {
        if self.failed || state.instance_identity != self.instance {
            Err(Error::Unsupported(
                "automation cursor failed or replaced state",
            ))
        } else {
            Ok(())
        }
    }
    pub fn fail(&mut self) {
        self.failed = true;
    }
    /// Dereference current live node then HasPendingOperations, BEFORE the
    /// per-unit UpdateAutomation callback. Subsequent fields are reread live.
    pub fn take_unit(&mut self, state: &LoadedState) -> Result<Option<UnitWork>, Error> {
        self.check(state)?;
        if self.current.is_some() {
            return Err(Error::Unsupported("automation unit update already pending"));
        }
        let Some(slot) = self.cursor else {
            return Ok(None);
        };
        let unit = state
            .world
            .player(self.owner)
            .map_err(missing)?
            .unit_records()
            .map_err(missing)?
            .iter()
            .find(|s| s.index == slot)
            .ok_or(Error::Unsupported("automation current live node removed"))?;
        self.current = Some((slot, unit.value.id));
        Ok(Some(UnitWork {
            actor: (self.owner, unit.value.id),
            human: self.human,
            had_pending: !unit.value.k_operations.nodes.is_empty(),
        }))
    }
    pub fn current_unit<'a>(&self, state: &'a LoadedState) -> Result<&'a Unit, Error> {
        self.check(state)?;
        let (slot, id) = required(self.current, "automation current unit pending")?;
        let u = &state
            .world
            .player(self.owner)
            .map_err(missing)?
            .unit_records()
            .map_err(missing)?
            .iter()
            .find(|s| s.index == slot)
            .ok_or(Error::Unsupported("automation unit node removed"))?
            .value;
        if u.id != id {
            return Err(Error::Unsupported("automation unit node replaced"));
        }
        Ok(u)
    }
    pub fn player_complete(&self, state: &LoadedState) -> Result<bool, Error> {
        self.check(state)?;
        Ok(state.retained.core.player_processor.player_complete[index(self.owner)?])
    }
    /// Native SetNeedsAttention(true) only marks its archive dirty then stores.
    /// This does not include subsequent stock log output or advance the cursor.
    pub fn set_attention(&mut self, state: &mut LoadedState) -> Result<(), Error> {
        let id = self.current_unit(state)?.id;
        state
            .unit_mut(self.owner, id)
            .map_err(missing)?
            .b_needs_attention = true;
        Ok(())
    }
    /// Called only AFTER actual attention logging (or qualified omission).
    pub fn attention_logged(&mut self) {
        self.any_attention = true;
    }
    pub fn advance(&mut self, state: &LoadedState) -> Result<(), Error> {
        self.current_unit(state)?;
        let (slot, _) = self.current.expect("checked");
        let records = state
            .world
            .player(self.owner)
            .map_err(missing)?
            .unit_records()
            .map_err(missing)?;
        let position = records
            .iter()
            .position(|s| s.index == slot)
            .ok_or(Error::Unsupported("automation live node invalidated"))?;
        // Logical native next pointer; current roster order is not sorted.
        // Insertions appended during callbacks are visible. A removed current
        // node is rejected rather than guessed from an initial ID snapshot.
        self.cursor = records.get(position + 1).map(|s| s.index);
        self.current = None;
        Ok(())
    }
    pub fn finish(self, state: &mut LoadedState) -> Result<RequiresDiplomacy, Error> {
        self.check(state)?;
        if self.cursor.is_some() || self.current.is_some() {
            return Err(Error::Unsupported(
                "automation units remain before diplomacy",
            ));
        }
        processing(state, -1)?;
        Ok(RequiresDiplomacy {
            owner: self.owner,
            human: self.human,
            any_attention: self.any_attention,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loaded_state::tests::current_update_player_fixture;
    #[test]
    fn processed_flag_skips_every_later_query_and_tactical_rejection_marks_only_processed() {
        let (mut state, _, owner) = current_update_player_fixture();
        state.retained.core.player_processor.automation_processed[owner as usize] = true;
        let stamp = state.action_stamp();
        assert!(matches!(
            begin(&mut state, owner, |_, _| panic!("unreached tactical")),
            Ok(Begin::Skipped)
        ));
        assert_eq!(state.action_stamp(), stamp);
        state.retained.core.player_processor.automation_processed[owner as usize] = false;
        state
            .controls
            .as_mut()
            .unwrap()
            .players
            .iter_mut()
            .find(|v| v.owner == owner)
            .unwrap()
            .is_alive = None;
        assert!(matches!(
            begin(&mut state, owner, |_, _| Ok(false)),
            Ok(Begin::Skipped)
        ));
        assert!(state.retained.core.player_processor.automation_processed[owner as usize]);
        assert_eq!(state.processing_automation().unwrap(), -1);
    }
    #[test]
    fn empty_human_unit_pass_keeps_mandatory_diplomacy_and_does_not_mark_processed() {
        let (mut state, _, owner) = current_update_player_fixture();
        state.retained.core.player_processor.automation_processed[owner as usize] = false;
        let Begin::Units(mut frame) = begin(&mut state, owner, |_, _| Ok(true)).unwrap() else {
            panic!()
        };
        assert_eq!(state.processing_automation().unwrap(), owner);
        assert_eq!(frame.take_unit(&state).unwrap(), None);
        let tail = frame.finish(&mut state).unwrap();
        assert!(tail.human);
        assert!(!tail.any_attention);
        assert_eq!(tail.owner, owner);
        assert_eq!(state.processing_automation().unwrap(), -1);
        assert!(!state.retained.core.player_processor.automation_processed[owner as usize]);
    }
    #[test]
    fn current_linked_order_is_preserved_without_sorting_unit_ids_or_slots() {
        use crate::{map::Slot, world};
        let (mut state, _, owner) = current_update_player_fixture();
        state.retained.core.player_processor.automation_processed[owner as usize] = false;
        let mut a = crate::loaded_state::tests::commit_fixture_unit();
        a.e_owner = owner;
        a.id = 0x10007;
        let mut b = a.clone();
        b.id = 0x10002;
        let player = world::Player::from_lookup_slots(
            owner,
            Some(world::CompleteSlots {
                entries: vec![Slot { index: 7, value: a }, Slot { index: 2, value: b }],
            }),
            None,
        )
        .unwrap();
        state.world = world::World::new(state.world.map().clone(), vec![player]).unwrap();
        let Begin::Units(mut frame) = begin(&mut state, owner, |_, _| Ok(true)).unwrap() else {
            panic!()
        };
        let first = frame.take_unit(&state).unwrap().unwrap();
        assert_eq!(first.actor, (owner, 0x10007));
        assert!(frame.take_unit(&state).is_err());
        frame.set_attention(&mut state).unwrap();
        frame.attention_logged();
        frame.advance(&state).unwrap();
        assert_eq!(
            frame.take_unit(&state).unwrap().unwrap().actor,
            (owner, 0x10002)
        );
        frame.advance(&state).unwrap();
        assert_eq!(frame.take_unit(&state).unwrap(), None);
        let tail = frame.finish(&mut state).unwrap();
        assert!(tail.any_attention);
        assert_eq!(state.processing_automation().unwrap(), -1);
    }
}
