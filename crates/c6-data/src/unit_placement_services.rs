//! Live adapters for the source-ordered Placement rule tails.
//! Runtime fields are owned current configuration/manager storage, not answers
//! to siege, contact or movement predicates. Reached unsupported effects fail.
use crate::{
    loaded_actions::city_range_attack,
    loaded_upkeep::placement_tails,
    registry::LinuxDefinitionRegistry,
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::{self, collection},
    unit_sight_state as sight, Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding,
    diplomacy_meeting as meeting,
    diplomacy_meeting::{FirstMeetingRequest, Report, VisibilitySource},
    game_diplomatic_relations as relations,
    loaded_state::LoadedState,
    map::{Plot, PlotFlag},
    movement_world::DefinitionLookup,
    player_diplomacy::GossipReports,
    unit::{ComponentId, PlotCoord, Unit},
    unit_placement_tails as tails,
    unit_placement_transition::PlacementReport,
    visibility_change::ViewContents,
    visibility_contents::{self as contents, Discovery, Notification, Signal},
};

pub mod siege;

pub type Actor = (i32, u32);
#[derive(Debug)]
pub enum Error<E> {
    Missing(&'static str),
    State(c6_core::loaded_state::Error),
    World(c6_core::world::MissingInput),
    Effect(E),
    Binding,
    AliveStorageChanged,
    Combat(crate::loaded_actions::Error),
    Zoc(crate::formation_movement_cost::Error),
    Placement(crate::movement_placement_state::Error),
    District(c6_core::loaded_state::district_initialization::DistrictError),
    Relations(c6_core::game_diplomatic_relations::Error),
    Meeting(Box<meeting::Error<Error<E>>>),
    Units(Box<tails::UnitsMovedError<Error<E>>>),
}

pub struct PlayerRuntime {
    pub owner: i32,
    pub civilization_level: Option<u32>,
    /// Actual initialized Visibility.Instance width, separate from layer width.
    pub native_visibility_width: Option<u32>,
    pub encountered_units: Option<Vec<bool>>,
}
/// Current GetAlive receiver. In-place writes are reread by retained cursors;
/// replacing its allocation invalidates earlier cursors explicitly.
#[derive(Debug)]
pub enum AliveBuildError<E = c6_core::loaded_state::Error> {
    MissingAllocationSlots,
    CurrentAlive(E),
    ResourceLimit,
}
fn ordered_alive<E>(
    slots: &[Option<i32>],
    mut alive: impl FnMut(i32) -> Result<bool, E>,
) -> Result<Vec<i32>, AliveBuildError<E>> {
    let slots = slots
        .get(..64)
        .ok_or(AliveBuildError::MissingAllocationSlots)?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(64)
        .map_err(|_| AliveBuildError::ResourceLimit)?;
    for owner in slots.iter().flatten() {
        if alive(*owner).map_err(AliveBuildError::CurrentAlive)? {
            values.push(*owner);
        }
    }
    Ok(values)
}
pub struct AlivePlayers {
    values: Vec<i32>,
}
impl AlivePlayers {
    /// Native InitializeLists2b51c10 scans physical slots0..63, skips nulls,
    /// then tests current IsAlive. Slot64 and logical player-ID sort are absent.
    pub fn from_current_allocation_slots(
        state: &LoadedState,
        slots: &[Option<i32>],
    ) -> Result<Self, AliveBuildError> {
        Ok(Self {
            values: ordered_alive(slots, |owner| state.player_alive(owner))?,
        })
    }

