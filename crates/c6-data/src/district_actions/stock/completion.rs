//! Source-ordered stock DistrictCompleted consumers. Current gameplay owners
//! stay in LoadedState/StockRuntime; registrations are current input facts.
use super::{error, Effects};
use crate::{
    city_production_turn as production, district_initialization::DistrictReference,
    registry::linux_type_hash, Rules,
};
use c6_core::loaded_state::LoadedState;

#[derive(Clone, Copy, Debug)]
pub struct Requirement {
    pub label: u64,
    pub owner: i32,
    pub district: Option<u32>,
}
#[derive(Clone, Copy, Debug)]
pub enum Listener {
    Attach,
    SummaryOutput,
    Script,
    Gossip,
    EraScore,
    History,
    Quests,
    Boost(i32),
    Player(i32),
    PlayerEvent(i32),
    Requirement(Requirement),
    Achievement,
    Unsupported(u64),
}
pub struct Cached {
    campus_modifiers: bool,
    era_chronology: Vec<(u32, i32)>,
    starting_buildings: Vec<(u32, i32)>,
}
impl Cached {
    pub fn from_rules(rules: &Rules) -> Result<Self, production::Error> {
        let table = |name| {
            rules
                .gameplay()
                .table(name)
                .ok_or(production::Error::Unsupported(name))
        };
        let mut campus_modifiers = false;
        for row in table("DistrictModifiers")?.rows() {
            if row.text("DistrictType").map_err(production::Error::Rules)? == "DISTRICT_CAMPUS" {
                campus_modifiers = true;
            }
        }
        let mut era_chronology = Vec::new();
        for row in table("Eras")?.rows() {
            era_chronology.push((
                linux_type_hash(
                    row.text("EraType")
                        .map_err(production::Error::Rules)?
                        .as_bytes(),
                ),
                row.integer("ChronologyIndex")
                    .map_err(production::Error::Rules)? as i32,
            ));
        }
        let mut starting_buildings = Vec::new();
        for row in table("StartingBuildings")?.rows() {
            let era = linux_type_hash(
                row.text("Era")
                    .map_err(production::Error::Rules)?
                    .as_bytes(),
            );
            let district = linux_type_hash(
                row.text("District")
                    .map_err(production::Error::Rules)?
                    .as_bytes(),
            );
            if let Some((_, chronology)) = era_chronology.iter().find(|p| p.0 == era) {
                starting_buildings.push((district, *chronology));
            }
        }
        Ok(Self {
            campus_modifiers,
            era_chronology,
            starting_buildings,
        })
    }
}
pub struct Services<'a> {
    pub cached: &'a Cached,
    pub listeners: &'a [Listener],
    pub player_listeners: &'a [(i32, usize)],
    /// Current zoned callbacks are only the source-qualified CityBuild observer;
    /// it may invalidate production policy caches but never choose an action.
    pub zoned_owners: &'a [i32],
    pub boosts: &'a production::district_boosts::Definitions,
    pub quests: &'a std::cell::RefCell<crate::game_update_state::round_quests::Runtime>,
    pub gossip_history: &'a [c6_core::city_power_activation::gossip_delivery::TurnVisibility],
}
impl Services<'_> {
    pub fn check_zoned(&self, owner: i32) -> Result<(), production::Error> {
        if self.zoned_owners.contains(&owner) {
            Ok(())
        } else {
            Err(production::Error::Unsupported(
                "current qualified DistrictZoned subscriptions",
            ))
        }
    }
    pub fn starting_buildings(
        &self,
        _: &LoadedState,
        district: DistrictReference,
        start_era: Option<u32>,
        maps: &[production::selection::DistrictMap],
        campus: u32,
    ) -> Result<(), production::Error> {
        let hash = start_era.ok_or(production::Error::Unsupported(
            "current configuration start era",
        ))?;
        let Some((_, chronology)) = self.cached.era_chronology.iter().find(|p| p.0 == hash) else {
            return Ok(());
        };
        if *chronology <= 0 {
            return Ok(());
        }
        let map = maps.iter().find(|p| p.owner == district.owner).ok_or(
            production::Error::Unsupported("starting building District owner"),
        )?;
        for &(hash, era) in &self.cached.starting_buildings {
            if era > *chronology {
                continue;
            }
            let mapped = map
                .values
                .iter()
                .rfind(|p| p.definition_hash == hash)
                .ok_or(production::Error::Unsupported(
                    "starting building District map",
                ))?;
            if mapped.value == Some(campus) {
                return Err(production::Error::Unsupported(
                    "positive district starting building",
                ));
            }
        }
        Ok(())
    }
}
pub fn dispatch<P>(
    effects: &mut Effects<'_, '_, '_, P>,
    state: &mut LoadedState,
    d: DistrictReference,
    bought: bool,
) -> Result<(), production::Error> {
    let services = effects.inputs.completion;
    for &listener in services.listeners {
        match listener {
            Listener::Attach => {
                if services.cached.campus_modifiers {
                    return Err(production::Error::Unsupported(
                        "positive District modifier attachments",
                    ));
                }
            }
            Listener::SummaryOutput | Listener::Achievement => {}
            Listener::Script => {
                let record = state
                    .district(d.owner, d.id)
                    .map_err(production::Error::District)?;
                let arguments = [
                    d.owner,
                    effects.inputs.definitions.campus_index,
                    record.location.0,
                    record.location.1,
                    i32::from(bought),
                ];
                effects
                    .backend
                    .script
                    .call_empty(state, "OnDistrictConstructed", &arguments)
                    .map_err(|e| error("DistrictConstructed script", e))?;
            }
            Listener::Gossip if services.gossip_history.is_empty() => {}
            Listener::Gossip => {
                return Err(production::Error::Unsupported(
                    "district Gossip propagation",
                ))
            }
            Listener::EraScore => {
                if bought {
                    let rate = state
                        .game_era_district_construction_rate(d.owner)
                        .map_err(|e| error("current district Era rate", e))?;
                    if rate.is_some_and(|r| r != 0) {
                        return Err(production::Error::Unsupported(
                            "positive district construction Era score",
                        ));
                    }
                }
            }
            Listener::History => {
                let query =
                    effects
                        .backend
                        .city_query_inputs()
                        .ok_or(production::Error::Unsupported(
                            "district History yield services",
                        ))?;
                let science = query.definitions.yield_index(b"YIELD_SCIENCE").ok_or(
                    production::Error::Unsupported("district History Science index"),
                )?;
                let adjacency = crate::city_turn::district_yields::adjacency(
                    state,
                    d.owner,
                    d.id,
                    science,
                    false,
                    &query.definitions.districts,
                )
                .map_err(|e| error("district History adjacency", e))?;
                if bought && adjacency > 2 {
                    return Err(production::Error::Unsupported(
                        "high-adjacency Campus Historic moment",
                    ));
                }
            }
            Listener::Quests => {
                let quests = services.quests.borrow();
                quests
                    .validate(state)
                    .map_err(|e| error("current district Quests", e))?;
                if !quests.current.is_empty() {
                    return Err(production::Error::Unsupported(
                        "DistrictConstructed active Quests",
                    ));
                }
            }
            Listener::Boost(id) => {
                let mut backend =
                    crate::loaded_actions::stock_notifications::research::Effects::new(
                        effects.backend,
                        effects.research_runtime,
                        effects.inputs.choices,
                        effects.inputs.culture,
                    );
                let mut context = crate::research::Context {
                    state,
                    definitions: effects.inputs.research,
                    effects: &mut backend,
                };
                production::district_boosts::completed(
                    &mut context,
                    services.boosts,
                    effects.inputs.current.district_maps,
                    d,
                    id,
                    bought,
                )
                .map_err(|e| error("DistrictCompleted boost", e))?;
            }
            Listener::Player(owner) if bought && owner == d.owner => {
                let p = state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == owner)
                    .ok_or(production::Error::Unsupported(
                        "district Player subscriber owner",
                    ))?;
                let types = p
                    .tail
                    .as_ref()
                    .and_then(|t| t.alive.as_ref())
                    .and_then(|a| a.district_naval_unit_hashes.as_ref())
                    .ok_or(production::Error::Unsupported(
                        "current district naval unit grants",
                    ))?;
                if types.contains(&effects.inputs.definitions.campus_hash) {
                    return Err(production::Error::Unsupported("district naval unit award"));
                }
            }
            Listener::Player(_) => {}
            Listener::PlayerEvent(owner) if owner == d.owner => {
                let p = state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == owner)
                    .ok_or(production::Error::Unsupported(
                        "district Event subscriber owner",
                    ))?;
                let record = state
                    .district(d.owner, d.id)
                    .map_err(production::Error::District)?;
                let plot = state
                    .world()
                    .map()
                    .plot_at(record.location.0, record.location.1)
                    .ok_or(production::Error::Unsupported("district resource plot"))?;
                if plot.resource_hash != u32::MAX {
                    return Err(production::Error::Unsupported(
                        "district resource extraction change",
                    ));
                }
                if bought
                    && (p
                        .districts
                        .district_and_buildings_creation_units
                        .iter()
                        .any(|v| v.district_hash == effects.inputs.definitions.campus_hash)
                        || p.districts
                            .free_building_when_specialty_district_constructed
                            != 0
                        || p.districts.specific_district_grant_envoys.iter().any(|v| {
                            v.definition_hash == effects.inputs.definitions.campus_hash
                                && v.value > 0
                        }))
                {
                    return Err(production::Error::Unsupported(
                        "district completion unit/building/envoy grant",
                    ));
                }
            }
            Listener::PlayerEvent(_) => {}
            Listener::Requirement(binding) if binding.owner == d.owner => {
                use c6_core::melee_attack::proposals::subject_sets::changes;
                let owners = effects
                    .backend
                    .modifier_owners_mut()
                    .map_err(|e| error("current district requirement owners", e))?;
                changes::invalidate(
                    &mut RequirementContext {
                        state,
                        requirements: &mut owners.requirements,
                        binding,
                    },
                    binding.label,
                )
                .map_err(|e| error("district requirement invalidation", e))?;
            }
            Listener::Requirement(_) => {}
            Listener::Unsupported(_) => {
                return Err(production::Error::Unsupported(
                    "unqualified DistrictCompleted listener",
                ))
            }
        }
    }
    Ok(())
}
pub fn aftermath<P>(
    effects: &mut Effects<'_, '_, '_, P>,
    state: &mut LoadedState,
    d: DistrictReference,
) -> Result<(), production::Error> {
    let p = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == d.owner)
        .ok_or(production::Error::Unsupported("district aftermath owner"))?;
    if p.culture
        .culture_bomb_triggers
        .iter()
        .any(|v| v.district_hash == effects.inputs.definitions.campus_hash)
    {
        return Err(production::Error::Unsupported("district CultureBomb"));
    }
    let location = state
        .district(d.owner, d.id)
        .map_err(production::Error::District)?
        .location;
    struct Appeal<'a> {
        subscribers: Option<&'a [u64]>,
    }
    impl production::appeal::Effects for Appeal<'_> {
        type Error = production::Error;
        fn appeal_changed(
            &mut self,
            _: &mut LoadedState,
            _: u32,
            _: i32,
        ) -> Result<(), Self::Error> {
            match self.subscribers {
                Some([]) => Ok(()),
                _ => Err(production::Error::Unsupported(
                    "district appeal subscribers",
                )),
            }
        }
        fn appeal_report(&mut self, _: &mut LoadedState, _: u32) -> Result<(), Self::Error> {
            Ok(())
        }
    }
    production::appeal::update(
        state,
        [location.0, location.1],
        effects.inputs.appeal,
        &mut Appeal {
            subscribers: effects.inputs.appeal_subscribers,
        },
    )
    .map_err(|e| error("district completion appeal", e))?;
    Ok(())
}

