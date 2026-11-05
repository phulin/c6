//! Government factories in the SAME object/effect/requirement lifetime owner.
use super::*;
use crate::loaded_actions::government_actions::modifiers::{Government, Recipe};
use c6_core::{
    melee_attack::proposals::subject_sets::{self as sets, changes},
    unit::ComponentId,
};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BuildingSubscription {
    pub requirement: u64,
    pub city: ComponentId,
}
struct BuildingRequirement {
    route: BuildingSubscription,
    building: Option<u32>,
    active: bool,
}
pub(super) struct Flat {
    pub owner: u64,
    instance: i32,
    effect: u64,
    destruction: Subscription,
    tracked_destruction: Subscription,
    applied: bool,
}
#[derive(Default)]
pub(super) struct State {
    pub constructed: Vec<u64>,
    pub flat: Vec<Flat>,
    definitions: Vec<(Vec<u8>, Option<u32>)>,
    building: Vec<BuildingRequirement>,
    maps: Vec<(i32, Vec<c6_core::unit::HashValue<Option<u32>>>)>,
}
fn test_building(
    state: &LoadedState,
    creation: &Creation,
    city: ComponentId,
    building: Option<u32>,
) -> Result<i32, Error> {
    let Some(building) = building else {
        return Ok(1);
    };
    let values = &creation
        .government
        .maps
        .iter()
        .find(|r| r.0 == i32::from(city.owner))
        .ok_or(Error::Missing(
            "CityHasBuilding Player building replacement map",
        ))?
        .1;
    let Some(building) = values
        .iter()
        .find(|r| r.definition_hash == building)
        .ok_or(Error::Missing("CityHasBuilding replacement element"))?
        .value
    else {
        return Ok(1);
    };
    let city = state
        .city(i32::from(city.owner), city.id)
        .map_err(|_| Error::Missing("CityHasBuilding current City"))?;
    let location = city
        .buildings
        .building_locations
        .iter()
        .find(|r| r.definition_hash == building)
        .ok_or(Error::Missing("CityHasBuilding location array"))?
        .value;
    if location < 0 {
        return Ok(1);
    }
    let pillaged = city
        .buildings
        .pillaged
        .iter()
        .find(|r| r.definition_hash == building)
        .ok_or(Error::Missing("CityHasBuilding pillage array"))?
        .value;
    Ok(i32::from(pillaged))
}
pub(super) fn initialize_building(
    state: &LoadedState,
    effects: &mut EffectRuntime,
    label: u64,
    definition: &sets::RequirementDefinition,
    subject: sets::Subject,
) -> Result<sets::InitializedRequirement, sets::Error<Error>> {
    let sets::Subject::ModifierObject(handle) = subject else {
        return Err(sets::Error::UnsupportedSubject);
    };
    let super::super::ContextObject::City(city) = effects
        .object(handle)
        .map_err(|e| sets::Error::Context(Error::Query(e)))?
        .kind
    else {
        return Err(sets::Error::UnsupportedSubject);
    };
    let creation = effects
        .city_creation
        .as_mut()
        .ok_or(sets::Error::Context(Error::Missing(
            "CityHasBuilding lifecycle",
        )))?;
    let building = creation
        .government
        .definitions
        .iter()
        .find(|r| r.0 == definition.id)
        .ok_or(sets::Error::Context(Error::Missing(
            "CityHasBuilding definition",
        )))?
        .1;
    // OnInitialize23753f0 subscribes completed+1f8, removed+210, pillage+228,
    // before SimpleRequirement.Initialize tests. One source receiver owns all 3.
    creation.government.building.push(BuildingRequirement {
        route: BuildingSubscription {
            requirement: label,
            city,
        },
        building,
        active: true,
    });
    let tested = test_building(state, creation, city, building).map_err(sets::Error::Context)?;
    Ok(sets::InitializedRequirement::Simple(
        c6_core::simple_requirement::Requirement {
            active: true,
            state: if definition.inverse {
                sets::inverse(tested)
            } else {
                tested
            },
            inverse: definition.inverse,
            persistent: definition.persistent,
            triggered: false,
        },
    ))
}
impl EffectRuntime {
    /// Current Player.Cities replacement arrays, reused by CityHasBuilding.Test.
    /// Call again when a supported transition changes those arrays.
    pub fn install_government_building_maps(
        &mut self,
        state: &LoadedState,
        maps: &[crate::city_production_turn::building_callbacks::BuildingMap],
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        let creation = self
            .city_creation
            .as_mut()
            .ok_or(Error::Missing("Government lifecycle"))?;
        creation.government.maps = maps.iter().map(|m| (m.owner, m.values.clone())).collect();
        Ok(())
    }

