//! Live declaration/removal requirement callbacks. Queries read current canonical
//! gameplay owners; changed delivery uses the shared requirement/set owner.
use crate::loaded_actions::government_actions::requirements::Continuations;
use crate::melee_attack::backend::requirements::Runtime;
use c6_core::{
    loaded_state::LoadedState,
    melee_attack::proposals::subject_sets::{self as sets, changes},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    DeclaredWar,
    AtPeace,
    AttackedContinent,
    Delegation,
    Embassy,
}
#[derive(Clone, Copy, Debug)]
pub struct Receiver {
    pub label: u64,
    pub kind: Kind,
    pub context: Option<i32>,
    pub subject: Option<i32>,
    pub minimum_turns: i32,
    pub transient_defender: i32,
    pub war_type: i32,
    pub require_met: bool,
    pub exclude_same_emergency: bool,
}
#[derive(Clone, Copy, Debug)]
pub enum Query {
    AtWar(i32, i32),
    HasMet(i32, i32),
    AtWarChangeTurn(i32, i32),
    CurrentTurn,
    StartedWar(i32, i32),
    WarType(i32, i32),
    JointWarTarget(i32, i32),
    FightingJointWar(i32, i32),
    CapitalContinent(i32),
    Delegation(i32, i32),
    Embassy(i32, i32),
    /// EmergencyManager.GetEmergency(defender,-1) contains issuer AND context.
    SameEmergency {
        defender: i32,
        issuer: i32,
        context: i32,
    },
}
/// Scalar queries are native getter values, never stored Test answers. Missing
/// gameplay inputs must return an error. Alive IDs preserve Manager vector order.
pub trait Queries {
    type Error;
    fn query(&mut self, state: &LoadedState, query: Query) -> Result<i32, Self::Error>;
    fn alive_ids(&mut self, state: &LoadedState) -> Result<Vec<i32>, Self::Error>;
}
#[derive(Debug)]
pub enum Error<E> {
    Query(E),
    Unsupported(&'static str),
    Changed(Box<sets::Error<Error<E>>>),
    Continuation,
}
fn test<Q: Queries>(
    state: &LoadedState,
    r: &mut Receiver,
    q: &mut Q,
) -> Result<i32, Error<Q::Error>> {
    let (Some(a), Some(b)) = (r.context, r.subject) else {
        return match r.kind {
            Kind::Delegation | Kind::Embassy => Ok(1),
            _ => Err(Error::Unsupported("war requirement Player interface")),
        };
    };
    let mut get = |query| q.query(state, query).map_err(Error::Query);
    match r.kind {
        Kind::Delegation => return Ok(i32::from(get(Query::Delegation(b, a))? == 0)),
        Kind::Embassy => return Ok(i32::from(get(Query::Embassy(b, a))? == 0)),
        Kind::AtPeace => {
            if get(Query::AtWar(a, b))? != 0 || get(Query::HasMet(a, b))? == 0 {
                return Ok(1);
            }
            if r.minimum_turns >= 0
                && get(Query::CurrentTurn)?.wrapping_sub(get(Query::AtWarChangeTurn(a, b))?)
                    < r.minimum_turns
            {
                return Ok(1);
            }
            return Ok(0);
        }
        _ => {}
    }
    if a == b {
        return Ok(3);
    }
    if r.kind == Kind::DeclaredWar && r.require_met && get(Query::HasMet(a, b))? == 0 {
        r.transient_defender = -1;
        return Ok(1);
    }
    for p in q.alive_ids(state).map_err(Error::Query)? {
        if p == b {
            continue;
        }
        let mut get = |query| q.query(state, query).map_err(Error::Query);
        if r.kind == Kind::DeclaredWar {
            if r.transient_defender != -1 && p != r.transient_defender {
                continue;
            }
            if r.war_type != -1 {
                if get(Query::WarType(b, p))? != r.war_type {
                    continue;
                }
                if r.war_type == 0x1d3fe969 {
                    if p != a && get(Query::FightingJointWar(a, b))? != 0 {
                        r.transient_defender = -1;
                        return Ok(0);
                    }
                    continue;
                }
            }
            if get(Query::StartedWar(b, p))? != 0
                && (r.war_type == -1 || get(Query::WarType(b, p))? == r.war_type)
            {
                r.transient_defender = -1;
                return Ok(0);
            }
        } else if get(Query::StartedWar(b, p))? != 0
            && get(Query::JointWarTarget(b, a))? != p
            && get(Query::WarType(b, p))? != 0xfb189687u32 as i32
            && get(Query::WarType(b, p))? != 0x01f4de45
            && get(Query::CapitalContinent(a))? == get(Query::CapitalContinent(p))?
        {
            return Ok(0);
        }
    }
    if r.kind == Kind::DeclaredWar {
        r.transient_defender = -1
    }
    Ok(1)
}
struct Context<'a, Q, C> {
    state: &'a mut LoadedState,
    runtime: &'a mut Runtime,
    receiver: &'a mut Receiver,
    queries: &'a mut Q,
    continuations: &'a mut C,
}
impl<Q: Queries, C: Continuations> changes::Context for Context<'_, Q, C> {
    type Error = Error<Q::Error>;
    fn subject_sets(&self) -> Result<&sets::Runtime, Self::Error> {
        Ok(self.runtime.sets())
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, Self::Error> {
        Ok(self.runtime.sets_mut())
    }
    fn test_requirement(&mut self, label: u64) -> Result<i32, sets::Error<Self::Error>> {
        if label != self.receiver.label {
            return Err(sets::Error::Context(Error::Unsupported(
                "war requirement receiver identity",
            )));
        }
        test(self.state, self.receiver, self.queries).map_err(sets::Error::Context)
    }
    fn modifier_changed(
        &mut self,
        l: sets::SetListener,
        s: u64,
        o: i32,
        n: i32,
    ) -> Result<(), sets::Error<Self::Error>> {
        self.continuations
            .modifier_changed(self.state, self.runtime, l, s, o, n)
            .map_err(|_| sets::Error::Context(Error::Continuation))
    }
    fn destroy_requirement(&mut self, l: u64) -> Result<(), sets::Error<Self::Error>> {
        self.continuations
            .destroy_requirement(self.state, self.runtime, l)
            .map_err(|_| sets::Error::Context(Error::Continuation))
    }
    fn destroy_set(&mut self, l: u64) -> Result<(), sets::Error<Self::Error>> {
        self.continuations
            .destroy_set(self.state, self.runtime, l)
            .map_err(|_| sets::Error::Context(Error::Continuation))
    }
    fn reset_trigger(&mut self, l: u64) -> Result<(), sets::Error<Self::Error>> {
        self.continuations
            .reset_trigger(self.state, self.runtime, l)
            .map_err(|_| sets::Error::Context(Error::Continuation))
    }
}
/// Call in the captured signal's subscriber order. Removal signals carry the
/// sending player first and receiving player second. No cached Test scalar is used.
#[allow(clippy::too_many_arguments)]
pub fn changed<Q: Queries, C: Continuations>(
    state: &mut LoadedState,
    runtime: &mut Runtime,
    r: &mut Receiver,
    issuer: i32,
    defender: i32,
    war_type: i32,
    queries: &mut Q,
    continuations: &mut C,
) -> Result<(), Error<Q::Error>> {
    let gate = match r.kind {
        Kind::Delegation | Kind::Embassy => {
            r.context == Some(defender) && r.subject == Some(issuer)
        }
        Kind::AtPeace => [r.context, r.subject]
            .iter()
            .all(|p| *p == Some(issuer) || *p == Some(defender)),
        Kind::AttackedContinent => r.subject == Some(issuer) || r.subject == Some(defender),
        Kind::DeclaredWar => {
            r.subject == Some(issuer) && (r.war_type == -1 || r.war_type == war_type)
        }
    };
    if !gate {
        return Ok(());
    }
    let name: &[u8] = match r.kind {
        Kind::DeclaredWar => b"REQUIREMENT_PLAYER_DECLARED_WAR",
        Kind::AtPeace => b"REQUIREMENT_PLAYER_IS_AT_PEACE",
        Kind::AttackedContinent => b"REQUIREMENT_PLAYER_ATTACKED_CONTINENT",
        Kind::Delegation => b"REQUIREMENT_DELEGATION",
        Kind::Embassy => b"REQUIREMENT_RESIDENT_EMBASSY",
    };
    if runtime
        .sets()
        .requirement_definition(r.label)
        .map(|d| &d.predicate)
        != Some(&sets::Predicate::Unsupported(
            crate::registry::linux_type_hash(name),
        ))
    {
        return Err(Error::Unsupported("war requirement definition identity"));
    }
    if r.kind == Kind::DeclaredWar {
        r.transient_defender = defender;
        if r.exclude_same_emergency
            && queries
                .query(
                    state,
                    Query::SameEmergency {
                        defender,
                        issuer,
                        context: r
                            .context
                            .ok_or(Error::Unsupported("DeclaredWar context Player"))?,
                    },
                )
                .map_err(Error::Query)?
                != 0
        {
            return Ok(());
        }
    }
    let label = r.label;
    changes::invalidate(
        &mut Context {
            state,
            runtime,
            receiver: r,
            queries,
            continuations,
        },
        label,
    )
    .map_err(|e| Error::Changed(Box::new(e)))
}

