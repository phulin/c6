//! Internal GetPlayersToDeactivate and SetTurnActive(false) prefix. Neither
//! selection nor an inactive assignment completes DoTurnDeactivate/Game.Update.
use super::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pending {
    SessionStatements,
    Action,
    Deal,
    HumanSessionStatements,
}
pub trait World {
    type Player: Copy;
    fn turn_order(&mut self, index: usize) -> Result<i32, Error>;
    fn valid(&mut self, id: i32) -> Result<bool, Error>;
    fn player(&mut self, id: i32) -> Result<Self::Player, Error>;
    fn active(&mut self, p: Self::Player) -> Result<bool, Error>;
    fn id(&mut self, p: Self::Player) -> Result<i32, Error>;
    fn complete(&mut self, p: Self::Player) -> Result<bool, Error>;
    fn game_complete(&mut self, p: Self::Player) -> Result<bool, Error>;
    fn set_auto_player_game(&mut self, value: bool) -> Result<(), Error>;
    fn human(&mut self, p: Self::Player) -> Result<bool, Error>;
    fn ai_done(&mut self, p: Self::Player) -> Result<bool, Error>;
    fn validate_pending(&mut self, id: i32) -> Result<(), Error>;
    fn pending(&mut self, id: i32, query: Pending) -> Result<bool, Error>;
    fn multiplayer(&mut self) -> Result<bool, Error>;
    fn unit_counts(&mut self, p: Self::Player) -> Result<(), Error>;
    fn active_count(&mut self) -> Result<u32, Error>;
    fn set_active_count(&mut self, value: u32) -> Result<(), Error>;
    /// Native CopyAssignmentDo inserts the variable into its archive dirty tree
    /// before storing. Preserve logical dirty/value effects, not allocator IDs.
    fn assign_active(&mut self, p: Self::Player, value: bool) -> Result<(), Error>;
    fn clear_automation_processed(&mut self, id: i32) -> Result<(), Error>;
}

/// 2a0c2b0: physical positions of the saved 64-entry turn order, not GetAlive.
pub fn select<W: World>(w: &mut W) -> Result<Vec<i32>, Error> {
    let mut selected = Vec::new();
    for slot in 0..64 {
        let id = w.turn_order(slot)?;
        if !w.valid(id)? {
            continue;
        }
        let p = w.player(id)?;
        if !w.active(p)? {
            continue;
        }
        if w.complete(p)? || w.game_complete(p)? {
            w.set_auto_player_game(false)?;
        } else {
            if w.human(p)? || !w.ai_done(p)? {
                continue;
            }
            let current = w.id(p)?;
            w.validate_pending(current)?;
            let current = w.id(p)?;
            let mut has_pending = w.pending(current, Pending::SessionStatements)?;
            if !has_pending {
                let current = w.id(p)?;
                has_pending = w.pending(current, Pending::Action)?;
            }
            if !has_pending {
                let current = w.id(p)?;
                has_pending = w.pending(current, Pending::Deal)?;
            }
            if !has_pending {
                has_pending = w.pending(1, Pending::HumanSessionStatements)?;
            }
            if has_pending && !w.multiplayer()? {
                continue;
            }
            // Native calls GetUnitCounts but ignores every returned counter.
            w.unit_counts(p)?;
        }
        selected.push(id);
    }
    Ok(selected)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effect<P> {
    /// Required Platform logging, followed by local/nonlocal reporting, BEFORE
    /// the caller enters DoAutomation. Not a completed SetTurnActive.
    TurnActiveLog {
        player: P,
        active: bool,
    },
    TurnActiveReport {
        player: P,
        event: u32,
    },
    Automation {
        player: P,
    },
}

/// 2b5ad20 -> 2b5adc0, only through the first required logging boundary.
/// ActiveCount is u32 wrapping arithmetic and precedes archived bool assignment.
pub fn begin<W: World>(w: &mut W, p: W::Player) -> Result<Effect<W::Player>, Error> {
    if !w.active(p)? {
        return Ok(Effect::Automation { player: p });
    }
    let count = w.active_count()?;
    w.set_active_count(count.wrapping_sub(1))?;
    w.assign_active(p, false)?;
    let id = w.id(p)?;
    w.clear_automation_processed(id)?;
    Ok(Effect::TurnActiveLog {
        player: p,
        active: false,
    })
}
