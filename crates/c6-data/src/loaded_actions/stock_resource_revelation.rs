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
pub struct State {
    definitions: resources::Definitions,
    resources: LinuxDefinitionRegistry,
    layer: usize,
    subscribers: Vec<u64>,
    pub dispatches: Vec<(i32, i32)>,
    pub reports: Vec<Report>,
}
impl<P> Runtime<'_, P> {
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
        self.resource_revelation = Some(State {
            definitions: resources::Definitions::from_rules(rules)
                .map_err(|e| Error::Requirement(format!("{e:?}")))?,
            resources: collection(rules, "Resources", "ResourceType")
                .map_err(|e| Error::Requirement(format!("{e:?}")))?,
            layer: layer as usize,
            subscribers,
            dispatches: Vec::new(),
            reports: Vec::new(),
        });
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
    pub(super) fn update_revealed_resource(
        &mut self,
        state: &mut LoadedState,
        sight: &crate::unit_sight_state::Runtime,
        player: i32,
        index: u32,
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
            if let Some(receiver) = binding.subscribers.first() {
                return Err(Error::Requirement(format!(
                    "ResourceRevealed subscriber {receiver} (player {player}, resource {actual})"
                )));
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
                binding.reports.push(Report {
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
