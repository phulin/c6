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
        let player = crate::core_player::tests::source_first_player();
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
            limits: AssemblyLimits::default(),
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
        state.bind_controls(absent).unwrap();
        assert!(state.world.player(owner).unwrap().is_alive.is_none());
        assert!(matches!(
            state.player_controls(owner),
            Err(Error::Missing(MissingInput::PlayerConfiguration(_)))
        ));
        assert!(matches!(
            state.pause_storage(),
            Err(Error::Missing(MissingInput::GamePause))
        ));
        assert!(matches!(
            state.unit_runtime(owner, 0),
            Err(Error::Missing(MissingInput::UnitRuntime))
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
    RuntimeIdentity,
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
    units: Vec<UnitRuntime>,
    formation_index: Vec<Option<usize>>,
    formations: Vec<FormationRuntime>,
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
    limits: AssemblyLimits,
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
            limits,
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

    /// Validate every control before changing any World field. Saved membership
    /// does not supply Manager validity or nonnull-slot facts.
    pub fn bind_controls(&mut self, controls: ControlInputs) -> Result<(), Error> {
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
        // Existing derived owner bindings must be rebuilt when controls change.
        self.runtime = None;
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
        if inputs.identities != &self.binding.inputs {
            return Err(Error::Binding);
        }
        let slots = &self
            .controls
            .as_ref()
            .ok_or(Error::Missing(MissingInput::ManagerSlots))?
            .player_slots;
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
        if array_count
            .checked_mul(self.statistics.units)
            .is_none_or(|n| n > self.limits.max_runtime_array_items)
        {
            return Err(Error::ResourceLimit);
        }
        let mut runtime = reserved(self.world.players().len())?;
        for player in self.world.players() {
            let unit_records = player.units().map_err(Error::World)?;
            let formation_records = player.formations().map_err(Error::World)?;
            let mut units = reserved(unit_records.entries.len())?;
            for u in &unit_records.entries {
                units.push(
                    UnitRuntime::from_read(&u.value, &self.binding.inputs, inputs, player)
                        .map_err(Error::UnitInitialization)?,
                );
            }
            let mut formations = reserved(formation_records.entries.len())?;
            for f in &formation_records.entries {
                formations.push(FormationRuntime::from_read(&f.value, |owner| {
                    let i = usize::try_from(owner)
                        .ok()
                        .filter(|i| *i < 65)
                        .ok_or(Error::FormationLookup)?;
                    if !slots[i] {
                        return Ok(None);
                    }
                    Ok(Some(
                        self.world.player(owner).map_err(Error::World)?.owner(),
                    ))
                })?);
            }
            runtime.push(RuntimePlayer {
                owner: player.owner(),
                unit_index: slot_index(unit_records.capacity, &unit_records.entries)?,
                units,
                formation_index: slot_index(
                    formation_records.capacity,
                    &formation_records.entries,
                )?,
                formations,
            });
        }
        self.runtime = Some(runtime);
        Ok(())
    }
    /// Existing pure legality queries can use the actual imported Unit without
    /// requiring unrelated UI or native dirty-tree machinery.
    pub fn unit_mut(&mut self, owner: i32, id: u32) -> Result<&mut unit::Unit, Error> {
        self.world
            .player_mut(owner)
            .map_err(Error::World)?
            .unit_mut(id)
            .map_err(Error::World)?
            .ok_or(Error::Missing(MissingInput::Unit { owner, id }))
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
        let identity = row.units[index].archive.identity();
        if identity.owner as i32 != found.record.e_owner || identity.id != found.record.id {
            return Err(Error::RuntimeIdentity);
        }
        Ok(&row.units[index])
    }
    pub fn formation_runtime(&self, owner: i32, id: u32) -> Result<&FormationRuntime, Error> {
        let row = self
            .runtime
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
        let identity = row.units[index].archive.identity();
        if identity.owner as i32 != unit.e_owner || identity.id != unit.id {
            return Err(Error::RuntimeIdentity);
        }
        Ok((unit, &mut row.units[index]))
    }
    pub fn formation_with_runtime_mut(
        &mut self,
        owner: i32,
        id: u32,
    ) -> Result<(&mut unit::Formation, &mut FormationRuntime), Error> {
        let row = self
            .runtime
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
        Ok((formation, &mut row.formations[index]))
    }
}
