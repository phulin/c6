//! Source-verified Linux XP2 prerequisite owner projections.
//!
//! The bool-array codec requires an already located owner field. Global nodes
//! require a complete live export after native Read/Validate, not raw wire nodes.
//! This is not a complete Techs/Culture/Core codec or native parity acceptance.
use crate::movement_world::DefinitionLookup;
use crate::unit::{HashValue, Limits};
use crate::world::{Player, World};
use crate::yields::PrerequisiteKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Trailing,
    InvalidBool,
    ResourceLimit,
    DefinitionMapping,
    DestinationSize,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux prerequisite owner input: {self:?}")
    }
}
impl std::error::Error for Error {}

fn checked_size(count: usize, width: usize, limits: Limits) -> Result<usize, Error> {
    let size = count.checked_mul(width).ok_or(Error::ResourceLimit)?;
    if count > limits.max_items || size > limits.max_bytes {
        return Err(Error::ResourceLimit);
    }
    Ok(size)
}
fn storage<T>(count: usize, limits: Limits) -> Result<Vec<T>, Error> {
    checked_size(count, std::mem::size_of::<T>(), limits)?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| Error::ResourceLimit)?;
    Ok(values)
}
fn definition_count(lookup: &dyn DefinitionLookup, limits: Limits) -> Result<usize, Error> {
    let count = lookup.count();
    if count > i32::MAX as usize {
        return Err(Error::DefinitionMapping);
    }
    checked_size(count, 1, limits)?;
    Ok(count)
}

/// Original u32 count followed by ordered (u32 definition hash, one-byte bool)
/// pairs. Unknown hashes still consume a bool; later duplicate pairs overwrite.
/// Noncanonical bool bytes are refused as outside this typed support boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexedBools {
    pub pairs: Vec<HashValue<bool>>,
}
impl IndexedBools {
    pub fn decode_prefix(bytes: &[u8], limits: Limits) -> Result<(Self, usize), Error> {
        let count = u32::from_le_bytes(bytes.get(..4).ok_or(Error::Truncated)?.try_into().unwrap())
            as usize;
        let size = checked_size(count, 5, limits)?
            .checked_add(4)
            .ok_or(Error::ResourceLimit)?;
        if size > limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let body = bytes.get(4..size).ok_or(Error::Truncated)?;
        let mut pairs = storage(count, limits)?;
        for pair in body.as_chunks::<5>().0 {
            let value = match pair[4] {
                0 => false,
                1 => true,
                _ => return Err(Error::InvalidBool),
            };
            pairs.push(HashValue {
                definition_hash: u32::from_le_bytes(pair[..4].try_into().unwrap()),
                value,
            });
        }
        Ok((Self { pairs }, size))
    }
    pub fn decode(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        let (value, used) = Self::decode_prefix(bytes, limits)?;
        if used != bytes.len() {
            return Err(Error::Trailing);
        }
        Ok(value)
    }
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, Error> {
        let count = u32::try_from(self.pairs.len()).map_err(|_| Error::ResourceLimit)?;
        let size = checked_size(self.pairs.len(), 5, limits)?
            .checked_add(4)
            .ok_or(Error::ResourceLimit)?;
        if size > limits.max_bytes {
            return Err(Error::ResourceLimit);
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| Error::ResourceLimit)?;
        bytes.extend(count.to_le_bytes());
        for pair in &self.pairs {
            bytes.extend(pair.definition_hash.to_le_bytes());
            bytes.push(u8::from(pair.value));
        }
        Ok(bytes)
    }
    /// Generic ReadIndexHashArray overlay. Existing storage must be complete;
    /// the wire pair count never sizes the destination and never clears it.
    pub fn apply(
        &self,
        prior: &[bool],
        lookup: &dyn DefinitionLookup,
        limits: Limits,
    ) -> Result<Vec<bool>, Error> {
        let count = definition_count(lookup, limits)?;
        checked_size(
            self.pairs.len(),
            std::mem::size_of::<HashValue<bool>>(),
            limits,
        )?;
        if prior.len() != count {
            return Err(Error::DestinationSize);
        }
        let mut values = storage(count, limits)?;
        values.extend_from_slice(prior);
        for pair in &self.pairs {
            if let Some(index) = lookup.find_index(pair.definition_hash) {
                if index >= count {
                    return Err(Error::DefinitionMapping);
                }
                values[index] = pair.value;
            }
        }
        Ok(values)
    }
    /// Player.Techs and Player.Culture Read both call their concrete Reset,
    /// which clears and resizes these acquired vectors with explicit false.
    /// GetNumTechs/GetNumCivics obtain the actual definition collection count.
    pub fn acquired_after_owner_read(
        &self,
        kind: PrerequisiteKind,
        lookup: &dyn DefinitionLookup,
        limits: Limits,
    ) -> Result<AcquiredState, Error> {
        let count = definition_count(lookup, limits)?;
        let mut reset = storage(count, limits)?;
        reset.resize(count, false);
        Ok(AcquiredState {
            kind,
            values: self.apply(&reset, lookup, limits)?,
        })
    }
}

