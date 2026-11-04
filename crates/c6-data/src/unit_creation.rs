//! Ordinary fresh Unit initialization over the current canonical World.
//! The native source orders MaxMoves, static charges, abilities and promotions;
//! later unsupported leaves retain that prefix and never count as completion.
use crate::{
    registry::LinuxDefinitionRegistry,
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{self, collection},
    unit_movement_state, Rules,
};
use c6_core::{
    core_player::initialization::InputIdentities,
    loaded_state::LoadedState,
    movement_world::DefinitionLookup,
    unit::{ComponentId, Unit},
    unit_creation as kernel,
};
#[derive(Debug)]
pub enum Error<E> {
    Resources(unit_initialization_definitions::Error),
    Rules(crate::Error),
    State(c6_core::loaded_state::Error),
    Storage(c6_core::loaded_state::unit_creation::CreationError),
    World(c6_core::world::MissingInput),
    Formation(c6_core::formation_upkeep::FormationUpkeepError<crate::loaded_upkeep::Error<E>>),
    Placement(Box<crate::unit_placement_state::Error<E>>),
    Movement(unit_movement_state::Error),
    Services(crate::unit_placement_services::Error<E>),
    Cost(crate::formation_movement_cost::Error),
    Constructor(kernel::Error),
    DefaultAi(kernel::default_ai::Error),
    Missing(&'static str),
    Effects(E),
}
#[derive(Clone, Copy)]
struct UnitProperties {
    base: kernel::Properties,
    initial_level: i32,
    ranged: i32,
    bombard: i32,
    spy: bool,
    artifacts: bool,
    track_religion: bool,
    can_form_military: Option<bool>,
}
pub struct Definitions {
    identities: InputIdentities,
    units: LinuxDefinitionRegistry,
    properties: Vec<UnitProperties>,
    initialization: unit_initialization_definitions::Definitions,
    promotions: LinuxDefinitionRegistry,
    districts: LinuxDefinitionRegistry,
    improvements: LinuxDefinitionRegistry,
    yields: LinuxDefinitionRegistry,
    ability_inactive: Vec<bool>,
    hero_units: Vec<u32>,
    operations: Vec<(u32, bool)>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, Error<std::convert::Infallible>> {
        if bound.destination_binding() != state.binding() {
            return Err(Error::Missing("Unit creation rules binding"));
        }
        let initialization = unit_initialization_definitions::Definitions::from_rules(rules, bound)
            .map_err(Error::Resources)?;
        let units = collection(rules, "Units", "UnitType").map_err(Error::Resources)?;
        let native = bound
            .unit_definitions()
            .map_err(|_| Error::Missing("bound Unit definitions"))?;
        if units.count() != native.lookup.count()
            || (0..units.count()).any(|i| units.type_hash(i) != native.lookup.type_hash(i))
        {
            return Err(Error::Missing("native Unit definition order"));
        }
        let table = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Missing("Units"))?;
        let mode = collection(rules, "Units_MODE", "UnitType").map_err(Error::Resources)?;
        let mode_table = rules
            .gameplay()
            .table("Units_MODE")
            .ok_or(Error::Missing("Units_MODE"))?;
        let xp2_units = collection(rules, "Units_XP2", "UnitType").map_err(Error::Resources)?;
        let xp2_units_table = rules
            .gameplay()
            .table("Units_XP2")
            .ok_or(Error::Missing("Units_XP2"))?;
        let mut properties = Vec::with_capacity(units.count());
        for entry in units.entries() {
            let row = table
                .row_by_id(entry.database_rowid)
                .ok_or(Error::Missing("Unit row"))?;
            let integer = |key| -> Result<i32, Error<std::convert::Infallible>> {
                i32::try_from(row.integer(key).map_err(Error::Rules)?)
                    .map_err(|_| Error::Missing("Unit i32 field"))
            };
            let hash = crate::registry::linux_type_hash(&entry.type_name);
            let action_charges = if let Some(index) = mode.find_index(hash) {
                let row = mode_table
                    .row_by_id(mode.entries()[index].database_rowid)
                    .ok_or(Error::Missing("Unit_MODE row"))?;
                i32::try_from(row.integer("ActionCharges").map_err(Error::Rules)?)
                    .map_err(|_| Error::Missing("ActionCharges"))?
            } else {
                0
            };
            properties.push(UnitProperties {
                base: kernel::Properties {
                    type_hash: hash,
                    combat: integer("Combat")?,
                    sight: integer("BaseSightRange")?,
                    range: integer("Range")?,
                    build_charges: integer("BuildCharges")?,
                    park_charges: integer("ParkCharges")?,
                    disaster_charges: integer("DisasterCharges")?,
                    action_charges,
                    religious_strength: integer("ReligiousStrength")?,
                    spread_charges: integer("SpreadCharges")?,
                    religious_heal_charges: integer("ReligiousHealCharges")?,
                    religion_evict_percent: integer("ReligionEvictPercent")?,
                    zone_of_control: integer("ZoneOfControl")? != 0,
                },
                initial_level: integer("InitialLevel")?,
                ranged: integer("RangedCombat")?,
                bombard: integer("Bombard")?,
                spy: integer("Spy")? != 0,
                artifacts: integer("ExtractsArtifacts")? != 0,
                track_religion: integer("TrackReligion")? != 0,
                can_form_military: xp2_units
                    .find_index(hash)
                    .map(|index| {
                        xp2_units_table
                            .row_by_id(xp2_units.entries()[index].database_rowid)
                            .ok_or(Error::Missing("Unit_XP2 row"))?
                            .integer("CanFormMilitaryFormation")
                            .map(|value| value != 0)
                            .map_err(Error::Rules)
                    })
                    .transpose()?,
            });
        }
        let abilities = rules
            .gameplay()
            .table("UnitAbilities")
            .ok_or(Error::Missing("UnitAbilities"))?;
        let ability_inactive = initialization
            .abilities()
            .entries()
            .iter()
            .map(|entry| {
                let row = abilities
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Missing("UnitAbility row"))?;
                Ok(row.integer("Inactive").map_err(Error::Rules)? != 0)
            })
            .collect::<Result<Vec<_>, Error<std::convert::Infallible>>>()?;
        let xp2 =
            collection(rules, "UnitOperations_XP2", "OperationType").map_err(Error::Resources)?;
        let xp2_table = rules
            .gameplay()
            .table("UnitOperations_XP2")
            .ok_or(Error::Missing("UnitOperations_XP2"))?;
        let operations = (0..bound.operation_lookup().count())
            .map(|index| {
                let hash = bound
                    .operation_lookup()
                    .type_hash(index)
                    .ok_or(Error::Missing("Operation hash"))?;
                let disabled = if let Some(at) = xp2.find_index(hash) {
                    xp2_table
                        .row_by_id(xp2.entries()[at].database_rowid)
                        .ok_or(Error::Missing("Operation_XP2 row"))?
                        .integer("IsDisabled")
                        .map_err(Error::Rules)?
                        != 0
                } else {
                    false
                };
                Ok((hash, disabled))
            })
            .collect::<Result<_, Error<std::convert::Infallible>>>()?;
        let hero_table = rules
            .gameplay()
            .table("HeroClasses")
            .ok_or(Error::Missing("HeroClasses"))?;
        let hero_units = hero_table
            .rows()
            .map(|row| {
                row.text("UnitType")
                    .map(|name| crate::registry::linux_type_hash(name.as_bytes()))
                    .map_err(Error::Rules)
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            hero_units,
            identities: state.binding().inputs.clone(),
            units,
            properties,
            initialization,
            promotions: collection(rules, "UnitPromotions", "UnitPromotionType")
                .map_err(Error::Resources)?,
            districts: collection(rules, "Districts", "DistrictType").map_err(Error::Resources)?,
            improvements: collection(rules, "Improvements", "ImprovementType")
                .map_err(Error::Resources)?,
            yields: collection(rules, "Yields", "YieldType").map_err(Error::Resources)?,
            ability_inactive,
            operations,
        })
    }
}
/// Observed current GlobalAI state, not a default AI result or copied Unit AI.
pub enum GlobalAi {
    Absent,
    Present {
        initialization_state: i32,
        role_words: Option<Vec<u32>>,
    },
}
pub struct DefinitionProperties {
    pub native_unit_index: usize,
    pub values: c6_core::unit::variants::TypedVariantMap,
}
pub struct Runtime {
    pub binding: c6_core::core_player::initialization::WireBinding,
    pub global_ai: Option<GlobalAi>,
    /// Complete current Definition.IProperties maps, not a lifespan query answer.
    pub definition_properties: Option<Vec<DefinitionProperties>>,
}
/// Specific synchronous signal leaves; unimplemented consumers remain errors.
pub trait Effects {
    type Error;
    fn ability_changed(
        &mut self,
        state: &mut LoadedState,
        unit: ComponentId,
        ability_index: usize,
    ) -> Result<(), Error<Self::Error>>;
}
fn actor<E>(state: &LoadedState, owner: i32, id: u32) -> Result<&Unit, Error<E>> {
    let found = state
        .world()
        .player(owner)
        .map_err(Error::World)?
        .unit(id)
        .map_err(Error::World)?
        .ok_or(Error::Missing("created Unit"))?;
    if found.record.id != id {
        return Err(Error::Missing("created Unit full identity"));
    }
    Ok(found.record)
}
/// Only the fresh Unit.Initialize prefix. Caller still owes Formation creation,
/// RestoreMovement and Player/Game creation signals before City placement.
#[allow(clippy::too_many_arguments)]
pub fn initialize<E: Effects>(
    state: &mut LoadedState,
    definitions: &Definitions,
    bound: &ResourceAuthority<'_>,
    movement: &unit_movement_state::Inputs<'_>,
    combat: &crate::formation_rules::Definitions,
    runtime: &Runtime,
    tags: &crate::loaded_actions::reporting::hold_cycling::commands::Tags,
    owner: i32,
    unit_hash: u32,
    effects: &mut E,
) -> Result<Option<ComponentId>, Error<E::Error>> {
    if definitions.identities != state.binding().inputs
        || runtime.binding != *state.binding()
        || bound.destination_binding() != state.binding()
    {
        return Err(Error::Missing("Unit creation binding"));
    }
    let Some(index) = definitions.units.find_index(unit_hash) else {
        return Ok(None);
    };
    let properties = definitions.properties[index];
    // Add's full-container return precedes Unit.Initialize/DefaultAI. Inspect
    // the same authoritative allocator only to preserve this lazy early out;
    // reserve still validates its complete metadata before returning None.
    let allocator = state
        .world()
        .player(owner)
        .map_err(Error::World)?
        .units()
        .map_err(Error::World)?;
    if allocator.capacity == 65536 && allocator.free_head == u32::MAX {
        let allocation = state.reserve_unit_creation(owner).map_err(Error::Storage)?;
        if allocation.is_some() {
            return Err(Error::Missing("Unit allocator full early-out mismatch"));
        }
        return Ok(None);
    }
    let default_ai = match runtime.global_ai.as_ref().ok_or(Error::Missing(
        "current GlobalAI initialization/role matrix",
    ))? {
        GlobalAi::Absent => -1,
        GlobalAi::Present {
            initialization_state,
            role_words,
        } => {
            let words = if *initialization_state == 2 {
                role_words
                    .as_deref()
                    .ok_or(Error::Missing("initialized GlobalAI role matrix"))?
            } else {
                &[]
            };
            kernel::default_ai::get(true, Some(*initialization_state), words, index)
                .map_err(Error::DefaultAi)?
        }
    };
    let Some(allocation) = state.reserve_unit_creation(owner).map_err(Error::Storage)? else {
        return Ok(None);
    };
    let hashes = |lookup: &dyn DefinitionLookup| -> Result<Vec<u32>, Error<E::Error>> {
        (0..lookup.count())
            .map(|index| {
                lookup
                    .type_hash(index)
                    .ok_or(Error::Missing("native indexed definition hash"))
            })
            .collect()
    };
    let abilities = hashes(definitions.initialization.abilities())?;
    let terrains = hashes(definitions.initialization.terrains())?;
    let features = hashes(definitions.initialization.features())?;
    let great_person_classes = hashes(definitions.initialization.great_person_classes())?;
    let promotions = hashes(&definitions.promotions)?;
    let districts = hashes(&definitions.districts)?;
    let improvements = hashes(&definitions.improvements)?;
    let yields = hashes(&definitions.yields)?;
    let arrays = kernel::Definitions {
        abilities: &abilities,
        terrains: &terrains,
        features: &features,
        great_person_classes: &great_person_classes,
        promotions: &promotions,
        districts: &districts,
        improvements: &improvements,
        yields: &yields,
        operations: &definitions.operations,
        max_array_items: 16_000_000,
    };
    let constructor_definition = definitions
        .properties
        .first()
        .ok_or(Error::Missing("default Unit definition0"))?
        .base;
    let attacks = combat
        .combat_max_num_attacks(state)
        .map_err(|_| Error::Missing("configured maximum Unit attacks"))?;
    let unit = kernel::constructor::before_max_moves(
        owner,
        allocation.id,
        &properties.base,
        &constructor_definition,
        &arrays,
        default_ai,
        attacks,
    )
    .map_err(Error::Constructor)?;
    state
        .insert_created_unit(owner, allocation, unit)
        .map_err(Error::Storage)?;
    let inputs = c6_core::unit_initialization::InitializationInputs {
        identities: &definitions.identities,
        units: bound
            .unit_definitions()
            .map_err(|_| Error::Missing("bound Unit definitions"))?,
        abilities: definitions.initialization.abilities(),
        terrains: definitions.initialization.terrains(),
        features: definitions.initialization.features(),
        great_person_classes: definitions.initialization.great_person_classes(),
        operations: bound.operation_lookup(),
    };
    state
        .register_created_unit_prefix(owner, allocation.id, &inputs)
        .map_err(Error::State)?;
    let maximum =
        unit_movement_state::unit_max_moves(state, actor(state, owner, allocation.id)?, movement)
            .map_err(Error::Movement)?;
    kernel::constructor::after_max_moves(
        state.unit_mut(owner, allocation.id).map_err(Error::State)?,
        &properties.base,
        maximum,
    );
    // Source SetInvalidTerrain(16), after static nested initialization and
    // before callbacks. Shared setters keep native-index arrays and projection
    // dependencies current on the same canonical Unit.
    let terrain_hash = *terrains
        .get(16)
        .ok_or(Error::Missing("Terrain16 definition"))?;
    state
        .set_unit_invalid_terrain(owner, allocation.id, 16, terrain_hash, true)
        .map_err(Error::State)?;
    kernel::abilities::add_initial(&mut Abilities {
        state,
        definitions,
        tags,
        effects,
        owner,
        id: allocation.id,
    })?;
    // AssignBasePromotions retains the native type selected before this loop.
    let current = actor(state, owner, allocation.id)?;
    let current_type = usize::try_from(
        state
            .unit_runtime(owner, allocation.id)
            .map_err(Error::State)?
            .definition_index,
    )
    .map_err(|_| Error::Missing("base promotion Unit type"))?;
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == current.e_owner)
        .ok_or(Error::Missing("base promotion owner"))?;
    for promotion in &player.units.base_promotions {
        if definitions.units.find_index(promotion.unit_hash) == Some(current_type)
            && promotion.count > 0
        {
            let Some(at) = definitions.promotions.find_index(promotion.promotion_hash) else {
                return Err(Error::Missing("unknown initial promotion setter"));
            };
            if !current
                .k_experience
                .ab_promotions_earned
                .get(at)
                .is_some_and(|p| p.value)
            {
                return Err(Error::Missing("initial Experience.SetPromotion effects"));
            }
        }
    }
    if properties.initial_level >= 2 {
        return Err(Error::Missing("initial-level scratch/experience effects"));
    }
    if properties.spy || properties.artifacts {
        return Err(Error::Missing(
            "specialized Unit naming/initialization continuation",
        ));
    }
    // GetUnitBaseLifespan unconditionally reads current Definition properties;
    // successful static initialization alone does not prove that map is empty.
    if definitions.hero_units.contains(&properties.base.type_hash) {
        return Err(Error::Missing("initial Unit.Hero.Set effects"));
    }
    let maps = runtime
        .definition_properties
        .as_ref()
        .ok_or(Error::Missing("current Unit definition property maps"))?;
    let mut matching = maps.iter().filter(|row| row.native_unit_index == index);
    let current = matching
        .next()
        .ok_or(Error::Missing("current Unit definition property map"))?;
    if matching.next().is_some() {
        return Err(Error::Missing("duplicate Unit definition property map"));
    }
    if current
        .values
        .entries
        .iter()
        .any(|entry| entry.key == 0xb16c65f9)
    {
        return Err(Error::Missing(
            "GameSpeed-scaled lifespan property conversion",
        ));
    }
    Ok(Some(ComponentId {
        owner: owner as i16,
        component_type: 1,
        id: allocation.id,
    }))
}
struct Abilities<'a, E> {
    state: &'a mut LoadedState,
    definitions: &'a Definitions,
    tags: &'a crate::loaded_actions::reporting::hold_cycling::commands::Tags,
    effects: &'a mut E,
    owner: i32,
    id: u32,
}
impl<E: Effects> kernel::abilities::Context for Abilities<'_, E> {
    type Error = Error<E::Error>;
    fn ability_count(&self) -> Result<usize, Self::Error> {
        Ok(self
            .state
            .unit_runtime(self.owner, self.id)
            .map_err(Error::State)?
            .abilities
            .len())
    }
    fn collection_index(&self, index: usize) -> Result<usize, Self::Error> {
        if index >= self.definitions.initialization.abilities().count() {
            return Err(Error::Missing("ability collection index"));
        }
        Ok(index)
    }
    fn can_have(&self, index: usize) -> Result<bool, Self::Error> {
        let unit = actor(self.state, self.owner, self.id)?;
        let Some(unit_index) = self.definitions.units.find_index(unit.e_type_index) else {
            return Ok(false);
        };
        let Some(hash) = self.definitions.initialization.abilities().type_hash(index) else {
            return Ok(false);
        };
        if self.tags.has(hash, b"CLASS_ALL_UNITS") {
            return Ok(true);
        }
        // Source combat/ranged/bombard tests are independent of a combat class.
        let properties = self.definitions.properties[unit_index];
        if self.tags.has(hash, b"CLASS_ALL_COMBAT_UNITS") {
            if properties.base.combat > 0 {
                return Ok(true);
            }
            // These static fields are retained explicitly by the rules adapter.
            if properties.ranged > 0 || properties.bombard > 0 {
                return Ok(true);
            }
        }
        self.tags
            .shares_named_type_tags(
                &self.definitions.units.entries()[unit_index].type_name,
                hash,
            )
            .map_err(|_| Error::Missing("current Tags full-name lookup"))
    }
    fn inactive(&self, index: usize) -> Result<bool, Self::Error> {
        self.definitions
            .ability_inactive
            .get(index)
            .copied()
            .ok_or(Error::Missing("ability Inactive"))
    }
    fn count(&self, index: usize) -> Result<i32, Self::Error> {
        self.state
            .unit_runtime(self.owner, self.id)
            .map_err(Error::State)?
            .abilities
            .get(index)
            .copied()
            .ok_or(Error::Missing("ability count"))
    }
    fn store_count(&mut self, index: usize, count: i32) -> Result<(), Self::Error> {
        let hash = self
            .definitions
            .initialization
            .abilities()
            .type_hash(index)
            .ok_or(Error::Missing("ability definition hash"))?;
        self.state
            .set_unit_ability_count(self.owner, self.id, index, hash, count)
            .map_err(Error::State)
    }
    fn changed(&mut self, index: usize) -> Result<(), Self::Error> {
        self.effects.ability_changed(
            self.state,
            ComponentId {
                owner: self.owner as i16,
                component_type: 1,
                id: self.id,
            },
            index,
        )
    }
}