struct RequirementContext<'a> {
    state: &'a LoadedState,
    requirements: &'a mut crate::melee_attack::backend::requirements::Runtime,
    binding: Requirement,
}
use c6_core::melee_attack::proposals::subject_sets::{self as sets, changes};
impl changes::Context for RequirementContext<'_> {
    type Error = production::Error;
    fn subject_sets(&self) -> Result<&sets::Runtime, Self::Error> {
        Ok(self.requirements.sets())
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, Self::Error> {
        Ok(self.requirements.sets_mut())
    }
    fn test_requirement(&mut self, label: u64) -> Result<i32, sets::Error<Self::Error>> {
        if label != self.binding.label {
            return Err(sets::Error::Context(production::Error::Unsupported(
                "district requirement identity",
            )));
        }
        let Some(hash) = self.binding.district else {
            return Ok(0);
        };
        let player = self
            .state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == self.binding.owner)
            .ok_or(sets::Error::Context(production::Error::Unsupported(
                "district requirement Player",
            )))?;
        Ok(i32::from(player.districts.records.entries.iter().any(
            |r| r.value.district_hash == hash && r.value.is_complete,
        )))
    }
    fn modifier_changed(
        &mut self,
        _: sets::SetListener,
        _: u64,
        _: i32,
        _: i32,
    ) -> Result<(), sets::Error<Self::Error>> {
        Err(sets::Error::Context(production::Error::Unsupported(
            "changed district requirement modifier",
        )))
    }
    fn destroy_requirement(&mut self, _: u64) -> Result<(), sets::Error<Self::Error>> {
        Err(sets::Error::Context(production::Error::Unsupported(
            "district requirement destruction",
        )))
    }
    fn destroy_set(&mut self, _: u64) -> Result<(), sets::Error<Self::Error>> {
        Err(sets::Error::Context(production::Error::Unsupported(
            "district requirement set destruction",
        )))
    }
    fn reset_trigger(&mut self, set: u64) -> Result<(), sets::Error<Self::Error>> {
        changes::reset_trigger_state(self.requirements.sets_mut(), set)
    }
}
