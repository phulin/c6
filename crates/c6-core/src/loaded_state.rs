//! Consuming current-Core ownership for state-backed queries and future actions.
//! This is not complete native Read or full action admission.
use crate::core_player::{self, initialization::WireBinding, PlayerInitializations};
use crate::core_stream::{ThroughRandom, ThroughRandomError};
use crate::formation_initialization::FormationRuntime;
use crate::loaded_components::{self, OwnedCore, RetainedPlayer};
use crate::unit_initialization::{InitializationInputs, UnitRuntime};
use crate::{map, unit, world};
use std::sync::Arc;

/// Unique input storage moves without copying; explicitly shared immutable
/// storage remains shared. Neither variant permits an implicit World clone.
#[derive(Debug)]
pub enum CoreImage {
    Unique(Vec<u8>),
    Shared(Arc<[u8]>),
}

#[cfg(test)]
mod tests {
    use super::*;
    // Private source-only state carrier: no public fabricated native factory.
    fn state() -> LoadedState {
        let (bytes, _, _) = crate::core_stream::tests::random_prefix_fixture();
        let binding = crate::core_stream::tests::context_binding(&bytes);
        let contexts = |_: core_player::initialization::PlayerIdentity| None;
        let parsed = ThroughRandom::decode(
            &bytes,
            map::Limits::default(),
            unit::Limits::default(),
            binding.clone(),
            &contexts,
        )
        .unwrap();
        let (mut map, core) = loaded_components::split_core(parsed.global_ai.players.core);
        map.header.width = 1;
        map.header.height = 1;
        map.plots = vec![map::Plot::decode(&[0; 55], 6, map::Limits::default()).unwrap()];
        map.units.width = 1;
        map.units.height = 1;
        let mut player = crate::core_player::tests::source_first_player();
        player.formations.capacity = 2;
        player.formations.entries = vec![map::Slot {
            index: 1,
            value: unit::Formation {
                id: 0x10001,
                owner: player.header.owner,
                location: unit::PlotCoord { x: 0, y: 0 },
                moves_remaining_raw: 0,
                domain: 0,
                is_embarked: false,
                can_embark: false,
                can_attack: false,
                is_ranged_combat: false,
                can_capture: false,
                has_moved: false,
                has_moved_into_zoc: false,
                ignores_zoc: false,
                last_path: vec![],
                last_path_cache_location: unit::PlotCoord { x: 0, y: 0 },
                unit_ids: vec![],
            },
        }];
        let (player, retained) = loaded_components::split_player(player).unwrap();
        let world = world::World::new(map, vec![player]).unwrap();
        let statistics = ImportStatistics {
            players: 1,
            units: world.players()[0].unit_records().unwrap().len(),
            formations: world.players()[0].formation_records().unwrap().len(),
            consumed: parsed.consumed,
            items_used: parsed.items_used,
        };
        let retained = RetainedComponents {
            core,
            religion: parsed.global_ai.players.player_prefix.religion,
            players: vec![retained],
            global_ai: parsed.global_ai.global_ai,
            barbarian: parsed.barbarian,
            free_cities: parsed.free_cities,
            random: parsed.random,
        };
        LoadedState {
            image: CoreImage::Unique(bytes),
            binding,
            world,
            retained,
            statistics,
            controls: None,
            runtime: None,
            formation_runtime: None,
            limits: AssemblyLimits::default(),
            revision: 0,
            observed_snapshot: None,
        }
    }
    fn controls(state: &LoadedState) -> ControlInputs {
        let owner = state.world.players()[0].owner();
        let mut slots = [false; 65];
        slots[owner as usize] = true;
        let mut mapping = [-1; 65];
        mapping[owner as usize] = 0;
        ControlInputs {
            binding: state.binding.clone(),
            player_mapping: mapping,
            player_slots: slots,
            players: vec![PlayerControls {
                owner,
                slot_status: 3,
                team: 4,
                is_alive: true,
                turn_active: false,
            }],
            processing_automation: Some(-1),
            pause_storage: Some(0),
        }
    }
    #[test]
    fn explicit_image_sharing_never_copies_bytes() {
        let bytes: Arc<[u8]> = Arc::from([1u8, 2, 3]);
        let pointer = bytes.as_ptr();
        let image = CoreImage::Shared(Arc::clone(&bytes));
        assert_eq!(image.as_ref().as_ptr(), pointer);
        assert_eq!(Arc::strong_count(&bytes), 2);
        let raw = vec![4, 5, 6];
        let pointer = raw.as_ptr();
        assert_eq!(CoreImage::Unique(raw).as_ref().as_ptr(), pointer);
    }
    #[test]
    fn live_controls_are_bound_atomic_and_never_inferred_from_saved_membership() {
        let mut state = state();
        let owner = state.world.players()[0].owner();
        assert!(state.world.player_mapping.is_none());
        assert!(state.world.players()[0].is_alive.is_none());
        assert!(matches!(
            state.turn_active(owner),
            Err(Error::Missing(MissingInput::ManagerSlots))
        ));
        assert!(matches!(
            state.processing_automation(),
            Err(Error::Missing(MissingInput::ProcessingAutomation))
        ));
        let mut bad = controls(&state);
        bad.binding.occurrence_id.push(1);
        assert!(matches!(state.bind_controls(bad), Err(Error::Binding)));
        assert!(state.world.player_mapping.is_none());
        let mut bad = controls(&state);
        bad.players.push(bad.players[0]);
        assert!(matches!(state.bind_controls(bad), Err(Error::Binding)));
        assert!(state.world.players()[0].is_alive.is_none());
        let actual = controls(&state);
        state.bind_controls(actual).unwrap();
        assert!(!state.turn_active(owner).unwrap());
        assert_eq!(state.processing_automation().unwrap(), -1);
        assert_eq!(state.world.player(owner).unwrap().team, Some(4));
        let mut absent = controls(&state);
        absent.players.clear();
        absent.pause_storage = None;
        assert!(matches!(
            state.bind_controls(absent),
            Err(Error::AlreadyBound)
        ));
        assert_eq!(state.world.player(owner).unwrap().is_alive, Some(true));
        assert_eq!(state.pause_storage().unwrap(), 0);
        assert!(matches!(
            state.unit_runtime(owner, 0),
            Err(Error::Missing(MissingInput::UnitRuntime))
        ));
    }
    fn witness(s: &LoadedState) -> ObservedSnapshotWitness {
        ObservedSnapshotWitness {
            source_role: core_player::initialization::WireRole::SavedOutputCore,
            snapshot_sha256: s.binding.core_sha256,
            snapshot_bytes: s.source_image().len(),
            capture_runtime_sha256: [7; 32],
            capture_facts_sha256: [8; 32],
            capture_configuration_sha256: [9; 32],
        }
    }
    #[test]
    fn snapshot_authority_is_one_shot_and_decoder_role_stays_separate() {
        let mut s = state();
        let before = s.binding.clone();
        let mut bad = witness(&s);
        bad.snapshot_bytes += 1;
        assert!(matches!(
            s.bind_observed_snapshot_controls(bad, controls(&s)),
            Err(Error::Binding)
        ));
        assert!(s.observed_snapshot().is_none());
        let w = witness(&s);
        s.bind_observed_snapshot_controls(w.clone(), controls(&s))
            .unwrap();
        assert_eq!(s.binding, before);
        assert_eq!(s.observed_snapshot(), Some(&w));
        assert!(matches!(
            s.bind_observed_snapshot_controls(w, controls(&s)),
            Err(Error::AlreadyBound)
        ));
        let mut advanced = state();
        advanced.initialize_formations().unwrap();
        let owner = advanced.world.players()[0].owner();
        advanced
            .formation_with_runtime_mut(owner, 0x20001)
            .unwrap()
            .0
            .moves_remaining_raw = 7;
        assert_eq!(advanced.revision(), 1);
        assert!(matches!(
            advanced.bind_observed_snapshot_controls(witness(&advanced), controls(&advanced)),
            Err(Error::NotPristine)
        ));
    }
    #[test]
    fn formation_initialization_needs_no_unrelated_controls_and_never_resets_live_changes() {
        let mut s = state();
        let owner = s.world.players()[0].owner();
        s.initialize_formations().unwrap();
        let (f, r) = s.formation_with_runtime_mut(owner, 0x90001).unwrap();
        f.moves_remaining_raw = 123;
        r.archive.mark_dirty(14).unwrap();
        assert!(matches!(
            s.initialize_formations(),
            Err(Error::AlreadyInitialized)
        ));
        assert_eq!(
            s.world
                .player(owner)
                .unwrap()
                .formation(1)
                .unwrap()
                .unwrap()
                .record
                .moves_remaining_raw,
            123
        );
        assert!(s
            .formation_runtime(owner, 1)
            .unwrap()
            .archive
            .dirty_calls()
            .contains(&14));
    }
    #[test]
    fn unit_initialization_is_one_shot_and_first_controls_do_not_discard_it() {
        struct Empty;
        impl crate::movement_world::DefinitionLookup for Empty {
            fn count(&self) -> usize {
                0
            }
            fn find_index(&self, _: u32) -> Option<usize> {
                None
            }
            fn type_hash(&self, _: usize) -> Option<u32> {
                None
            }
        }
        let mut s = state();
        let identity = s.binding.inputs.clone();
        let empty = Empty;
        let properties: Vec<crate::unit_initialization::UnitDefinition> = vec![];
        let inputs = InitializationInputs {
            identities: &identity,
            units: crate::movement_world::DefinitionTable::new(&empty, &properties).unwrap(),
            abilities: &empty,
            terrains: &empty,
            features: &empty,
            great_person_classes: &empty,
            operations: &empty,
        };
        assert!(s.controls.is_none());
        s.initialize_rosters(&inputs).unwrap();
        let pointer = s.runtime.as_ref().unwrap().as_ptr();
        assert!(matches!(
            s.initialize_rosters(&inputs),
            Err(Error::AlreadyInitialized)
        ));
        s.bind_controls(controls(&s)).unwrap();
        assert_eq!(s.runtime.as_ref().unwrap().as_ptr(), pointer);
        assert!(matches!(
            s.bind_controls(controls(&s)),
            Err(Error::AlreadyBound)
        ));
    }
}
impl AsRef<[u8]> for CoreImage {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Unique(v) => v,
            Self::Shared(v) => v,
        }
    }
}
#[derive(Debug)]
pub enum Error {
    Decode(ThroughRandomError),
    Split(loaded_components::Error),
    World(world::MissingInput),
    IncompletePlayers,
    Binding,
    ResourceLimit,
    Missing(MissingInput),
    UnitInitialization(crate::unit_initialization::Error),
    FormationLookup,
    FormationOwner(crate::formation_initialization::OwnerLookupError),
    RuntimeIdentity,
    RuntimeDependenciesChanged,
    AlreadyInitialized,
    AlreadyBound,
    NotPristine,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "loaded state: {self:?}")
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MissingInput {
    ManagerSlots,
    PlayerConfiguration(i32),
    PlayerTurnActive(i32),
    Player(i32),
    Unit { owner: i32, id: u32 },
    Formation { owner: i32, id: u32 },
    UnitRuntime,
    FormationRuntime,
    ProcessingAutomation,
    GamePause,
}