    pub fn new(values: Vec<i32>) -> Self {
        Self { values }
    }
    pub fn values(&self) -> &[i32] {
        &self.values
    }
    pub fn values_mut(&mut self) -> &mut [i32] {
        &mut self.values
    }
    fn begin(&self) -> (AliveToken, usize) {
        (
            AliveToken {
                address: self.values.as_ptr() as usize,
            },
            self.values.len(),
        )
    }
    fn at<E>(&self, token: AliveToken, index: usize) -> Result<i32, Error<E>> {
        if token.address != self.values.as_ptr() as usize {
            return Err(Error::AliveStorageChanged);
        }
        self.values
            .get(index)
            .copied()
            .ok_or(Error::AliveStorageChanged)
    }
}
pub struct Runtime {
    pub binding: WireBinding,
    pub alive_players: Option<AlivePlayers>,
    pub players: Vec<PlayerRuntime>,
    pub clans_active: Option<bool>,
}
#[derive(Clone, Copy)]
pub struct AliveToken {
    address: usize,
}
impl Runtime {
    fn check<E>(&self, state: &LoadedState) -> Result<(), Error<E>> {
        if self.binding != *state.binding() {
            return Err(Error::Binding);
        }
        Ok(())
    }
    pub fn alive_begin<E>(&self) -> Result<(AliveToken, usize), Error<E>> {
        Ok(self
            .alive_players
            .as_ref()
            .ok_or(Error::Missing("current Player.Manager.GetAlive storage"))?
            .begin())
    }
    pub fn alive_at<E>(&self, token: AliveToken, index: usize) -> Result<i32, Error<E>> {
        self.alive_players
            .as_ref()
            .ok_or(Error::Missing("current Player.Manager.GetAlive storage"))?
            .at(token, index)
    }
    pub fn player<E>(&self, owner: i32) -> Result<&PlayerRuntime, Error<E>> {
        self.players
            .iter()
            .find(|p| p.owner == owner)
            .ok_or(Error::Missing("current placement Player runtime"))
    }
    fn player_mut<E>(&mut self, owner: i32) -> Result<&mut PlayerRuntime, Error<E>> {
        self.players
            .iter_mut()
            .find(|p| p.owner == owner)
            .ok_or(Error::Missing("current placement Player runtime"))
    }
}

pub struct Definitions {
    binding: WireBinding,
    districts: LinuxDefinitionRegistry,
    improvements: LinuxDefinitionRegistry,
    camps: Vec<bool>,
    abilities: LinuxDefinitionRegistry,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, crate::loaded_actions::Error> {
        use crate::loaded_actions::Error;
        if bound.destination_binding() != state.binding() {
            return Err(Error::ConfigurationIdentity);
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, bound)
            .map_err(|_| Error::Unsupported("placement services resources"))?;
        let districts = collection(rules, "Districts", "DistrictType")
            .map_err(|_| Error::Unsupported("Districts registry"))?;
        let improvements = collection(rules, "Improvements", "ImprovementType")
            .map_err(|_| Error::Unsupported("Improvements registry"))?;
        let table = rules
            .gameplay()
            .table("Improvements")
            .ok_or(Error::Unsupported("Improvements"))?;
        let camps = improvements
            .entries()
            .iter()
            .map(|entry| {
                match table
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Unsupported("Improvements rowid"))?
                    .get("BarbarianCamp")
                    .map_err(|_| Error::Unsupported("BarbarianCamp"))?
                {
                    SqlValue::Integer(0) => Ok(false),
                    SqlValue::Integer(1) => Ok(true),
                    _ => Err(Error::Unsupported("BarbarianCamp scalar")),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            districts,
            improvements,
            camps,
            abilities: collection(rules, "UnitAbilities", "UnitAbilityType")
                .map_err(|_| Error::Unsupported("UnitAbilities registry"))?,
        })
    }
}

/// Only leaf services: a complete rules handler cannot be replaced by success.
/// Defaults reject missing gameplay/report implementations when reached.
pub trait Services {
    type Error;
    fn placement_service_runtime(&self) -> Result<&Runtime, Error<Self::Error>> {
        Err(Error::Missing("current placement service runtime"))
    }
    fn placement_service_runtime_mut(&mut self) -> Result<&mut Runtime, Error<Self::Error>> {
        Err(Error::Missing("current placement service runtime"))
    }
    fn placement_service_definitions(&self) -> Result<&Definitions, Error<Self::Error>> {
        Err(Error::Missing("admitted placement service definitions"))
    }
    /// Pure support admission immediately before the encountered flag write.
    fn preflight_encountered_unit(
        &self,
        _state: &LoadedState,
        _player: i32,
        _unit_type: i32,
    ) -> Result<(), Error<Self::Error>> {
        Err(Error::Missing("GameSignals.EncounteredUnit support"))
    }
    fn signal_encountered_unit(
        &mut self,
        _state: &mut LoadedState,
        _sight: &mut sight::Runtime,
        _player: i32,
        _unit_type: i32,
    ) -> Result<(), Error<Self::Error>> {
        Err(Error::Missing("GameSignals.EncounteredUnit dispatch"))
    }
    fn foreign_unit_moved_near_camp(
        &mut self,
        _state: &mut LoadedState,
        _sight: &mut sight::Runtime,
        _actor: Actor,
        _plot: u32,
    ) -> Result<(), Error<Self::Error>> {
        Err(Error::Missing("ClansManager.OnForeignUnitMovedNearCamp"))
    }
}

struct Context<'a, E> {
    state: &'a mut LoadedState,
    sight: &'a mut sight::Runtime,
    effects: &'a mut E,
    actor: Actor,
    encountered_type: Option<i32>,
}
impl<E: Services> Context<'_, E> {
    fn runtime(&self) -> Result<&Runtime, Error<E::Error>> {
        let runtime = self.effects.placement_service_runtime()?;
        runtime.check(self.state)?;
        Ok(runtime)
    }
    fn runtime_mut(&mut self) -> Result<&mut Runtime, Error<E::Error>> {
        let runtime = self.effects.placement_service_runtime_mut()?;
        runtime.check(self.state)?;
        Ok(runtime)
    }
    fn definition(&self) -> Result<&Definitions, Error<E::Error>> {
        let definitions = self.effects.placement_service_definitions()?;
        if definitions.binding != *self.state.binding() {
            return Err(Error::Binding);
        }
        Ok(definitions)
    }
    fn current_unit(&self, actor: Actor) -> Result<&Unit, Error<E::Error>> {
        self.state
            .world()
            .player(actor.0)
            .map_err(Error::World)?
            .unit(actor.1)
            .map_err(Error::World)?
            .map(|found| found.record)
            .ok_or(Error::Missing("current placement Unit"))
    }
    fn current_plot(&self, plot: u32) -> Result<&Plot, Error<E::Error>> {
        let p = self
            .state
            .world()
            .map()
            .plot_by_index(plot)
            .ok_or(Error::Missing("current placement Plot"))?;
        if p.flags.get(PlotFlag::HasCityAttributes) != p.city_attributes.is_some() {
            return Err(Error::Missing("current Plot city attributes"));
        }
        Ok(p)
    }
    fn plot_at_coord(&self, location: PlotCoord) -> Result<Option<u32>, Error<E::Error>> {
        plot_index_at(self.state, location)
    }

    fn level(&self, player: i32) -> Result<u32, Error<E::Error>> {
        self.runtime()?
            .player(player)?
            .civilization_level
            .ok_or(Error::Missing("current player civilization level"))
    }
    fn visible(&self, viewer: i32, location: PlotCoord) -> Result<bool, Error<E::Error>> {
        let width = self
            .runtime()?
            .player(viewer)?
            .native_visibility_width
            .ok_or(Error::Missing("current Visibility.Instance width"))?;
        placement_tails::plot_visible(self.sight, viewer, width, location).map_err(Error::Missing)
    }
    fn plot_owner_value(&self, plot: u32) -> Result<i32, Error<E::Error>> {
        Ok(self
            .current_plot(plot)?
            .city_attributes
            .as_ref()
            .map_or(-1, |a| i32::from(a.player)))
    }
    fn district_value(&self, plot: u32) -> Result<Option<Actor>, Error<E::Error>> {
        plot_district(self.state, self.effects, plot)
    }
}

