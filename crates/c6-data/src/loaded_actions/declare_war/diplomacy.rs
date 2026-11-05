//! Reached Player.Diplomacy.OnWarDeclared major surprise-war callbacks.
use super::Definitions;
use c6_core::{game_diplomatic_relations::war::GrievanceLogItem, loaded_state::LoadedState};
#[derive(Debug)]
pub enum Error<E> {
    State(c6_core::loaded_state::Error),
    Relations(c6_core::game_diplomatic_relations::Error),
    Definitions(super::Error<std::convert::Infallible>),
    Missing(&'static str),
    Report(E),
}
pub trait Reports<P> {
    type Error;
    /// Reporting.Event7989583f is synchronous between score mutation and log.
    fn grievances(
        &mut self,
        state: &mut LoadedState,
        backend: &mut P,
        holder: i32,
        source: i32,
    ) -> Result<(), Self::Error>;
}
fn diplomacy<E>(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::player_diplomacy::Diplomacy, Error<E>> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .map(|p| &p.diplomacy)
        .ok_or(Error::Missing("current Player.Diplomacy"))
}
fn native_i32(value: f64) -> i32 {
    // CVTTSD2SI returns its indefinite integer for NaN/out-of-range, unlike
    // Rust's saturating float cast. Arithmetic preceding this is signed32.
    if !value.is_finite() || value < f64::from(i32::MIN) || value >= 2147483648.0 {
        i32::MIN
    } else {
        value as i32
    }
}
pub fn apply_grievances<P, R: Reports<P>>(
    state: &mut LoadedState,
    backend: &mut P,
    holder: i32,
    source: i32,
    amount: i32,
    reports: &mut R,
) -> Result<(), Error<R::Error>> {
    if amount == 0 {
        return Ok(());
    }
    let modifiers = diplomacy::<R::Error>(state, holder)?
        .grievance_generation_modifier
        .wrapping_add(diplomacy::<R::Error>(state, source)?.grievance_generation_modifier)
        .wrapping_add(100);
    let adjusted = native_i32(f64::from(modifiers.wrapping_mul(amount)) * 0.01);
    let turn = state.retained().core.current_turn;
    state
        .war_relations_mut()
        .map_err(Error::State)?
        .change_grievance_and_tension(holder, source, adjusted, turn)
        .map_err(Error::Relations)?;
    reports
        .grievances(state, backend, holder, source)
        .map_err(Error::Report)?;
    state
        .war_relations_mut()
        .map_err(Error::State)?
        .append_grievance_log(GrievanceLogItem {
            holder,
            source,
            turn,
            amount: adjusted,
            reason: b"LOC_DIPLO_GRIEVANCE_LOG_WAR_DECLARED".to_vec(),
            other_reason: vec![],
        })
        .map_err(Error::Relations)
}
/// Both principals must be admitted major civilizations by the action. The
/// current target-major branch has no city-state war-count adjustment. Allied
/// third-party sharing, defensive-pact war propagation and broken promises
/// require their positive continuations instead of silently changing nothing.
pub fn on_war_declared<P, R: Reports<P>>(
    state: &mut LoadedState,
    backend: &mut P,
    defs: &Definitions,
    receiver: i32,
    issuer: i32,
    target: i32,
    reports: &mut R,
) -> Result<(), Error<R::Error>> {
    super::principal_war_counters(state, receiver, issuer, target).map_err(Error::State)?;
    if receiver == issuer {
        return Ok(());
    }
    let relations = state.game_relations().map_err(Error::State)?;
    if relations
        .is_in_defensive_pact(receiver, target)
        .map_err(Error::Relations)?
        && !relations
            .is_at_war(receiver, issuer)
            .map_err(Error::Relations)?
    {
        return Err(Error::Missing("defensive-pact declaration continuation"));
    }
    if relations
        .has_met(receiver, issuer)
        .map_err(Error::Relations)?
    {
        // The source reads grievance score before ComputeDOWWarmongerPoints,
        // including for a nonfriendly third party whose share is zero.
        let _score = relations
            .grievance_score(issuer, target)
            .map_err(Error::Relations)?;
        let amount = defs
            .warmonger_points(state, issuer)
            .map_err(Error::Definitions)?;
        if receiver == target {
            apply_grievances(state, backend, receiver, issuer, amount, reports)?;
        } else if relations
            .is_allied(receiver, target)
            .map_err(Error::Relations)?
            || relations
                .has_declared_friendship(receiver, target)
                .map_err(Error::Relations)?
        {
            return Err(Error::Missing("allied/friendly third-party grievances"));
        }
    }
    if receiver == target {
        // IsPromiseMade first searches the per-player map. Its empty source
        // map reaches neither promise type lookup nor BreakPromise.
        if !diplomacy::<R::Error>(state, receiver)?
            .promises_made
            .is_empty()
        {
            return Err(Error::Missing(
                "current IsPromiseMade/BreakPromise continuation",
            ));
        }
    }
    Ok(())
}
