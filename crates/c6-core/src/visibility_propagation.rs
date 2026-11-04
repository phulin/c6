//! Linux hidden-unit visibility propagation over the actual layered World.
//!
//! This implements mask 0x80 only. Nonempty cells require real unit visibility
//! queries and synchronous reporting consumers, including their gameplay/RNG.
//! An empty cell emits no report. Its balanced block uses the visibility hash;
//! the stock host only changes block state for the distinct conflict hash.
use crate::{
    map::UnitReference,
    unit::ComponentId,
    visibility_change::Propagation,
    world::{self, World},
};

pub const VISIBILITY_BLOCK: u32 = 0x8272_89e6;
pub const UNIT_VISIBILITY_CHANGED: u32 = 0xc130_0ee7;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnitVisibilityChanged {
    pub unit: ComponentId,
    pub revealed_state: i32,
    pub origin: i32,
    pub sight_range: i32,
    pub source: ComponentId,
    pub player: i32,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Error<E> {
    Context(E),
    World(world::MissingInput),
    UnsupportedMask(u8),
    MissingVisibility(i32),
    NoncanonicalRevealed,
    Allocation,
}

/// Source all-layer emptiness, independent of local observer and player records.
/// Missing imported Unit records never prove a nonempty grid cell empty.
pub fn hidden_propagation_is_empty(world: &World, plot: u32) -> Result<bool, world::MissingInput> {
    if plot as usize >= world.map().plots.len() {
        return Err(world::MissingInput::InvalidCell);
    }
    for layer in &world.map().units.layers {
        if plot < layer.maximum && !world.plot_units(layer.layer_id, plot as usize)?.is_empty() {
            return Ok(false);
        }
    }
    Ok(true)
}

pub trait Context {
    type Error;
    fn world(&self) -> &World;
    /// Includes actual reporting registry/consumer availability and stable unit
    /// identity/membership lifetimes while the native iterable is active.
    fn preflight_hidden(&self, event: Propagation) -> Result<(), Self::Error>;
    /// Actual Core reporting block stack. Stock host visibility Begin/EndBlock
    /// has no extra effect; arbitrary EngineUtility implementations unsupported.
    fn reporting_blocks(&mut self) -> Result<&mut Vec<u32>, Self::Error>;
    /// Source IsVisible(Unit*) using current shared counts and actual unit rules.
    fn unit_visible(&mut self, player: i32, unit: UnitReference) -> Result<bool, Self::Error>;
    /// Full ordered reporting Send/CacheDispatcher/EngineUtility consumers.
    fn unit_visibility_changed(&mut self, event: UnitVisibilityChanged) -> Result<(), Self::Error>;
}

fn revealed<C: Context>(c: &C, player: i32, unit: UnitReference) -> Result<bool, Error<C::Error>> {
    let world = c.world();
    let found = world
        .unit(unit)
        .map_err(Error::World)?
        .ok_or(Error::World(world::MissingInput::InvalidSlot))?;
    let p = found.record.k_location;
    if p.x <= -9999 || p.y <= -9999 {
        return Ok(false);
    }
    let index = (p.y as u32)
        .wrapping_mul(world.map().header.width as u32)
        .wrapping_add(p.x as u32) as usize;
    let mapped = world
        .player_mapping
        .as_ref()
        .and_then(|m| usize::try_from(player).ok().and_then(|p| m.get(p)))
        .and_then(|i| usize::try_from(*i).ok())
        .ok_or(Error::MissingVisibility(player))?;
    let rows = &world.map().visibility;
    let visibility = rows
        .get(mapped)
        .filter(|r| r.player == player && r.visibility.player_id == player)
        .ok_or(Error::MissingVisibility(player))?;
    if rows.iter().filter(|r| r.player == player).count() != 1 {
        return Err(Error::MissingVisibility(player));
    }
    match visibility.visibility.revealed.get(index) {
        Some(1) => Ok(true),
        Some(0) | None => Ok(false),
        Some(_) => Err(Error::NoncanonicalRevealed),
    }
}

/// PropagateVisibilityChanges' hidden branch, retaining native layer and per-cell
/// order and original ComponentID bits even when lookup resolves another slot
/// generation. Other masks reject before any mutation. Error retains prefix.
pub fn propagate_hidden<C: Context>(c: &mut C, event: Propagation) -> Result<(), Error<C::Error>> {
    if event.mask != 0x80 {
        return Err(Error::UnsupportedMask(event.mask));
    }
    if hidden_propagation_is_empty(c.world(), event.plot).map_err(Error::World)? {
        return Ok(());
    }
    c.preflight_hidden(event).map_err(Error::Context)?;
    let blocks = c.reporting_blocks().map_err(Error::Context)?;
    blocks.try_reserve(1).map_err(|_| Error::Allocation)?;
    blocks.push(VISIBILITY_BLOCK);
    let mut ordinal = 0;
    while ordinal < c.world().map().units.layers.len() {
        let layer = &c.world().map().units.layers[ordinal];
        let units = if event.plot < layer.maximum {
            c.world()
                .plot_units(layer.layer_id, event.plot as usize)
                .map_err(Error::World)?
                .to_vec()
        } else {
            vec![]
        };
        for unit in units {
            if c.world().unit(unit).map_err(Error::World)?.is_none() {
                continue;
            }
            let state = if revealed(c, event.player, unit)? {
                1 + i32::from(c.unit_visible(event.player, unit).map_err(Error::Context)?)
            } else {
                0
            };
            c.unit_visibility_changed(UnitVisibilityChanged {
                unit: unit.component_id(),
                revealed_state: state,
                origin: event.origin,
                sight_range: event.sight_range,
                source: event.source,
                player: event.player,
            })
            .map_err(Error::Context)?;
        }
        ordinal += 1;
    }
    // Native EndBlock pops the last item whenever nonempty; it does not search
    // for a matching ID. Reentrant reporting must maintain its own balance.
    c.reporting_blocks().map_err(Error::Context)?.pop();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        map::{PlayerVisibility, Slot, UnitCell, UnitLayer, Visibility},
        unit::PlotCoord,
        world::{CompleteSlots, Player},
    };
    struct Fixture {
        world: World,
        blocks: Vec<u32>,
        events: Vec<UnitVisibilityChanged>,
        visible_calls: usize,
        hide_after_first: bool,
        effects_available: bool,
    }
    fn fixture(occupied: bool) -> Fixture {
        let mut map = crate::world::tests::map();
        let id = |n| {
            UnitReference::from_component_id(ComponentId {
                owner: 0,
                component_type: 9,
                id: n,
            })
        };
        map.units.layers = vec![
            UnitLayer {
                layer_id: 7,
                maximum: 2,
                cells: if occupied {
                    vec![UnitCell {
                        plot_index: 0,
                        units: vec![id(1), id(2)],
                    }]
                } else {
                    vec![]
                },
            },
            UnitLayer {
                layer_id: 2,
                maximum: 2,
                cells: if occupied {
                    vec![UnitCell {
                        plot_index: 0,
                        units: vec![id(1)],
                    }]
                } else {
                    vec![]
                },
            },
        ];
        map.visibility = vec![PlayerVisibility {
            player: 0,
            visibility: Visibility {
                player_id: 0,
                cached_num_revealed_hexes: 1,
                cached_old: false,
                revealed: vec![1, 0],
                layers: vec![],
                outgoing_visibility: vec![],
                outgoing_visibility_blockers: vec![],
            },
        }];
        let units = (1..=2)
            .map(|id| {
                let mut u = crate::unit_turn_upkeep::tests::source_only_unit();
                u.id = id;
                u.e_owner = 0;
                u.k_location = PlotCoord { x: 0, y: 0 };
                Slot {
                    index: id as i32,
                    value: u,
                }
            })
            .collect();
        let player =
            Player::from_lookup_slots(0, Some(CompleteSlots { entries: units }), None).unwrap();
        let mut world = World::new(map, vec![player]).unwrap();
        let mut mapping = [-1; 65];
        mapping[0] = 0;
        world.player_mapping = Some(mapping);
        Fixture {
            world,
            blocks: vec![123],
            events: vec![],
            visible_calls: 0,
            hide_after_first: false,
            effects_available: true,
        }
    }
    fn event() -> Propagation {
        Propagation {
            player: 0,
            plot: 0,
            revealed_state: 2,
            mask: 128,
            first_reveal: false,
            origin: 77,
            sight_range: 2,
            source: ComponentId {
                owner: 0,
                component_type: 1,
                id: 99,
            },
        }
    }
    impl Context for Fixture {
        type Error = &'static str;
        fn world(&self) -> &World {
            &self.world
        }
        fn preflight_hidden(&self, _: Propagation) -> Result<(), Self::Error> {
            if self.effects_available {
                Ok(())
            } else {
                Err("unknown reporting registry")
            }
        }
        fn reporting_blocks(&mut self) -> Result<&mut Vec<u32>, Self::Error> {
            Ok(&mut self.blocks)
        }
        fn unit_visible(&mut self, _: i32, _: UnitReference) -> Result<bool, Self::Error> {
            self.visible_calls += 1;
            Ok(true)
        }
        fn unit_visibility_changed(
            &mut self,
            event: UnitVisibilityChanged,
        ) -> Result<(), Self::Error> {
            assert_eq!(self.blocks.last(), Some(&VISIBILITY_BLOCK));
            self.events.push(event);
            if self.hide_after_first {
                self.world.visibility_mut(0).unwrap().revealed[0] = 0;
            }
            Ok(())
        }
    }
    #[test]
    fn native_layer_order_identity_bits_and_callback_rereads() {
        let mut f = fixture(true);
        f.hide_after_first = true;
        propagate_hidden(&mut f, event()).unwrap();
        assert_eq!(
            f.events
                .iter()
                .map(|e| (e.unit.id, e.unit.component_type, e.revealed_state))
                .collect::<Vec<_>>(),
            [(1, 9, 2), (2, 9, 0), (1, 9, 0)]
        );
        assert!(f
            .events
            .iter()
            .all(|e| e.origin == 77 && e.source.id == 99 && e.player == 0));
        assert_eq!(f.visible_calls, 1);
        assert_eq!(f.blocks, [123]);
    }
    #[test]
    fn nonempty_unknown_unit_collection_cannot_prove_empty() {
        let f = fixture(true);
        assert!(!hidden_propagation_is_empty(&f.world, 0).unwrap());
        let mut map = f.world.map().clone();
        map.units.layers[0].maximum = 0;
        map.units.layers[1].maximum = 0;
        let world = World::new(map, vec![]).unwrap();
        assert!(hidden_propagation_is_empty(&world, 0).unwrap());
        assert!(hidden_propagation_is_empty(&world, 2).is_err());
    }
    #[test]
    fn empty_branch_needs_no_observer_unit_records_or_reporting_registry() {
        let mut f = fixture(false);
        f.effects_available = false;
        let mut map = f.world.map().clone();
        map.visibility.clear();
        f.world = World::new(map, vec![]).unwrap();
        propagate_hidden(&mut f, event()).unwrap();
        assert!(f.events.is_empty());
        assert_eq!(f.blocks, [123]);
        assert_eq!(f.visible_calls, 0);
        assert_eq!(
            propagate_hidden(
                &mut f,
                Propagation {
                    mask: 255,
                    ..event()
                }
            ),
            Err(Error::UnsupportedMask(255))
        );
        assert_eq!(f.blocks, [123]);
    }
    #[test]
    fn reached_unknown_reporting_rejects_before_block_or_event_writes() {
        let mut f = fixture(true);
        f.effects_available = false;
        assert_eq!(
            propagate_hidden(&mut f, event()),
            Err(Error::Context("unknown reporting registry"))
        );
        assert_eq!(f.blocks, [123]);
        assert!(f.events.is_empty());
    }
}