/// Exact signal leaves surrounding concrete Player.Units.Create/City.CreateUnit.
/// Ordinary callbacks run on this same state; no whole factory is delegated.
pub trait PlayerCreationEffects:
    Effects + crate::loaded_upkeep::FormationRestoreEffects<Error = <Self as Effects>::Error>
{
    fn game_unit_created(
        &mut self,
        state: &mut LoadedState,
        unit: Option<ComponentId>,
        city: ComponentId,
    ) -> Result<(), Error<<Self as Effects>::Error>>;
    fn player_unit_created(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        unit: Option<ComponentId>,
    ) -> Result<(), Error<<Self as Effects>::Error>>;
}
pub trait FactoryEffects:
    PlayerCreationEffects
    + crate::unit_placement_state::Effects
    + crate::unit_sight_state::Effects<Error = <Self as Effects>::Error>
{
    fn city_unit_created(
        &mut self,
        state: &mut LoadedState,
        unit: ComponentId,
        city: ComponentId,
    ) -> Result<(), Error<<Self as Effects>::Error>>;
}
/// Player.Units.Create: allocation, initialization, formation and synchronous
/// creation signals on the same canonical state. Placement belongs to the caller.
pub struct PlayerContext<'a, 'd, 'r, E> {
    pub state: &'a mut LoadedState,
    pub definitions: &'a Definitions,
    pub resources: &'a ResourceAuthority<'r>,
    pub movement: &'a unit_movement_state::Inputs<'d>,
    pub combat: &'a crate::formation_rules::Definitions,
    pub runtime: &'a Runtime,
    pub tags: &'a crate::loaded_actions::reporting::hold_cycling::commands::Tags,
    pub effects: &'a mut E,
}
impl<E: PlayerCreationEffects> PlayerContext<'_, '_, '_, E> {
    pub fn create(
        &mut self,
        owner: i32,
        unit_hash: u32,
        city: ComponentId,
    ) -> Result<Option<ComponentId>, Error<<E as Effects>::Error>> {
        // Native Find-definition failure precedes allocation and both signals.
        if self.definitions.units.find_index(unit_hash).is_none() {
            return Ok(None);
        }
        let created = initialize(
            self.state,
            self.definitions,
            self.resources,
            self.movement,
            self.combat,
            self.runtime,
            self.tags,
            owner,
            unit_hash,
            self.effects,
        )?;
        if let Some(unit) = created {
            self.state
                .record_created_unit_peak(owner)
                .map_err(Error::State)?;
            if let Some(allocation) = self
                .state
                .reserve_formation_creation(owner)
                .map_err(Error::Storage)?
            {
                let formation = kernel::constructor::formation(owner, allocation.id);
                self.state
                    .insert_created_formation(owner, allocation, formation)
                    .map_err(Error::Storage)?;
                self.state
                    .register_created_formation(owner, allocation.id)
                    .map_err(Error::State)?;
                // CanAddUnit(current nonnull Unit) accepts the empty list before
                // any domain/combat predicates. Native appends before SetFormation.
                self.state
                    .formation_mut(owner, allocation.id)
                    .map_err(Error::State)?
                    .unit_ids
                    .push(unit.id);
                self.state
                    .attach_created_unit(owner, unit.id, allocation.id)
                    .map_err(Error::State)?;
                crate::loaded_upkeep::update_formation(
                    self.state,
                    owner,
                    allocation.id,
                    self.movement,
                    self.effects,
                )
                .map_err(Error::Formation)?;
            }
            if let Some(formation) = self
                .state
                .unit_runtime(owner, unit.id)
                .map_err(Error::State)?
                .formation
            {
                crate::loaded_upkeep::restore_formation_movement(
                    self.state,
                    formation.owner,
                    formation.requested_id,
                    self.movement,
                    self.effects,
                )
                .map_err(Error::Formation)?;
            }
        }
        // Allocation failure still reaches both native creation signals; failed
        // initial definition lookup returned earlier without these signals.
        self.effects.game_unit_created(self.state, created, city)?;
        self.effects
            .player_unit_created(self.state, owner, created)?;
        Ok(created)
    }
}
pub struct Parameters {
    pub city: ComponentId,
    pub unit_hash: u32,
    pub military_formation: i32,
    pub location: c6_core::unit::PlotCoord,
}
pub struct Context<'a, 'd, 'r, E> {
    pub state: &'a mut LoadedState,
    pub definitions: &'a Definitions,
    pub resources: &'a ResourceAuthority<'r>,
    pub movement: &'a unit_movement_state::Inputs<'d>,
    pub combat: &'a crate::formation_rules::Definitions,
    pub costs: &'a crate::research_unit_costs::Definitions,
    pub placement: &'a crate::unit_placement_state::Definitions<'d, 'r>,
    pub queries: &'a crate::unit_placement_state::Queries<'d>,
    pub runtime: &'a Runtime,
    pub tags: &'a crate::loaded_actions::reporting::hold_cycling::commands::Tags,
    pub sight: &'a mut crate::unit_sight_state::Runtime,
    pub effects: &'a mut E,
}
impl<E: FactoryEffects> Context<'_, '_, '_, E> {
    /// Ordinary Builder completion. Reuses canonical allocation, actual Unit
    /// getters, Formation restoration, placement/occupancy/sight and signals.
    /// Other unit classes return explicit unsupported branches, never dummy units.
    pub fn create_from_city(
        &mut self,
        parameters: Parameters,
    ) -> Result<Option<ComponentId>, Error<<E as Effects>::Error>> {
        let city = self
            .state
            .city(i32::from(parameters.city.owner), parameters.city.id)
            .map_err(Error::State)?;
        if parameters.city.component_type != 2 {
            return Err(Error::Missing("City.CreateUnit City identity"));
        }
        let owner = city.header.owner;
        let Some(index) = self.definitions.units.find_index(parameters.unit_hash) else {
            return Ok(None);
        };
        if self.definitions.units.entries()[index].type_name != b"UNIT_BUILDER" {
            return Err(Error::Missing("ordinary Builder creation profile"));
        }
        let properties = self.definitions.properties[index];
        if properties.base.combat > 0 {
            return Err(Error::Missing("combat Unit district spawn selection"));
        }
        if properties.track_religion {
            return Err(Error::Missing("religious Unit city religion selection"));
        }
        // The target HeroesManager.IsHeroUnitSpawnAtHolySite2658c00 is xor eax;ret.
        // No current capture answer supplies this false result.
        if !c6_core::skip_turn::placement::coordinate_valid(parameters.location) {
            return Ok(None);
        }
        let created = PlayerContext {
            state: self.state,
            definitions: self.definitions,
            resources: self.resources,
            movement: self.movement,
            combat: self.combat,
            runtime: self.runtime,
            tags: self.tags,
            effects: self.effects,
        }
        .create(owner, parameters.unit_hash, parameters.city)?;
        let Some(unit) = created else { return Ok(None) };
        let previous = actor(self.state, owner, unit.id)?.e_military_formation;
        if previous != parameters.military_formation {
            self.state
                .unit_mut(owner, unit.id)
                .map_err(Error::State)?
                .e_military_formation = parameters.military_formation;
            // Setter2c27440 emits report then Game1698/16b0 only for 2/1.
            // Ordinary Builder formation0 has no such callback branch.
            if matches!(parameters.military_formation, 1 | 2) {
                return Err(Error::Missing("military formation report/signal effects"));
            }
        }
        if let Some(plot) =
            crate::unit_placement_services::plot_index_at(self.state, parameters.location)
                .map_err(Error::Services)?
        {
            let plot = self
                .state
                .world()
                .map()
                .plots
                .get(plot as usize)
                .ok_or(Error::Missing("Unit spawn plot"))?;
            if let Some(district) = self
                .placement
                .movement
                .district_at(self.state, plot)
                .map_err(Error::Cost)?
            {
                if district.is_complete
                    && self
                        .state
                        .unit_runtime(owner, unit.id)
                        .map_err(Error::State)?
                        .domain
                        == 1
                {
                    return Err(Error::Missing("AIR creation district pairing"));
                }
            }
        }
        // City.CreateUnit ignores the Set return, including validation rejection.
        let _ = crate::unit_placement_state::creation::set(
            self.state,
            self.placement,
            self.queries,
            self.sight,
            self.effects,
            crate::unit_placement_state::Request {
                actor: (owner, unit.id),
                destination: parameters.location,
                validation_flags: 0,
            },
            self.tags,
        )
        .map_err(|error| Error::Placement(Box::new(error)))?;
        let current_city = self
            .state
            .city(i32::from(parameters.city.owner), parameters.city.id)
            .map_err(Error::State)?;
        if (0..64).contains(&current_city.header.owner) {
            self.effects
                .city_unit_created(self.state, unit, parameters.city)?;
        }
        self.record_constructed(owner, unit.id)?;
        self.recompute_cost(owner, unit.id)?;
        Ok(Some(unit))
    }
    fn record_constructed(
        &mut self,
        owner: i32,
        id: u32,
    ) -> Result<(), Error<<E as Effects>::Error>> {
        let current = actor(self.state, owner, id)?;
        let index = self
            .state
            .unit_runtime(owner, id)
            .map_err(Error::State)?
            .definition_index;
        let index = usize::try_from(index)
            .map_err(|_| Error::Missing("Unit statistics definition index"))?;
        let properties = self
            .definitions
            .properties
            .get(index)
            .ok_or(Error::Missing("Unit statistics definition"))?;
        if self
            .state
            .unit_runtime(owner, id)
            .map_err(Error::State)?
            .domain
            != 1
        {
            let maps = self
                .runtime
                .definition_properties
                .as_ref()
                .ok_or(Error::Missing("Unit statistics definition properties"))?;
            let map = maps
                .iter()
                .find(|map| map.native_unit_index == index)
                .ok_or(Error::Missing("Unit statistics definition property map"))?;
            if map
                .values
                .entries
                .iter()
                .any(|entry| entry.key == 0x36c125c9)
            {
                return Err(Error::Missing(
                    "Unit strength exclusion property conversion",
                ));
            }
            if properties.can_form_military != Some(false) {
                return Err(Error::Missing(
                    "combat Unit constructed max-strength effects",
                ));
            }
        }
        let hash = current.e_type_index;
        self.state
            .increment_units_constructed(owner, hash)
            .map_err(Error::Storage)
    }
    fn recompute_cost(&mut self, owner: i32, id: u32) -> Result<(), Error<<E as Effects>::Error>> {
        use c6_core::research_acquisition::units;
        let index = usize::try_from(
            self.state
                .unit_runtime(owner, id)
                .map_err(Error::State)?
                .definition_index,
        )
        .map_err(|_| Error::Missing("Unit cost definition index"))?;
        let definition = *self
            .costs
            .units()
            .get(index)
            .ok_or(Error::Missing("Unit cost definition"))?;
        if self.definitions.units.type_hash(index) != Some(definition.hash) {
            return Err(Error::Missing("Unit cost definition order"));
        }
        let model = units::progression_model(definition.progression_model_hash);
        let player = self
            .state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or(Error::Missing("Unit cost owner"))?;
        let constructed = if matches!(model, 0 | 2) {
            player
                .stats
                .num_units_constructed
                .iter()
                .rev()
                .find(|entry| entry.definition_hash == definition.hash)
                .ok_or(Error::Missing("Unit constructed cost count"))?
                .value
        } else {
            0
        };
        if matches!(model, 1 | 2) {
            return Err(Error::Missing("cost research graph percentages"));
        }
        let mut current = 0i32;
        if model == 6 {
            for entry in self
                .state
                .world()
                .player(owner)
                .map_err(Error::World)?
                .unit_records()
                .map_err(Error::World)?
            {
                let unit = &entry.value;
                if self
                    .state
                    .unit_runtime(owner, unit.id)
                    .map_err(Error::State)?
                    .definition_index
                    == index as i32
                    && !unit.b_delayed_death
                    && !unit.b_is_levied
                    && matches!(unit.e_military_formation, -1 | 0)
                {
                    current = current.wrapping_add(1);
                }
            }
        }
        // Models requiring research percentages returned above. These unused
        // arguments do not assert any current technology/civic percentage.
        let value = units::cost(
            definition,
            constructed,
            current,
            units::ResearchCompletion {
                technologies: 0,
                civics: 0,
            },
            self.costs.economic(),
        );
        let arrays = self
            .state
            .player_unit_costs_mut(owner)
            .map_err(Error::State)?;
        let at = arrays
            .production
            .iter()
            .rposition(|entry| entry.definition_hash == definition.hash)
            .ok_or(Error::Missing("Unit production cost cell"))?;
        arrays.production[at].value = value;
        Ok(())
    }
}