/// Actual supplied live facts, distinct from saved flags and roster presence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlayerControls {
    pub owner: i32,
    pub slot_status: i32,
    pub team: i32,
    pub is_alive: bool,
    pub turn_active: bool,
}
/// Bind external facts to this exact admitted stream role. These values must
/// come from an attributed capture or faithful initialization; this API checks
/// consistency, not execution authenticity. Unknown facts are not defaults.
#[derive(Debug)]
pub struct ControlInputs {
    pub binding: WireBinding,
    pub player_mapping: [i32; 65],
    pub player_slots: [bool; 65],
    pub players: Vec<PlayerControls>,
    pub processing_automation: Option<i32>,
    pub pause_storage: Option<u8>,
}
/// Provenance of an observed mutable-state export, independent of the decoder's
/// Read/Write framing role. The resource adapter validates actual preimages.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservedSnapshotWitness {
    pub source_role: core_player::initialization::WireRole,
    pub snapshot_sha256: [u8; 32],
    pub snapshot_bytes: usize,
    pub capture_runtime_sha256: [u8; 32],
    pub capture_facts_sha256: [u8; 32],
    pub capture_configuration_sha256: [u8; 32],
}
#[derive(Clone, Copy, Debug)]
pub struct AssemblyLimits {
    pub max_roster_records: usize,
    pub max_runtime_array_items: usize,
}
impl Default for AssemblyLimits {
    fn default() -> Self {
        Self {
            max_roster_records: 4_000_000,
            max_runtime_array_items: 16_000_000,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImportStatistics {
    pub players: usize,
    pub units: usize,
    pub formations: usize,
    pub consumed: usize,
    pub items_used: usize,
}
#[derive(Debug)]
pub struct RetainedComponents {
    pub core: OwnedCore,
    pub religion: core_player::Religion,
    pub players: Vec<RetainedPlayer>,
    pub global_ai: crate::game_global_ai::GlobalAi,
    pub barbarian: crate::game_barbarian::Manager,
    pub free_cities: crate::game_free_cities::Manager,
    pub random: crate::game_random_wire::Random,
}
#[derive(Debug)]
struct RuntimePlayer {
    owner: i32,
    unit_index: Vec<Option<usize>>,
    units: Vec<RuntimeUnit>,
}
#[derive(Debug)]
struct RuntimeFormations {
    owner: i32,
    formation_index: Vec<Option<usize>>,
    formations: Vec<FormationRuntime>,
}
#[derive(Debug)]
struct RuntimeUnit {
    value: UnitRuntime,
    dependencies: UnitDependencies,
}
/// Exact saved inputs used by the covered Read application. Captured once;
/// comparisons allocate nothing and do not reset mutable live arrays.
#[derive(Debug)]
struct UnitDependencies {
    type_hash: u32,
    formation_id: u32,
    abilities: Vec<unit::HashValue<i32>>,
    invalid_terrain: Vec<unit::HashValue<bool>>,
    ignore_terrain: Vec<unit::HashValue<bool>>,
    ignore_feature: Vec<unit::HashValue<bool>>,
    bonus: Vec<unit::HashValue<i32>>,
    strength: Vec<unit::HashValue<i32>>,
    disabled: Vec<unit::HashValue<bool>>,
}
impl UnitDependencies {
    fn capture(u: &unit::Unit) -> Result<Self, Error> {
        fn copy<T: Clone>(v: &[T]) -> Result<Vec<T>, Error> {
            let mut out = reserved(v.len())?;
            out.extend_from_slice(v);
            Ok(out)
        }
        Ok(Self {
            type_hash: u.e_type_index,
            formation_id: u.formation_id,
            abilities: copy(&u.ai_abilities_possessed)?,
            invalid_terrain: copy(&u.ab_invalid_terrain_types)?,
            ignore_terrain: copy(&u.ab_ignore_terrain_types)?,
            ignore_feature: copy(&u.ab_ignore_feature_types)?,
            bonus: copy(&u.ai_bonus_points_per_kill)?,
            strength: copy(&u.ai_bonus_points_per_kill_strength)?,
            disabled: copy(&u.k_operations_disabled)?,
        })
    }
    fn matches(&self, u: &unit::Unit) -> bool {
        self.type_hash == u.e_type_index
            && self.formation_id == u.formation_id
            && self.abilities == u.ai_abilities_possessed
            && self.invalid_terrain == u.ab_invalid_terrain_types
            && self.ignore_terrain == u.ab_ignore_terrain_types
            && self.ignore_feature == u.ab_ignore_feature_types
            && self.bonus == u.ai_bonus_points_per_kill
            && self.strength == u.ai_bonus_points_per_kill_strength
            && self.disabled == u.k_operations_disabled
    }
    fn items(u: &unit::Unit) -> Option<usize> {
        [
            u.ai_abilities_possessed.len(),
            u.ab_invalid_terrain_types.len(),
            u.ab_ignore_terrain_types.len(),
            u.ab_ignore_feature_types.len(),
            u.ai_bonus_points_per_kill.len(),
            u.ai_bonus_points_per_kill_strength.len(),
            u.k_operations_disabled.len(),
        ]
        .into_iter()
        .try_fold(0usize, usize::checked_add)
    }
}
#[derive(Debug)]
pub struct LoadedState {
    image: CoreImage,
    binding: WireBinding,
    world: world::World,
    retained: RetainedComponents,
    statistics: ImportStatistics,
    controls: Option<ControlInputs>,
    runtime: Option<Vec<RuntimePlayer>>,
    formation_runtime: Option<Vec<RuntimeFormations>>,
    limits: AssemblyLimits,
    revision: u64,
    observed_snapshot: Option<ObservedSnapshotWitness>,
}
fn reserved<T>(n: usize) -> Result<Vec<T>, Error> {
    n.checked_mul(std::mem::size_of::<T>())
        .filter(|n| *n <= isize::MAX as usize)
        .ok_or(Error::ResourceLimit)?;
    let mut v = Vec::new();
    v.try_reserve_exact(n).map_err(|_| Error::ResourceLimit)?;
    Ok(v)
}
fn slot_index<T>(capacity: u32, entries: &[map::Slot<T>]) -> Result<Vec<Option<usize>>, Error> {
    if capacity > 65_536 {
        return Err(Error::World(world::MissingInput::InvalidSlot));
    }
    let mut index = reserved(capacity as usize)?;
    index.resize(capacity as usize, None);
    for (i, e) in entries.iter().enumerate() {
        let slot =
            usize::try_from(e.index).map_err(|_| Error::World(world::MissingInput::InvalidSlot))?;
        let target = index
            .get_mut(slot)
            .ok_or(Error::World(world::MissingInput::InvalidSlot))?;
        if target.replace(i).is_some() {
            return Err(Error::World(world::MissingInput::DuplicateSlot));
        }
    }
    Ok(index)
}
impl LoadedState {
    /// Parse once, move all decoded records, retain the entire original image.
    /// Shared byte storage is allowed explicitly; parsed records are never shared.
    pub fn decode(
        image: CoreImage,
        shared: map::Limits,
        post_map: unit::Limits,
        binding: WireBinding,
        contexts: &impl PlayerInitializations,
        limits: AssemblyLimits,
    ) -> Result<Self, Error> {
        let parsed =
            ThroughRandom::decode(image.as_ref(), shared, post_map, binding.clone(), contexts)
                .map_err(Error::Decode)?;
        if !parsed
            .global_ai
            .players
            .player_prefix
            .complete_player_manager_wire()
        {
            return Err(Error::IncompletePlayers);
        }
        let consumed = parsed.consumed;
        let items_used = parsed.items_used;
        let prefix = parsed.global_ai.players.player_prefix;
        if prefix.initialization_binding.as_ref() != Some(&binding) {
            return Err(Error::Binding);
        }
        let count = prefix.players().count();
        if count > limits.max_roster_records {
            return Err(Error::ResourceLimit);
        }
        let mut players = reserved(count)?;
        let mut retained_players = reserved(count)?;
        let mut units = 0usize;
        let mut formations = 0usize;
        for p in prefix
            .first_player
            .into_iter()
            .chain(prefix.additional_players)
        {
            units = units
                .checked_add(p.units.records.entries.len())
                .ok_or(Error::ResourceLimit)?;
            formations = formations
                .checked_add(p.formations.entries.len())
                .ok_or(Error::ResourceLimit)?;
            if units
                .checked_add(formations)
                .is_none_or(|n| n > limits.max_roster_records)
            {
                return Err(Error::ResourceLimit);
            }
            let (player, retained) = loaded_components::split_player(p).map_err(Error::Split)?;
            players.push(player);
            retained_players.push(retained);
        }
        let (map, core) = loaded_components::split_core(parsed.global_ai.players.core);
        let world = world::World::new(map, players).map_err(Error::World)?;
        let retained = RetainedComponents {
            core,
            religion: prefix.religion,
            players: retained_players,
            global_ai: parsed.global_ai.global_ai,
            barbarian: parsed.barbarian,
            free_cities: parsed.free_cities,
            random: parsed.random,
        };
        Ok(Self {
            image,
            binding,
            world,
            retained,
            statistics: ImportStatistics {
                players: count,
                units,
                formations,
                consumed,
                items_used,
            },
            controls: None,
            runtime: None,
            formation_runtime: None,
            limits,
            revision: 0,
            observed_snapshot: None,
        })
    }
    pub fn world(&self) -> &world::World {
        &self.world
    }
    pub fn retained(&self) -> &RetainedComponents {
        &self.retained
    }
    pub fn binding(&self) -> &WireBinding {
        &self.binding
    }
    pub fn statistics(&self) -> ImportStatistics {
        self.statistics
    }
    pub fn source_image(&self) -> &[u8] {
        self.image.as_ref()
    }
    pub fn unparsed_tail(&self) -> &[u8] {
        &self.image.as_ref()[self.statistics.consumed..]
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn is_pristine(&self) -> bool {
        self.revision == 0
    }
    pub fn observed_snapshot(&self) -> Option<&ObservedSnapshotWitness> {
        self.observed_snapshot.as_ref()
    }
    fn require_initial_controls(&self) -> Result<(), Error> {
        if self.controls.is_some() || self.observed_snapshot.is_some() {
            return Err(Error::AlreadyBound);
        }
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        Ok(())
    }
    /// Consume one observed SavedOutput snapshot export. Decoder provenance is
    /// retained unchanged; these are not facts from its Read process.
    pub fn bind_observed_snapshot_controls(
        &mut self,
        witness: ObservedSnapshotWitness,
        controls: ControlInputs,
    ) -> Result<(), Error> {
        self.require_initial_controls()?;
        if witness.source_role != core_player::initialization::WireRole::SavedOutputCore
            || witness.snapshot_sha256 != self.binding.core_sha256
            || witness.snapshot_bytes != self.image.as_ref().len()
        {
            return Err(Error::Binding);
        }
        self.install_controls(controls)?;
        self.observed_snapshot = Some(witness);
        Ok(())
    }

    /// Validate every control before changing any World field. Saved membership
    /// does not supply Manager validity or nonnull-slot facts.
    pub fn bind_controls(&mut self, controls: ControlInputs) -> Result<(), Error> {
        self.require_initial_controls()?;
        self.install_controls(controls)
    }
    fn install_controls(&mut self, controls: ControlInputs) -> Result<(), Error> {
        if controls.binding != self.binding {
            return Err(Error::Binding);
        }
        if controls.pause_storage.is_some_and(|p| p > 1) {
            return Err(Error::Binding);
        }
        for (i, p) in controls.players.iter().enumerate() {
            if !(0..65).contains(&p.owner)
                || !controls.player_slots[p.owner as usize]
                || controls.players[..i].iter().any(|q| q.owner == p.owner)
            {
                return Err(Error::Binding);
            }
            self.world.player(p.owner).map_err(Error::World)?;
        }
        let mut owners = reserved(self.world.players().len())?;
        owners.extend(self.world.players().iter().map(|p| p.owner()));
        for owner in owners {
            let player = self.world.player_mut(owner).map_err(Error::World)?;
            player.slot_status = None;
            player.team = None;
            player.is_alive = None;
        }
        for p in &controls.players {
            let player = self.world.player_mut(p.owner).map_err(Error::World)?;
            player.slot_status = Some(p.slot_status);
            player.team = Some(p.team);
            player.is_alive = Some(p.is_alive);
        }
        self.world.player_mapping = Some(controls.player_mapping);
        self.world.player_slot_present = Some(controls.player_slots);
        self.controls = Some(controls);
        Ok(())
    }
    pub fn player_controls(&self, owner: i32) -> Result<&PlayerControls, Error> {
        self.controls
            .as_ref()
            .ok_or(Error::Missing(MissingInput::ManagerSlots))?
            .players
            .iter()
            .find(|p| p.owner == owner)
            .ok_or(Error::Missing(MissingInput::PlayerConfiguration(owner)))
    }
    pub fn turn_active(&self, owner: i32) -> Result<bool, Error> {
        Ok(self.player_controls(owner)?.turn_active)
    }
    pub fn processing_automation(&self) -> Result<i32, Error> {
        self.controls
            .as_ref()
            .and_then(|c| c.processing_automation)
            .ok_or(Error::Missing(MissingInput::ProcessingAutomation))
    }
    pub fn pause_storage(&self) -> Result<u8, Error> {
        self.controls
            .as_ref()
            .and_then(|c| c.pause_storage)
            .ok_or(Error::Missing(MissingInput::GamePause))
    }

    /// Prepare bounded source-covered Read sidecars without cloning/mutating
    /// imported records. Nested Unit remaps and global load completion remain open.
    pub fn initialize_rosters(&mut self, inputs: &InitializationInputs<'_>) -> Result<(), Error> {
        if self.runtime.is_some() {
            return Err(Error::AlreadyInitialized);
        }
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        if inputs.identities != &self.binding.inputs {
            return Err(Error::Binding);
        }
        let array_count = inputs
            .abilities
            .count()
            .checked_add(
                inputs
                    .terrains
                    .count()
                    .checked_mul(2)
                    .ok_or(Error::ResourceLimit)?,
            )
            .and_then(|n| n.checked_add(inputs.features.count()))
            .and_then(|n| n.checked_add(inputs.great_person_classes.count().checked_mul(2)?))
            .and_then(|n| n.checked_add(inputs.operations.count()))
            .ok_or(Error::ResourceLimit)?;
        let wire_items = self
            .world
            .players()
            .iter()
            .try_fold(0usize, |n, p| {
                p.unit_records()
                    .ok()?
                    .iter()
                    .try_fold(n, |n, u| n.checked_add(UnitDependencies::items(&u.value)?))
            })
            .ok_or(Error::ResourceLimit)?;
        if array_count
            .checked_mul(self.statistics.units)
            .and_then(|n| n.checked_add(wire_items))
            .is_none_or(|n| n > self.limits.max_runtime_array_items)
        {
            return Err(Error::ResourceLimit);
        }
        let mut runtime = reserved(self.world.players().len())?;
        for player in self.world.players() {
            let records = player.units().map_err(Error::World)?;
            let mut units = reserved(records.entries.len())?;
            for u in &records.entries {
                units.push(RuntimeUnit {
                    value: UnitRuntime::from_read(&u.value, &self.binding.inputs, inputs, player)
                        .map_err(Error::UnitInitialization)?,
                    dependencies: UnitDependencies::capture(&u.value)?,
                });
            }
            runtime.push(RuntimePlayer {
                owner: player.owner(),
                unit_index: slot_index(records.capacity, &records.entries)?,
                units,
            });
        }
        self.runtime = Some(runtime);
        Ok(())
    }
    /// Owner lookup is independent of Unit definition/array preparation.
    pub fn initialize_formations(&mut self) -> Result<(), Error> {
        if self.formation_runtime.is_some() {
            return Err(Error::AlreadyInitialized);
        }
        if !self.is_pristine() {
            return Err(Error::NotPristine);
        }
        let mut runtime = reserved(self.world.players().len())?;
        for player in self.world.players() {
            let records = player.formations().map_err(Error::World)?;
            let mut formations = reserved(records.entries.len())?;
            for f in &records.entries {
                let retained = self
                    .retained
                    .players
                    .iter()
                    .find(|p| p.header.owner == player.owner())
                    .ok_or(Error::FormationLookup)?;
                formations.push(
                    FormationRuntime::from_owning_player_read(
                        &f.value,
                        retained.requested_owner,
                        retained.header.owner,
                    )
                    .map_err(Error::FormationOwner)?,
                );
            }
            runtime.push(RuntimeFormations {
                owner: player.owner(),
                formation_index: slot_index(records.capacity, &records.entries)?,
                formations,
            });
        }
        self.formation_runtime = Some(runtime);
        Ok(())
    }
    /// Existing pure legality queries can use the actual imported Unit without
    /// requiring unrelated UI or native dirty-tree machinery.
    pub fn unit_mut(&mut self, owner: i32, id: u32) -> Result<&mut unit::Unit, Error> {
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        let value = self
            .world
            .player_mut(owner)
            .map_err(Error::World)?
            .unit_mut(id)
            .map_err(Error::World)?
            .ok_or(Error::Missing(MissingInput::Unit { owner, id }))?;
        self.revision = next;
        Ok(value)
    }
    pub fn unit_runtime(&self, owner: i32, id: u32) -> Result<&UnitRuntime, Error> {
        let row = self
            .runtime
            .as_ref()
            .ok_or(Error::Missing(MissingInput::UnitRuntime))?
            .iter()
            .find(|p| p.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        let index = row
            .unit_index
            .get((id & 0xffff) as usize)
            .copied()
            .flatten()
            .filter(|_| id != u32::MAX)
            .ok_or(Error::Missing(MissingInput::Unit { owner, id }))?;
        let found = self
            .world
            .player(owner)
            .map_err(Error::World)?
            .unit(id)
            .map_err(Error::World)?
            .ok_or(Error::Missing(MissingInput::Unit { owner, id }))?;
        let identity = row.units[index].value.archive.identity();
        if identity.owner as i32 != found.record.e_owner || identity.id != found.record.id {
            return Err(Error::RuntimeIdentity);
        }
        if !row.units[index].dependencies.matches(found.record) {
            return Err(Error::RuntimeDependenciesChanged);
        }
        let current = self
            .world
            .player(owner)
            .map_err(Error::World)?
            .formation(found.record.formation_id)
            .map_err(Error::World)?;
        match (row.units[index].value.formation, current) {
            (None, None) => {}
            (Some(saved), Some(actual))
                if saved.owner == actual.record.owner
                    && saved.requested_id == actual.requested_id
                    && saved.resolved_id == actual.resolved_id => {}
            _ => return Err(Error::RuntimeDependenciesChanged),
        }
        Ok(&row.units[index].value)
    }
    pub fn formation_runtime(&self, owner: i32, id: u32) -> Result<&FormationRuntime, Error> {
        let row = self
            .formation_runtime
            .as_ref()
            .ok_or(Error::Missing(MissingInput::FormationRuntime))?
            .iter()
            .find(|p| p.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        let index = row
            .formation_index
            .get((id & 0xffff) as usize)
            .copied()
            .flatten()
            .filter(|_| id != u32::MAX)
            .ok_or(Error::Missing(MissingInput::Formation { owner, id }))?;
        let found = self
            .world
            .player(owner)
            .map_err(Error::World)?
            .formation(id)
            .map_err(Error::World)?
            .ok_or(Error::Missing(MissingInput::Formation { owner, id }))?;
        let identity = row.formations[index].archive.identity();
        if identity.owner as i32 != found.record.owner || identity.id != found.record.id {
            return Err(Error::RuntimeIdentity);
        }
        Ok(&row.formations[index])
    }
    pub fn unit_with_runtime_mut(
        &mut self,
        owner: i32,
        id: u32,
    ) -> Result<(&mut unit::Unit, &mut UnitRuntime), Error> {
        self.unit_runtime(owner, id)?;
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        let row = self
            .runtime
            .as_mut()
            .ok_or(Error::Missing(MissingInput::UnitRuntime))?
            .iter_mut()
            .find(|p| p.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        let index = row
            .unit_index
            .get((id & 0xffff) as usize)
            .copied()
            .flatten()
            .filter(|_| id != u32::MAX)
            .ok_or(Error::Missing(MissingInput::Unit { owner, id }))?;
        let unit = self
            .world
            .player_mut(owner)
            .map_err(Error::World)?
            .unit_mut(id)
            .map_err(Error::World)?
            .ok_or(Error::Missing(MissingInput::Unit { owner, id }))?;
        let identity = row.units[index].value.archive.identity();
        if identity.owner as i32 != unit.e_owner || identity.id != unit.id {
            return Err(Error::RuntimeIdentity);
        }
        self.revision = next;
        Ok((unit, &mut row.units[index].value))
    }
    pub fn formation_with_runtime_mut(
        &mut self,
        owner: i32,
        id: u32,
    ) -> Result<(&mut unit::Formation, &mut FormationRuntime), Error> {
        self.formation_runtime(owner, id)?;
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        let row = self
            .formation_runtime
            .as_mut()
            .ok_or(Error::Missing(MissingInput::FormationRuntime))?
            .iter_mut()
            .find(|p| p.owner == owner)
            .ok_or(Error::Missing(MissingInput::Player(owner)))?;
        let index = row
            .formation_index
            .get((id & 0xffff) as usize)
            .copied()
            .flatten()
            .filter(|_| id != u32::MAX)
            .ok_or(Error::Missing(MissingInput::Formation { owner, id }))?;
        let formation = self
            .world
            .player_mut(owner)
            .map_err(Error::World)?
            .formation_mut(id)
            .map_err(Error::World)?
            .ok_or(Error::Missing(MissingInput::Formation { owner, id }))?;
        let identity = row.formations[index].archive.identity();
        if identity.owner as i32 != formation.owner || identity.id != formation.id {
            return Err(Error::RuntimeIdentity);
        }
        self.revision = next;
        Ok((formation, &mut row.formations[index]))
    }
}