/// Complete acquired owner vector in actual current definition collection order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcquiredState {
    kind: PrerequisiteKind,
    values: Vec<bool>,
}
impl AcquiredState {
    /// A live export must include every byte of the actual acquired vector.
    /// No mask may be inferred from research queues, boosts, or global nodes.
    pub fn from_complete_live(
        kind: PrerequisiteKind,
        values: Vec<bool>,
        lookup: &dyn DefinitionLookup,
        limits: Limits,
    ) -> Result<Self, Error> {
        if values.len() != definition_count(lookup, limits)? {
            return Err(Error::DestinationSize);
        }
        Ok(Self { kind, values })
    }
    pub fn values(&self) -> &[bool] {
        &self.values
    }
    pub fn install(self, player: &mut Player) {
        match self.kind {
            PrerequisiteKind::Technology => player.technologies = Some(self.values),
            PrerequisiteKind::Civic => player.civics = Some(self.values),
        }
    }
}

/// Actual GetType and IsActive from one original live global node. Types are
/// signed32 native enum values, already remapped by original Read/Validate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LiveNode {
    pub definition_index: i32,
    pub active: bool,
}
/// Complete ordered global TechNode/CivicNode vector after native validation.
/// A complete empty vector proves absence; an uncaptured vector has no value
/// of this type. Never construct this from pre-validation serialized nodes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlobalNodes {
    kind: PrerequisiteKind,
    nodes: Vec<LiveNode>,
}
impl GlobalNodes {
    pub fn from_complete_live(
        kind: PrerequisiteKind,
        nodes: Vec<LiveNode>,
        limits: Limits,
    ) -> Result<Self, Error> {
        checked_size(nodes.len(), std::mem::size_of::<LiveNode>(), limits)?;
        if nodes.len() > i32::MAX as usize {
            return Err(Error::ResourceLimit);
        }
        Ok(Self { kind, nodes })
    }
    /// Original GetNode first tries nodes[type] when nonnegative and in range,
    /// accepting it only if its type matches; otherwise it finds the first
    /// matching type in original order. Sorting/deduplication changes behavior.
    pub fn node(&self, index: i32) -> Option<&LiveNode> {
        if index == -1 {
            return None;
        }
        self.nodes
            .get(index as u32 as usize)
            .filter(|node| node.definition_index == index)
            .or_else(|| {
                self.nodes
                    .iter()
                    .find(|node| node.definition_index == index)
            })
    }
    pub fn availability(
        &self,
        lookup: &dyn DefinitionLookup,
        limits: Limits,
    ) -> Result<Vec<bool>, Error> {
        let count = definition_count(lookup, limits)?;
        let mut values = storage(count, limits)?;
        values.extend(
            (0..count).map(|index| self.node(index as i32).is_some_and(|node| node.active)),
        );
        Ok(values)
    }
    /// Builds all values before changing World. Native player validity remains
    /// the separately imported Manager mapping; this does not synthesize it.
    pub fn install(
        &self,
        world: &mut World,
        lookup: &dyn DefinitionLookup,
        limits: Limits,
    ) -> Result<(), Error> {
        let values = self.availability(lookup, limits)?;
        match self.kind {
            PrerequisiteKind::Technology => world.technology_availability = Some(values),
            PrerequisiteKind::Civic => world.civic_availability = Some(values),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Registry;
    impl DefinitionLookup for Registry {
        fn count(&self) -> usize {
            3
        }
        fn find_index(&self, hash: u32) -> Option<usize> {
            match hash {
                0..=2 => Some(hash as usize),
                10 => Some(2),
                20 => Some(0),
                _ => None,
            }
        }
        fn type_hash(&self, index: usize) -> Option<u32> {
            [20, 30, 10].get(index).copied()
        }
    }
    fn pairs() -> IndexedBools {
        IndexedBools {
            pairs: vec![
                HashValue {
                    definition_hash: 10,
                    value: true,
                },
                HashValue {
                    definition_hash: 99,
                    value: true,
                },
                HashValue {
                    definition_hash: 20,
                    value: true,
                },
                HashValue {
                    definition_hash: 10,
                    value: false,
                },
            ],
        }
    }
    #[test]
    fn wire_boundaries_and_prefix() {
        let limits = Limits::default();
        let bytes = pairs().encode(limits).unwrap();
        assert_eq!(bytes.len(), 24);
        assert_eq!(IndexedBools::decode(&bytes, limits).unwrap(), pairs());
        for end in 0..bytes.len() {
            assert_eq!(
                IndexedBools::decode(&bytes[..end], limits),
                Err(Error::Truncated)
            );
        }
        let mut trailing = bytes.clone();
        trailing.push(7);
        assert_eq!(
            IndexedBools::decode(&trailing, limits),
            Err(Error::Trailing)
        );
        assert_eq!(
            IndexedBools::decode_prefix(&trailing, limits).unwrap().1,
            bytes.len()
        );
        let mut bad = bytes;
        bad[8] = 2;
        assert_eq!(IndexedBools::decode(&bad, limits), Err(Error::InvalidBool));
        assert_eq!(
            IndexedBools::decode(&u32::MAX.to_le_bytes(), limits),
            Err(Error::ResourceLimit)
        );
    }
    #[test]
    fn overlay_remapping_reset_and_install() {
        let limits = Limits::default();
        assert_eq!(
            pairs()
                .apply(&[false, true, true], &Registry, limits)
                .unwrap(),
            [true, true, false]
        );
        assert_eq!(
            pairs().apply(&[], &Registry, limits),
            Err(Error::DestinationSize)
        );
        let acquired = pairs()
            .acquired_after_owner_read(PrerequisiteKind::Technology, &Registry, limits)
            .unwrap();
        assert_eq!(acquired.values(), [true, false, false]);
        let mut player = Player::new(7, None, None).unwrap();
        acquired.install(&mut player);
        assert_eq!(player.technologies, Some(vec![true, false, false]));
        assert_eq!(player.civics, None);
        let civic = AcquiredState::from_complete_live(
            PrerequisiteKind::Civic,
            vec![false, true, true],
            &Registry,
            limits,
        )
        .unwrap();
        civic.install(&mut player);
        assert_eq!(player.civics, Some(vec![false, true, true]));
        assert_eq!(
            AcquiredState::from_complete_live(PrerequisiteKind::Civic, vec![], &Registry, limits),
            Err(Error::DestinationSize)
        );
    }
    #[test]
    fn global_fast_path_order_and_known_absence() {
        let nodes = GlobalNodes::from_complete_live(
            PrerequisiteKind::Civic,
            vec![
                LiveNode {
                    definition_index: 1,
                    active: false,
                },
                LiveNode {
                    definition_index: 1,
                    active: true,
                },
                LiveNode {
                    definition_index: 0,
                    active: false,
                },
                LiveNode {
                    definition_index: 0,
                    active: true,
                },
                LiveNode {
                    definition_index: -1,
                    active: true,
                },
            ],
            Limits::default(),
        )
        .unwrap();
        // Index 1 takes the positional node even though an earlier equal exists.
        assert_eq!(
            nodes.availability(&Registry, Limits::default()).unwrap(),
            [false, true, false]
        );
        assert!(nodes.node(-1).is_none());
        assert!(nodes.node(2).is_none());
        assert_eq!(
            GlobalNodes::from_complete_live(
                PrerequisiteKind::Technology,
                vec![],
                Limits::default()
            )
            .unwrap()
            .availability(&Registry, Limits::default())
            .unwrap(),
            [false; 3]
        );
    }
    #[test]
    fn projected_inputs_feed_world_provider() {
        use crate::map::{Header, Map, Plot, Recycled, UnitContainer};
        use crate::yields::PrerequisiteState;
        fn empty<T>() -> Recycled<T> {
            Recycled {
                capacity: 0,
                entries: vec![],
                current_id_bits: 0x10000,
                free_head: 0,
                free_tail: 0,
                free_links: vec![],
            }
        }
        let map = Map {
            header: Header {
                width: 1,
                height: 1,
                top_latitude: 90,
                bottom_latitude: -90,
                wrap_x: false,
                wrap_y: false,
                map_size_hash: 0,
                land_plots: 1,
                owned_plots: 0,
                natural_wonders: 0,
                resource_counts: vec![],
                land_resource_counts: vec![],
            },
            visibility: vec![],
            areas: empty(),
            landmasses: empty(),
            regions: empty(),
            chokepoints: empty(),
            continents: vec![],
            feature_layouts: empty(),
            plots: vec![Plot::decode(&[0; 55], 6, crate::map::Limits::default()).unwrap()],
            units: UnitContainer {
                width: 1,
                height: 1,
                layers: vec![],
            },
            components: vec![],
        };
        let mut world = World::new(map, vec![Player::new(7, None, None).unwrap()]).unwrap();
        assert!(world
            .acquired_mask(7, PrerequisiteKind::Technology)
            .is_err());
        pairs()
            .acquired_after_owner_read(PrerequisiteKind::Technology, &Registry, Limits::default())
            .unwrap()
            .install(world.player_mut(7).unwrap());
        let nodes = GlobalNodes::from_complete_live(
            PrerequisiteKind::Technology,
            vec![LiveNode {
                definition_index: 0,
                active: true,
            }],
            Limits::default(),
        )
        .unwrap();
        nodes
            .install(&mut world, &Registry, Limits::default())
            .unwrap();
        assert_eq!(
            world
                .acquired_mask(7, PrerequisiteKind::Technology)
                .unwrap(),
            [true, false, false]
        );
        assert!(world
            .globally_available(PrerequisiteKind::Technology, 0)
            .unwrap());
        assert!(!world
            .globally_available(PrerequisiteKind::Technology, 2)
            .unwrap());
        assert!(world.player_mapping.is_none());
        assert!(world.civic_availability.is_none());
    }
    #[test]
    fn invalid_mapping_and_limits_are_errors() {
        struct Bad;
        impl DefinitionLookup for Bad {
            fn count(&self) -> usize {
                1
            }
            fn find_index(&self, _: u32) -> Option<usize> {
                Some(1)
            }
            fn type_hash(&self, _: usize) -> Option<u32> {
                None
            }
        }
        assert_eq!(
            pairs().apply(&[false], &Bad, Limits::default()),
            Err(Error::DefinitionMapping)
        );
        let limits = Limits {
            max_items: 1,
            max_bytes: 100,
        };
        assert_eq!(pairs().encode(limits), Err(Error::ResourceLimit));
        assert_eq!(
            pairs().acquired_after_owner_read(PrerequisiteKind::Civic, &Registry, limits),
            Err(Error::ResourceLimit)
        );
    }
}
