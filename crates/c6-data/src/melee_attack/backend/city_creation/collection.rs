//! Existing collection modifiers receiving current City subjects in the SAME
//! EffectRuntime. Owner context, tracked subjects and effect payloads are distinct.
use super::{definitions::Attachment, Apply, Dynamic, EffectRuntime, Error};
use crate::melee_attack::backend::{self, ContextObject};
use c6_core::{
    loaded_state::LoadedState,
    melee_attack::proposals::subject_sets as sets,
    unit_moved_signal::{modifiers, Subscription},
};
pub struct Subject {
    pub subject: u64,
    pub state: modifiers::TrackedSubject,
    pub destruction: Subscription,
}
/// Explicit fixed-stock leaf scope: collection hooks are inherited RETs and
/// effect metadata getters are pure. This does not implement concrete Apply or
/// Remove; unsupported effect kinds still fail if their gameplay leaf is reached.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Hooks {
    BaseNoops,
    Unsupported,
}
pub struct InitialModifier {
    pub identity: u64,
    pub instance: i32,
    pub context: u64,
    pub effect_identity: u64,
    pub definition: Attachment,
    pub control: modifiers::Modifier,
    /// Complete current City tracked/applied lists, in native comparator order.
    pub tracked: Vec<Subject>,
    pub applied_subjects: Vec<u64>,
    /// Unknown imported owner subscription stays unavailable; it is not reset.
    pub owner_destruction: Option<Subscription>,
    pub activated: bool,
    pub hooks: Hooks,
}
/// Additional source-specific Simple factories borrow the canonical state and
/// the same requirements/effect owners. No predicate result is an input.
pub trait RequirementFactory {
    #[allow(clippy::too_many_arguments)]
    fn initialize(
        &mut self,
        state: &LoadedState,
        effects: &mut EffectRuntime,
        requirements: &mut backend::requirements::Runtime,
        label: u64,
        definition: &sets::RequirementDefinition,
        context: i32,
        subject: sets::Subject,
    ) -> Result<sets::InitializedRequirement, sets::Error<Error>>;
}
pub struct MissingRequirements;
impl RequirementFactory for MissingRequirements {
    fn initialize(
        &mut self,
        _: &LoadedState,
        _: &mut EffectRuntime,
        _: &mut backend::requirements::Runtime,
        _: u64,
        _: &sets::RequirementDefinition,
        _: i32,
        _: sets::Subject,
    ) -> Result<sets::InitializedRequirement, sets::Error<Error>> {
        Err(sets::Error::Context(Error::Missing(
            "additional current City requirement factory",
        )))
    }
}
pub(super) fn city_key(runtime: &EffectRuntime, subject: u64) -> Result<u64, Error> {
    let city = match runtime.object(subject).map_err(Error::Query)?.kind {
        ContextObject::City(city) => city,
        ContextObject::Building { district, .. } => district,
        // GovernorID.GetComponentID242fd90 uses component type77. Reached
        // Governor wrappers have subtype/subvalue0/0, as do these subjects.
        ContextObject::Governor { owner, id } => c6_core::unit::ComponentId {
            owner: owner as i16,
            component_type: 77,
            id: id as u32,
        },
        _ => {
            return Err(Error::Missing("current normal City subject"));
        }
    };
    // ComponentID.GetBits unsigned comparison, then normal City subtype 0/0.
    Ok((u64::from(city.id) << 32)
        | (u64::from(city.component_type as u16) << 16)
        | u64::from(city.owner as u16))
}
impl EffectRuntime {
    /// Read the live subjects from the same modifier owner after callbacks.
    pub fn city_collection_subjects(&self, receiver: u64) -> Result<&[Subject], Error> {
        self.city_creation
            .as_ref()
            .ok_or(Error::Missing("current City creation domain"))?
            .modifiers
            .iter()
            .find(|r| r.identity == receiver)
            .map(|r| r.tracked.as_slice())
            .ok_or(Error::Missing("current City collection modifier"))
    }
    /// Import an existing modifier without re-running its constructor/Activate
    /// and without advancing either shared allocation counter.
    pub fn import_city_collection_modifier(
        &mut self,
        state: &LoadedState,
        initial: InitialModifier,
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        if !state.is_pristine() {
            return Err(Error::Missing(
                "initial City collection modifier after mutation",
            ));
        }
        self.object(initial.context).map_err(Error::Query)?;
        let creation = self
            .city_creation
            .as_ref()
            .ok_or(Error::Missing("current City creation domain"))?;
        if creation
            .modifiers
            .iter()
            .any(|r| r.identity == initial.identity || r.instance == initial.instance)
        {
            return Err(Error::DuplicateModifier(initial.instance));
        }
        let mut previous = None;
        for row in &initial.tracked {
            let key = city_key(self, row.subject)?;
            if previous.is_some_and(|old| old >= key) {
                return Err(Error::Missing("current tracked City comparator order"));
            }
            previous = Some(key);
        }
        let mut previous = None;
        for subject in &initial.applied_subjects {
            let key = city_key(self, *subject)?;
            if previous.is_some_and(|old| old >= key) {
                return Err(Error::Missing("current applied City comparator order"));
            }
            previous = Some(key);
        }
        self.register_city_effect(
            state,
            initial.effect_identity,
            initial.context,
            initial.definition.effect.clone(),
        )
        .map_err(Error::Query)?;
        self.city_creation
            .as_mut()
            .unwrap()
            .modifiers
            .push(Dynamic {
                identity: initial.identity,
                instance: initial.instance,
                owner: initial.context,
                effect: initial.effect_identity,
                definition: initial.definition,
                modifier: initial.control,
                tracked: initial.tracked,
                owner_destruction: initial.owner_destruction,
                applied_subjects: initial.applied_subjects,
                activated: initial.activated,
                hooks: initial.hooks,
            });
        Ok(())
    }
}
pub struct Invocation<'a> {
    pub receiver: u64,
    pub subject: u64,
    pub added: bool,
    pub maximum_loyalty: Option<i32>,
    /// Current initialized Feature collection hash order, not native answers.
    pub features: &'a [u32],
}
/// Called once per actual ordered ItemChanged listener. The common kernel owns
/// enabled/suppressed/owner-set gates before subscriptions, Test or effects.
pub fn item_changed(
    state: &mut LoadedState,
    runtime: &mut EffectRuntime,
    requirements: &mut backend::requirements::Runtime,
    invocation: Invocation<'_>,
    factory: &mut dyn RequirementFactory,
    effects: &mut impl crate::city_production_turn::modifier_effects::Effects,
) -> Result<(), Error> {
    runtime.current_owner(state).map_err(Error::Query)?;
    let index = runtime
        .city_creation
        .as_ref()
        .ok_or(Error::Missing("current City creation domain"))?
        .modifiers
        .iter()
        .position(|r| r.identity == invocation.receiver)
        .ok_or(Error::Missing("current City collection modifier"))?;
    c6_core::unit_moved_signal::collection_addition::item_changed(
        &mut Apply {
            state,
            runtime,
            requirements,
            subject: invocation.subject,
            factory,
            features: invocation.features,
            maximum_loyalty: invocation.maximum_loyalty,
            index,
            effects,
        },
        invocation.added,
    )
}
