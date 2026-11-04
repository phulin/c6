//! Current constant combat requirements and their persistent native-ID owner.
use super::{ContextObject, EffectRuntime, Requirements};
use c6_core::{
    loaded_state::LoadedState,
    map::UnitReference,
    melee_attack::proposals::{registry, subject_predicates as predicates, subject_sets as sets},
    unit::ComponentId,
};

#[derive(Debug)]
pub enum Error {
    Missing(&'static str),
    Query(super::QueryError),
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Registry(registry::Error),
    Access(c6_core::loaded_state::combat_proposals::AccessError),
    Actor(Box<crate::melee_attack::Error>),
    Factory(Box<sets::Error<Error>>),
    Predicate(Box<predicates::Error<Error>>),
    UnitPredicate(Box<c6_core::melee_attack::proposals::unit_subject_predicates::Error<Error>>),
    Promotion(crate::melee_attack::proposals::unit_subject_predicates::Error),
}
pub struct Runtime {
    sets: sets::Runtime,
    existing: Vec<(u64, i32)>,
    levels: Vec<(i32, u32)>,
    logging_priority: Option<i32>,
    units: Option<crate::melee_attack::proposals::unit_subject_predicates::Definitions>,
}
impl Runtime {
    pub fn new(
        sets: sets::Runtime,
        existing: Vec<(u64, i32)>,
        levels: Vec<(i32, u32)>,
        logging_priority: Option<i32>,
    ) -> Result<Self, Error> {
        if existing
            .iter()
            .enumerate()
            .any(|(i, r)| existing[..i].iter().any(|p| p.0 == r.0))
            || levels
                .iter()
                .enumerate()
                .any(|(i, r)| levels[..i].iter().any(|p| p.0 == r.0))
        {
            return Err(Error::Missing("duplicate requirement/player identity"));
        }
        Ok(Self {
            sets,
            existing,
            levels,
            logging_priority,
            units: None,
        })
    }
    pub fn sets(&self) -> &sets::Runtime {
        &self.sets
    }
    pub fn with_unit_definitions(
        mut self,
        definitions: crate::melee_attack::proposals::unit_subject_predicates::Definitions,
    ) -> Self {
        self.units = Some(definitions);
        self
    }
}
impl Requirements for Runtime {
    type Error = Error;
    fn state(&self, _: &LoadedState, set: u64) -> Result<i32, Error> {
        self.sets
            .state(set)
            .or_else(|| self.existing.iter().find(|r| r.0 == set).map(|r| r.1))
            .ok_or(Error::Missing("current requirement set"))
    }
    fn create_subject_set(
        &mut self,
        state: &mut LoadedState,
        receiver: u64,
        subject: registry::Handle,
        objects: &EffectRuntime,
    ) -> Result<Option<u64>, Error> {
        let modifier = state
            .combat_proposals()
            .map_err(Error::Access)?
            .collection_modifiers()
            .map_err(Error::Registry)?
            .modifier(receiver)
            .ok_or(Error::Missing("current collection modifier"))?;
        let object = objects.object(modifier.context).map_err(Error::Query)?;
        let id = modifier.subject_requirements.clone();
        sets::get_or_create(
            &mut Context {
                state,
                runtime: self,
                object: object.kind,
                instance: object.instance_id,
                subject,
            },
            receiver,
            object.instance_id,
            subject,
            &id,
        )
        .map_err(|e| Error::Factory(Box::new(e)))
    }
    fn unsubscribe_subject_set(
        &mut self,
        _: &mut LoadedState,
        receiver: u64,
        _: registry::Handle,
        set: u64,
    ) -> Result<(), Error> {
        self.sets
            .unsubscribe(receiver, set)
            .map_err(|e| Error::Factory(Box::new(e)))
    }
    fn release_subject_set(&mut self, _: &mut LoadedState, set: u64) -> Result<(), Error> {
        self.sets
            .release(set)
            .map_err(|e| Error::Factory(Box::new(e)))
    }
}
struct Context<'a> {
    state: &'a mut LoadedState,
    runtime: &'a mut Runtime,
    object: ContextObject,
    instance: i32,
    subject: registry::Handle,
}
impl registry::Context for Context<'_> {
    type Error = Error;
    fn runtime(&self) -> Result<&registry::Runtime, Error> {
        self.state.combat_proposals().map_err(Error::Access)
    }
    fn runtime_mut(&mut self) -> Result<&mut registry::Runtime, Error> {
        self.state.combat_proposals_mut().map_err(Error::Access)
    }
    fn owner(&mut self, p: registry::Participant) -> Result<i32, Error> {
        match p {
            registry::Participant::Unit(u) => Ok(crate::melee_attack::actor(self.state, u)
                .map_err(|e| Error::Actor(Box::new(e)))?
                .e_owner),
            registry::Participant::District(_) => {
                Err(Error::Missing("current District requirement owner"))
            }
        }
    }
    fn unit_id(&mut self, u: UnitReference) -> Result<ComponentId, Error> {
        let u = crate::melee_attack::actor(self.state, u).map_err(|e| Error::Actor(Box::new(e)))?;
        Ok(ComponentId {
            owner: u.e_owner as i16,
            component_type: 1,
            id: u.id,
        })
    }
    fn callback(&mut self, _: u64, _: registry::Handle) -> Result<(), Error> {
        Err(Error::Missing("nested requirement proposal callback"))
    }
}
impl predicates::Context for Context<'_> {
    fn alliance_pair(&mut self) -> Result<Option<(i32, i32)>, Error> {
        Ok(match self.object {
            ContextObject::Alliance {
                first_player,
                second_player,
            } => Some((first_player, second_player)),
            _ => None,
        })
    }
    fn subject_proposal(&mut self) -> Result<Option<registry::Handle>, Error> {
        self.state
            .combat_proposals()
            .map_err(Error::Access)?
            .proposal(self.subject)
            .map_err(Error::Registry)?;
        Ok(Some(self.subject))
    }
    fn valid_player(&mut self, p: i32) -> Result<bool, Error> {
        Ok(self
            .state
            .world()
            .valid_player(p)
            .map_err(Error::World)?
            .is_some())
    }
    fn is_alive(&mut self, p: i32) -> Result<bool, Error> {
        self.state.player_alive(p).map_err(Error::State)
    }
    fn is_minor(&mut self, p: i32) -> Result<bool, Error> {
        Ok(self
            .runtime
            .levels
            .iter()
            .find(|r| r.0 == p)
            .ok_or(Error::Missing("current civilization level"))?
            .1
            == 0xc2577075)
    }
    fn is_major(&mut self, p: i32) -> Result<bool, Error> {
        Ok(self
            .runtime
            .levels
            .iter()
            .find(|r| r.0 == p)
            .ok_or(Error::Missing("current civilization level"))?
            .1
            == 0x253718b0)
    }
    fn war_type(&mut self, a: i32, b: i32) -> Result<i32, Error> {
        self.state
            .game_relations()
            .map_err(Error::State)?
            .war_type(a, b)
            .map_err(|_| Error::Missing("current native war type"))
    }
    fn unit_religion(&mut self, _: UnitReference) -> Result<i32, Error> {
        Err(Error::Missing("runtime religion identity mapping"))
    }
    fn founded_religion(&mut self, _: i32) -> Result<i32, Error> {
        Err(Error::Missing("runtime religion identity mapping"))
    }
}
impl sets::Context for Context<'_> {
    type Error = Error;
    fn subject_sets(&self) -> Result<&sets::Runtime, Error> {
        Ok(&self.runtime.sets)
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, Error> {
        Ok(&mut self.runtime.sets)
    }
    fn evaluate_constant(
        &mut self,
        p: &sets::Predicate,
        instance: i32,
        subject: registry::Handle,
    ) -> Result<i32, Error> {
        if instance != self.instance || subject != self.subject {
            return Err(Error::Missing("requirement context identity changed"));
        }
        let value = match p {
            sets::Predicate::AllianceWar => predicates::alliance_at_war_with_combat_target(self),
            sets::Predicate::CombatVersus { expected } => {
                predicates::combat_versus_type_matches(self, *expected)
            }
            sets::Predicate::AllianceReligion => {
                predicates::alliance_not_combat_target_religion(self)
            }
            sets::Predicate::OpponentPromotionClass { expected } => {
                return c6_core::melee_attack::proposals::unit_subject_predicates::opponent_promotion_class_matches(self, expected)
                    .map_err(|e|Error::UnitPredicate(Box::new(e)))
            }
            sets::Predicate::PlayerIsAttacking => {
                return c6_core::melee_attack::proposals::unit_subject_predicates::player_is_attacking(self)
                    .map_err(|e|Error::UnitPredicate(Box::new(e)))
            }
            sets::Predicate::Unsupported(_) => {
                return Err(Error::Missing("source-qualified requirement predicate"))
            }
        };
        value.map_err(|e| Error::Predicate(Box::new(e)))
    }
    fn log_missing_set(&mut self, _: &[u8]) -> Result<(), Error> {
        log_disabled(self.runtime.logging_priority, 1)
    }
    fn log_created_requirement(&mut self, _: u32) -> Result<(), Error> {
        log_disabled(self.runtime.logging_priority, 4)
    }
    fn log_created_set(&mut self, _: u32) -> Result<(), Error> {
        log_disabled(self.runtime.logging_priority, 4)
    }
}
impl c6_core::melee_attack::proposals::unit_subject_predicates::Context for Context<'_> {
    type PlayerInterface = ContextObject;
    fn context_player_interface(&mut self) -> Result<Option<ContextObject>, Error> {
        Ok(match self.object {
            ContextObject::Alliance { .. } => None,
            other => Some(other),
        })
    }
    fn subject_proposal_interface(&mut self) -> Result<Option<registry::Handle>, Error> {
        <Self as predicates::Context>::subject_proposal(self)
    }
    fn context_player_id(&mut self, interface: ContextObject) -> Result<i32, Error> {
        match interface {
            ContextObject::Player(id) => Ok(id),
            ContextObject::Unit(unit) => Ok(i32::from(unit.component_id().owner)),
            ContextObject::Alliance { .. } => Err(Error::Missing("IPlayer on alliance context")),
        }
    }
    fn promotion_class(&mut self, unit: UnitReference) -> Result<Option<&[u8]>, Error> {
        self.runtime
            .units
            .as_ref()
            .ok_or(Error::Missing("current promotion definitions"))?
            .promotion_class(self.state, unit)
            .map_err(Error::Promotion)
    }
}
fn log_disabled(priority: Option<i32>, enabled_at: i32) -> Result<(), Error> {
    match priority {
        Some(p) if p < enabled_at => Ok(()),
        _ => Err(Error::Missing("enabled requirement log callback")),
    }
}
#[cfg(test)]
mod tests {
    use super::log_disabled;
    #[test]
    fn missing_definition_logging_has_a_different_native_threshold() {
        assert!(log_disabled(Some(0), 1).is_ok());
        assert!(log_disabled(Some(1), 1).is_err());
        assert!(log_disabled(Some(3), 4).is_ok());
        assert!(log_disabled(Some(4), 4).is_err());
        assert!(log_disabled(None, 4).is_err());
    }
}
