//! Typed callbacks for the current shared City collection signal owner.
use super::*;
use c6_core::unit::ComponentId;
#[derive(Clone, Copy, Debug)]
pub struct CitySubscription {
    pub receiver: u64,
    pub owner: i32,
    pub capital: bool,
}
impl EffectRuntime {
    /// CityRemoved+2e8 registration order across PlayerCities and CapitalCity.
    pub fn city_removed_subscriptions(&self) -> Result<&[CitySubscription], Error> {
        Ok(&self
            .city_creation
            .as_ref()
            .ok_or(Error::Missing("current City creation domain"))?
            .city_removed_subscriptions)
    }
    /// Invoke at one actual PlayerCities CityAdded/Removed listener position.
    pub fn notify_player_city_item(
        &mut self,
        state: &mut LoadedState,
        requirements: &mut super::super::requirements::Runtime,
        route: PlayerCitiesSubscription,
        city: ComponentId,
        added: bool,
        effects: &mut impl crate::city_production_turn::modifier_effects::Effects,
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        if i32::from(city.owner) != route.owner {
            return Ok(());
        }
        let subject = self.make_city_object(state, city)?;
        collection::item_changed(
            state,
            self,
            requirements,
            collection::Invocation {
                receiver: route.receiver,
                subject,
                added,
                maximum_loyalty: None,
                features: &[],
            },
            &mut collection::MissingRequirements,
            effects,
        )
    }
    /// PlayerCapitalCity invalidates on the owner's CapitalChanged+408 or
    /// CityRemoved+2e8. OnCollectionChanged2332c30 then reconciles the sorted
    /// current GetItems result; retained subjects are rechecked, not reapplied.
    pub fn notify_player_capital_changed(
        &mut self,
        state: &mut LoadedState,
        requirements: &mut super::super::requirements::Runtime,
        route: PlayerCitiesSubscription,
        changed_city_owner: i32,
        effects: &mut impl crate::city_production_turn::modifier_effects::Effects,
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        if changed_city_owner != route.owner {
            return Ok(());
        }
        let index = self
            .city_creation
            .as_ref()
            .ok_or(Error::Missing("current City creation domain"))?
            .modifiers
            .iter()
            .position(|r| r.identity == route.receiver)
            .ok_or(Error::Missing("current capital modifier"))?;
        let m = &self.city_creation.as_ref().unwrap().modifiers[index];
        if m.modifier.suppressed || !m.modifier.enabled || !m.activated {
            return Ok(());
        }
        if m.definition.owner_requirements.is_some()
            || m.definition.new_only
            || m.definition.permanent
        {
            return Err(Error::Missing("capital modifier requirements/lifetime"));
        }
        let cities = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == route.owner)
            .ok_or(Error::Missing("current capital owner"))?
            .cities
            .records
            .entries
            .iter()
            .filter(|r| r.value.header.capital && !r.value.header.delayed_death)
            .map(|r| ComponentId {
                owner: route.owner as i16,
                component_type: 2,
                id: r.value.header.id,
            })
            .collect::<Vec<_>>();
        let mut subjects = Vec::with_capacity(cities.len());
        for city in cities {
            let h = self.make_city_object(state, city)?;
            subjects.push((collection::city_key(self, h)?, h));
        }
        subjects.sort_by_key(|r| r.0);
        let previous = self.city_creation.as_ref().unwrap().modifiers[index]
            .tracked
            .iter()
            .map(|r| (r.subject, r.state.removed))
            .collect::<Vec<_>>();
        let mut factory = collection::MissingRequirements;
        // UpdateTrackedSubjects2333df0 erases missing, nonremoved subjects first.
        for (subject, removed) in previous {
            if !removed && !subjects.iter().any(|r| r.1 == subject) {
                c6_core::unit_moved_signal::collection_addition::Context::remove_tracked_subject(
                    &mut Apply {
                        state,
                        runtime: self,
                        requirements,
                        subject,
                        factory: &mut factory,
                        features: &[],
                        maximum_loyalty: None,
                        index,
                        effects,
                    },
                )?;
            }
        }
        // AddTrackedSubject runs for every current item, including retained
        // items (temporary destruction subscription then CheckApplyEffects).
        for (_, subject) in subjects {
            c6_core::unit_moved_signal::collection_addition::add(&mut Apply {
                state,
                runtime: self,
                requirements,
                subject,
                factory: &mut factory,
                features: &[],
                maximum_loyalty: None,
                index,
                effects,
            })?;
        }
        Ok(())
    }
}