pub fn run_siege<E: Services>(
    state: &mut LoadedState,
    runtime: &mut sight::Runtime,
    effects: &mut E,
    location: PlotCoord,
    owner: i32,
) -> Result<(), Error<E::Error>> {
    tails::check_for_siege_near(
        &mut Context {
            state,
            sight: runtime,
            effects,
            actor: (owner, u32::MAX),
            encountered_type: None,
        },
        location,
        owner,
    )
}
pub fn run_contact<E: Services>(
    state: &mut LoadedState,
    runtime: &mut sight::Runtime,
    effects: &mut E,
    actor: Actor,
) -> Result<(), Error<E::Error>> {
    tails::diplomacy_unit_moved(
        &mut Context {
            state,
            sight: runtime,
            effects,
            actor,
            encountered_type: None,
        },
        actor,
    )
    .map_err(|e| Error::Meeting(Box::new(e)))
}
pub fn run_units_moved<E: Services>(
    state: &mut LoadedState,
    runtime: &mut sight::Runtime,
    effects: &mut E,
    actor: Actor,
) -> Result<(), Error<E::Error>> {
    tails::units_unit_moved(
        &mut Context {
            state,
            sight: runtime,
            effects,
            actor,
            encountered_type: None,
        },
        actor,
    )
    .map_err(|e| Error::Units(Box::new(e)))
}
pub fn run_barbarian<E: Services>(
    state: &mut LoadedState,
    runtime: &mut sight::Runtime,
    effects: &mut E,
    actor: Actor,
    location: PlotCoord,
) -> Result<(), Error<E::Error>> {
    tails::notify_barbarian_approach(
        &mut Context {
            state,
            sight: runtime,
            effects,
            actor,
            encountered_type: None,
        },
        location,
    )
}

impl<E: Services> meeting::Context for Context<'_, E> {
    type Error = Error<E::Error>;
    type Player = i32;
    type Unit = Actor;
    type Plot = u32;
    type City = Actor;
    #[allow(unused_variables)]
    fn player(&mut self, owner: i32) -> Result<Self::Player, Self::Error> {
        self.state.world().player(owner).map_err(Error::World)?;
        Ok(owner)
    }
    #[allow(unused_variables)]
    fn player_valid(&mut self, owner: i32) -> Result<bool, Self::Error> {
        self.state
            .world()
            .player_is_valid(owner)
            .map_err(Error::World)
    }
    #[allow(unused_variables)]
    fn id(&mut self, player: Self::Player) -> Result<i32, Self::Error> {
        self.state.world().player(player).map_err(Error::World)?;
        Ok(player)
    }
    #[allow(unused_variables)]
    fn alive(&mut self, player: Self::Player) -> Result<bool, Self::Error> {
        self.state.player_alive(player).map_err(Error::State)
    }
    #[allow(unused_variables)]
    fn major(&mut self, player: Self::Player) -> Result<bool, Self::Error> {
        Ok(Context::level(self, player)? == meeting::MAJOR_LEVEL)
    }
    #[allow(unused_variables)]
    fn barbarian(&mut self, player: Self::Player) -> Result<bool, Self::Error> {
        Ok(Context::level(self, player)? == 0xd959_ec24)
    }
    #[allow(unused_variables)]
    fn human(&mut self, player: Self::Player) -> Result<bool, Self::Error> {
        Ok(self
            .state
            .partial_player_controls(player)
            .map_err(Error::State)?
            .slot_status
            == 3)
    }
    #[allow(unused_variables)]
    fn level(&mut self, player: Self::Player) -> Result<u32, Self::Error> {
        Context::level(self, player)
    }
    #[allow(unused_variables)]
    fn any_multiplayer(&mut self) -> Result<bool, Self::Error> {
        Err(Error::Missing("Context.any_multiplayer"))
    }
    #[allow(unused_variables)]
    fn generating(&mut self) -> Result<bool, Self::Error> {
        Err(Error::Missing("Context.generating"))
    }
    #[allow(unused_variables)]
    fn relations(&self) -> Result<&relations::GameDiplomaticRelations, Self::Error> {
        self.state.game_relations().map_err(Error::State)
    }
    #[allow(unused_variables)]
    fn relations_mut(&mut self) -> Result<&mut relations::GameDiplomaticRelations, Self::Error> {
        Err(Error::Missing("Context.relations_mut"))
    }
    #[allow(unused_variables)]
    fn current_turn(&mut self) -> Result<i32, Self::Error> {
        Ok(self.state.retained().core.current_turn)
    }
    #[allow(unused_variables)]
    fn preflight_meet(
        &self,
        first: i32,
        second: i32,
        plot: Option<Self::Plot>,
        source: ComponentId,
    ) -> Result<(), Self::Error> {
        Err(Error::Missing("Context.preflight_meet"))
    }
    #[allow(unused_variables)]
    fn gossip_mut(&mut self, player: Self::Player) -> Result<&mut GossipReports, Self::Error> {
        self.state
            .player_gossip_reports_mut(player)
            .map_err(Error::State)
    }
    #[allow(unused_variables)]
    fn visibility_mut(&mut self, player: Self::Player) -> Result<&mut [i32], Self::Error> {
        Err(Error::Missing("Context.visibility_mut"))
    }
    #[allow(unused_variables)]
    fn visibility_source_mut(&mut self, player: Self::Player) -> Result<&mut [i32], Self::Error> {
        Err(Error::Missing("Context.visibility_source_mut"))
    }
    #[allow(unused_variables)]
    fn visibility_sources(&self) -> &[VisibilitySource] {
        unreachable!("first-contact effects rejected by preflight_meet")
    }
    #[allow(unused_variables)]
    fn visibility_source_active(
        &mut self,
        player: Self::Player,
        other: i32,
        source: i32,
    ) -> Result<bool, Self::Error> {
        Err(Error::Missing("Context.visibility_source_active"))
    }
    #[allow(unused_variables)]
    fn dynamic_visibility(
        &mut self,
        player: Self::Player,
        source: i32,
    ) -> Result<Vec<i32>, Self::Error> {
        Err(Error::Missing("Context.dynamic_visibility"))
    }
    #[allow(unused_variables)]
    fn visibility_changed(
        &mut self,
        event: u32,
        owner: i32,
        other: i32,
    ) -> Result<(), Self::Error> {
        Err(Error::Missing("Context.visibility_changed"))
    }
    #[allow(unused_variables)]
    fn log_has_met(&mut self, owner: i32, other: i32) -> Result<(), Self::Error> {
        Err(Error::Missing("Context.log_has_met"))
    }
    #[allow(unused_variables)]
    fn find_unit(&mut self, source: ComponentId) -> Result<Option<Self::Unit>, Self::Error> {
        Err(Error::Missing("Context.find_unit"))
    }
    #[allow(unused_variables)]
    fn unit(&self, unit: Self::Unit) -> Result<&Unit, Self::Error> {
        self.current_unit(unit)
    }
    #[allow(unused_variables)]
    fn unit_mut(&mut self, unit: Self::Unit) -> Result<&mut Unit, Self::Error> {
        self.state.unit_mut(unit.0, unit.1).map_err(Error::State)
    }
    #[allow(unused_variables)]
    fn preflight_movement_report(&self, unit: Self::Unit, event: u32) -> Result<(), Self::Error> {
        Err(Error::Missing("Context.preflight_movement_report"))
    }
    #[allow(unused_variables)]
    fn movement_report(&mut self, report: PlacementReport) -> Result<(), Self::Error> {
        Err(Error::Missing("Context.movement_report"))
    }
    #[allow(unused_variables)]
    fn notify_contact(
        &mut self,
        recipient: i32,
        other: Self::Player,
        plot: Self::Plot,
    ) -> Result<(), Self::Error> {
        Err(Error::Missing("Context.notify_contact"))
    }
    #[allow(unused_variables)]
    fn report(&mut self, report: Report) -> Result<(), Self::Error> {
        Err(Error::Missing("Context.report"))
    }
    #[allow(unused_variables)]
    fn closest_city(
        &mut self,
        player: Self::Player,
        plot: Self::Plot,
    ) -> Result<Option<Self::City>, Self::Error> {
        Err(Error::Missing("Context.closest_city"))
    }
    #[allow(unused_variables)]
    fn city_distance(&mut self, city: Self::City, plot: Self::Plot) -> Result<i32, Self::Error> {
        Err(Error::Missing("Context.city_distance"))
    }
    #[allow(unused_variables)]
    fn city_location(&mut self, city: Self::City) -> Result<PlotCoord, Self::Error> {
        Err(Error::Missing("Context.city_location"))
    }
    #[allow(unused_variables)]
    fn hospitality_distance(&mut self) -> Result<i32, Self::Error> {
        Err(Error::Missing("Context.hospitality_distance"))
    }
    #[allow(unused_variables)]
    fn request_first_meeting(&mut self, request: FirstMeetingRequest) -> Result<(), Self::Error> {
        Err(Error::Missing("Context.request_first_meeting"))
    }
    #[allow(unused_variables)]
    fn signal_meet(
        &mut self,
        first: Self::Player,
        second: Self::Player,
    ) -> Result<(), Self::Error> {
        Err(Error::Missing("Context.signal_meet"))
    }
}

