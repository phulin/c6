//! Rules.Players.OnPlotVisibilityChanged2bf1a80 on the persistent stock owner.
//! Geographic discovery retains names and every reached game-sync draw.
use super::{movement::Error, Runtime};
use crate::unit_sight_state::PlotVisibilityChanged;
use c6_core::{geographic_names as kernel, loaded_state::LoadedState, map::Component};

#[derive(Clone, Copy, Debug)]
pub enum UnitVolcanoSubscriber {
    /// StartupGame::$_49, OnUnitDiscoveredVolcano28fb070. Only the
    /// non-Toa early return is supported; the platform unlock path is explicit.
    Achievement,
    Unavailable {
        receiver: u64,
    },
}
pub struct Geography {
    definitions: crate::geographic_names::Definitions,
    civilizations: Vec<(i32, u32)>,
    ever_alive_major_civilizations: Vec<u32>,
    pub callbacks: Vec<PlotVisibilityChanged>,
    unit_volcano_subscribers: Option<Vec<UnitVolcanoSubscriber>>,
    toa_index: Option<u32>,
}
impl<P> Runtime<'_, P> {
    /// Current configuration civilizations and the actual WasEverAlive major
    /// player list. Neither is inferred from the currently alive list.
    pub fn bind_sight_geography(
        &mut self,
        state: &LoadedState,
        rules: &crate::Rules,
        civilizations: Vec<(i32, u32)>,
        ever_alive_major_players: &[i32],
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Missing("sight geography identity"))?;
        for (i, row) in civilizations.iter().enumerate() {
            if civilizations[..i].iter().any(|p| p.0 == row.0) {
                return Err(Error::Missing("ambiguous current civilization"));
            }
        }
        let mut ever_alive_major_civilizations = Vec::new();
        for player in ever_alive_major_players {
            ever_alive_major_civilizations.push(
                civilizations
                    .iter()
                    .find(|v| v.0 == *player)
                    .ok_or(Error::Missing("ever alive major civilization"))?
                    .1,
            );
        }
        let binding = self
            .resource_revelation
            .as_mut()
            .ok_or(Error::Missing("sight discovery services"))?;
        if binding.geography.is_some() {
            return Err(Error::Missing("sight geography already bound"));
        }
        binding.geography = Some(Geography {
            definitions: crate::geographic_names::Definitions::from_rules(rules)
                .map_err(|e| Error::Requirement(format!("{e:?}")))?,
            civilizations,
            ever_alive_major_civilizations,
            callbacks: Vec::new(),
            unit_volcano_subscribers: None,
            toa_index: crate::unit_initialization_definitions::collection(
                rules, "Units", "UnitType",
            )
            .map_err(|e| Error::Requirement(format!("{e:?}")))?
            .find_index(crate::registry::linux_type_hash(b"UNIT_MAORI_TOA"))
            .map(|v| v as u32),
        });
        Ok(())
    }
    /// Complete current GameSignals+1488 registration order. An empty ring
    /// permits dispatch without callbacks. The known achievement observer has
    /// a read-only preamble and returns before platform output for non-Toa
    /// Units. Unknown callbacks and its positive unlock path remain explicit.
    pub fn bind_sight_unit_volcano_discovery(
        &mut self,
        state: &LoadedState,
        subscribers: Vec<UnitVolcanoSubscriber>,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Missing("UnitVolcanoDiscovery identity"))?;
        let geography = self
            .resource_revelation
            .as_mut()
            .and_then(|v| v.geography.as_mut())
            .ok_or(Error::Missing("current sight geographic discovery"))?;
        if geography.unit_volcano_subscribers.is_some() {
            return Err(Error::Missing("UnitVolcanoDiscovery already bound"));
        }
        geography.unit_volcano_subscribers = Some(subscribers);
        Ok(())
    }
    pub fn sight_discovery_callbacks(&self) -> &[PlotVisibilityChanged] {
        self.resource_revelation
            .as_ref()
            .and_then(|v| v.geography.as_ref())
            .map_or(&[], |v| &v.callbacks)
    }
    pub(super) fn on_plot_visibility_changed(
        &mut self,
        state: &mut LoadedState,
        event: PlotVisibilityChanged,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Missing("sight discovery identity"))?;
        if !state
            .player_initialized(event.player)
            .map_err(Error::State)?
        {
            return Ok(());
        }
        let level = if let Some(services) = &self.player_services {
            services
                .player::<Error>(event.player)
                .map_err(|e| Error::Requirement(format!("{e:?}")))?
                .civilization_level
                .ok_or(Error::Missing("sight civilization level"))?
        } else {
            self.movement()?
                .civilization_levels
                .iter()
                .find(|v| v.0 == event.player)
                .ok_or(Error::Missing("sight civilization level"))?
                .1
        };
        if level != 0x253718b0 {
            return Ok(());
        }
        let plot = state
            .world()
            .map()
            .plot_by_index(event.plot)
            .ok_or(Error::Missing("sight discovery plot"))?;
        let river = plot.river_crossing_count > 0; // IsRiver20ded90/GetRiverCrossingCount2c929e0.
        let map_size = state.world().map().header.map_size_hash;
        let source = if event.source.component_type == 1 {
            state
                .world()
                .unit(c6_core::map::UnitReference::from_component_id(event.source))
                .map_err(Error::World)?
                .map(|v| v.record.e_type_index)
        } else {
            None
        };
        let geography = self
            .resource_revelation
            .as_mut()
            .and_then(|v| v.geography.as_mut())
            .ok_or(Error::Missing("current sight geographic discovery"))?;
        let civilization = geography
            .civilizations
            .iter()
            .find(|v| v.0 == event.player)
            .ok_or(Error::Missing("current sight civilization"))?
            .1;
        let definitions = &geography.definitions;
        let ever_alive = &geography.ever_alive_major_civilizations;
        let result = state
            .with_geographic_components(|components, draw| -> Result<(), Error> {
                let mut draw = |bound| draw(bound);
                let feature = components
                    .iter_mut()
                    .find_map(|v| {
                        if let Component::Feature(v) = v {
                            Some(v)
                        } else {
                            None
                        }
                    })
                    .ok_or(Error::Missing("current geographic features"))?;
                let volcano = feature
                    .volcanoes
                    .iter()
                    .any(|v| v.plot == event.plot as i32)
                    || feature
                        .natural_wonder_volcanoes
                        .iter()
                        .any(|v| v.plots.contains(&(event.plot as i32)));
                if volcano {
                    kernel::discover_volcano(
                        feature,
                        event.plot as i32,
                        civilization,
                        &definitions.volcanoes,
                        ever_alive,
                        &mut draw,
                    )
                    .map_err(|e| Error::Requirement(format!("{e:?}")))?;
                    if source.is_some() {
                        let subscribers = geography
                            .unit_volcano_subscribers
                            .as_ref()
                            .ok_or(Error::Missing("current UnitVolcanoDiscovery signal"))?;
                        for subscriber in subscribers {
                            match subscriber {
                                // 28fb070 eligibility and named-volcano getters
                                // are read-only. 28fb0f4..105 returns for a Unit
                                // whose current type differs from UNIT_MAORI_TOA.
                                UnitVolcanoSubscriber::Achievement
                                    if geography.toa_index != source => {}
                                UnitVolcanoSubscriber::Achievement => {
                                    return Err(Error::Missing(
                                        "UnitVolcanoDiscovery Toa achievement path",
                                    ));
                                }
                                UnitVolcanoSubscriber::Unavailable { .. } => {
                                    return Err(Error::Missing(
                                        "current UnitVolcanoDiscovery subscriber",
                                    ));
                                }
                            }
                        }
                    }
                }
                if river {
                    let manager = components
                        .iter_mut()
                        .find_map(|v| {
                            if let Component::River(v) = v {
                                Some(v)
                            } else {
                                None
                            }
                        })
                        .ok_or(Error::Missing("current geographic rivers"))?;
                    kernel::discover_river(
                        manager,
                        event.plot as i32,
                        civilization,
                        &definitions.rivers,
                        ever_alive,
                        &mut draw,
                    )
                    .map_err(|e| Error::Requirement(format!("{e:?}")))?;
                }
                let mut terrain = None;
                let mut territories = None;
                for component in components {
                    match component {
                        Component::Terrain(v) => terrain = Some(v),
                        Component::Territories {
                            container,
                            plot_territory_ids,
                        } => territories = Some((container, plot_territory_ids)),
                        _ => {}
                    }
                }
                let (container, ids) =
                    territories.ok_or(Error::Missing("current geographic territories"))?;
                let id = *ids
                    .get(event.plot as usize)
                    .ok_or(Error::Missing("current plot territory ID"))?;
                if id == u32::MAX {
                    return Ok(());
                }
                let territory = container
                    .entries
                    .iter_mut()
                    .find(|v| v.value.id as u32 == id)
                    .ok_or(Error::Missing("current plot territory"))?;
                if territory.value.name.base.is_empty() {
                    kernel::discover_territory(
                        terrain.ok_or(Error::Missing("current geographic terrain"))?,
                        &mut territory.value,
                        civilization,
                        &definitions.territories,
                        definitions.label_thresholds(map_size),
                        ever_alive,
                        &mut draw,
                    )
                    .map_err(|e| Error::Requirement(format!("{e:?}")))?;
                }
                Ok(())
            })
            .map_err(|e| Error::Requirement(format!("{e:?}")))?;
        result?;
        geography.callbacks.push(event);
        Ok(())
    }
}
