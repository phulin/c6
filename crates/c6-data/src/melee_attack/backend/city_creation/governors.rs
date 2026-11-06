//! Governor establishment effects in the existing persistent modifier owner.
//! Only unconditional COLLECTION_OWNER city yield modifiers are constructed.
use super::super::{CityEffect, ContextObject};
use super::*;
use c6_core::unit::ComponentId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Recipe {
    pub id: Vec<u8>,
    pub yield_hash: u32,
    pub amount: i32,
}

/// Borrowed canonical Governor lifetime. These are the same affected entries
/// and destruction ring used by application/removal, rather than the optional
/// Unit-dispatch role carried by the normal object record.
pub struct GovernorObservation<'a> {
    pub context: u64,
    pub context_instance: i32,
    pub affected: &'a AffectedModifiers,
    pub destroyed: &'a ListenerRing<DestroyedListener>,
    modifiers: &'a [Dynamic],
}
/// Actual persistent DynamicModifier fields; handles are logical runtime
/// identities, not reconstructed native addresses.
pub struct GovernorModifierObservation<'a> {
    pub identity: u64,
    pub instance: i32,
    pub owner: u64,
    pub effect: u64,
    pub definition: &'a definitions::Attachment,
    pub control: &'a c6_core::unit_moved_signal::modifiers::Modifier,
    pub tracked: &'a [collection::Subject],
    pub applied: &'a [u64],
    pub activated: bool,
    pub owner_destruction: Option<Subscription>,
}
impl GovernorObservation<'_> {
    /// Preserve the actual runtime order of this owner's modifiers.
    pub fn modifiers(&self) -> impl Iterator<Item = GovernorModifierObservation<'_>> {
        self.modifiers
            .iter()
            .filter(move |m| m.owner == self.context)
            .map(|m| GovernorModifierObservation {
                identity: m.identity,
                instance: m.instance,
                owner: m.owner,
                effect: m.effect,
                definition: &m.definition,
                control: &m.modifier,
                tracked: &m.tracked,
                applied: &m.applied_subjects,
                activated: m.activated,
                owner_destruction: m.owner_destruction,
            })
    }
}
impl EffectRuntime {
    /// Observe this owner's canonical modifier lifecycle without applying or
    /// importing anything. Missing imported lifecycle data stays an error.
    pub fn governor_modifier_observation(
        &self,
        owner: i32,
        id: i32,
    ) -> Result<Option<GovernorObservation<'_>>, Error> {
        let Some((context, object)) = self
            .objects
            .iter()
            .find(|(_, o)| o.kind == ContextObject::Governor { owner, id })
        else {
            return Ok(None);
        };
        let creation = self
            .city_creation
            .as_ref()
            .ok_or(Error::Missing("Governor current modifier lifecycle"))?;
        let lifecycle = creation
            .objects
            .iter()
            .find(|o| o.handle == *context)
            .ok_or(Error::Missing("Governor canonical object lifecycle"))?;
        Ok(Some(GovernorObservation {
            context: *context,
            context_instance: object.instance_id,
            affected: &lifecycle.affected,
            destroyed: &lifecycle.destroyed,
            modifiers: &creation.modifiers,
        }))
    }
}

pub(in crate::melee_attack::backend) fn governor(
    state: &LoadedState,
    owner: i32,
    id: i32,
) -> Result<&c6_core::player_governors::Governor, QueryError> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .and_then(|p| p.governors.entries.iter().find(|s| s.governor.id == id))
        .map(|s| &s.governor)
        .filter(|g| g.owning_player == owner)
        .ok_or(QueryError::Missing("current Governor object instance"))
}

impl ContextObject {
    /// Governor.GetCityInterface resolves its current assigned city on each call.
    pub(crate) fn current_city_component(
        self,
        state: &LoadedState,
    ) -> Result<Option<ComponentId>, QueryError> {
        let Self::Governor { owner, id } = self else {
            return Ok(self.city_component());
        };
        let city = &governor(state, owner, id)?.assigned_city;
        if city.component_type != 2 {
            return Ok(None);
        }
        state
            .city(i32::from(city.owner), city.id)
            .map_err(|_| QueryError::Missing("Governor assigned City"))?;
        Ok(Some(ComponentId {
            owner: city.owner,
            component_type: city.component_type,
            id: city.id,
        }))
    }
}

