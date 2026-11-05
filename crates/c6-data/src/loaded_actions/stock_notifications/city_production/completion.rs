//! Ordered ordinary-building completion using the persistent modifier owners.
use super::super::Runtime;
use crate::{
    city_production_turn::{
        self as production, building_callbacks as callbacks, buildings, modifier_effects,
    },
    melee_attack::backend::{self, city_creation::definitions},
    Rules,
};
use c6_core::{
    loaded_state::LoadedState,
    melee_attack::proposals::subject_sets::{self as sets, changes},
};

#[derive(Clone, Copy, Debug)]
pub struct Requirement {
    pub label: u64,
    pub owner: i32,
    pub building: Option<u32>,
}
#[derive(Clone, Copy, Debug)]
pub enum Listener {
    Attach,
    /// Source-qualified HallOfFame/dataset output consumers.
    SummaryOutput,
    Script,
    Gossip,
    EraScore,
    History,
    Statistics,
    Boost(callbacks::BoostBinding),
    PlayerEvent(i32),
    Requirement(Requirement),
    Achievement,
    Unsupported(u64),
}
/// Current registrations and definition owners, borrowed for a production step.
/// Gameplay owners are supplied by the SAME StockRuntime, never copied here.
#[derive(Clone, Copy)]
pub struct Services<'a> {
    pub rules: &'a Rules,
    pub attachments: &'a definitions::Definitions,
    pub listeners: &'a [Listener],
    pub building_maps: &'a [callbacks::BuildingMap],
    /// Borrow the current Gossip history owner. Nonempty history needs its real
    /// propagation continuation rather than an initial empty-history answer.
    pub gossip_history: &'a [c6_core::city_power_activation::gossip_delivery::TurnVisibility],
    pub history_buildings: &'a [u32],
    pub player_signal_listeners: &'a [(i32, usize)],
}
fn missing(value: &'static str) -> production::Error {
    production::Error::Unsupported(value)
}
struct ModifierReports;
impl modifier_effects::Effects for ModifierReports {
    fn report(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
        _: modifier_effects::Report,
    ) -> Result<(), production::Error> {
        // Qualified stock City/yield consumers only publish recomputable caches.
        Ok(())
    }
}
pub(super) fn dispatch<P>(
    runtime: &mut Runtime<'_, P>,
    state: &mut LoadedState,
    event: callbacks::Event,
    plot: i32,
    defs: &buildings::Definitions,
    services: &Services<'_>,
) -> Result<(), production::Error> {
    let callbacks::Event {
        owner,
        city,
        building,
    } = event;
    for listener in services.listeners {
        match *listener {
            Listener::Attach => {
                let owners = runtime
                    .modifier_owners_mut()
                    .map_err(|_| missing("current building modifier owners"))?;
                let handle = owners
                    .effects
                    .objects()
                    .iter()
                    .find_map(|(handle, object)| match object.kind {
                        backend::ContextObject::City(cid)
                            if i32::from(cid.owner) == owner && cid.id == city =>
                        {
                            Some(*handle)
                        }
                        _ => None,
                    })
                    .ok_or(missing("current building City object"))?;
                for attachment in services
                    .attachments
                    .building(building)
                    .map_err(|_| missing("current building attachment definitions"))?
                {
                    owners
                        .effects
                        .attach_city_owner(
                            state,
                            &mut owners.requirements,
                            handle,
                            attachment,
                            services.attachments.maximum_loyalty(),
                            &mut ModifierReports,
                        )
                        .map_err(|_| missing("stock building modifier attachment"))?;
                }
            }
            Listener::SummaryOutput => {}
            Listener::Script => {
                let index = defs
                    .buildings
                    .iter()
                    .position(|v| v.hash == building)
                    .ok_or(missing("current native building index"))?;
                runtime
                    .script
                    .call_empty(
                        state,
                        "BuildingConstructed",
                        &[owner, city as i32, index as i32, plot, 1],
                    )
                    .map_err(|_| missing("BuildingConstructed script"))?;
            }
            Listener::Gossip if services.gossip_history.is_empty() => {}
            Listener::Gossip => return Err(missing("building Gossip visibility propagation")),
            Listener::EraScore => {
                if !defs.building_era_score_noop_current(state, owner, building)? {
                    return Err(missing("building Era score award"));
                }
            }
            Listener::History => {
                let row = services
                    .rules
                    .gameplay()
                    .table("Buildings")
                    .ok_or(missing("Buildings"))?
                    .rows()
                    .find(|row| {
                        row.text("BuildingType")
                            .ok()
                            .map(|v| crate::registry::linux_type_hash(v.as_bytes()))
                            == Some(building)
                    })
                    .ok_or(missing("current History building"))?;
                if !matches!(
                    row.get("IsWonder").map_err(production::Error::Rules)?,
                    crate::SqlValue::Integer(0)
                ) || !matches!(
                    row.get("TraitType").map_err(production::Error::Rules)?,
                    crate::SqlValue::Null
                ) || services.history_buildings.contains(&building)
                {
                    return Err(missing("positive building History"));
                }
                let name = row.text("BuildingType").map_err(production::Error::Rules)?;
                for replacement in services
                    .rules
                    .gameplay()
                    .table("BuildingReplaces")
                    .ok_or(missing("BuildingReplaces"))?
                    .rows()
                {
                    if replacement
                        .text("CivUniqueBuildingType")
                        .map_err(production::Error::Rules)?
                        == name
                        || replacement
                            .text("ReplacesBuildingType")
                            .map_err(production::Error::Rules)?
                            == name
                    {
                        return Err(missing("replacement building History"));
                    }
                }
            }
            Listener::Statistics => {
                state
                    .record_game_building_constructed(Some(building), plot, true)
                    .map_err(production::Error::Statistics)?;
            }
            Listener::Boost(binding) => callbacks::boosts(
                state,
                event,
                defs,
                services.rules,
                &callbacks::Current {
                    boosts: &[binding],
                    building_maps: services.building_maps,
                    player_event_owners: &[],
                    requirements: &[],
                },
            )?,
            Listener::PlayerEvent(player) => {
                callbacks::player_events(state, event, defs, services.rules, &[player])?
            }
            Listener::Requirement(binding) => {
                // OnCompleted guards nonnull City and completion=true, then owner
                // only; target BuildingTypes is a Test input, not an event filter.
                if binding.owner == owner {
                    let owners = runtime
                        .modifier_owners_mut()
                        .map_err(|_| missing("current building requirement owner"))?;
                    changes::invalidate(
                        &mut RequirementContext {
                            state,
                            requirements: &mut owners.requirements,
                            binding,
                        },
                        binding.label,
                    )
                    .map_err(|_| missing("building requirement change continuation"))?;
                }
            }
            Listener::Achievement => callbacks::achievements(state, event)?,
            Listener::Unsupported(_) => {
                return Err(missing("unqualified BuildingConstructed listener"))
            }
        }
    }
    let count = services
        .player_signal_listeners
        .iter()
        .find(|v| v.0 == owner)
        .ok_or(missing("current Player.BuildingConstructed registrations"))?
        .1;
    if count != 0 {
        return Err(missing("Player.BuildingConstructed consumers"));
    }
    Ok(())
}
struct RequirementContext<'a> {
    state: &'a LoadedState,
    requirements: &'a mut backend::requirements::Runtime,
    binding: Requirement,
}
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
            return Err(sets::Error::Context(missing(
                "building requirement identity",
            )));
        }
        callbacks::test_player_has_building(self.state, self.binding.owner, self.binding.building)
            .map_err(sets::Error::Context)
    }
    fn modifier_changed(
        &mut self,
        _: sets::SetListener,
        _: u64,
        _: i32,
        _: i32,
    ) -> Result<(), sets::Error<Self::Error>> {
        Err(sets::Error::Context(missing(
            "changed building modifier continuation",
        )))
    }
    fn destroy_requirement(&mut self, _: u64) -> Result<(), sets::Error<Self::Error>> {
        Err(sets::Error::Context(missing(
            "building requirement destruction",
        )))
    }
    fn destroy_set(&mut self, _: u64) -> Result<(), sets::Error<Self::Error>> {
        Err(sets::Error::Context(missing(
            "building requirement set destruction",
        )))
    }
    fn reset_trigger(&mut self, set: u64) -> Result<(), sets::Error<Self::Error>> {
        changes::reset_trigger_state(self.requirements.sets_mut(), set)
    }
}