impl<E: Services> tails::ContactContext for Context<'_, E> {
    type AlivePlayers = AliveToken;
    #[allow(unused_variables)]
    fn alive_players(&mut self) -> Result<(Self::AlivePlayers, usize), Self::Error> {
        self.runtime()?.alive_begin()
    }
    #[allow(unused_variables)]
    fn alive_player(
        &mut self,
        list: Self::AlivePlayers,
        index: usize,
    ) -> Result<Self::Player, Self::Error> {
        self.runtime()?.alive_at(list, index)
    }
    #[allow(unused_variables)]
    fn plot_visible(&mut self, viewer: i32, location: PlotCoord) -> Result<bool, Self::Error> {
        self.visible(viewer, location)
    }
    #[allow(unused_variables)]
    fn current_unit_plot(&mut self, unit: Self::Unit) -> Result<Option<Self::Plot>, Self::Error> {
        self.plot_at_coord(self.current_unit(unit)?.k_location)
    }
}

impl<E: Services> contents::Context for Context<'_, E> {
    type Error = Error<E::Error>;
    type Plot = u32;
    type Unit = Actor;
    type UnitList = ();
    type Capital = Actor;
    #[allow(unused_variables)]
    fn preflight_contents(&self, event: ViewContents) -> Result<(), Self::Error> {
        Err(Error::Missing("Context.preflight_contents"))
    }
    #[allow(unused_variables)]
    fn map_present(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
    #[allow(unused_variables)]
    fn plot(&mut self, index: i32) -> Result<Option<Self::Plot>, Self::Error> {
        if index < 0 {
            return Ok(None);
        }
        Ok(self
            .state
            .world()
            .map()
            .plot_by_index(index as u32)
            .map(|_| index as u32))
    }
    #[allow(unused_variables)]
    fn plot_owner(&mut self, plot: Self::Plot) -> Result<i32, Self::Error> {
        self.plot_owner_value(plot)
    }
    #[allow(unused_variables)]
    fn location(&mut self, plot: Self::Plot) -> Result<PlotCoord, Self::Error> {
        let (x, y) = self
            .state
            .world()
            .map()
            .coordinates(plot)
            .ok_or(Error::Missing("current plot coordinates"))?;
        Ok(PlotCoord { x, y })
    }
    #[allow(unused_variables)]
    fn continent(&mut self, plot: Self::Plot) -> Result<i32, Self::Error> {
        Err(Error::Missing("Context.continent"))
    }
    #[allow(unused_variables)]
    fn feature(&mut self, plot: Self::Plot) -> Result<i32, Self::Error> {
        Err(Error::Missing("Context.feature"))
    }
    #[allow(unused_variables)]
    fn goody_notify(&mut self, plot: Self::Plot) -> Result<bool, Self::Error> {
        Err(Error::Missing("Context.goody_notify"))
    }
    #[allow(unused_variables)]
    fn natural_wonder(&mut self, plot: Self::Plot) -> Result<bool, Self::Error> {
        Err(Error::Missing("Context.natural_wonder"))
    }
    #[allow(unused_variables)]
    fn player_valid(&mut self, player: i32) -> Result<bool, Self::Error> {
        self.state
            .world()
            .player_is_valid(player)
            .map_err(Error::World)
    }
    #[allow(unused_variables)]
    fn human(&mut self, player: i32) -> Result<bool, Self::Error> {
        Ok(self
            .state
            .partial_player_controls(player)
            .map_err(Error::State)?
            .slot_status
            == 3)
    }
    #[allow(unused_variables)]
    fn is_revealed(&mut self, player: i32, plot: u32) -> Result<bool, Self::Error> {
        Err(Error::Missing("Context.is_revealed"))
    }
    #[allow(unused_variables)]
    fn discovery(&self, player: i32) -> Result<&Discovery, Self::Error> {
        Err(Error::Missing("Context.discovery"))
    }
    #[allow(unused_variables)]
    fn discovery_mut(&mut self, player: i32) -> Result<&mut Discovery, Self::Error> {
        Err(Error::Missing("Context.discovery_mut"))
    }
    #[allow(unused_variables)]
    fn find_unit(&mut self, reference: ComponentId) -> Result<Option<Self::Unit>, Self::Error> {
        Err(Error::Missing("Context.find_unit"))
    }
    #[allow(unused_variables)]
    fn unit_owner(&mut self, unit: Self::Unit) -> Result<i32, Self::Error> {
        Ok(self.current_unit(unit)?.e_owner)
    }
    #[allow(unused_variables)]
    fn player_id(&mut self, owner: i32) -> Result<i32, Self::Error> {
        self.state.world().player(owner).map_err(Error::World)?;
        Ok(owner)
    }
    #[allow(unused_variables)]
    fn unit_layer_count(&mut self) -> Result<i32, Self::Error> {
        Err(Error::Missing("Context.unit_layer_count"))
    }
    #[allow(unused_variables)]
    fn unit_list(&mut self, plot: Self::Plot, layer: i32) -> Result<Self::UnitList, Self::Error> {
        Err(Error::Missing("Context.unit_list"))
    }
    #[allow(unused_variables)]
    fn unit_list_count(&mut self, list: Self::UnitList) -> Result<usize, Self::Error> {
        Err(Error::Missing("Context.unit_list_count"))
    }
    #[allow(unused_variables)]
    fn unit_reference(
        &mut self,
        list: Self::UnitList,
        index: usize,
    ) -> Result<ComponentId, Self::Error> {
        Err(Error::Missing("Context.unit_reference"))
    }
    #[allow(unused_variables)]
    fn meet(
        &mut self,
        first: i32,
        second: i32,
        plot: Option<Self::Plot>,
        source: ComponentId,
    ) -> Result<(), Self::Error> {
        Err(Error::Missing("Context.meet"))
    }
    #[allow(unused_variables)]
    fn view_unit_rules(
        &mut self,
        player: i32,
        unit: Self::Unit,
        origin: i32,
        first_reveal: bool,
    ) -> Result<(), Self::Error> {
        Err(Error::Missing("Context.view_unit_rules"))
    }
    #[allow(unused_variables)]
    fn signal(&mut self, event: Signal<Self::Unit>) -> Result<(), Self::Error> {
        Err(Error::Missing("Context.signal"))
    }
    #[allow(unused_variables)]
    fn notify(
        &mut self,
        player: i32,
        plot: Self::Plot,
        kind: Notification,
    ) -> Result<(), Self::Error> {
        Err(Error::Missing("Context.notify"))
    }
    #[allow(unused_variables)]
    fn natural_wonder_relic_count(&mut self, player: i32) -> Result<i32, Self::Error> {
        Err(Error::Missing("Context.natural_wonder_relic_count"))
    }
    #[allow(unused_variables)]
    fn capital(&mut self, player: i32) -> Result<Option<Self::Capital>, Self::Error> {
        Err(Error::Missing("Context.capital"))
    }
    #[allow(unused_variables)]
    fn capital_location(&mut self, city: Self::Capital) -> Result<PlotCoord, Self::Error> {
        Err(Error::Missing("Context.capital_location"))
    }
    #[allow(unused_variables)]
    fn create_relic(
        &mut self,
        player: i32,
        location: PlotCoord,
        source: u32,
    ) -> Result<(), Self::Error> {
        Err(Error::Missing("Context.create_relic"))
    }
    #[allow(unused_variables)]
    fn grid_width(&mut self) -> Result<i32, Self::Error> {
        Ok(self.state.world().map().header.width)
    }
    #[allow(unused_variables)]
    fn grid_height(&mut self) -> Result<i32, Self::Error> {
        Ok(self.state.world().map().header.height)
    }
    #[allow(unused_variables)]
    fn revealed_xy(&mut self, player: i32, x: i32, y: i32) -> Result<bool, Self::Error> {
        Err(Error::Missing("Context.revealed_xy"))
    }
}

