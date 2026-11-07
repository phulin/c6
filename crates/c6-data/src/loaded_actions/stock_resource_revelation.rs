//! Linux Map.Resource.UpdateRevealedAt (2d0c230). The canonical layer write
//! precedes Game.ResourceRevealed, including NO_RESOURCE dispatches.
use super::{movement::Error, Runtime};
use crate::{
    city_turn::resources, registry::LinuxDefinitionRegistry,
    unit_initialization_definitions::collection,
};
use c6_core::{
    loaded_state::LoadedState, map::VisibilityLayer, visibility_change::BASE_LAYER_HASH,
};

/// Semantic presentation payload after gameplay mutation and synchronous signal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Report {
    pub event_hash: u32,
    pub player: i32,
    pub plot: u32,
    pub resource: i32,
    pub revealed_state: i32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnitPlotSubscriber {
    Targets { owner: i32 },
    Barbarian { owner: i32 },
    Unavailable { receiver: u64 },
}
#[derive(Clone, Copy, Debug)]
pub enum ContinentSubscriber {
    EraScore,
    History,
    Boost,
}
#[derive(Clone, Debug)]
pub(super) struct ResourceBoostCheck {
    receiver: u64,
    resource: Option<i32>,
    compatible_improvements: Vec<u32>,
    boost_index: usize,
    improvement_reference: bool,
    boost_value: i32,
}
pub struct State {
    pub(super) definitions: resources::Definitions,
    resources: LinuxDefinitionRegistry,
    layer: usize,
    subscribers: Vec<u64>,
    resource_boost_checks: Option<Vec<ResourceBoostCheck>>,
    pub(super) continent_subscribers: Option<Vec<ContinentSubscriber>>,
    pub(super) unit_continent_empty: bool,
    pub(super) wonders: Option<super::wonder_discovery::Bindings>,
    pub dispatches: Vec<(i32, i32)>,
    pub reports: Vec<Report>,
    pub(super) natural_wonders: Vec<u32>,
    pub(super) continents: LinuxDefinitionRegistry,
    pub(super) goody_notify: Vec<u32>,
    pub(super) geography: Option<super::sight_discovery::Geography>,
    pub propagation: Vec<c6_core::visibility_change::Propagation>,
    pub visibility_blocks: Vec<u32>,
    pub unit_visibility: Vec<c6_core::visibility_propagation::UnitVisibilityChanged>,
    unit_plot_subscribers: Option<Vec<UnitPlotSubscriber>>,
    pub unit_plot_dispatches: Vec<((i32, u32), u32)>,
}
impl<P> Runtime<'_, P> {
    /// Current native GetRevealedResourceType layer, converted to wire identity.
    pub(crate) fn improvement_revealed_resource(
        &self,
        state: &LoadedState,
        owner: i32,
        plot: usize,
    ) -> Result<u32, &'static str> {
        let binding = self
            .resource_revelation
            .as_ref()
            .ok_or("Farm revealed resource binding")?;
        let layer = state
            .world()
            .visibility(owner)
            .and_then(|v| v.layers.get(binding.layer))
            .ok_or("Farm revealed resource layer")?;
        let index = *layer
            .values
            .get(plot)
            .ok_or("Farm revealed resource cell")?;
        if index == -1 {
            return Ok(u32::MAX);
        }
        let definition = binding
            .resources
            .entries()
            .get(usize::try_from(index).map_err(|_| "Farm revealed resource index")?)
            .ok_or("Farm revealed resource definition")?;
        Ok(crate::registry::linux_type_hash(&definition.type_name))
    }

    /// `subscribers` is the complete current GameSignals+12c0 registration
    /// order. Unknown nonempty routes remain explicit after the layer write.
    pub fn bind_resource_revelation(
        &mut self,
        state: &LoadedState,
        rules: &crate::Rules,
        layer: u32,
        subscribers: Vec<u64>,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Missing("resource revelation identity"))?;
        if self.resource_revelation.is_some() {
            return Err(Error::Missing("resource revelation already bound"));
        }
        let mut natural_wonders = Vec::new();
        for row in rules
            .gameplay()
            .table("Features")
            .ok_or(Error::Missing("Features"))?
            .rows()
        {
            if crate::game_parameters::sql_integer(
                row.get("NaturalWonder")
                    .map_err(|e| Error::Requirement(format!("{e:?}")))?,
            )
            .ok_or(Error::Missing("Feature NaturalWonder"))?
                != 0
            {
                natural_wonders.push(crate::registry::linux_type_hash(
                    row.text("FeatureType")
                        .map_err(|e| Error::Requirement(format!("{e:?}")))?
                        .as_bytes(),
                ));
            }
        }
        let mut goody_notify = Vec::new();
        for row in rules
            .gameplay()
            .table("Improvements")
            .ok_or(Error::Missing("Improvements"))?
            .rows()
        {
            let flag = |key| -> Result<bool, Error> {
                Ok(crate::game_parameters::sql_integer(
                    row.get(key)
                        .map_err(|e| Error::Requirement(format!("{e:?}")))?,
                )
                .ok_or(Error::Missing("goody discovery definition flag"))?
                    != 0)
            };
            if flag("Goody")? && flag("GoodyNotify")? {
                goody_notify.push(crate::registry::linux_type_hash(
                    row.text("ImprovementType")
                        .map_err(|e| Error::Requirement(format!("{e:?}")))?
                        .as_bytes(),
                ));
            }
        }
        self.resource_revelation = Some(State {
            definitions: resources::Definitions::from_rules(rules)
                .map_err(|e| Error::Requirement(format!("{e:?}")))?,
            resources: collection(rules, "Resources", "ResourceType")
                .map_err(|e| Error::Requirement(format!("{e:?}")))?,
            layer: layer as usize,
            subscribers,
            dispatches: Vec::new(),
            reports: Vec::new(),
            natural_wonders,
            continents: collection(rules, "Continents", "ContinentType")
                .map_err(|e| Error::Requirement(format!("{e:?}")))?,
            goody_notify,
            geography: None,
            propagation: Vec::new(),
            visibility_blocks: Vec::new(),
            unit_visibility: Vec::new(),
            resource_boost_checks: None,
            continent_subscribers: None,
            unit_continent_empty: false,
            wonders: None,
            unit_plot_subscribers: None,
            unit_plot_dispatches: Vec::new(),
        });
        Ok(())
    }
    /// Complete current GameSignals+10c8 order, supplied from the same initial
    /// runtime. Unknown nonempty consumers fail at their actual dispatch.
    /// Bind source PlayerResourceCheck2844a40 consumers in the already
    /// observed ResourceRevealed order. Parameters are immutable rule references,
    /// not captured predicate answers or a suppressed signal.
    pub fn bind_resource_boost_checks(
        &mut self,
        state: &LoadedState,
        rules: &crate::Rules,
        subscribers: &[(u64, i64)],
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Missing("resource boost identity"))?;
        let binding = self
            .resource_revelation
            .as_mut()
            .ok_or(Error::Missing("resource revelation services"))?;
        if binding.resource_boost_checks.is_some()
            || subscribers.len() != binding.subscribers.len()
            || subscribers
                .iter()
                .zip(&binding.subscribers)
                .any(|(a, b)| a.0 != *b)
        {
            return Err(Error::Missing("current resource boost registration order"));
        }
        let checks = subscribers
            .iter()
            .map(|&(receiver, row)| resource_boost_check(rules, receiver, row))
            .collect::<Result<Vec<_>, _>>()?;
        binding.resource_boost_checks = Some(checks);
        Ok(())
    }
    /// Actual ordered GameSignals+660 and complete empty +690 registry.
    pub fn bind_continent_discovery_signal(
        &mut self,
        state: &LoadedState,
        subscribers: Vec<ContinentSubscriber>,
        unit_continent_empty: bool,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Missing("continent signal identity"))?;
        let binding = self
            .resource_revelation
            .as_mut()
            .ok_or(Error::Missing("resource revelation services"))?;
        if binding.continent_subscribers.is_some() {
            return Err(Error::Missing("continent signal already bound"));
        }
        binding.continent_subscribers = Some(subscribers);
        binding.unit_continent_empty = unit_continent_empty;
        Ok(())
    }
    pub fn bind_unit_plot_visibility_signal(
        &mut self,
        state: &LoadedState,
        subscribers: Vec<UnitPlotSubscriber>,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Missing("unit plot signal identity"))?;
        let current = self
            .resource_revelation
            .as_mut()
            .ok_or(Error::Missing("sight discovery services"))?;
        if current.unit_plot_subscribers.is_some() {
            return Err(Error::Missing("unit plot signal already bound"));
        }
        current.unit_plot_subscribers = Some(subscribers);
        Ok(())
    }
    pub fn unit_plot_visibility_dispatches(&self) -> &[((i32, u32), u32)] {
        self.resource_revelation
            .as_ref()
            .map_or(&[], |v| &v.unit_plot_dispatches)
    }
    pub(super) fn dispatch_unit_plot_visibility(
        &mut self,
        state: &LoadedState,
        actor: (i32, u32),
        plot: u32,
    ) -> Result<(), Error> {
        self.check(state)
            .map_err(|_| Error::Missing("unit plot signal identity"))?;
        let unit = state
            .world()
            .player(actor.0)
            .map_err(Error::World)?
            .unit(actor.1)
            .map_err(Error::World)?
            .ok_or(Error::Missing("unit plot signal actor"))?
            .record;
        let actor_owner = unit.e_owner;
        let tribe = unit.i_barbarian_tribe_index;
        state
            .world()
            .map()
            .plot_by_index(plot)
            .ok_or(Error::Missing("unit plot signal plot"))?;
        let current = self
            .resource_revelation
            .as_mut()
            .ok_or(Error::Missing("sight discovery services"))?;
        let subscribers = current
            .unit_plot_subscribers
            .as_ref()
            .ok_or(Error::Missing("current unit plot signal registry"))?;
        current.unit_plot_dispatches.push((actor, plot));
        for route in subscribers {
            match *route {
                UnitPlotSubscriber::Targets { owner } => {
                    if owner != actor_owner {
                        continue;
                    }
                    let attrs = state
                        .world()
                        .map()
                        .plot_by_index(plot)
                        .ok_or(Error::Missing("unit plot signal plot"))?
                        .city_attributes
                        .as_ref();
                    if attrs.is_some_and(|v| v.district != u32::MAX) {
                        return Err(Error::Missing(
                            "current Targets.UpdateCity knowledge continuation",
                        ));
                    }
                }
                UnitPlotSubscriber::Barbarian { owner } => {
                    if owner != actor_owner {
                        continue;
                    }
                    if tribe < 0
                        || state
                            .world()
                            .map()
                            .plot_by_index(plot)
                            .ok_or(Error::Missing("unit plot signal plot"))?
                            .city_attributes
                            .as_ref()
                            .map_or(-1, |v| i32::from(v.player))
                            == -1
                    {
                        continue;
                    }
                    return Err(Error::Missing(
                        "current Barbarian.OnCitySpotted continuation",
                    ));
                }
                UnitPlotSubscriber::Unavailable { receiver } => {
                    return Err(Error::Requirement(format!(
                        "UnitPlotVisibility subscriber {receiver}"
                    )));
                }
            }
        }
        Ok(())
    }
    pub fn resource_revelation_dispatches(&self) -> &[(i32, i32)] {
        self.resource_revelation
            .as_ref()
            .map_or(&[], |v| &v.dispatches)
    }
    pub fn resource_revelation_reports(&self) -> &[Report] {
        self.resource_revelation
            .as_ref()
            .map_or(&[], |v| &v.reports)
    }
    pub fn sight_propagation_reports(&self) -> &[c6_core::visibility_change::Propagation] {
        self.resource_revelation
            .as_ref()
            .map_or(&[], |v| &v.propagation)
    }
    /// The stock headless boundary retains semantic propagation, not display
    /// object/cache enumeration. The source's natural-wonder completion remains
    /// a gameplay continuation and cannot be omitted with those reports.
    pub(super) fn propagate_sight(
        &mut self,
        state: &mut LoadedState,
        event: c6_core::visibility_change::Propagation,
    ) -> Result<(), Error> {
        let plot = state
            .world()
            .map()
            .plot_by_index(event.plot)
            .ok_or(Error::Missing("sight propagation plot"))?;
        let feature = plot.feature_hash;
        let binding = self
            .resource_revelation
            .as_mut()
            .ok_or(Error::Missing("sight propagation services"))?;
        // BeginBlock and the mask1 terrain report precede the feature branch.
        binding.propagation.push(event);
        if wonder_completion_gate(event, binding.natural_wonders.contains(&feature)) {
            let feature = self
                .movement()?
                .definitions
                .features
                .find_index(feature)
                .ok_or(Error::Missing("sight natural wonder definition"))?
                as i32;
            let revealed = crate::unit_sight_state::revealed_feature::get(
                state,
                self.movement()?.definitions.features,
                event.player,
                feature,
            )
            .map_err(|e| Error::Requirement(format!("{e:?}")))?;
            if !revealed {
                // CompleteMovement(ComponentID) rejects invalid/non-Unit IDs
                // and a missing current Unit, then delegates to the same live
                // completion flag/report path used by ordinary movement.
                let source = event.source;
                if source.owner != -1
                    && source.id != u32::MAX
                    && source.component_type == 1
                    && state
                        .world()
                        .player(i32::from(source.owner))
                        .map_err(Error::World)?
                        .unit(source.id)
                        .map_err(Error::World)?
                        .is_some()
                {
                    crate::loaded_upkeep::complete_unit_movement(
                        state,
                        i32::from(source.owner),
                        source.id,
                        self,
                    )
                    .map_err(|e| {
                        Error::Requirement(format!("wonder movement completion: {e:?}"))
                    })?;
                }
                self.report_wonder_revealed(state, event, feature)?;
            }
        }
        Ok(())
    }
    pub(super) fn sight_visibility_blocks(&mut self) -> Result<&mut Vec<u32>, Error> {
        Ok(&mut self
            .resource_revelation
            .as_mut()
            .ok_or(Error::Missing("sight propagation services"))?
            .visibility_blocks)
    }
    pub(super) fn sight_unit_visibility(
        &mut self,
        event: c6_core::visibility_propagation::UnitVisibilityChanged,
    ) -> Result<(), Error> {
        self.resource_revelation
            .as_mut()
            .ok_or(Error::Missing("sight propagation services"))?
            .unit_visibility
            .push(event);
        Ok(())
    }
    pub(super) fn update_revealed_resource(
        &mut self,
        state: &mut LoadedState,
        sight: &crate::unit_sight_state::Runtime,
        player: i32,
        index: u32,
    ) -> Result<(), Error> {
        self.update_revealed_resource_with(state, sight, player, index, |_, _, _, _| {
            Err(Error::Missing(
                "positive PlayerResourceCheck TriggerBoost continuation",
            ))
        })
    }
    pub(super) fn update_revealed_resource_with(
        &mut self,
        state: &mut LoadedState,
        sight: &crate::unit_sight_state::Runtime,
        player: i32,
        index: u32,
        mut trigger: impl FnMut(&mut Self, &mut LoadedState, i32, usize) -> Result<(), Error>,
    ) -> Result<(), Error> {
        if player == -1 || (player != 1000 && !state.player_alive(player).map_err(Error::State)?) {
            return Ok(());
        }
        let own = state
            .world()
            .visibility(player)
            .ok_or(Error::Missing("resource reveal visibility"))?;
        let normal = sight
            .shared(player)
            .map_err(Error::Missing)?
            .0
            .iter()
            .find(|v| v.type_hash == BASE_LAYER_HASH)
            .ok_or(Error::Missing("resource reveal normal layer"))?;
        let visible = *normal
            .values
            .get(index as usize)
            .ok_or(Error::Missing("resource reveal normal cell"))?
            > 0;
        let revealed = *own
            .revealed
            .get(index as usize)
            .ok_or(Error::Missing("resource reveal revealed cell"))?;
        if revealed > 1 {
            return Err(Error::Missing("resource reveal noncanonical byte"));
        }
        let revealed_state = if visible { 2 } else { i32::from(revealed) };
        if revealed_state == 0 {
            return Ok(());
        }
        let binding = self
            .resource_revelation
            .as_mut()
            .ok_or(Error::Missing("resource revelation services"))?;
        let plot = state
            .world()
            .map()
            .plot_by_index(index)
            .ok_or(Error::Missing("resource reveal plot"))?;
        let hash = plot.resource_hash;
        let actual = binding.resources.find_index(hash).map_or(-1, |v| v as i32);
        let resource = if player == 1000 {
            actual
        } else {
            let visible_hash = binding
                .definitions
                .visible_resource(state, player, hash as i32)
                .map_err(|e| Error::Requirement(format!("{e:?}")))?;
            if visible_hash == -1 {
                -1
            } else {
                actual
            }
        };
        let layer = state
            .world_mut()
            .map_err(Error::State)?
            .visibility_mut(player)
            .ok_or(Error::Missing("resource reveal visibility"))?
            .layers
            .get_mut(binding.layer);
        let changed = match layer {
            Some(layer) => set_entry(layer, index, resource)?,
            None => false, // Source EditLayerIndex returns null for absent ordinal.
        };
        if player != 1000 {
            binding.dispatches.push((player, actual));
            let checks = binding.subscribers.iter().enumerate().map(|(position, receiver)| {
                binding.resource_boost_checks.as_ref().and_then(|checks| checks.get(position))
                    .filter(|check| check.receiver == *receiver).cloned()
                    .ok_or_else(|| Error::Requirement(format!("ResourceRevealed subscriber {receiver} (player {player}, resource {actual})")))
            }).collect::<Result<Vec<_>, _>>()?;
            for check in checks {
                for boost in resource_boost_matches(state, &check, player, actual, index)? {
                    trigger(self, state, player, boost)?;
                }
            }
        }
        if sight
            .local_observer
            .ok_or(Error::Missing("resource reveal local observer"))?
            == player
            && !sight.messages_disabled
            && changed
        {
            for event_hash in report_events(actual) {
                self.resource_revelation
                    .as_mut()
                    .unwrap()
                    .reports
                    .push(Report {
                        event_hash: *event_hash,
                        player,
                        plot: index,
                        resource: actual,
                        revealed_state,
                    });
            }
        }
        Ok(())
    }
}
/// Immutable source references for Boost.Manager.PlayerResourceCheck2844a40.
pub(super) fn resource_boost_check(
    rules: &crate::Rules,
    receiver: u64,
    row_id: i64,
) -> Result<ResourceBoostCheck, Error> {
    let resources = collection(rules, "Resources", "ResourceType")
        .map_err(|e| Error::Requirement(format!("{e:?}")))?;
    let improvements = collection(rules, "Improvements", "ImprovementType")
        .map_err(|e| Error::Requirement(format!("{e:?}")))?;
    let boosts = rules
        .gameplay()
        .table("Boosts")
        .ok_or(Error::Missing("Boosts"))?;
    let (boost_index, row) = boosts
        .rows()
        .enumerate()
        .find(|(_, row)| row.row_id() == Some(crate::RulesRowId(row_id)))
        .ok_or(Error::Missing("resource boost definition"))?;
    let reference = |column| -> Result<Option<u32>, Error> {
        match row
            .get(column)
            .map_err(|e| Error::Requirement(format!("{e:?}")))?
        {
            crate::SqlValue::Null => Ok(None),
            crate::SqlValue::Text(name) => Ok(Some(crate::registry::linux_type_hash(name))),
            _ => Err(Error::Missing("resource boost reference")),
        }
    };
    let resource = reference("ResourceType")?
        .and_then(|hash| resources.find_index(hash))
        .map(|v| v as i32);
    let mut compatible_improvements = Vec::new();
    if let Some(resource) = resource {
        let name = &resources.entries()[resource as usize].type_name;
        for row in rules
            .gameplay()
            .table("Improvement_ValidResources")
            .ok_or(Error::Missing("Improvement_ValidResources"))?
            .rows()
        {
            if row
                .text("ResourceType")
                .map_err(|e| Error::Requirement(format!("{e:?}")))?
                .as_bytes()
                == name
            {
                let hash = crate::registry::linux_type_hash(
                    row.text("ImprovementType")
                        .map_err(|e| Error::Requirement(format!("{e:?}")))?
                        .as_bytes(),
                );
                if improvements.find_index(hash).is_some() {
                    compatible_improvements.push(hash);
                }
            }
        }
    }
    let class = row
        .text("BoostClass")
        .map_err(|e| Error::Requirement(format!("{e:?}")))?;
    let name = rules
        .gameplay()
        .table("BoostNames")
        .ok_or(Error::Missing("BoostNames"))?
        .rows()
        .find(|r| r.text("BoostType").ok() == Some(class))
        .ok_or(Error::Missing("resource boost name"))?;
    Ok(ResourceBoostCheck {
        receiver,
        resource,
        compatible_improvements,
        boost_index,
        improvement_reference: reference("ImprovementType")?
            .is_some_and(|hash| improvements.find_index(hash).is_some()),
        boost_value: i32::try_from(
            name.integer("BoostValue")
                .map_err(|e| Error::Requirement(format!("{e:?}")))?,
        )
        .map_err(|_| Error::Missing("resource boost value"))?,
    })
}
pub(super) fn resource_boost_matches(
    state: &LoadedState,
    check: &ResourceBoostCheck,
    player: i32,
    actual: i32,
    index: u32,
) -> Result<Vec<usize>, Error> {
    if check.resource.is_none() || !check.improvement_reference || check.boost_value != 11 {
        return Ok(Vec::new());
    }
    let plot = state
        .world()
        .map()
        .plot_by_index(index)
        .ok_or(Error::Missing("resource boost plot"))?;
    if crate::movement_placement_state::plot_owner(plot).map_err(Error::Placement)? != player
        || check.resource != Some(actual)
    {
        return Ok(Vec::new());
    }
    // Native iterates the resource's ordered improvement references. The boost's
    // ImprovementType is only a presence guard; it does not select this list.
    Ok(check
        .compatible_improvements
        .iter()
        .filter(|&&hash| {
            hash == plot.improvement_hash
                && !plot.flags.get(c6_core::map::PlotFlag::ImprovementPillaged)
        })
        .map(|_| check.boost_index)
        .collect())
}
fn wonder_completion_gate(event: c6_core::visibility_change::Propagation, wonder: bool) -> bool {
    event.mask & 2 != 0 && event.revealed_state != 0 && event.first_reveal && wonder
}
fn report_events(resource: i32) -> &'static [u32] {
    if resource == -1 {
        &[0x265857f7]
    } else {
        &[0x195088c7, 0x9aea8529]
    }
}
fn set_entry(layer: &mut VisibilityLayer, plot: u32, resource: i32) -> Result<bool, Error> {
    if layer.values.len() != layer.count as usize {
        return Err(Error::Missing("resource reveal layer extent"));
    }
    let Some(cell) = layer.values.get_mut(plot as usize) else {
        return Ok(false);
    };
    if i32::from(*cell) == resource {
        return Ok(false);
    }
    *cell = resource as i16;
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wonder_completion_requires_feature_mask_and_new_nonhidden_discovery() {
        let mut event = c6_core::visibility_change::Propagation {
            player: 0,
            plot: 0,
            revealed_state: 2,
            mask: 0xff,
            first_reveal: true,
            origin: 0,
            sight_range: 2,
            source: c6_core::unit::ComponentId {
                owner: 0,
                component_type: 1,
                id: 7,
            },
        };
        assert!(wonder_completion_gate(event, true));
        assert!(!wonder_completion_gate(event, false));
        event.mask = 0x80;
        assert!(!wonder_completion_gate(event, true));
        event.mask = 2;
        event.revealed_state = 0;
        assert!(!wonder_completion_gate(event, true));
        event.revealed_state = 1;
        event.first_reveal = false;
        assert!(!wonder_completion_gate(event, true));
    }
    #[test]
    fn resource_report_order_and_removal_are_distinct() {
        assert_eq!(report_events(0), &[0x195088c7, 0x9aea8529]);
        assert_eq!(report_events(-1), &[0x265857f7]);
    }
    #[test]
    fn signed_cell_comparison_precedes_narrowing() {
        let mut layer = VisibilityLayer {
            type_hash: 0,
            index: 2,
            count: 1,
            grid_width: 1,
            values: vec![-1],
        };
        assert!(!set_entry(&mut layer, 0, -1).unwrap());
        assert!(set_entry(&mut layer, 0, 65535).unwrap());
        assert_eq!(layer.values, vec![-1]);
        assert!(set_entry(&mut layer, 0, 4).unwrap());
        assert_eq!(layer.values, vec![4]);
        assert!(!set_entry(&mut layer, 1, 5).unwrap());
        assert_eq!(layer.values, vec![4]);
    }
}
