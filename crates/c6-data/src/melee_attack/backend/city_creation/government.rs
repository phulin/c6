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
/// Borrowed gameplay identity/lifetime of a live reached research modifier.
/// Native allocation addresses are intentionally absent from this view.
#[derive(Clone, Debug)]
pub struct ResearchTradeCapacityModifier<'a> {
    pub instance: i32,
    pub context_instance: i32,
    pub owner: i32,
    pub kind: crate::research::Kind,
    pub hash: u32,
    pub definition: &'a [u8],
    pub amount: i32,
    pub effect: crate::research::attached_modifiers::Effect,
    pub applied: bool,
    pub owner_destroyed_subscription: bool,
    pub tracked_destroyed_subscription: bool,
}
/// Initial live records already reflected in canonical Player.Trade capacity.
/// Importing these records restores ownership without applying the effect again.
pub struct InitialResearchTradeCapacityModifier {
    pub instance: i32,
    pub context: u64,
    pub effect: u64,
    pub amount: i32,
    pub applied: bool,
}
impl EffectRuntime {
    pub fn install_initial_research_trade_capacity_modifiers(
        &mut self,
        state: &LoadedState,
        records: &[InitialResearchTradeCapacityModifier],
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        if !state.is_pristine() {
            return Err(Error::Missing(
                "research modifier import requires initial state",
            ));
        }
        for row in records {
            if !self.has_modifier_instance(row.instance) {
                return Err(Error::Missing("initial research modifier shared ID"));
            }
            if !matches!(
                self.object(row.context).map_err(Error::Query)?.kind,
                super::super::ContextObject::Research { .. }
            ) {
                return Err(Error::Missing("initial research wrapper"));
            }
            if self.effects.iter().any(|r| r.0 == row.effect) {
                return Err(Error::Query(QueryError::DuplicateEffect(row.effect)));
            }
            let creation = self
                .city_creation
                .as_mut()
                .ok_or(Error::Missing("initial research modifier lifecycle"))?;
            if creation
                .government
                .flat
                .iter()
                .any(|r| r.instance == row.instance)
            {
                return Err(Error::DuplicateModifier(row.instance));
            }
            let lifecycle = creation.object_mut(row.context)?;
            if !lifecycle
                .affected
                .entries()
                .iter()
                .any(|r| r.instance == row.instance)
            {
                return Err(Error::Missing("initial research affected entry"));
            }
            let destruction = lifecycle
                .destroyed
                .subscription_for(
                    |s| matches!(s, DestroyedListener::ModifierOwner(i) if i == row.instance),
                )
                .map_err(Error::Ring)?;
            let tracked_destruction = lifecycle
                .destroyed
                .subscription_for(
                    |s| matches!(s, DestroyedListener::Modifier(i) if i == row.instance),
                )
                .map_err(Error::Ring)?;
            self.effects.push((
                row.effect,
                super::super::EffectState::TradeRouteCapacity {
                    context: row.context,
                    amount: row.amount,
                },
            ));
            creation.government.flat.push(Flat {
                owner: row.context,
                instance: row.instance,
                effect: row.effect,
                destruction,
                tracked_destruction,
                applied: row.applied,
            });
        }
        Ok(())
    }
    pub fn research_trade_capacity_modifiers(
        &self,
    ) -> Result<Vec<ResearchTradeCapacityModifier<'_>>, Error> {
        Ok(self
            .research_modifiers()?
            .into_iter()
            .filter(|r| r.effect == crate::research::attached_modifiers::Effect::TradeCapacity)
            .collect())
    }
    /// All supported research modifier families share the same instance order,
    /// affected list and owner/tracked destruction subscriptions.
    pub fn research_modifiers(&self) -> Result<Vec<ResearchTradeCapacityModifier<'_>>, Error> {
        let creation = self
            .city_creation
            .as_ref()
            .ok_or(Error::Missing("current shared modifier lifecycle"))?;
        let mut out = Vec::new();
        for row in &creation.government.flat {
            let (context, amount, effect) = match *self.effect(row.effect).map_err(Error::Query)? {
                super::super::EffectState::TradeRouteCapacity { context, amount } => (
                    context,
                    amount,
                    crate::research::attached_modifiers::Effect::TradeCapacity,
                ),
                super::super::EffectState::ResearchCounter {
                    context,
                    effect,
                    amount,
                    ..
                } => (context, amount, effect),
                _ => continue,
            };
            let object = self.object(context).map_err(Error::Query)?;
            let super::super::ContextObject::Research { owner, kind, hash } = object.kind else {
                return Err(Error::Missing("research capacity owner"));
            };
            let lifecycle = creation
                .objects
                .iter()
                .find(|o| o.handle == context)
                .ok_or(Error::Missing("research modifier lifecycle"))?;
            let definition = lifecycle
                .affected
                .entries()
                .iter()
                .find(|r| r.instance == row.instance)
                .ok_or(Error::Missing("research affected modifier"))?
                .definition
                .as_slice();
            out.push(ResearchTradeCapacityModifier {
                instance: row.instance,
                context_instance: object.instance_id,
                owner,
                kind,
                hash,
                definition,
                amount,
                effect,
                applied: row.applied,
                owner_destroyed_subscription: lifecycle
                    .destroyed
                    .ordered_subscribers()
                    .any(|s| matches!(s, DestroyedListener::ModifierOwner(i) if i == row.instance)),
                tracked_destroyed_subscription: lifecycle
                    .destroyed
                    .ordered_subscribers()
                    .any(|s| matches!(s, DestroyedListener::Modifier(i) if i == row.instance)),
            });
        }
        Ok(out)
    }
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
                self.attach_flat_player_effect(
                    state,
                    owner,
                    id,
                    super::super::EffectState::GovernmentBonus {
                        context: owner,
                        bonus: *bonus,
                        amount: *amount,
                    },
                )?;
            }
        }
        Ok(())
    }
    /// COLLECTION_OWNER uses the existing modifier ID, affected-list and
    /// ObjectDestroyed lifetime machinery, shared with government flat effects.
    fn attach_flat_player_effect(
        &mut self,
        state: &mut LoadedState,
        owner: u64,
        id: &[u8],
        value: super::super::EffectState,
    ) -> Result<(), Error> {
        let instance = self.allocate_modifier_id(state).map_err(Error::Query)?;
        let effect = (1u64 << 62) | u64::from(instance as u32);
        if self.effects.iter().any(|r| r.0 == effect) {
            return Err(Error::Query(QueryError::DuplicateEffect(effect)));
        }
        self.effects.push((effect, value));
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
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn attach_research_modifiers(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        kind: crate::research::Kind,
        hash: u32,
        recipes: &[crate::research::attached_modifiers::Recipe],
        civilization_levels: &[(i32, u32)],
        borders_report_cache_only: bool,
    ) -> Result<(), Error> {
        self.current_owner(state).map_err(Error::Query)?;
        if recipes.is_empty() {
            return Ok(());
        }
        let is_major = if recipes.iter().any(|r| {
            matches!(
                r.effect,
                crate::research::attached_modifiers::Effect::GovernorPoints
                    | crate::research::attached_modifiers::Effect::GrantSpy
            )
        }) {
            let level = civilization_levels
                .iter()
                .find(|r| r.0 == owner)
                .map(|r| r.1)
                .ok_or(Error::Missing("current governor player civilization level"))?;
            if level == 0x253718b0
                && recipes.iter().any(|r| {
                    r.effect == crate::research::attached_modifiers::Effect::GovernorPoints
                }) && state.governor_current(owner).is_none()
            {
                return Err(Error::Missing("current major governor maximum"));
            }
            level == 0x253718b0
        } else {
            false
        };
        for recipe in recipes {
            if recipe.effect == crate::research::attached_modifiers::Effect::EnforceBorders {
                let current = state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == owner)
                    .ok_or(Error::Missing("current border diplomacy Player"))?
                    .diplomacy
                    .can_enforce_borders;
                if current != (recipe.amount != 0) {
                    qualify_border_change(state, owner, borders_report_cache_only)?;
                }
            }
        }
        let kind = super::super::ContextObject::Research { owner, kind, hash };
        let context = if let Some((h, _)) = self.objects.iter().find(|r| r.1.kind == kind) {
            *h
        } else {
            let h = self
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
                .ok_or(Error::Missing("research shared modifier lifecycle"))?
                .government
                .constructed
                .push(h);
            self.ensure_policy_city_lifecycle(h)?;
            h
        };
        for recipe in recipes {
            self.attach_flat_player_effect(
                state,
                context,
                &recipe.id,
                match recipe.effect {
                    crate::research::attached_modifiers::Effect::TradeCapacity => {
                        super::super::EffectState::TradeRouteCapacity {
                            context,
                            amount: recipe.amount,
                        }
                    }
                    effect => super::super::EffectState::ResearchCounter {
                        context,
                        effect,
                        amount: recipe.amount,
                        is_major,
                        borders_report_cache_only,
                    },
                },
            )?;
        }
        Ok(())
    }
    fn apply_government_bonus(
        &self,
        state: &mut LoadedState,
        effect: u64,
        remove: bool,
    ) -> Result<(), Error> {
        if let super::super::EffectState::ResearchCounter {
            context,
            effect: counter,
            amount,
            is_major,
            borders_report_cache_only,
        } = *self.effect(effect).map_err(Error::Query)?
        {
            let super::super::ContextObject::Research { owner, .. } =
                self.object(context).map_err(Error::Query)?.kind
            else {
                return Err(Error::Missing("research counter Player interface"));
            };
            match counter {
                crate::research::attached_modifiers::Effect::GovernorPoints => {
                    // Positive major deltas update points up to the actual
                    // unsaved maximum, and the external title-decision flag. Negative
                    // Remove still reports but does not subtract native points.
                    state.change_governor_points(owner, is_major, if remove { amount.wrapping_neg() } else { amount })
                        .map_err(|_| Error::Missing("current Governor points and maximum"))?;
                    // da3ef23d is a stock Player.Governors cache publication;
                    // appointment and promotion remain explicit player choices.
                }
                crate::research::attached_modifiers::Effect::ReligiousTourismReduction => {
                    let culture = state
                        .research_components_mut(owner)
                        .map_err(|_| Error::Missing("current religious tourism Culture"))?
                        .1;
                    culture.religious_tourism_reduction =
                        culture.religious_tourism_reduction.wrapping_add(if remove {
                            amount.wrapping_neg()
                        } else {
                            amount
                        });
                }
                crate::research::attached_modifiers::Effect::GrantSpy => {
                    // Apply has an IsMajor guard; Remove always reaches ChangeSpyCapacity,
                    // whose nonmajor branch assigns zero rather than adding the delta.
                    if remove || is_major {
                        let espionage = state
                            .activation_managers_mut(owner)
                            .map_err(|_| Error::Missing("current spy Espionage"))?
                            .espionage;
                        espionage.spy_capacity = if is_major {
                            espionage.spy_capacity.wrapping_add(if remove {
                                amount.wrapping_neg()
                            } else {
                                amount
                            })
                        } else {
                            0
                        };
                    }
                }
                crate::research::attached_modifiers::Effect::EnforceBorders => {
                    let enabled = (amount != 0) ^ remove;
                    let current = state
                        .retained()
                        .players
                        .iter()
                        .find(|p| p.header.owner == owner)
                        .ok_or(Error::Missing("current border diplomacy Player"))?
                        .diplomacy
                        .can_enforce_borders;
                    if current != enabled {
                        qualify_border_change(state, owner, borders_report_cache_only)?;
                        state
                            .activation_managers_mut(owner)
                            .map_err(|_| Error::Missing("current border diplomacy Player"))?
                            .diplomacy
                            .can_enforce_borders = enabled;
                    }
                }
                crate::research::attached_modifiers::Effect::TradeCapacity => {
                    return Err(Error::Missing("research counter effect family"))
                }
            }
            return Ok(());
        }
        if let super::super::EffectState::TradeRouteCapacity { context, amount } =
            *self.effect(effect).map_err(Error::Query)?
        {
            let super::super::ContextObject::Research { owner, .. } =
                self.object(context).map_err(Error::Query)?.kind
            else {
                return Err(Error::Missing("research capacity Player interface"));
            };
            let delta = if remove {
                amount.wrapping_neg()
            } else {
                amount
            };
            if delta != 0 {
                let trade = state
                    .activation_managers_mut(owner)
                    .map_err(|_| Error::Missing("current trade capacity Player"))?
                    .trade;
                trade.outgoing_route_capacity = trade.outgoing_route_capacity.wrapping_add(delta);
            }
            return Ok(());
        }
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
        if creation
            .requirement_cleanup
            .destroy(label)
            .map_err(sets::Error::Context)?
        {
            return Ok(());
        }
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

/// SetCanEnforceBorders calls ExpelFromTerritory for every other alive player.
/// With no plot owned by this player, its per-unit plot-owner equality cannot
/// reach pathfinding, movement, notifications or RNG. Positive territory cases
/// remain unsupported rather than silently skipping those gameplay effects.
fn qualify_border_change(
    state: &LoadedState,
    owner: i32,
    report_cache_only: bool,
) -> Result<(), Error> {
    if !report_cache_only {
        return Err(Error::Missing(
            "enforce borders reporting consumers unqualified",
        ));
    }
    // The captured OnAllPlayerDiplomacyChanged subscriber publishes each
    // alive player's cache. DiplomaticAI threat/trust queries are bounded by
    // their visibility guards only below two; keep that condition current.
    // Checking every represented player also covers the captured alive list.
    for player in &state.retained().players {
        if player.diplomacy.visibility.len() != 64
            || player.diplomacy.visibility.iter().any(|&level| level >= 2)
        {
            return Err(Error::Missing(
                "enforce borders current cache AI diplomacy visibility unqualified",
            ));
        }
    }
    for plot in &state.world().map().plots {
        if crate::movement_placement_state::plot_owner(plot)
            .map_err(|_| Error::Missing("current border Plot owner"))?
            == owner
        {
            return Err(Error::Missing(
                "enforce borders with owned territory: expulsion continuation unqualified",
            ));
        }
    }
    Ok(())
}