impl<E: Services> contents::UnitViewContext for Context<'_, E> {
    #[allow(unused_variables)]
    fn unit_type(&mut self, unit: Self::Unit) -> Result<i32, Self::Error> {
        let index = self
            .state
            .unit_runtime(unit.0, unit.1)
            .map_err(Error::State)?
            .definition_index;
        self.encountered_type = Some(index);
        Ok(index)
    }
    #[allow(unused_variables)]
    fn encountered_units(&self, player: i32) -> Result<&[bool], Self::Error> {
        self.runtime()?
            .player(player)?
            .encountered_units
            .as_deref()
            .ok_or(Error::Missing(
                "initialized Player.Units encountered unit array",
            ))
    }
    #[allow(unused_variables)]
    fn encountered_units_mut(&mut self, player: i32) -> Result<&mut [bool], Self::Error> {
        let index = self
            .encountered_type
            .ok_or(Error::Missing("current ViewUnit type"))?;
        self.effects
            .preflight_encountered_unit(self.state, player, index)?;
        self.runtime_mut()?
            .player_mut(player)?
            .encountered_units
            .as_deref_mut()
            .ok_or(Error::Missing(
                "initialized Player.Units encountered unit array",
            ))
    }
    #[allow(unused_variables)]
    fn player_barbarian(&mut self, player: i32) -> Result<bool, Self::Error> {
        Ok(Context::level(self, player)? == 0xd959_ec24)
    }
    #[allow(unused_variables)]
    fn clans_active(&mut self) -> Result<bool, Self::Error> {
        self.runtime()?
            .clans_active
            .ok_or(Error::Missing("current Clans active flag"))
    }
    #[allow(unused_variables)]
    fn signal_encountered_unit(&mut self, player: i32, unit_type: i32) -> Result<(), Self::Error> {
        self.effects
            .signal_encountered_unit(self.state, self.sight, player, unit_type)
    }
    #[allow(unused_variables)]
    fn clan_view_foreign_unit(
        &mut self,
        unit: Self::Unit,
        origin: Self::Plot,
    ) -> Result<(), Self::Error> {
        Err(Error::Missing("UnitViewContext.clan_view_foreign_unit"))
    }
}