/// Current gameplay getters for the supported loaded action. Joint-war and
/// emergency continuations remain explicit missing inputs when actually reached.
pub struct CanonicalQueries;
#[derive(Debug)]
pub enum CanonicalError {
    State(c6_core::loaded_state::Error),
    Relations(c6_core::game_diplomatic_relations::Error),
    World(c6_core::world::MissingInput),
    Missing(&'static str),
}
impl Queries for CanonicalQueries {
    type Error = CanonicalError;
    fn query(&mut self, state: &LoadedState, query: Query) -> Result<i32, Self::Error> {
        let player = |owner| {
            state
                .retained()
                .players
                .iter()
                .find(|p| p.header.owner == owner)
                .ok_or(CanonicalError::Missing("current requirement Player"))
        };
        let relations = || state.game_relations().map_err(CanonicalError::State);
        let relation = |r: Result<i32, c6_core::game_diplomatic_relations::Error>| {
            r.map_err(CanonicalError::Relations)
        };
        match query {
            Query::CurrentTurn => Ok(state.retained().core.current_turn),
            Query::AtWar(a, b) => relations()?
                .is_at_war(a, b)
                .map(i32::from)
                .map_err(CanonicalError::Relations),
            Query::HasMet(a, b) => {
                if a == b
                    || !state
                        .world()
                        .player_is_valid(b)
                        .map_err(CanonicalError::World)?
                {
                    Ok(0)
                } else {
                    relations()?
                        .has_met(a, b)
                        .map(i32::from)
                        .map_err(CanonicalError::Relations)
                }
            }
            Query::AtWarChangeTurn(a, b) => relation(relations()?.war_changed_turn(a, b)),
            Query::WarType(a, b) => relation(relations()?.war_type(a, b)),
            Query::StartedWar(a, b) => {
                relation(relations()?.war_aggressor(a, b)).map(|p| i32::from(a != b && p == a))
            }
            Query::CapitalContinent(a) => Ok(player(a)?.cities.home_continent_hash as i32),
            Query::Delegation(a, b) | Query::Embassy(a, b) => {
                if a == b
                    || !state
                        .world()
                        .player_is_valid(b)
                        .map_err(CanonicalError::World)?
                {
                    return Ok(0);
                }
                let index = usize::try_from(b)
                    .map_err(|_| CanonicalError::Missing("delegation Player index"))?;
                let diplomacy = &player(a)?.diplomacy;
                let value = match query {
                    Query::Delegation(..) => diplomacy.has_delegation.get(index),
                    _ => diplomacy.has_embassy.get(index),
                }
                .ok_or(CanonicalError::Missing("delegation Player index"))?;
                Ok(i32::from(*value))
            }
            Query::JointWarTarget(..) => {
                Err(CanonicalError::Missing("current joint-war target owner"))
            }
            Query::FightingJointWar(..) => {
                Err(CanonicalError::Missing("current fighting joint-war owner"))
            }
            Query::SameEmergency { .. } => Err(CanonicalError::Missing(
                "current emergency membership owner",
            )),
        }
    }
    fn alive_ids(&mut self, state: &LoadedState) -> Result<Vec<i32>, Self::Error> {
        let mut result = Vec::new();
        for p in state
            .diplomacy_manager()
            .map_err(CanonicalError::State)?
            .ever_alive()
        {
            if state.player_alive(p.owner).map_err(CanonicalError::State)? {
                result.push(p.owner)
            }
        }
        Ok(result)
    }
}
