//! Linux XP2 Quests.Manager.DoTurn26a7710 / CleanupFinishedQuests26a7720.
//! World upkeep only cleans the two quest lists. Generation, completion rewards,
//! and player decisions belong to their respective action/callback owners.
use c6_core::loaded_state::{ActionStamp, LoadedState};

/// Complete native QuestData value (nine signed words). Definition ordinals
/// remain ordinals; they are never substituted for serialized definition hashes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Quest {
    pub state: i32,
    pub client: i32,
    pub actor: i32,
    pub quest_type: i32,
    pub id: i32,
    pub turn_started: i32,
    pub turn_completed: i32,
    pub primary_parameter: i32,
    pub secondary_parameter: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    MissingInstance,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "round quests: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Sole current/completed quest-list owner, initialized from the same BEFORE
/// as LoadedState. Other quest callbacks must update these same lists. The
/// wire projection alone omits inline parameters and cannot initialize them.
#[derive(Debug)]
pub struct Runtime {
    instance: ActionStamp,
    pub current: Vec<Quest>,
    pub completed: Vec<Quest>,
}
impl Runtime {
    pub fn from_initial(state: &LoadedState, current: Vec<Quest>, completed: Vec<Quest>) -> Self {
        Self {
            instance: state.action_stamp(),
            current,
            completed,
        }
    }
    pub fn validate(&self, state: &LoadedState) -> Result<(), Error> {
        if !self.instance.same_instance(state.action_stamp()) {
            return Err(Error::MissingInstance);
        }
        Ok(())
    }
}

fn cleanup(current: &mut Vec<Quest>, completed: &mut Vec<Quest>) {
    let mut index = 0;
    while index < current.len() {
        match current[index].state {
            2 => {
                completed.push(current[index]);
                current.remove(index);
            }
            3 => {
                current.remove(index);
            }
            _ => {
                index += 1;
            }
        }
    }
}

/// Native DoTurn is a tail jump to cleanup. It neither tests the current turn
/// nor invokes registered quest logic, generation/timers, signals, or RNG.
/// Completed records append in current-list order; cancelled records disappear;
/// every other state value (including unknown ordinals) remains unchanged.
pub fn run(state: &LoadedState, runtime: &mut Runtime) -> Result<(), Error> {
    runtime.validate(state)?;
    cleanup(&mut runtime.current, &mut runtime.completed);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn quest(id: i32, state: i32) -> Quest {
        Quest {
            state,
            client: 2,
            actor: 7,
            quest_type: -1,
            id,
            turn_started: -8,
            turn_completed: 11,
            primary_parameter: i32::MIN,
            secondary_parameter: i32::MAX,
        }
    }
    #[test]
    fn cleanup_preserves_records_and_order_across_adjacent_removals() {
        let original = vec![
            quest(1, 2),
            quest(2, 3),
            quest(3, 2),
            quest(4, 1),
            quest(5, -1),
            quest(6, 2),
            quest(7, 3),
        ];
        let mut current = original.clone();
        let mut completed = vec![quest(0, 2)];
        cleanup(&mut current, &mut completed);
        assert_eq!(current, vec![original[3], original[4]]);
        assert_eq!(
            completed,
            vec![quest(0, 2), original[0], original[2], original[5]]
        );
        let expected = (current.clone(), completed.clone());
        cleanup(&mut current, &mut completed);
        assert_eq!((current, completed), expected);
    }
}