impl<E: Services> tails::UnitsMovedContext for Context<'_, E> {
    type AlivePlayers = AliveToken;
    type Player = i32;
    #[allow(unused_variables)]
    fn alive_players(&mut self) -> Result<(Self::AlivePlayers, usize), Self::Error> {
        self.runtime()?.alive_begin()
    }
    #[allow(unused_variables)]
    fn alive_player(
        &mut self,
        list: Self::AlivePlayers,
        index: usize,
    ) -> Result<Self::Player, Self::Error> {
        self.runtime()?.alive_at(list, index)
    }
    #[allow(unused_variables)]
    fn current_player_id(&mut self, player: Self::Player) -> Result<i32, Self::Error> {
        self.state.world().player(player).map_err(Error::World)?;
        Ok(player)
    }
    #[allow(unused_variables)]
    fn current_player_barbarian(&mut self, player: Self::Player) -> Result<bool, Self::Error> {
        Ok(Context::level(self, player)? == 0xd959_ec24)
    }
    #[allow(unused_variables)]
    fn unit_location(&mut self, unit: Self::Unit) -> Result<PlotCoord, Self::Error> {
        Ok(self.current_unit(unit)?.k_location)
    }
    #[allow(unused_variables)]
    fn plot_visible(&mut self, viewer: i32, location: PlotCoord) -> Result<bool, Self::Error> {
        self.visible(viewer, location)
    }
    #[allow(unused_variables)]
    fn plot_at(&mut self, location: PlotCoord) -> Result<Option<Self::Plot>, Self::Error> {
        self.plot_at_coord(location)
    }
    #[allow(unused_variables)]
    fn barbarian_camp(&mut self, plot: Self::Plot) -> Result<bool, Self::Error> {
        let hash = self.current_plot(plot)?.improvement_hash;
        let d = self.definition()?;
        Ok(d.improvements.find_index(hash).is_some_and(|i| d.camps[i]))
    }
    #[allow(unused_variables)]
    fn foreign_unit_moved_near_camp(
        &mut self,
        unit: Self::Unit,
        plot: Self::Plot,
    ) -> Result<(), Self::Error> {
        self.effects
            .foreign_unit_moved_near_camp(self.state, self.sight, unit, plot)
    }
}

impl<E: Services> tails::SiegeContext for Context<'_, E> {
    type Error = Error<E::Error>;
    type Map = ();
    type Plot = u32;
    type District = Actor;
    #[allow(unused_variables)]
    fn map(&mut self) -> Result<Self::Map, Self::Error> {
        Ok(())
    }
    #[allow(unused_variables)]
    fn plot_at(
        &mut self,
        map: Self::Map,
        location: PlotCoord,
    ) -> Result<Option<Self::Plot>, Self::Error> {
        self.plot_at_coord(location)
    }
    #[allow(unused_variables)]
    fn location(&mut self, plot: Self::Plot) -> Result<PlotCoord, Self::Error> {
        let (x, y) = self
            .state
            .world()
            .map()
            .coordinates(plot)
            .ok_or(Error::Missing("current plot coordinates"))?;
        Ok(PlotCoord { x, y })
    }
    #[allow(unused_variables)]
    fn distance(&mut self, map: Self::Map, a: PlotCoord, b: PlotCoord) -> Result<i32, Self::Error> {
        Ok(city_range_attack::plot_distance(
            self.state,
            (a.x, a.y),
            (b.x, b.y),
        ))
    }
    #[allow(unused_variables)]
    fn owner(&mut self, plot: Self::Plot) -> Result<i32, Self::Error> {
        self.plot_owner_value(plot)
    }
    #[allow(unused_variables)]
    fn is_city(&mut self, plot: Self::Plot) -> Result<bool, Self::Error> {
        plot_is_city(self.state, self.effects, plot)
    }
    #[allow(unused_variables)]
    fn district(&mut self, plot: Self::Plot) -> Result<Option<Self::District>, Self::Error> {
        self.district_value(plot)
    }
    #[allow(unused_variables)]
    fn is_besieged(&mut self, district: Self::District) -> Result<bool, Self::Error> {
        Err(Error::Missing("SiegeContext.is_besieged"))
    }
    #[allow(unused_variables)]
    fn set_siege_status(
        &mut self,
        district: Self::District,
        value: bool,
    ) -> Result<(), Self::Error> {
        Err(Error::Missing("SiegeContext.set_siege_status"))
    }
}

