//! Concrete Linux fresh-Read manager inputs. Not complete live action state.
//! Definitions/control fields require genuine matching configuration provenance.
use crate::registry::{linux_type_hash, LinuxDefinitionHashRegistry};
use crate::{Rules, RulesProfile, RulesRowId, SqlValue};
use c6_core::core_player::PlayerHeader;
use c6_core::core_stream::PhaseProcessor;
use c6_core::unit::{HashValue, Unit};
use c6_core::world::{Player, World};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    DefinitionOrder,
    ResourceLimit,
    RulesProfile,
    RulesData,
    PhaseOrderTie,
    CurrentPhase(i32),
    MissingPlayerSlot(i32),
    MissingPlayer(i32),
    MissingPlayerHeader(i32),
    HeaderOwner,
    MissingSlotStatus(i32),
    InvalidPauseByte(u8),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

/// Definition collection position, never SQLite rowid or operation hash.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeOperationId(pub i32);
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationDefinition {
    pub collection_index: u32,
    pub hash: u32,
}

/// Genuine original collection order. Registry hash collisions preserve native
/// last-definition precedence; small keys retain native direct-index priority.
pub struct OperationDefinitions {
    lookup: LinuxDefinitionHashRegistry,
}
impl OperationDefinitions {
    pub fn from_native_order(entries: &[OperationDefinition]) -> Result<Self, Error> {
        if entries.len() > i32::MAX as usize
            || entries
                .iter()
                .enumerate()
                .any(|(i, e)| e.collection_index as usize != i)
        {
            return Err(Error::DefinitionOrder);
        }
        Ok(Self {
            lookup: LinuxDefinitionHashRegistry::from_native_order(
                entries.iter().map(|e| e.hash).collect(),
            ),
        })
    }
    pub fn count(&self) -> usize {
        self.lookup.hashes().len()
    }
    pub fn find_index(&self, key: u32) -> Option<RuntimeOperationId> {
        self.lookup
            .find_index(key)
            .map(|i| RuntimeOperationId(i as i32))
    }
    /// Reset(false) sizes storage to actual definitions before ordered remapping.
    /// Unknown saved keys are consumed/ignored; repeated resolved keys overwrite.
    pub fn restore_disabled<'a>(
        &'a self,
        pairs: &[HashValue<bool>],
    ) -> Result<DisabledOperations<'a>, Error> {
        let mut values = Vec::new();
        values
            .try_reserve_exact(self.count())
            .map_err(|_| Error::ResourceLimit)?;
        values.resize(self.count(), false);
        for pair in pairs {
            if let Some(id) = self.find_index(pair.definition_hash) {
                values[id.0 as usize] = pair.value;
            }
        }
        Ok(DisabledOperations {
            definitions: self,
            values,
        })
    }
    pub fn restore_unit_disabled<'a>(
        &'a self,
        unit: &Unit,
    ) -> Result<DisabledOperations<'a>, Error> {
        self.restore_disabled(&unit.k_operations_disabled)
    }
}

