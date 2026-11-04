//! Linux movement expenditure shared by normal Formation movement and upkeep.
//!
//! This changes authoritative movement fields and executes ordered continuations;
//! it does not perform Placement::Set, pathfinding, visibility/contact or a move
//! action. Required synchronous callbacks/RNG and current-formation updates are
//! explicit providers. Callers must resolve their support before mutation.
use crate::formation_upkeep::{self, FormationUpkeepContext, FormationUpkeepError};
use crate::unit::{ComponentId, Unit};
use crate::unit_turn_upkeep::{
    MovementReport, MovementStage, MOVES_EXHAUSTED, MOVES_REMAINING_CHANGED,
};

pub const FORMATION_MOVES_CHANGED: u32 = 0x3d75b3ed;

/// Same retained live Unit across callbacks. No copyback or default subscribers.
pub trait UnitMovementContext {
    type Error;
    fn unit(&self) -> Result<&Unit, Self::Error>;
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error>;
    fn send_movement_report(&mut self, report: MovementReport) -> Result<(), Self::Error>;
    /// Read current EditFormation, and when nonnull reread it and execute the
    /// existing SyncMovesRemaining -> UpdateAttributes continuation in full.
    fn sync_current_formation(&mut self) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnitMovementError<E> {
    UnitUnavailable(E),
    ContinuationFailed { stage: MovementStage, error: E },
}

fn identity(unit: &Unit) -> ComponentId {
    ComponentId {
        owner: unit.e_owner as i16,
        component_type: 1,
        id: unit.id,
    }
}

/// Unit::ChangeMovesRemaining (0x2c277f0). Wrap signed Q8 addition, then clamp
/// negative results to zero. Report only an actual field change. Exhaustion and
/// formation lookup reread state after subscribers; the returned bool retains
/// the initial change even when a subscriber changes the field again.
pub fn change_unit_moves<C: UnitMovementContext>(
    c: &mut C,
    delta_q8: i32,
) -> Result<bool, UnitMovementError<C::Error>> {
    let old = c
        .unit()
        .map_err(UnitMovementError::UnitUnavailable)?
        .x_moves_remaining;
    let changed = old.wrapping_add(delta_q8).max(0);
    if changed == old {
        return Ok(false);
    }
    c.unit_mut()
        .map_err(UnitMovementError::UnitUnavailable)?
        .x_moves_remaining = changed;
    let read = |error| UnitMovementError::ContinuationFailed {
        stage: MovementStage::UnitReadback,
        error,
    };
    let report = MovementReport {
        event_hash: MOVES_REMAINING_CHANGED,
        identity: identity(c.unit().map_err(read)?),
        raw_q8: Some(c.unit().map_err(read)?.x_moves_remaining),
    };
    c.send_movement_report(report)
        .map_err(|error| UnitMovementError::ContinuationFailed {
            stage: MovementStage::ChangedReport,
            error,
        })?;
    if c.unit().map_err(read)?.x_moves_remaining == 0 {
        let report = MovementReport {
            event_hash: MOVES_EXHAUSTED,
            identity: identity(c.unit().map_err(read)?),
            raw_q8: None,
        };
        c.send_movement_report(report)
            .map_err(|error| UnitMovementError::ContinuationFailed {
                stage: MovementStage::ExhaustedReport,
                error,
            })?;
    }
    c.sync_current_formation()
        .map_err(|error| UnitMovementError::ContinuationFailed {
            stage: MovementStage::FormationSync,
            error,
        })?;
    Ok(true)
}

/// FinishMoves retains the pre-report Formation, unlike ChangeMovesRemaining.
pub trait FinishMovementContext: UnitMovementContext {
    type Formation: Copy;
    fn current_formation(&mut self) -> Result<Option<Self::Formation>, Self::Error>;
    fn sync_retained_formation(&mut self, formation: Self::Formation) -> Result<(), Self::Error>;
}
/// Unit.FinishMoves2c27d40: no movement or null Formation returns unchanged.
/// Exhausted is unconditional after the changed report, even if a subscriber
/// grants movement. Synchronization targets the Formation captured before writes.
pub fn finish_unit_moves<C: FinishMovementContext>(
    c: &mut C,
) -> Result<(), UnitMovementError<C::Error>> {
    if c.unit()
        .map_err(UnitMovementError::UnitUnavailable)?
        .x_moves_remaining
        == 0
    {
        return Ok(());
    }
    let Some(formation) = c
        .current_formation()
        .map_err(UnitMovementError::UnitUnavailable)?
    else {
        return Ok(());
    };
    let unit = c.unit_mut().map_err(UnitMovementError::UnitUnavailable)?;
    unit.x_moves_remaining = 0;
    unit.b_eligible_to_heal = false;
    let read = |error| UnitMovementError::ContinuationFailed {
        stage: MovementStage::UnitReadback,
        error,
    };
    let report = MovementReport {
        event_hash: MOVES_REMAINING_CHANGED,
        identity: identity(c.unit().map_err(read)?),
        raw_q8: Some(c.unit().map_err(read)?.x_moves_remaining),
    };
    c.send_movement_report(report)
        .map_err(|error| UnitMovementError::ContinuationFailed {
            stage: MovementStage::ChangedReport,
            error,
        })?;
    let report = MovementReport {
        event_hash: MOVES_EXHAUSTED,
        identity: identity(c.unit().map_err(read)?),
        raw_q8: None,
    };
    c.send_movement_report(report)
        .map_err(|error| UnitMovementError::ContinuationFailed {
            stage: MovementStage::ExhaustedReport,
            error,
        })?;
    c.sync_retained_formation(formation)
        .map_err(|error| UnitMovementError::ContinuationFailed {
            stage: MovementStage::FormationSync,
            error,
        })
}

pub trait FinishFormationContext: FormationUpkeepContext {
    fn finish_unit(&mut self, unit: Self::UnitHandle)
        -> Result<(), UnitMovementError<Self::Error>>;
}
/// Formation.FinishMoves293d350 assigns its own movement/HasMoved before
/// visiting current member IDs; it does not call SetHasMoved's attention loop.
pub fn finish_formation_moves<C: FinishFormationContext>(
    c: &mut C,
) -> Result<(), FormationMovementError<C::Error>> {
    let f = c
        .formation_mut()
        .map_err(FormationMovementError::ContinuationFailed)?;
    f.moves_remaining_raw = 0;
    f.has_moved = true;
    let units = c
        .units()
        .map_err(FormationMovementError::ContinuationFailed)?;
    let mut index = 0;
    loop {
        let ids = &c
            .formation()
            .map_err(FormationMovementError::ContinuationFailed)?
            .unit_ids;
        if index == ids.len() {
            break;
        }
        let id = *ids
            .get(index)
            .ok_or(FormationMovementError::InvalidatedMemberCursor { index })?;
        if let Some(unit) = c
            .find_unit(units, id)
            .map_err(FormationMovementError::ContinuationFailed)?
        {
            c.finish_unit(unit).map_err(FormationMovementError::Unit)?;
        }
        index += 1;
    }
    formation_upkeep::update_attributes(c).map_err(FormationMovementError::Attributes)
}

/// Formation movement expenditure borrows the existing attribute computation.
/// Unit/Units handles keep their retained native receiver identity. The native
/// member cursor must remain valid across callbacks (no invalidating vector
/// reallocation/erasure); replacement contents/end are reread in native order.
pub trait FormationMovementContext: FormationUpkeepContext {
    fn unit_mut(&mut self, unit: Self::UnitHandle) -> Result<&mut Unit, Self::Error>;
    /// Pure support check before any write, covering reports, nested current
    /// formations and the final existing UpdateAttributes continuation.
    fn preflight_change_movement(&self) -> Result<(), Self::Error>;
    fn sync_unit_current_formation(&mut self, unit: Self::UnitHandle) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormationMovementError<E> {
    EffectsUnavailable(E),
    ContinuationFailed(E),
    InvalidatedMemberCursor { index: usize },
    Unit(UnitMovementError<E>),
    Attributes(FormationUpkeepError<E>),
}
struct Member<'a, C: FormationMovementContext> {
    context: &'a mut C,
    unit: C::UnitHandle,
}
impl<C: FormationMovementContext> UnitMovementContext for Member<'_, C> {
    type Error = C::Error;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        self.context.unit(self.unit)
    }
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
        self.context.unit_mut(self.unit)
    }
    fn send_movement_report(&mut self, r: MovementReport) -> Result<(), Self::Error> {
        self.context.send_member_movement(r)
    }
    fn sync_current_formation(&mut self) -> Result<(), Self::Error> {
        self.context.sync_unit_current_formation(self.unit)
    }
}