    pub fn make_government_object(
        &mut self,
        state: &LoadedState,
        owner: i32,
        government_hash: u32,
    ) -> Result<u64, Error> {
        self.current_owner(state).map_err(Error::Query)?;
        let kind = super::super::ContextObject::Government {
            owner,
            government_hash,
        };
        if let Some((h, _)) = self.objects.iter().find(|r| r.1.kind == kind) {
            return Ok(*h);
        }
        let handle = self
            .append_normal_object(
                state,
                super::super::Object {
                    kind,
                    instance_id: 0,
                    unit_sort_suffix: None,
                    unit_lifecycle: None,
                },
            )
            .map_err(Error::Query)?;
        self.city_creation
            .as_mut()
            .ok_or(Error::Missing("Government shared lifecycle"))?
            .government
            .constructed
            .push(handle);
        self.ensure_policy_city_lifecycle(handle)?;
        Ok(handle)
    }
    /// Static factories are installed without changing current counters/states.
    pub fn install_government_definitions(
        &mut self,
        state: &LoadedState,
        requirements: &mut super::super::requirements::Runtime,
        definition: &Government,
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        let mut source = sets::Runtime::from_initial(
            0,
            definition.sets.clone(),
            definition.requirements.clone(),
        )
        .map_err(Error::Missing)?;
        for id in &definition.any_sets {
            source.install_any(id).map_err(Error::Missing)?;
        }
        requirements
            .sets_mut()
            .install_factory_definitions(&source)
            .map_err(Error::Missing)?;
        let creation = self
            .city_creation
            .as_mut()
            .ok_or(Error::Missing("Government shared City lifecycle"))?;
        for r in &definition.buildings {
            if let Some(old) = creation.government.definitions.iter().find(|v| v.0 == r.0) {
                if old.1 != r.1 {
                    return Err(Error::Missing("CityHasBuilding changed definition"));
                }
            } else {
                creation.government.definitions.push(r.clone())
            }
        }
        Ok(())
    }
    pub fn attach_government_recipe(
        &mut self,
        state: &mut LoadedState,
        requirements: &mut super::super::requirements::Runtime,
        owner: u64,
        recipe: &Recipe,
        reports: &mut impl crate::city_production_turn::modifier_effects::Effects,
    ) -> Result<(), Error> {
        match recipe {
            Recipe::City(d) => {
                self.attach_city_owner(state, requirements, owner, d, None, reports)?;
            }
            Recipe::FlatBonus { id, bonus, amount } => {
                let instance = self.allocate_modifier_id(state).map_err(Error::Query)?;
                let effect = (1u64 << 62) | u64::from(instance as u32);
                if self.effects.iter().any(|r| r.0 == effect) {
                    return Err(Error::Query(QueryError::DuplicateEffect(effect)));
                }
                self.effects.push((
                    effect,
                    super::super::EffectState::GovernmentBonus {
                        context: owner,
                        bonus: *bonus,
                        amount: *amount,
                    },
                ));
                let creation = self
                    .city_creation
                    .as_mut()
                    .ok_or(Error::Missing("Government flat lifecycle"))?;
                let destruction = creation
                    .object_mut(owner)?
                    .destroyed
                    .subscribe(DestroyedListener::ModifierOwner(instance))
                    .map_err(Error::Ring)?;
                let tracked_destruction = creation
                    .object_mut(owner)?
                    .destroyed
                    .subscribe(DestroyedListener::Modifier(instance))
                    .map_err(Error::Ring)?;
                creation
                    .object_mut(owner)?
                    .affected
                    .mark(id, instance)
                    .map_err(|_| Error::Missing("FlatBonus affected ordering"))?;
                creation.government.flat.push(Flat {
                    owner,
                    instance,
                    effect,
                    destruction,
                    tracked_destruction,
                    applied: false,
                });
                self.apply_government_bonus(state, effect, false)?;
                self.city_creation
                    .as_mut()
                    .unwrap()
                    .government
                    .flat
                    .last_mut()
                    .unwrap()
                    .applied = true;
            }
        }
        Ok(())
    }
    fn apply_government_bonus(
        &self,
        state: &mut LoadedState,
        effect: u64,
        remove: bool,
    ) -> Result<(), Error> {
        let super::super::EffectState::GovernmentBonus {
            context,
            bonus,
            amount,
        } = *self.effect(effect).map_err(Error::Query)?
        else {
            return Err(Error::Missing("Government bonus effect"));
        };
        let super::super::ContextObject::Government { owner, .. } =
            self.object(context).map_err(Error::Query)?.kind
        else {
            return Err(Error::Missing("Government bonus owner"));
        };
        let Some(bonus) = bonus else { return Ok(()) }; // GetIndex -1 leaf is a real skip.
        let value = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("Government bonus Player"))?
            .bonuses
            .flat_bonuses
            .iter()
            .position(|r| r.definition_hash == bonus)
            .ok_or(Error::Missing("Government flat bonus indexed array"))?;
        let bonuses = state
            .player_bonuses_mut(owner)
            .map_err(|_| Error::Missing("Government mutable bonuses"))?;
        bonuses.flat_bonuses[value].value =
            bonuses.flat_bonuses[value].value.wrapping_add(if remove {
                amount.wrapping_neg()
            } else {
                amount
            });
        Ok(())
    }
    /// ClearOwnedModifiers ordered by actual shared modifier ID. Mixed families
    /// interleave through this owner; no government-specific effect totals exist.
    pub fn remove_government_modifiers(
        &mut self,
        state: &mut LoadedState,
        requirements: &mut super::super::requirements::Runtime,
        owner: u64,
        reports: &mut impl crate::city_production_turn::modifier_effects::Effects,
    ) -> Result<(), Error> {
        loop {
            let creation = self
                .city_creation
                .as_ref()
                .ok_or(Error::Missing("Government lifecycle"))?;
            let city = creation
                .modifiers
                .iter()
                .filter(|r| r.owner == owner)
                .map(|r| r.instance)
                .min();
            let flat = creation
                .government
                .flat
                .iter()
                .filter(|r| r.owner == owner)
                .min_by_key(|r| r.instance)
                .map(|r| r.instance);
            match (city, flat) {
                (None, None) => break,
                (Some(c), f) if f.is_none_or(|f| c < f) => {
                    self.remove_one_city_modifier(state, requirements, owner, reports)?;
                }
                (_, Some(f)) => {
                    let creation = self.city_creation.as_mut().unwrap();
                    let at = creation
                        .government
                        .flat
                        .iter()
                        .position(|r| r.owner == owner && r.instance == f)
                        .unwrap();
                    let row = creation.government.flat.remove(at);
                    self.city_creation
                        .as_mut()
                        .unwrap()
                        .object_mut(owner)?
                        .destroyed
                        .unsubscribe(row.tracked_destruction)
                        .map_err(Error::Ring)?;
                    self.city_creation
                        .as_mut()
                        .unwrap()
                        .object_mut(owner)?
                        .affected
                        .clear_instance(row.instance);
                    if row.applied {
                        self.apply_government_bonus(state, row.effect, true)?;
                    }
                    self.city_creation
                        .as_mut()
                        .unwrap()
                        .object_mut(owner)?
                        .destroyed
                        .unsubscribe(row.destruction)
                        .map_err(Error::Ring)?;
                    self.retire_effect(state, row.effect)
                        .map_err(Error::Query)?;
                }
                _ => unreachable!(),
            }
        }
        Ok(())
    }
    /// Append these receivers at their actual registration positions in all
    /// three source GameEffects building signals. Removed receivers stay inert.
    pub fn city_building_subscriptions(&self) -> Result<Vec<BuildingSubscription>, Error> {
        Ok(self
            .city_creation
            .as_ref()
            .ok_or(Error::Missing("CityHasBuilding registrations"))?
            .government
            .building
            .iter()
            .filter(|r| r.active)
            .map(|r| r.route)
            .collect())
    }
    pub fn notify_city_building_changed(
        &mut self,
        state: &mut LoadedState,
        requirements: &mut super::super::requirements::Runtime,
        route: BuildingSubscription,
        city: ComponentId,
        reports: &mut impl crate::city_production_turn::modifier_effects::Effects,
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        if route.city != city {
            return Ok(());
        }
        if !self
            .city_creation
            .as_ref()
            .ok_or(Error::Missing("CityHasBuilding registrations"))?
            .government
            .building
            .iter()
            .any(|r| r.active && r.route == route)
        {
            return Ok(());
        }
        changes::invalidate(
            &mut Changes {
                state,
                runtime: self,
                requirements,
                reports,
            },
            route.requirement,
        )
        .map_err(|e| Error::Requirement(Box::new(e)))
    }
}
struct Changes<'a, E> {
    state: &'a mut LoadedState,
    runtime: &'a mut EffectRuntime,
    requirements: &'a mut super::super::requirements::Runtime,
    reports: &'a mut E,
}
impl<E: crate::city_production_turn::modifier_effects::Effects> changes::Context
    for Changes<'_, E>
{
    type Error = Error;
    fn subject_sets(&self) -> Result<&sets::Runtime, Error> {
        Ok(self.requirements.sets())
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, Error> {
        Ok(self.requirements.sets_mut())
    }
    fn test_requirement(&mut self, label: u64) -> Result<i32, sets::Error<Error>> {
        let creation = self
            .runtime
            .city_creation
            .as_ref()
            .ok_or(sets::Error::Context(Error::Missing(
                "CityHasBuilding lifecycle",
            )))?;
        let r = creation
            .government
            .building
            .iter()
            .find(|r| r.active && r.route.requirement == label)
            .ok_or(sets::Error::Context(Error::Missing(
                "CityHasBuilding live receiver",
            )))?;
        test_building(self.state, creation, r.route.city, r.building).map_err(sets::Error::Context)
    }
    fn modifier_changed(
        &mut self,
        listener: sets::SetListener,
        set: u64,
        _old: i32,
        _new: i32,
    ) -> Result<(), sets::Error<Error>> {
        let sets::SetListener::Subject(receiver) = listener else {
            return Err(sets::Error::Context(Error::Missing(
                "Government non-subject set listener",
            )));
        };
        let creation = self
            .runtime
            .city_creation
            .as_ref()
            .ok_or(sets::Error::Context(Error::Missing(
                "Government City lifecycle",
            )))?;
        let index = creation
            .modifiers
            .iter()
            .position(|m| m.identity == receiver)
            .ok_or(sets::Error::Context(Error::Missing(
                "Government City modifier",
            )))?;
        let subject = creation.modifiers[index]
            .tracked
            .iter()
            .find(|r| r.state.requirement_set == Some(set))
            .ok_or(sets::Error::Context(Error::Missing(
                "Government tracked set",
            )))?
            .subject;
        c6_core::unit_moved_signal::modifiers::check_apply_effects(&mut Apply {
            state: self.state,
            runtime: self.runtime,
            requirements: self.requirements,
            subject,
            factory: &mut collection::MissingRequirements,
            features: &[],
            maximum_loyalty: None,
            index,
            effects: self.reports,
        })
        .map_err(sets::Error::Context)?;
        Ok(())
    }
    fn destroy_requirement(&mut self, label: u64) -> Result<(), sets::Error<Error>> {
        let creation = self
            .runtime
            .city_creation
            .as_mut()
            .ok_or(sets::Error::Context(Error::Missing(
                "CityHasBuilding lifecycle",
            )))?;
        let r = creation
            .government
            .building
            .iter_mut()
            .find(|r| r.active && r.route.requirement == label)
            .ok_or(sets::Error::Context(Error::Missing(
                "current derived City requirement destructor",
            )))?;
        r.active = false;
        Ok(())
    }
    fn destroy_set(&mut self, _: u64) -> Result<(), sets::Error<Error>> {
        Ok(())
    }
    fn reset_trigger(&mut self, set: u64) -> Result<(), sets::Error<Error>> {
        changes::reset_trigger_state(self.requirements.sets_mut(), set)
    }
}
pub(super) fn release<E: crate::city_production_turn::modifier_effects::Effects>(
    state: &mut LoadedState,
    runtime: &mut EffectRuntime,
    requirements: &mut super::super::requirements::Runtime,
    set: u64,
    reports: &mut E,
) -> Result<(), Error> {
    changes::release(
        &mut Changes {
            state,
            runtime,
            requirements,
            reports,
        },
        set,
    )
    .map_err(|e| Error::Requirement(Box::new(e)))
}