impl<E: Services> tails::BarbarianApproachContext for Context<'_, E> {
    type Error = Error<E::Error>;
    type Map = ();
    type AlivePlayers = AliveToken;
    type Player = i32;
    type Cities = i32;
    type Cursor = usize;
    type City = Actor;
    #[allow(unused_variables)]
    fn unit_is_barbarian(&mut self) -> Result<bool, Self::Error> {
        Ok(Context::level(self, self.current_unit(self.actor)?.e_owner)? == 0xd959_ec24)
    }
    #[allow(unused_variables)]
    fn current_turn(&mut self) -> Result<i32, Self::Error> {
        Ok(self.state.retained().core.current_turn)
    }
    #[allow(unused_variables)]
    fn map(&mut self) -> Result<Self::Map, Self::Error> {
        Ok(())
    }
    #[allow(unused_variables)]
    fn alive_players(&mut self) -> Result<(Self::AlivePlayers, usize), Self::Error> {
        self.runtime()?.alive_begin()
    }
    #[allow(unused_variables)]
    fn alive_player(
        &mut self,
        list: Self::AlivePlayers,
        index: usize,
    ) -> Result<Self::Player, Self::Error> {
        self.runtime()?.alive_at(list, index)
    }
    #[allow(unused_variables)]
    fn cities_begin(
        &mut self,
        player: Self::Player,
    ) -> Result<(Self::Cities, Self::Cursor), Self::Error> {
        self.state.world().player(player).map_err(Error::World)?;
        self.state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == player)
            .ok_or(Error::Missing("barbarian approach player cities"))?;
        Ok((player, 0))
    }
    #[allow(unused_variables)]
    fn city_at(
        &mut self,
        cities: Self::Cities,
        cursor: Self::Cursor,
    ) -> Result<Option<Self::City>, Self::Error> {
        let player = self
            .state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == cities)
            .ok_or(Error::Missing("barbarian approach current cities"))?;
        Ok(player
            .cities
            .records
            .entries
            .get(cursor)
            .map(|entry| (cities, entry.value.header.id)))
    }
    #[allow(unused_variables)]
    fn advance_city(
        &mut self,
        cities: Self::Cities,
        cursor: &mut Self::Cursor,
    ) -> Result<(), Self::Error> {
        let _ = cities;
        *cursor = cursor
            .checked_add(1)
            .ok_or(Error::Missing("barbarian approach city cursor capacity"))?;
        Ok(())
    }
    #[allow(unused_variables)]
    fn city_location(&mut self, city: Self::City) -> Result<PlotCoord, Self::Error> {
        let city = self.state.city(city.0, city.1).map_err(Error::State)?;
        Ok(PlotCoord {
            x: city.header.location_x,
            y: city.header.location_y,
        })
    }
    #[allow(unused_variables)]
    fn distance(&mut self, map: Self::Map, a: PlotCoord, b: PlotCoord) -> Result<i32, Self::Error> {
        Ok(city_range_attack::plot_distance(
            self.state,
            (a.x, a.y),
            (b.x, b.y),
        ))
    }
    #[allow(unused_variables)]
    fn last_invasion(&mut self, city: Self::City) -> Result<i32, Self::Error> {
        Ok(self
            .state
            .city(city.0, city.1)
            .map_err(Error::State)?
            .header
            .last_barbarian_invasion_turn)
    }
    #[allow(unused_variables)]
    fn set_last_invasion(&mut self, city: Self::City, turn: i32) -> Result<(), Self::Error> {
        self.state
            .city_mut(city.0, city.1)
            .map_err(Error::State)?
            .header
            .last_barbarian_invasion_turn = turn;
        Ok(())
    }
    #[allow(unused_variables)]
    fn notify_approach(
        &mut self,
        city: Self::City,
        location: PlotCoord,
    ) -> Result<(), Self::Error> {
        Err(Error::Missing("BarbarianApproachContext.notify_approach"))
    }
}

/// Native Map coordinate normalization, preserving out-of-range null results
/// and wrapped duplicates rather than inventing a valid target plot.
pub fn plot_index_at<E>(state: &LoadedState, location: PlotCoord) -> Result<Option<u32>, Error<E>> {
    if location.x == -9999 || location.y == -9999 {
        return Ok(None);
    }
    let map = state.world().map();
    if map.header.width <= 0 || map.header.height <= 0 {
        return Err(Error::Missing("Map geometry"));
    }
    let x = c6_core::movement_world::coord_range(location.x, map.header.width, map.header.wrap_x);
    let y = c6_core::movement_world::coord_range(location.y, map.header.height, map.header.wrap_y);
    Ok(map
        .plot_at(x, y)
        .map(|_| y.wrapping_mul(map.header.width).wrapping_add(x) as u32))
}