/// Formation::SetHasMoved(bool), 0x293d2c0. The assignment does not suppress an
/// equal-value call: every found member has NeedsAttention set to FALSE, even
/// when HasMoved is set to true. No movement query, report or RNG is performed.
/// The setter closure writes the retained actual Unit; null Find skips it.
pub fn set_formation_has_moved<C: FormationUpkeepContext>(
    c: &mut C,
    value: bool,
    mut set_needs_attention: impl FnMut(&mut C, C::UnitHandle, bool) -> Result<(), C::Error>,
) -> Result<(), FormationUpkeepError<C::Error>> {
    let result = FormationUpkeepError::ContinuationFailed;
    c.formation_mut().map_err(result)?.has_moved = value;
    let units = c.units().map_err(result)?;
    let mut index = 0;
    loop {
        let members = &c.formation().map_err(result)?.unit_ids;
        if index == members.len() {
            break;
        }
        let id = *members
            .get(index)
            .ok_or(FormationUpkeepError::InvalidatedMemberCursor { index })?;
        if let Some(unit) = c.find_unit(units, id).map_err(result)? {
            set_needs_attention(c, unit, false).map_err(result)?;
        }
        index += 1;
    }
    Ok(())
}

/// Formation::ChangeMovesRemaining (0x293cf50), reached by the normal MoveTo
/// step *after* Placement::Set. The same original delta applies to every found
/// member, not the formation's clamped difference. The original formation-change
/// bit remains captured across nested callbacks. Final attributes always update.
/// Errors retain completed writes/callback effects; this is not an atomic move.
pub fn change_formation_moves<C: FormationMovementContext>(
    c: &mut C,
    delta_q8: i32,
) -> Result<(), FormationMovementError<C::Error>> {
    c.preflight_change_movement()
        .map_err(FormationMovementError::EffectsUnavailable)?;
    let old = c
        .formation()
        .map_err(FormationMovementError::ContinuationFailed)?
        .moves_remaining_raw;
    let next = old.wrapping_add(delta_q8).max(0);
    let changed = next != old;
    c.formation_mut()
        .map_err(FormationMovementError::ContinuationFailed)?
        .moves_remaining_raw = next;
    let units = c
        .units()
        .map_err(FormationMovementError::ContinuationFailed)?;
    let mut index = 0;
    loop {
        let ids = &c
            .formation()
            .map_err(FormationMovementError::ContinuationFailed)?
            .unit_ids;
        if index == ids.len() {
            break;
        }
        let id = *ids
            .get(index)
            .ok_or(FormationMovementError::InvalidatedMemberCursor { index })?;
        if let Some(unit) = c
            .find_unit(units, id)
            .map_err(FormationMovementError::ContinuationFailed)?
        {
            let heal = c
                .unit(unit)
                .map_err(FormationMovementError::ContinuationFailed)?
                .k_health_modifiers
                .b_can_heal_after_action;
            c.unit_mut(unit)
                .map_err(FormationMovementError::ContinuationFailed)?
                .b_eligible_to_heal = heal;
            let unit_changed = change_unit_moves(&mut Member { context: c, unit }, delta_q8)
                .map_err(FormationMovementError::Unit)?;
            if changed && !unit_changed {
                let f = c
                    .formation()
                    .map_err(FormationMovementError::ContinuationFailed)?;
                let report = MovementReport {
                    event_hash: FORMATION_MOVES_CHANGED,
                    identity: ComponentId {
                        owner: f.owner as i16,
                        component_type: 0x2c,
                        id: f.id,
                    },
                    raw_q8: Some(f.moves_remaining_raw),
                };
                c.send_member_movement(report)
                    .map_err(FormationMovementError::ContinuationFailed)?;
            }
        }
        index += 1;
    }
    formation_upkeep::update_attributes(c).map_err(FormationMovementError::Attributes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formation_upkeep::{BooleanQuery, IntegerQuery};
    use crate::unit::{Formation, PlotCoord};

    struct Context {
        f: Formation,
        units: Vec<Unit>,
        reports: Vec<MovementReport>,
        syncs: usize,
        admitted: bool,
        mutate_first_report: bool,
        fail_report: bool,
    }
    impl Context {
        fn new(moves: &[i32], formation_moves: i32) -> Self {
            let units = moves
                .iter()
                .enumerate()
                .map(|(id, &moves)| {
                    let mut u = crate::unit_turn_upkeep::tests::source_only_unit();
                    u.id = id as u32;
                    u.e_owner = 0;
                    u.x_moves_remaining = moves;
                    u.e_activity = 0;
                    u.k_health_modifiers.b_can_heal_after_action = id == 0;
                    u
                })
                .collect();
            Self {
                f: Formation {
                    id: 17,
                    owner: 0,
                    location: PlotCoord { x: 2, y: 3 },
                    moves_remaining_raw: formation_moves,
                    domain: 2,
                    is_embarked: false,
                    can_embark: false,
                    can_attack: false,
                    is_ranged_combat: false,
                    can_capture: false,
                    has_moved: false,
                    has_moved_into_zoc: false,
                    ignores_zoc: false,
                    last_path: vec![],
                    last_path_cache_location: PlotCoord { x: -1, y: -1 },
                    unit_ids: (0..moves.len() as u32).collect(),
                },
                units,
                reports: vec![],
                syncs: 0,
                admitted: true,
                mutate_first_report: false,
                fail_report: false,
            }
        }
    }
    impl FormationUpkeepContext for Context {
        type Error = &'static str;
        type UnitHandle = usize;
        type Definitions = ();
        type Units = ();
        fn formation(&self) -> Result<&Formation, Self::Error> {
            Ok(&self.f)
        }
        fn formation_mut(&mut self) -> Result<&mut Formation, Self::Error> {
            Ok(&mut self.f)
        }
        fn preflight_update_attributes(&self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn definitions(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn units(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn find_unit(&mut self, _: (), id: u32) -> Result<Option<usize>, Self::Error> {
            Ok(self.units.iter().position(|u| u.id == id))
        }
        fn unit(&self, h: usize) -> Result<&Unit, Self::Error> {
            self.units.get(h).ok_or("retained unit absent")
        }
        fn integer(&mut self, _: usize, q: IntegerQuery) -> Result<i32, Self::Error> {
            Ok(match q {
                IntegerQuery::Domain => 2,
                IntegerQuery::MaxMoves => 2,
                _ => 0,
            })
        }
        fn boolean(&mut self, _: usize, _: BooleanQuery) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn native_type_index(&mut self, _: usize) -> Result<u32, Self::Error> {
            Ok(0)
        }
        fn definition_can_capture(&mut self, _: (), _: u32) -> Result<Option<bool>, Self::Error> {
            Ok(Some(false))
        }
        fn send_member_movement(&mut self, r: MovementReport) -> Result<(), Self::Error> {
            self.reports.push(r);
            if self.fail_report {
                return Err("subscriber failed");
            }
            if self.mutate_first_report {
                self.mutate_first_report = false;
                self.units[0].x_moves_remaining = 0;
                self.units[0].e_owner = 7;
            }
            Ok(())
        }
    }
    impl FormationMovementContext for Context {
        fn unit_mut(&mut self, h: usize) -> Result<&mut Unit, Self::Error> {
            self.units.get_mut(h).ok_or("retained unit absent")
        }
        fn preflight_change_movement(&self) -> Result<(), Self::Error> {
            if self.admitted {
                Ok(())
            } else {
                Err("unknown movement effects")
            }
        }
        fn sync_unit_current_formation(&mut self, _: usize) -> Result<(), Self::Error> {
            self.syncs += 1;
            formation_upkeep::update_attributes(self).map_err(|_| "nested attributes failed")
        }
    }
    #[test]
    fn expenditure_uses_original_delta_per_member_and_existing_attribute_fold() {
        let mut c = Context::new(&[100, 300], 100);
        change_formation_moves(&mut c, -200).unwrap();
        assert_eq!(
            c.units
                .iter()
                .map(|u| u.x_moves_remaining)
                .collect::<Vec<_>>(),
            [0, 100]
        );
        assert_eq!(c.f.moves_remaining_raw, 0);
        assert!(c.units[0].b_eligible_to_heal);
        assert!(!c.units[1].b_eligible_to_heal);
        assert_eq!(c.syncs, 2);
        assert!(c.f.has_moved);
        assert!(c
            .reports
            .iter()
            .any(|r| r.event_hash == MOVES_EXHAUSTED && r.identity.id == 0));
    }
    #[test]
    fn equal_formation_still_visits_members_and_updates_attributes() {
        let mut c = Context::new(&[100], 0);
        change_formation_moves(&mut c, -200).unwrap();
        assert_eq!(c.units[0].x_moves_remaining, 0);
        assert_eq!(c.syncs, 1);
        assert!(c.f.has_moved);
    }
    #[test]
    fn unchanged_unit_emits_formation_identity_event_when_formation_changed() {
        let mut c = Context::new(&[0], 100);
        change_formation_moves(&mut c, -200).unwrap();
        assert_eq!(c.syncs, 0);
        assert_eq!(
            c.reports[0],
            MovementReport {
                event_hash: FORMATION_MOVES_CHANGED,
                identity: ComponentId {
                    owner: 0,
                    component_type: 0x2c,
                    id: 17
                },
                raw_q8: Some(0)
            }
        );
    }
    #[test]
    fn repeated_member_is_charged_twice_not_deduplicated() {
        let mut c = Context::new(&[700], 700);
        c.f.unit_ids.push(0);
        change_formation_moves(&mut c, -200).unwrap();
        assert_eq!(c.units[0].x_moves_remaining, 300);
        assert_eq!(c.syncs, 2);
        assert_eq!(c.f.moves_remaining_raw, 300);
    }
    #[test]
    fn report_mutation_controls_exhaustion_and_identity_without_copyback() {
        let mut c = Context::new(&[500], 500);
        c.mutate_first_report = true;
        change_formation_moves(&mut c, -100).unwrap();
        assert_eq!(c.units[0].x_moves_remaining, 0);
        assert_eq!(c.reports[0].raw_q8, Some(400));
        assert_eq!(c.reports[1].event_hash, MOVES_EXHAUSTED);
        assert_eq!(c.reports[1].identity.owner, 7);
    }
    #[test]
    fn preflight_refusal_is_inert_but_failed_subscriber_retains_prefix() {
        let mut c = Context::new(&[500], 500);
        c.admitted = false;
        assert!(matches!(
            change_formation_moves(&mut c, -100),
            Err(FormationMovementError::EffectsUnavailable(_))
        ));
        assert_eq!(
            (c.f.moves_remaining_raw, c.units[0].x_moves_remaining),
            (500, 500)
        );
        assert!(c.reports.is_empty());
        c.admitted = true;
        c.fail_report = true;
        assert!(matches!(
            change_formation_moves(&mut c, -100),
            Err(FormationMovementError::Unit(
                UnitMovementError::ContinuationFailed {
                    stage: MovementStage::ChangedReport,
                    ..
                }
            ))
        ));
        assert_eq!(
            (c.f.moves_remaining_raw, c.units[0].x_moves_remaining),
            (400, 400)
        );
        assert_eq!(c.syncs, 0);
    }
    #[test]
    fn signed_wrapping_addition_precedes_nonnegative_clamp() {
        let mut c = Context::new(&[i32::MAX], i32::MAX);
        change_formation_moves(&mut c, 1).unwrap();
        assert_eq!(
            (c.f.moves_remaining_raw, c.units[0].x_moves_remaining),
            (0, 0)
        );
    }
    #[test]
    fn has_moved_true_clears_attention_even_when_equal_and_skips_missing_members() {
        let mut c = Context::new(&[256, 512], 256);
        c.f.has_moved = true;
        c.f.has_moved_into_zoc = true;
        c.f.unit_ids.insert(1, 999);
        for unit in &mut c.units {
            unit.b_needs_attention = true;
        }
        c.admitted = false; // No reporting or attribute-query support is needed.
        let mut visited = vec![];
        set_formation_has_moved(&mut c, true, |c, handle, attention| {
            visited.push(handle);
            c.units[handle].b_needs_attention = attention;
            Ok(())
        })
        .unwrap();
        assert_eq!(visited, vec![0, 1]);
        assert!(c.f.has_moved && c.f.has_moved_into_zoc);
        assert!(c.units.iter().all(|u| !u.b_needs_attention));
        assert_eq!(c.f.moves_remaining_raw, 256);
        assert!(c.reports.is_empty());
    }
    struct Finish {
        unit: Unit,
        current: Option<u32>,
        reports: Vec<MovementReport>,
        synced: Vec<u32>,
        fail: bool,
    }
    impl UnitMovementContext for Finish {
        type Error = &'static str;
        fn unit(&self) -> Result<&Unit, Self::Error> {
            Ok(&self.unit)
        }
        fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
            Ok(&mut self.unit)
        }
        fn send_movement_report(&mut self, r: MovementReport) -> Result<(), Self::Error> {
            self.reports.push(r);
            if r.event_hash == MOVES_REMAINING_CHANGED {
                self.unit.x_moves_remaining = 512;
                self.current = Some(99);
            } else if self.fail {
                return Err("exhausted callback");
            }
            Ok(())
        }
        fn sync_current_formation(&mut self) -> Result<(), Self::Error> {
            panic!("must retain earlier Formation")
        }
    }
    impl FinishMovementContext for Finish {
        type Formation = u32;
        fn current_formation(&mut self) -> Result<Option<u32>, Self::Error> {
            Ok(self.current)
        }
        fn sync_retained_formation(&mut self, f: u32) -> Result<(), Self::Error> {
            self.synced.push(f);
            Ok(())
        }
    }
    fn finish_fixture() -> Finish {
        let mut unit = crate::unit_turn_upkeep::tests::source_only_unit();
        unit.x_moves_remaining = 256;
        unit.b_eligible_to_heal = true;
        Finish {
            unit,
            current: Some(17),
            reports: vec![],
            synced: vec![],
            fail: false,
        }
    }
    #[test]
    fn finish_retains_formation_and_exhausted_report_after_callback_grants_moves() {
        let mut c = finish_fixture();
        finish_unit_moves(&mut c).unwrap();
        assert_eq!(c.unit.x_moves_remaining, 512);
        assert!(!c.unit.b_eligible_to_heal);
        assert_eq!(
            c.reports.iter().map(|r| r.event_hash).collect::<Vec<_>>(),
            [MOVES_REMAINING_CHANGED, MOVES_EXHAUSTED]
        );
        assert_eq!(c.synced, [17]);
    }
    #[test]
    fn finish_failure_keeps_completed_writes_and_does_not_sync() {
        let mut c = finish_fixture();
        c.fail = true;
        assert!(matches!(
            finish_unit_moves(&mut c),
            Err(UnitMovementError::ContinuationFailed {
                stage: MovementStage::ExhaustedReport,
                ..
            })
        ));
        assert_eq!(c.unit.x_moves_remaining, 512);
        assert!(!c.unit.b_eligible_to_heal);
        assert!(c.synced.is_empty());
    }
    #[test]
    fn finish_without_formation_keeps_unit_and_healing_unchanged() {
        let mut c = finish_fixture();
        c.current = None;
        finish_unit_moves(&mut c).unwrap();
        assert_eq!(c.unit.x_moves_remaining, 256);
        assert!(c.unit.b_eligible_to_heal);
        assert!(c.reports.is_empty());
    }
}