/// Exact fresh-Read bool IndexList values, tied to their actual definitions.
/// Rebuild or update through separately proven native mutations after gameplay;
/// this snapshot must not silently stand in for changed live state.
pub struct DisabledOperations<'a> {
    definitions: &'a OperationDefinitions,
    values: Vec<bool>,
}
impl DisabledOperations<'_> {
    pub fn values(&self) -> &[bool] {
        &self.values
    }
    /// Original FindIndex again, unsigned IsValidIndex, then const byte value.
    pub fn is_disabled(&self, operation: RuntimeOperationId) -> bool {
        self.definitions
            .find_index(operation.0 as u32)
            .is_some_and(|id| self.values[id.0 as usize])
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TurnSegmentDefinition {
    pub row_id: RulesRowId,
    /// Actual raw definition flags at +0x50, not a predicate answer.
    pub flags: u8,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TurnPhaseDefinition {
    pub row_id: RulesRowId,
    pub phase_type_hash: u32,
    pub phase_order: i32,
    pub active_segment: Option<TurnSegmentDefinition>,
}

/// Actual selected runtime phase slots. None is a proven null pointer, never an
/// unknown definition. Use from_rules only with matching configured phase type.
pub struct PhaseDefinitions {
    slots: Vec<Option<TurnPhaseDefinition>>,
}
impl PhaseDefinitions {
    pub fn from_native_slots(slots: Vec<Option<TurnPhaseDefinition>>) -> Self {
        Self { slots }
    }
    pub fn slots(&self) -> &[Option<TurnPhaseDefinition>] {
        &self.slots
    }
    /// Source-proven InitializeTurnPhaseData filtering/signed ordering. Native
    /// sort ties are not inferred: require observed slots for those profiles.
    pub fn select(
        definitions: impl IntoIterator<Item = TurnPhaseDefinition>,
        configured_type: u32,
    ) -> Result<Self, Error> {
        let mut selected: Vec<_> = definitions
            .into_iter()
            .filter(|d| d.phase_type_hash == configured_type)
            .collect();
        selected.sort_unstable_by_key(|d| d.phase_order);
        if selected
            .windows(2)
            .any(|w| w[0].phase_order == w[1].phase_order)
        {
            return Err(Error::PhaseOrderTie);
        }
        Ok(Self {
            slots: selected.into_iter().map(Some).collect(),
        })
    }
    /// Exact pinned SQL columns/join. Rules are already source-identity gated;
    /// their SQLite rowids remain distinct from runtime current_phase indexes.
    pub fn from_rules(rules: &Rules, configured_type: u32) -> Result<Self, Error> {
        if rules.profile() == RulesProfile::WindowsXp2Historical {
            return Err(Error::RulesProfile);
        }
        let phases = rules
            .gameplay()
            .table("TurnPhases")
            .ok_or(Error::RulesData)?;
        let segments = rules
            .gameplay()
            .table("TurnSegments")
            .ok_or(Error::RulesData)?;
        let mut definitions = Vec::new();
        for phase in phases.rows() {
            let row_id = phase.row_id().ok_or(Error::RulesData)?;
            let phase_type_hash = linux_type_hash(
                phase
                    .text("TurnPhaseType")
                    .map_err(|_| Error::RulesData)?
                    .as_bytes(),
            );
            let phase_order =
                i32::try_from(phase.integer("PhaseOrder").map_err(|_| Error::RulesData)?)
                    .map_err(|_| Error::RulesData)?;
            let active = phase
                .text("ActiveSegmentType")
                .map_err(|_| Error::RulesData)?;
            let mut matching = segments
                .rows()
                .filter(|s| s.text("TurnSegmentType").is_ok_and(|name| name == active));
            let active_segment = if let Some(segment) = matching.next() {
                let row_id = segment.row_id().ok_or(Error::RulesData)?;
                // SQL is ORDER BY rowid ASC LIMIT 1. Table iteration already
                // has intrinsic rowid order; reject an incompatible source.
                if matching.any(|s| s.row_id().is_none_or(|r| r.0 < row_id.0)) {
                    return Err(Error::RulesData);
                }
                let integer = |name| -> Result<bool, Error> {
                    match segment.get(name).map_err(|_| Error::RulesData)? {
                        SqlValue::Integer(v @ (0 | 1)) => Ok(*v != 0),
                        _ => Err(Error::RulesData),
                    }
                };
                let flags = u8::from(integer("AllowStrategicCommands")?)
                    | (u8::from(integer("AllowTacticalCommands")?) << 1)
                    | (u8::from(integer("AllowTurnUnready")?) << 2);
                Some(TurnSegmentDefinition { row_id, flags })
            } else {
                None
            };
            definitions.push(TurnPhaseDefinition {
                row_id,
                phase_type_hash,
                phase_order,
                active_segment,
            });
        }
        Self::select(definitions, configured_type)
    }
    /// PlayerTypes is ignored in the original target. Only NO_PHASE(-1) and
    /// actual null phase/segment permit missing-data true branches. Invalid
    /// unchecked vector indexes are unsupported rather than invented true.
    pub fn allow_tactical_commands(&self, current_phase: i32, _owner: i32) -> Result<bool, Error> {
        if current_phase == -1 {
            return Ok(true);
        }
        let phase = self
            .slots
            .get(usize::try_from(current_phase).map_err(|_| Error::CurrentPhase(current_phase))?)
            .ok_or(Error::CurrentPhase(current_phase))?;
        Ok(phase
            .as_ref()
            .and_then(|p| p.active_segment.as_ref())
            .is_none_or(|segment| segment.flags & 2 != 0))
    }
    pub fn from_processor(&self, processor: &PhaseProcessor, owner: i32) -> Result<bool, Error> {
        self.allow_tactical_commands(processor.current_phase, owner)
    }
}

/// Retained actual Player identity; resolution does not add an IsValid gate.
/// Header state must describe the same proven imported live/fresh-Read context.
#[derive(Clone, Copy)]
pub struct WorldPlayer<'a> {
    player: &'a Player,
    header: Option<&'a PlayerHeader>,
}
impl<'a> WorldPlayer<'a> {
    pub fn resolve(
        world: &'a World,
        owner: i32,
        header: Option<&'a PlayerHeader>,
    ) -> Result<Self, Error> {
        let present = world
            .player_slot_present
            .as_ref()
            .ok_or(Error::MissingPlayerSlot(owner))?;
        if owner as u32 >= 65 || !present[owner as usize] {
            return Err(Error::MissingPlayerSlot(owner));
        }
        let player = world
            .player(owner)
            .map_err(|_| Error::MissingPlayer(owner))?;
        if header.is_some_and(|h| h.owner != player.owner()) {
            return Err(Error::HeaderOwner);
        }
        Ok(Self { player, header })
    }
    pub fn id(&self) -> i32 {
        self.player.owner()
    }
    pub fn is_human(&self) -> Result<bool, Error> {
        Ok(self
            .player
            .slot_status
            .ok_or(Error::MissingSlotStatus(self.id()))?
            == 3)
    }
    /// Original wrapper+0x280/value+0x10, third ordered saved Player47 bool.
    pub fn is_turn_active(&self) -> Result<bool, Error> {
        Ok(self
            .header
            .ok_or(Error::MissingPlayerHeader(self.id()))?
            .ordered_flags[2])
    }
}

/// Genuine original raw fields, not expected Manager answers. Current fixtures
/// do not yet capture both fields under a complete full-manager profile.
pub struct LiveControlFields {
    pub player_processing_automation: i32,
    pause_storage: u8,
}
impl LiveControlFields {
    pub fn new(player_processing_automation: i32, game_pause_storage: u8) -> Result<Self, Error> {
        if game_pause_storage > 1 {
            return Err(Error::InvalidPauseByte(game_pause_storage));
        }
        Ok(Self {
            player_processing_automation,
            pause_storage: game_pause_storage,
        })
    }
    pub fn is_paused(&self) -> bool {
        self.pause_storage != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use c6_core::map::{Header, Map, Plot, Recycled, UnitContainer};
    fn empty<T>() -> Recycled<T> {
        Recycled {
            capacity: 0,
            entries: vec![],
            current_id_bits: 0,
            free_head: 0,
            free_tail: 0,
            free_links: vec![],
        }
    }
    fn world() -> World {
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
            plots: vec![Plot::decode(&[0; 55], 6, c6_core::map::Limits::default()).unwrap()],
            units: UnitContainer {
                width: 1,
                height: 1,
                layers: vec![],
            },
            components: vec![],
        };
        let mut player = Player::new(2, None, None).unwrap();
        player.slot_status = Some(3);
        let mut world = World::new(map, vec![player]).unwrap();
        world.player_slot_present = Some([false; 65]);
        world.player_slot_present.as_mut().unwrap()[2] = true;
        // Direct GetPlayer is independent of Manager.IsValid.
        world.player_mapping = Some([-1; 65]);
        world
    }
    fn header() -> PlayerHeader {
        PlayerHeader {
            owner: 2,
            starting_plot: c6_core::unit::PlotCoord { x: 0, y: 0 },
            ordered_scalars: [0; 15],
            ordered_flags: [false, false, true, false, false, false],
            handicap_hash: 0,
            district_hash: 0,
            project_hash: 0,
            final_scalar: 0,
        }
    }
    fn definitions(hashes: &[u32]) -> OperationDefinitions {
        OperationDefinitions::from_native_order(
            &hashes
                .iter()
                .enumerate()
                .map(|(i, &hash)| OperationDefinition {
                    collection_index: i as u32,
                    hash,
                })
                .collect::<Vec<_>>(),
        )
        .unwrap()
    }
    fn pair(hash: u32, value: bool) -> HashValue<bool> {
        HashValue {
            definition_hash: hash,
            value,
        }
    }
    #[test]
    fn disabled_reset_remap_unknown_duplicate_and_namespace_boundaries() {
        let d = definitions(&[0x100, 0x200, 0x100, u32::MAX]);
        let restored = d
            .restore_disabled(&[
                pair(0x100, true),
                pair(0x700, true),
                pair(0x100, false),
                pair(0x200, true),
                pair(1, false),
                pair(u32::MAX, true),
            ])
            .unwrap();
        assert_eq!(restored.values(), [false, false, false, true]);
        assert!(!restored.is_disabled(RuntimeOperationId(0)));
        assert!(!restored.is_disabled(RuntimeOperationId(0x100)));
        assert!(restored.is_disabled(RuntimeOperationId(-1)));
        assert!(!restored.is_disabled(RuntimeOperationId(-2)));
        assert_eq!(d.restore_disabled(&[]).unwrap().values(), [false; 4]);
        assert!(
            OperationDefinitions::from_native_order(&[OperationDefinition {
                collection_index: 1,
                hash: 99
            }])
            .is_err()
        );
        // A saved key below definition count means index before hash, even
        // when a definition's actual hash has those same low bits.
        let low = definitions(&[1, 0x200]);
        assert_eq!(
            low.restore_disabled(&[pair(1, true)]).unwrap().values(),
            [false, true]
        );
        assert_eq!(low.find_index(0x200), Some(RuntimeOperationId(1)));
    }
    #[test]
    fn tactical_selected_signed_order_nulls_ignored_owner_and_ties() {
        let segment = |flags| {
            Some(TurnSegmentDefinition {
                row_id: RulesRowId(50),
                flags,
            })
        };
        let phase = |order, ty, active_segment| TurnPhaseDefinition {
            row_id: RulesRowId(i64::from(order)),
            phase_type_hash: ty,
            phase_order: order,
            active_segment,
        };
        let phases = PhaseDefinitions::select(
            [
                phase(4, 100, segment(0)),
                phase(-2, 100, segment(2)),
                phase(1, 200, segment(0)),
                phase(0, 100, None),
            ],
            100,
        )
        .unwrap();
        assert_eq!(
            phases
                .slots()
                .iter()
                .map(|d| d.unwrap().phase_order)
                .collect::<Vec<_>>(),
            [-2, 0, 4]
        );
        for owner in [i32::MIN, -1, 0, 64, i32::MAX] {
            assert_eq!(phases.allow_tactical_commands(-1, owner), Ok(true));
            assert_eq!(phases.allow_tactical_commands(0, owner), Ok(true));
            assert_eq!(phases.allow_tactical_commands(1, owner), Ok(true));
            assert_eq!(phases.allow_tactical_commands(2, owner), Ok(false));
        }
        assert_eq!(
            phases.allow_tactical_commands(-2, 0),
            Err(Error::CurrentPhase(-2))
        );
        assert_eq!(
            phases.allow_tactical_commands(3, 0),
            Err(Error::CurrentPhase(3))
        );
        assert!(matches!(
            PhaseDefinitions::select([phase(1, 100, None), phase(1, 100, segment(2))], 100),
            Err(Error::PhaseOrderTie)
        ));
        assert_eq!(
            PhaseDefinitions::from_native_slots(vec![None]).allow_tactical_commands(0, 0),
            Ok(true)
        );
    }
    #[test]
    fn concrete_world_player_and_raw_control_fields() {
        let mut w = world();
        let mut h = header();
        let p = WorldPlayer::resolve(&w, 2, Some(&h)).unwrap();
        assert_eq!(p.id(), 2);
        assert_eq!(p.is_human(), Ok(true));
        assert_eq!(p.is_turn_active(), Ok(true));
        assert_eq!(
            WorldPlayer::resolve(&w, 2, None).unwrap().is_turn_active(),
            Err(Error::MissingPlayerHeader(2))
        );
        h.owner = 3;
        assert!(matches!(
            WorldPlayer::resolve(&w, 2, Some(&h)),
            Err(Error::HeaderOwner)
        ));
        assert!(matches!(
            WorldPlayer::resolve(&w, -1, None),
            Err(Error::MissingPlayerSlot(-1))
        ));
        w.player_slot_present = None;
        assert!(matches!(
            WorldPlayer::resolve(&w, 2, None),
            Err(Error::MissingPlayerSlot(2))
        ));
        for value in [-1, 0, 64, i32::MIN, i32::MAX] {
            for paused in [0, 1] {
                let c = LiveControlFields::new(value, paused).unwrap();
                assert_eq!(c.player_processing_automation, value);
                assert_eq!(c.is_paused(), paused == 1);
            }
        }
        assert!(matches!(
            LiveControlFields::new(0, 2),
            Err(Error::InvalidPauseByte(2))
        ));
    }
}