fn query_plot<E>(state: &LoadedState, index: u32) -> Result<&Plot, Error<E>> {
    let plot = state
        .world()
        .map()
        .plot_by_index(index)
        .ok_or(Error::Missing("current placement Plot"))?;
    if plot.flags.get(PlotFlag::HasCityAttributes) != plot.city_attributes.is_some() {
        return Err(Error::Missing("current Plot city attributes"));
    }
    Ok(plot)
}
/// Actual CityAttributes district reference resolved through current owned slots.
pub fn plot_district<E: Services>(
    state: &LoadedState,
    _effects: &E,
    plot: u32,
) -> Result<Option<Actor>, Error<E::Error>> {
    let Some(attrs) = query_plot(state, plot)?.city_attributes.as_ref() else {
        return Ok(None);
    };
    if attrs.player == -1 || attrs.district == u32::MAX {
        return Ok(None);
    }
    let owner = i32::from(attrs.player);
    if state
        .world()
        .valid_player(owner)
        .map_err(Error::World)?
        .is_none()
    {
        return Ok(None);
    }
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("current district owner"))?;
    let district = player
        .districts
        .records
        .entries
        .iter()
        .find(|d| d.index == (attrs.district & 0xffff) as i32)
        .ok_or(Error::Missing("current CityAttributes district"))?;
    Ok(Some((owner, district.value.id)))
}
pub fn district_type<E: Services>(
    state: &LoadedState,
    effects: &E,
    plot: u32,
) -> Result<i32, Error<E::Error>> {
    let Some((owner, id)) = plot_district(state, effects, plot)? else {
        return Ok(-1);
    };
    let definitions = effects.placement_service_definitions()?;
    if definitions.binding != *state.binding() {
        return Err(Error::Binding);
    }
    let district = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .and_then(|p| {
            p.districts
                .records
                .entries
                .iter()
                .find(|d| d.value.id == id)
        })
        .ok_or(Error::Missing("current district"))?;
    Ok(definitions
        .districts
        .find_index(district.value.district_hash)
        .map_or(-1, |i| i as i32))
}
pub fn plot_is_city<E: Services>(
    state: &LoadedState,
    effects: &E,
    plot: u32,
) -> Result<bool, Error<E::Error>> {
    let index = district_type(state, effects, plot)?;
    if index < 0 {
        return Ok(false);
    }
    Ok(effects
        .placement_service_definitions()?
        .districts
        .type_hash(index as usize)
        == Some(c6_core::city_district::CITY_CENTER_HASH))
}
/// OwningCity is independent of District and WorkingCity, and uses low16 slot
/// lookup just like the native Cities.Find; a generation mismatch is not a miss.
pub fn owning_city<E: Services>(
    state: &LoadedState,
    _effects: &E,
    plot: u32,
) -> Result<Option<Actor>, Error<E::Error>> {
    let Some(attrs) = query_plot(state, plot)?.city_attributes.as_ref() else {
        return Ok(None);
    };
    if attrs.player == -1 || attrs.owning_city == u32::MAX {
        return Ok(None);
    }
    let owner = i32::from(attrs.player);
    if state
        .world()
        .valid_player(owner)
        .map_err(Error::World)?
        .is_none()
    {
        return Ok(None);
    }
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == owner)
        .ok_or(Error::Missing("current city owner"))?;
    Ok(player
        .cities
        .records
        .entries
        .iter()
        .find(|c| c.index == (attrs.owning_city & 0xffff) as i32)
        .map(|c| (owner, c.value.header.id)))
}

impl<E> From<crate::loaded_actions::Error> for Error<E> {
    fn from(value: crate::loaded_actions::Error) -> Self {
        Self::Combat(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_alive_cursor_rereads_in_place_but_rejects_replaced_allocation() {
        let mut first = AlivePlayers::new(vec![9, 2, 7]);
        let (token, end) = first.begin();
        assert_eq!(end, 3);
        first.values_mut()[1] = 11;
        assert_eq!(first.at::<()>(token, 1).unwrap(), 11);
        assert!(matches!(
            first.at::<()>(token, 3),
            Err(Error::AliveStorageChanged)
        ));
        // Keep the old allocation live so a different list cannot reuse it.
        let second = AlivePlayers::new(vec![9, 2, 7]);
        assert!(matches!(
            second.at::<()>(token, 0),
            Err(Error::AliveStorageChanged)
        ));
        assert_eq!(first.values(), &[9, 11, 7]);
    }
    #[test]
    fn alive_rebuild_follows_physical_slots_and_excludes_slot64_without_guessing_unknowns() {
        let mut slots = vec![None; 65];
        slots[0] = Some(9);
        slots[2] = Some(2);
        slots[3] = Some(7);
        slots[63] = Some(1);
        slots[64] = Some(64);
        let mut visited = Vec::new();
        let values = ordered_alive(&slots, |owner| {
            visited.push(owner);
            Ok::<_, ()>(owner != 2)
        })
        .unwrap();
        assert_eq!(visited, vec![9, 2, 7, 1]);
        assert_eq!(values, vec![9, 7, 1]);
        assert!(matches!(
            ordered_alive(&slots[..63], |_| Ok::<_, ()>(true)),
            Err(AliveBuildError::MissingAllocationSlots)
        ));
        assert!(matches!(
            ordered_alive(&slots, |owner| if owner == 2 {
                Err("unknown")
            } else {
                Ok(true)
            }),
            Err(AliveBuildError::CurrentAlive("unknown"))
        ));
    }
    struct Missing;
    impl Services for Missing {
        type Error = ();
    }
    #[test]
    fn missing_runtime_and_encountered_effect_support_are_explicit() {
        assert!(matches!(
            Missing.placement_service_runtime(),
            Err(Error::Missing(_))
        ));
        assert!(matches!(
            Missing.placement_service_definitions(),
            Err(Error::Missing(_))
        ));
        assert!(matches!(
            Missing.placement_service_runtime_mut(),
            Err(Error::Missing(_))
        ));
    }
}
