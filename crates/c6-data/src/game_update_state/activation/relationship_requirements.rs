//! Live relationship predicates on the PlayerTurnStarted signal.
//! SimpleRequirement state/Changed delivery belongs to the shared requirement
//! owner; these functions compute source Tests and preserve event gates.
use crate::{resource_binding::ResourceAuthority, unit_initialization_definitions, Rules};
mod border_army;
use c6_core::{loaded_components::RetainedPlayer, loaded_state::LoadedState};
#[derive(Debug)]
pub enum Error {
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Relations(c6_core::game_diplomatic_relations::Error),
    Definitions(unit_initialization_definitions::Error),
    Rules(crate::Error),
    Placement(crate::movement_placement_state::Error),
    Missing(&'static str),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    TurnStarted,
    Ally,
    DeclaredFriend,
    FriendlyCityOccupied,
    PillageLead {
        minimum: i32,
        percentage_bits: u32,
    },
    NearCultureBorder {
        radius: u32,
        lookback: i32,
        threshold: i32,
    },
    MetXTurns {
        turns: i32,
    },
    AtPeace {
        turns: i32,
    },
    OpenBorders {
        grace_turns: i32,
    },
}
#[derive(Clone, Copy, Debug)]
pub struct Receiver {
    pub kind: Kind,
    pub context: i32,
    pub subject: i32,
}
pub struct Definitions {
    units: crate::registry::LinuxDefinitionRegistry,
    border_units: Vec<border_army::UnitDefinition>,
}
impl Definitions {
    pub fn from_rules(rules: &Rules, authority: &ResourceAuthority<'_>) -> Result<Self, Error> {
        unit_initialization_definitions::Definitions::validate_resources(rules, authority)
            .map_err(Error::Definitions)?;
        let units = unit_initialization_definitions::collection(rules, "Units", "UnitType")
            .map_err(Error::Definitions)?;
        let table = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Missing("current army Units table"))?;
        let border_units = units
            .entries()
            .iter()
            .map(|entry| {
                let row = table
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Missing("current army Unit row"))?;
                Ok(border_army::UnitDefinition {
                    trade: row.integer("MakeTradeRoute").map_err(Error::Rules)? != 0,
                    spy: row.integer("Spy").map_err(Error::Rules)? != 0,
                    civilian: crate::registry::linux_type_hash(
                        row.text("FormationClass").map_err(Error::Rules)?.as_bytes(),
                    ) == 0xc2510aa7,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Self {
            units,
            border_units,
        })
    }
}
fn player(state: &LoadedState, owner: i32) -> Result<&RetainedPlayer, Error> {
    state.world().player(owner).map_err(Error::World)?;
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("current requirement player"))
}
/// Callback gates precede SimpleRequirement's active guard and virtual Test.
/// In particular MetX does not query HasMet for another reported owner.
pub fn should_invalidate(state: &LoadedState, r: Receiver, reported: i32) -> Result<bool, Error> {
    event_gate(r, reported, || player_has_met(state, r.context, r.subject))
}
fn player_has_met(state: &LoadedState, context: i32, subject: i32) -> Result<bool, Error> {
    // Player.Diplomacy.HasMet2aa65a0 rejects invalid/self before the matrix.
    if !state
        .world()
        .player_is_valid(subject)
        .map_err(Error::World)?
        || context == subject
    {
        return Ok(false);
    }
    state
        .game_relations()
        .map_err(Error::State)?
        .has_met(context, subject)
        .map_err(Error::Relations)
}
fn event_gate(
    r: Receiver,
    reported: i32,
    has_met: impl FnOnce() -> Result<bool, Error>,
) -> Result<bool, Error> {
    match r.kind {
        Kind::TurnStarted | Kind::AtPeace { .. } | Kind::PillageLead { .. } => {
            Ok(r.context == reported)
        }
        Kind::Ally
        | Kind::DeclaredFriend
        | Kind::FriendlyCityOccupied
        | Kind::NearCultureBorder { .. } => Ok(r.subject == reported),
        Kind::MetXTurns { .. } => {
            if r.context != reported {
                return Ok(false);
            }
            has_met()
        }
        Kind::OpenBorders { .. } => Ok(true),
    }
}
/// Actual native result convention: 0 satisfied, 1 unsatisfied, 2 permanently
/// satisfied (elapsed contact), 3 invalid.
/// Positive occupation and existing-promise changes remain explicit until their
/// gameplay continuation is supplied.
pub fn test(
    state: &LoadedState,
    defs: &Definitions,
    alive_majors: &[i32],
    r: Receiver,
) -> Result<i32, Error> {
    let context = player(state, r.context)?;
    let subject = player(state, r.subject)?;
    match r.kind {
        Kind::TurnStarted => Ok(0),
        Kind::PillageLead {
            minimum,
            percentage_bits,
        } => Ok(pillage_lead(
            context.stats.tiles_pillaged,
            subject.stats.tiles_pillaged,
            minimum,
            percentage_bits,
        )),
        Kind::Ally => Ok(i32::from(
            !state
                .game_relations()
                .map_err(Error::State)?
                .is_allied(r.subject, r.context)
                .map_err(Error::Relations)?,
        )),
        Kind::DeclaredFriend => Ok(i32::from(
            !state
                .game_relations()
                .map_err(Error::State)?
                .has_declared_friendship(r.subject, r.context)
                .map_err(Error::Relations)?,
        )),
        Kind::FriendlyCityOccupied => {
            let relations = state.game_relations().map_err(Error::State)?;
            for &major in alive_majors {
                player(state, major)?;
                if (relations
                    .has_declared_friendship(r.context, major)
                    .map_err(Error::Relations)?
                    || relations
                        .is_allied(r.context, major)
                        .map_err(Error::Relations)?)
                    && !subject.cities.records.entries.is_empty()
                {
                    return Err(Error::Missing("current original-city occupation test"));
                }
            }
            Ok(1)
        }
        Kind::NearCultureBorder {
            radius,
            lookback,
            threshold,
        } => {
            if r.context == r.subject {
                return Ok(3);
            }
            let since = state
                .retained()
                .core
                .current_turn
                .wrapping_sub(lookback)
                .max(0);
            if border_army::has_recent_incursion(&context.diplomacy, r.subject, since) {
                return Ok(3);
            }
            let army = &context
                .ai_targets
                .as_ref()
                .ok_or(Error::Missing("current border army knowledge"))?
                .knowledge_base
                .player_armies
                .0
                .get(
                    usize::try_from(r.subject)
                        .map_err(|_| Error::Missing("border subject index"))?,
                )
                .ok_or(Error::Missing("border subject index"))?;
            let count = border_army::count(
                army,
                radius,
                r.context,
                r.subject,
                |hash| {
                    let index = defs
                        .units
                        .find_index(hash)
                        .ok_or(Error::Missing("current army Unit definition"))?;
                    defs.border_units
                        .get(index)
                        .copied()
                        .ok_or(Error::Missing("current army Unit definition"))
                },
                |location| border_army::plot_owner(state, location),
            )?;
            if count < threshold {
                return Ok(1);
            }
            // UpdatePromise first asks IsPromiseMade. Absence returns without
            // mutation, callbacks or RNG. An existing promise may BreakPromise;
            // keep that reached gameplay transition explicit.
            if border_army::has_border_promise(&context.diplomacy, r.subject) {
                return Err(Error::Missing("current border promise update"));
            }
            Ok(0)
        }
        Kind::OpenBorders { grace_turns } => {
            if crate::movement_placement_state::player_open_borders(state, r.context, r.subject)
                .map_err(Error::Placement)?
            {
                return Ok(0);
            }
            if grace_turns > 0 {
                let ended = *context
                    .diplomacy
                    .open_borders_ended_turn
                    .get(
                        usize::try_from(r.subject)
                            .map_err(|_| Error::Missing("open border subject index"))?,
                    )
                    .ok_or(Error::Missing("open border ending turn"))?;
                // GetTurnsSinceOpenBordersEnded returns -1 for a negative ending turn
                // before subtracting from the current turn.
                let elapsed = if ended < 0 {
                    -1
                } else {
                    state.retained().core.current_turn.wrapping_sub(ended)
                };
                if elapsed >= 0 && elapsed < grace_turns {
                    return Ok(0);
                }
            }
            Ok(1)
        }
        Kind::MetXTurns { turns } => {
            if !player_has_met(state, r.context, r.subject)? {
                return Ok(1);
            }
            let met = state
                .game_relations()
                .map_err(Error::State)?
                .met_turn(r.context, r.subject)
                .map_err(Error::Relations)?;
            // 2387da1 adds before the signed comparison, including overflow.
            Ok(
                if met.wrapping_add(turns) <= state.retained().core.current_turn {
                    2
                } else {
                    1
                },
            )
        }
        Kind::AtPeace { turns } => {
            let relations = state.game_relations().map_err(Error::State)?;
            if relations
                .is_at_war(r.context, r.subject)
                .map_err(Error::Relations)?
                || !player_has_met(state, r.context, r.subject)?
            {
                return Ok(1);
            }
            if turns < 0 {
                return Ok(0);
            }
            let changed = relations
                .war_changed_turn(r.context, r.subject)
                .map_err(Error::Relations)?;
            Ok(i32::from(
                state.retained().core.current_turn.wrapping_sub(changed) < turns,
            ))
        }
    }
}

// PlayerPillageLead23c2a00: 64-bit difference, separate SSE f32 casts/mul,
// and an ordered comparison. NaN follows the nonpositive arm and fails.
fn pillage_lead(context: i32, subject: i32, minimum: i32, percentage_bits: u32) -> i32 {
    if (context <= 0 && subject <= 0)
        || (i64::from(context) - i64::from(subject)).abs() < i64::from(minimum)
    {
        return 1;
    }
    let percentage = f32::from_bits(percentage_bits);
    let satisfied = if percentage > 0.0 {
        subject as f32 >= percentage * context as f32
    } else {
        context as f32 >= f32::from_bits((percentage * subject as f32).to_bits() ^ 0x8000_0000)
    };
    i32::from(!satisfied)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pillage_lead_uses_native_direction_minimum_and_ordered_float_comparison() {
        assert_eq!(pillage_lead(0, 0, 0, 0f32.to_bits()), 1);
        assert_eq!(pillage_lead(10, 14, 5, 1.25f32.to_bits()), 1);
        assert_eq!(pillage_lead(10, 15, 5, 1.5f32.to_bits()), 0);
        assert_eq!(pillage_lead(15, 10, 5, (-1.5f32).to_bits()), 0);
        assert_eq!(
            pillage_lead(i32::MIN, i32::MAX, i32::MAX, 1f32.to_bits()),
            0
        );
        assert_eq!(pillage_lead(10, 15, 5, f32::NAN.to_bits()), 1);
        let r = Receiver {
            kind: Kind::PillageLead {
                minimum: 5,
                percentage_bits: 1.5f32.to_bits(),
            },
            context: 0,
            subject: 1,
        };
        assert!(event_gate(r, 0, || panic!("no contact query")).unwrap());
        assert!(!event_gate(r, 1, || panic!("no contact query")).unwrap());
    }
    #[test]
    fn subject_and_context_callbacks_have_distinct_native_filters() {
        let mut r = Receiver {
            kind: Kind::Ally,
            context: 0,
            subject: 1,
        };
        assert!(matches!(
            event_gate(r, 0, || panic!("unexpected contact query")),
            Ok(false)
        ));
        assert!(matches!(
            event_gate(r, 1, || panic!("unexpected contact query")),
            Ok(true)
        ));
        r.kind = Kind::TurnStarted;
        assert!(matches!(
            event_gate(r, 0, || panic!("unexpected contact query")),
            Ok(true)
        ));
        assert!(matches!(
            event_gate(r, 1, || panic!("unexpected contact query")),
            Ok(false)
        ));
    }
    #[test]
    fn elapsed_contact_callback_does_not_resolve_unreached_contact_storage() {
        let r = Receiver {
            kind: Kind::MetXTurns { turns: 10 },
            context: 0,
            subject: 1,
        };
        assert!(matches!(
            event_gate(r, 1, || panic!("other player cannot reach HasMet")),
            Ok(false)
        ));
        assert!(matches!(
            event_gate(r, 0, || Err(Error::Missing("contact"))),
            Err(Error::Missing("contact"))
        ));
    }
}

#[derive(Debug)]
pub enum DispatchError<E> {
    Binding,
    Missing(u64),
    Predicate(Error),
    Change(super::requirement_changes::Error<E>),
}
#[derive(Clone, Copy, Debug)]
pub struct Invocation {
    pub receiver: u64,
    pub descriptor: Receiver,
    pub reported: i32,
}
/// One actual signal subscriber. The shared owner supplies the first active
/// read and performs the second read, inverse/trigger store and Changed fanout.
pub fn dispatch<S: super::requirement_changes::Services>(
    state: &mut LoadedState,
    defs: &Definitions,
    alive_majors: &[i32],
    runtime: &mut super::requirement_changes::Runtime,
    backend: &mut S,
    invocation: Invocation,
) -> Result<(), DispatchError<S::Error>> {
    let Invocation {
        receiver,
        descriptor,
        reported,
    } = invocation;
    if runtime.binding() != state.binding() {
        return Err(DispatchError::Binding);
    }
    if !should_invalidate(state, descriptor, reported).map_err(DispatchError::Predicate)? {
        return Ok(());
    }
    if !runtime
        .state(receiver)
        .ok_or(DispatchError::Missing(receiver))?
        .active
    {
        return Ok(());
    }
    let tested = test(state, defs, alive_majors, descriptor).map_err(DispatchError::Predicate)?;
    runtime
        .set_state(state, backend, receiver, tested, true)
        .map_err(DispatchError::Change)
}