pub(in crate::melee_attack::backend) fn adjust_yield_modifier(
    state: &mut LoadedState,
    city: ComponentId,
    yield_hash: u32,
    amount: i32,
    remove: bool,
    effects: &mut impl crate::city_production_turn::modifier_effects::Effects,
) -> Result<(), crate::city_production_turn::Error> {
    let values = &mut state
        .city_mut(i32::from(city.owner), city.id)?
        .header
        .yield_modifiers;
    let cell = values
        .iter_mut()
        .find(|v| v.definition_hash == yield_hash)
        .ok_or(crate::city_production_turn::Error::Unsupported(
            "City.ChangeYieldModifier yield index",
        ))?;
    cell.value = cell.value.wrapping_add(if remove {
        amount.wrapping_neg()
    } else {
        amount
    });
    effects.report(
        state,
        i32::from(city.owner),
        city.id,
        crate::city_production_turn::modifier_effects::Report::YieldChange { yield_hash },
    )
}

impl EffectRuntime {
    /// Return allocated modifier instance IDs in ModifierId byte order.
    /// Effects and subscriptions remain in this runtime until explicit removal.
    #[allow(clippy::too_many_arguments)]
    pub fn attach_governor_modifiers(
        &mut self,
        state: &mut LoadedState,
        requirements: &mut super::super::requirements::Runtime,
        owner: i32,
        id: i32,
        recipes: &[Recipe],
        effects: &mut impl crate::city_production_turn::modifier_effects::Effects,
    ) -> Result<Vec<i32>, Error> {
        self.current_owner(state).map_err(Error::Query)?;
        let kind = ContextObject::Governor { owner, id };
        kind.current_city_component(state)
            .map_err(Error::Query)?
            .ok_or(Error::Missing("Governor attachment assigned City"))?;
        if self.city_creation.is_none() {
            return Err(Error::Missing("Governor current modifier lifecycle"));
        }
        let context = if let Some((handle, _)) = self.objects.iter().find(|(_, o)| o.kind == kind) {
            *handle
        } else {
            let handle = self
                .append_normal_object(
                    state,
                    super::super::Object {
                        kind,
                        instance_id: 0,
                        unit_sort_suffix: None,
                        unit_lifecycle: Some(c6_core::unit_kill::objects::Object {
                            pool_ordinal: 0,
                            owned_modifiers: Vec::new(),
                            affected: AffectedModifiers::from_sorted(Vec::new()).unwrap(),
                            destroyed: ListenerRing::from_ordered(&[]).map_err(Error::Ring)?,
                        }),
                    },
                )
                .map_err(Error::Query)?;
            self.city_creation.as_mut().unwrap().governors.push(handle);
            handle
        };
        if self
            .city_creation
            .as_ref()
            .unwrap()
            .modifiers
            .iter()
            .any(|m| m.owner == context)
        {
            return Err(Error::Missing("Governor modifiers already attached"));
        }
        let mut ordered = recipes.iter().collect::<Vec<_>>();
        ordered.sort_by(|a, b| a.id.cmp(&b.id));
        let mut instances = Vec::with_capacity(ordered.len());
        for recipe in ordered {
            instances.push(self.attach_city_owner(
                state,
                requirements,
                context,
                &definitions::Attachment {
                    collection: definitions::Collection::Owner,
                    id: recipe.id.clone(),
                    effect: CityEffect::YieldModifier {
                        yield_hash: recipe.yield_hash,
                        amount: recipe.amount,
                    },
                    subject_requirements: None,
                    owner_requirements: None,
                    repeatable: false,
                    run_once: false,
                    permanent: false,
                    new_only: false,
                    owner_stack_limit: 0,
                    subject_stack_limit: 0,
                },
                None,
                effects,
            )?);
        }
        Ok(instances)
    }

    /// Remove before clearing assigned_city; the native Governor ICity interface
    /// still resolves that city while each persistent effect executes Remove.
    pub fn detach_governor_modifiers(
        &mut self,
        state: &mut LoadedState,
        requirements: &mut super::super::requirements::Runtime,
        owner: i32,
        id: i32,
        effects: &mut impl crate::city_production_turn::modifier_effects::Effects,
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        let context = self
            .objects
            .iter()
            .find(|(_, o)| o.kind == ContextObject::Governor { owner, id })
            .map(|r| r.0)
            .ok_or(Error::Missing("Governor modifier owner"))?;
        self.remove_policy_city_modifiers(state, requirements, context, effects)?;
        self.retire_policy_object(state, context)
    }
}
