//! Live normal Placement over the canonical loaded Unit and World.
//! Definitions are retained across actions; query views borrow current state.
use crate::{movement_placement_state as placement, unit_sight_state as sight};
use c6_core::{loaded_state::LoadedState, unit::PlotCoord};

pub struct Definitions<'a, 'r> {
    pub placement: &'a placement::Definitions<'a, 'r>,
    pub sight: &'a crate::loaded_actions::city_range_attack::Definitions,
    pub mechanics: &'a Mechanics,
    pub bonus: &'a crate::loaded_upkeep::placement_tails::Definitions,
}

/// Current owned configuration and rebuilt city-domain inputs. Absence remains
/// unknown; no native predicate result is accepted as a replacement.
pub struct Queries<'a> {
    pub civilization_levels: Option<&'a [(i32, u32)]>,
    pub city_domains: Option<&'a [Option<c6_core::city_district::DomainMask>]>,
    pub civic_activity: Option<&'a crate::loaded_actions::culture_notifications::GameCulture>,
    pub local_player: Option<i32>,
}
impl Queries<'_> {
    pub fn placement_context<'a, 'r>(
        &'a self,
        state: &'a LoadedState,
        definitions: &'a placement::Definitions<'a, 'r>,
    ) -> Result<placement::Context<'a, 'a, 'r>, placement::Error> {
        let mut context = placement::Context::from_definitions(state, definitions)?;
        if let Some(values) = self.civilization_levels {
            context = context.with_civilization_levels(placement::CurrentCivilizationLevels {
                binding: state.binding(),
                values,
            })?;
        }
        if let Some(masks) = self.city_domains {
            context =
                context.with_city_domain_masks(state.binding(), state.action_stamp(), masks)?;
        }
        if let Some(culture) = self.civic_activity {
            context = context.with_civic_activity(culture)?;
        }
        Ok(context)
    }
}

pub type EffectsError<E> = <E as sight::Effects>::Error;
/// Required reached synchronous effects. Defaults reject; they never discard
/// a report, callback, operation transition, or RNG draw.
pub trait Effects:
    sight::Effects + crate::loaded_upkeep::OperationReports<Error = EffectsError<Self>>
{
    fn preflight_normal(
        &self,
        state: &LoadedState,
        runtime: &sight::Runtime,
        actor: (i32, u32),
        destination: PlotCoord,
    ) -> Result<(), Error<EffectsError<Self>>>;
    fn signal_unit_moved(
        &mut self,
        _state: &mut LoadedState,
        _runtime: &mut sight::Runtime,
        _actor: (i32, u32),
        _old: PlotCoord,
        _new: PlotCoord,
    ) -> Result<(), Error<EffectsError<Self>>> {
        Err(Error::Missing("GameSignals.UnitMoved"))
    }
    fn embarked_state_change(
        &mut self,
        _state: &mut LoadedState,
        _runtime: &mut sight::Runtime,
        _actor: (i32, u32),
        _target: bool,
        _old_valid: bool,
    ) -> Result<(), Error<EffectsError<Self>>> {
        Err(Error::Missing("Rules.Placement.EmbarkedStateChange"))
    }
    fn damage_changed(
        &mut self,
        _state: &mut LoadedState,
        _report: c6_core::unit_healing::DamageChangedReport,
    ) -> Result<(), Error<EffectsError<Self>>> {
        Err(Error::Missing("DamageChanged reporting"))
    }
    fn kill(
        &mut self,
        _state: &mut LoadedState,
        _actor: (i32, u32),
        _delayed: bool,
        _attacker: i32,
    ) -> Result<(), Error<EffectsError<Self>>> {
        Err(Error::Missing("Manager.Kill"))
    }
    fn clear_operations(
        &mut self,
        _state: &mut LoadedState,
        _actor: (i32, u32),
    ) -> Result<(), Error<EffectsError<Self>>> {
        Err(Error::Missing("Operations.Clear"))
    }
    fn improvement_visibility(
        &mut self,
        _state: &mut LoadedState,
        _runtime: &mut sight::Runtime,
        _plot: u32,
        _direction: i32,
    ) -> Result<(), Error<EffectsError<Self>>> {
        Err(Error::Missing("HandleImprovementVisUpdates"))
    }
    fn improvement_damage(
        &mut self,
        _state: &mut LoadedState,
        _actor: (i32, u32),
        _owner: i32,
        _amount: i32,
    ) -> Result<(), Error<EffectsError<Self>>> {
        Err(Error::Missing("GameSignals.ImprovementDamage"))
    }
    fn city_unit_changed(
        &mut self,
        _state: &mut LoadedState,
        _city: c6_core::unit::ComponentId,
    ) -> Result<(), Error<EffectsError<Self>>> {
        Err(Error::Missing("CulturalIdentity.DoCityUnitChanged"))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Request {
    pub actor: (i32, u32),
    pub destination: PlotCoord,
    pub validation_flags: u32,
}
#[derive(Debug)]
pub enum Error<E> {
    Placement(placement::Error),
    Sight(sight::Error<E>),
    Missing(&'static str),
    Effect(E),
    World(c6_core::world::MissingInput),
    State(c6_core::loaded_state::Error),
    Query(crate::loaded_actions::Error),
    Transition(Box<c6_core::unit_placement_transition::normal_move::Error<Error<E>>>),
}

/// Normal flags3 Placement. An unavailable continuation is an error before
/// mutation; callers must not treat a supported path query as movement success.
pub fn set<E: Effects>(
    state: &mut LoadedState,
    definitions: &Definitions<'_, '_>,
    queries: &Queries<'_>,
    runtime: &mut sight::Runtime,
    effects: &mut E,
    request: Request,
) -> Result<i32, Error<EffectsError<E>>> {
    if state.binding() != &definitions.mechanics.binding {
        return Err(Error::Query(
            crate::loaded_actions::Error::ConfigurationIdentity,
        ));
    }
    let mut context = Context {
        sight: sight::Context {
            state,
            runtime,
            effects,
            sight_definitions: definitions.sight,
            actor: request.actor,
        },
        definitions,
        queries,
    };
    c6_core::unit_placement_transition::normal_move::set(
        &mut context,
        request.destination,
        request.validation_flags,
    )
    .map_err(|e| Error::Transition(Box::new(e)))
}

impl<E> From<crate::loaded_actions::Error> for Error<E> {
    fn from(e: crate::loaded_actions::Error) -> Self {
        Self::Query(e)
    }
}
struct Context<'a, 'd, 'r, E> {
    sight: sight::Context<'a, E>,
    definitions: &'a Definitions<'d, 'r>,
    queries: &'a Queries<'d>,
}
impl<E: Effects> Context<'_, '_, '_, E> {
    fn actor(&self) -> Result<&c6_core::unit::Unit, Error<EffectsError<E>>> {
        c6_core::unit_sight::SightContext::unit(&self.sight).map_err(Error::Sight)
    }
    fn edit(&mut self) -> Result<&mut c6_core::unit::Unit, Error<EffectsError<E>>> {
        c6_core::unit_sight::SightContext::unit_mut(&mut self.sight).map_err(Error::Sight)
    }
}

use c6_core::{
    map::Plot,
    unit::{ComponentId, SightPlotEntry, Unit},
    unit_healing::{DamageChangedReport, HealingRules, HealingWorldState, UnitHealedSignal},
    unit_placement_transition::normal_move::Validation,
    unit_placement_transition::PlacementReport,
    unit_sight::VisibilityChange,
    world::World,
};

impl<E: Effects> c6_core::unit_sight::SightContext for Context<'_, '_, '_, E> {
    type Error = Error<EffectsError<E>>;
    type Map = ();
    type VisibilityManager = ();
    type Visibility = i32;
    type Player = i32;
    type Plot = u32;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        c6_core::unit_sight::SightContext::unit(&self.sight).map_err(Error::Sight)
    }
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
        c6_core::unit_sight::SightContext::unit_mut(&mut self.sight).map_err(Error::Sight)
    }
    fn preflight_sight(&self, add: bool) -> Result<(), Self::Error> {
        c6_core::unit_sight::SightContext::preflight_sight(&self.sight, add).map_err(Error::Sight)
    }
    fn sight_plots(
        &mut self,
        owner: i32,
        range: i32,
        origin: PlotCoord,
        through_features: bool,
        through_terrain: bool,
    ) -> Result<Vec<SightPlotEntry>, Self::Error> {
        c6_core::unit_sight::SightContext::sight_plots(
            &mut self.sight,
            owner,
            range,
            origin,
            through_features,
            through_terrain,
        )
        .map_err(Error::Sight)
    }
    fn globals_map(&mut self) -> Result<Self::Map, Self::Error> {
        c6_core::unit_sight::SightContext::globals_map(&mut self.sight).map_err(Error::Sight)
    }
    fn map_instance(&mut self) -> Result<Self::Map, Self::Error> {
        c6_core::unit_sight::SightContext::map_instance(&mut self.sight).map_err(Error::Sight)
    }
    fn visibility_manager(&mut self) -> Result<Self::VisibilityManager, Self::Error> {
        c6_core::unit_sight::SightContext::visibility_manager(&mut self.sight).map_err(Error::Sight)
    }
    fn owner_visibility(
        &mut self,
        manager: Self::VisibilityManager,
        owner: i32,
    ) -> Result<Self::Visibility, Self::Error> {
        c6_core::unit_sight::SightContext::owner_visibility(&mut self.sight, manager, owner)
            .map_err(Error::Sight)
    }
    fn player(&mut self, owner: i32) -> Result<Self::Player, Self::Error> {
        c6_core::unit_sight::SightContext::player(&mut self.sight, owner).map_err(Error::Sight)
    }
    fn domain(&mut self) -> Result<i32, Self::Error> {
        c6_core::unit_sight::SightContext::domain(&mut self.sight).map_err(Error::Sight)
    }
    fn plot_index(&mut self, map: Self::Map, origin: PlotCoord) -> Result<i32, Self::Error> {
        c6_core::unit_sight::SightContext::plot_index(&mut self.sight, map, origin)
            .map_err(Error::Sight)
    }
    fn plot_xy(&mut self, map: Self::Map, index: i32) -> Result<PlotCoord, Self::Error> {
        c6_core::unit_sight::SightContext::plot_xy(&mut self.sight, map, index)
            .map_err(Error::Sight)
    }
    fn plot(&mut self, map: Self::Map, index: u32) -> Result<Self::Plot, Self::Error> {
        c6_core::unit_sight::SightContext::plot(&mut self.sight, map, index).map_err(Error::Sight)
    }
    fn water(&mut self, plot: Self::Plot) -> Result<bool, Self::Error> {
        c6_core::unit_sight::SightContext::water(&mut self.sight, plot).map_err(Error::Sight)
    }
    fn feature(&mut self, plot: Self::Plot) -> Result<i32, Self::Error> {
        c6_core::unit_sight::SightContext::feature(&mut self.sight, plot).map_err(Error::Sight)
    }
    fn revealed_feature(
        &mut self,
        player: Self::Player,
        feature: i32,
    ) -> Result<bool, Self::Error> {
        c6_core::unit_sight::SightContext::revealed_feature(&mut self.sight, player, feature)
            .map_err(Error::Sight)
    }
    fn is_revealed(
        &mut self,
        visibility: Self::Visibility,
        index: u32,
    ) -> Result<bool, Self::Error> {
        c6_core::unit_sight::SightContext::is_revealed(&mut self.sight, visibility, index)
            .map_err(Error::Sight)
    }
    fn change_visibility(
        &mut self,
        manager: Self::VisibilityManager,
        change: VisibilityChange,
    ) -> Result<i32, Self::Error> {
        c6_core::unit_sight::SightContext::change_visibility(&mut self.sight, manager, change)
            .map_err(Error::Sight)
    }
    fn promotion_class(&mut self) -> Result<Option<&str>, Self::Error> {
        c6_core::unit_sight::SightContext::promotion_class(&mut self.sight).map_err(Error::Sight)
    }
    fn natural_wonder_experience(&mut self) -> Result<i32, Self::Error> {
        c6_core::unit_sight::SightContext::natural_wonder_experience(&mut self.sight)
            .map_err(Error::Sight)
    }
    fn experience_modifier(&mut self) -> Result<i32, Self::Error> {
        c6_core::unit_sight::SightContext::experience_modifier(&mut self.sight)
            .map_err(Error::Sight)
    }
    fn change_experience(&mut self, amount: i32) -> Result<(), Self::Error> {
        c6_core::unit_sight::SightContext::change_experience(&mut self.sight, amount)
            .map_err(Error::Sight)
    }
    fn plot_visibility_changed(
        &mut self,
        plot: Self::Plot,
        player: Self::Player,
        source: ComponentId,
        other_player: i32,
        first_reveal: bool,
    ) -> Result<(), Self::Error> {
        c6_core::unit_sight::SightContext::plot_visibility_changed(
            &mut self.sight,
            plot,
            player,
            source,
            other_player,
            first_reveal,
        )
        .map_err(Error::Sight)
    }
    fn signal_unit_plot(&mut self, plot: Self::Plot) -> Result<(), Self::Error> {
        c6_core::unit_sight::SightContext::signal_unit_plot(&mut self.sight, plot)
            .map_err(Error::Sight)
    }
}

impl<E: Effects> c6_core::unit_placement_transition::PlacementMembershipContext
    for Context<'_, '_, '_, E>
{
    type Error = Error<EffectsError<E>>;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        self.actor()
    }
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
        self.edit()
    }
    fn world_mut(&mut self) -> Result<&mut World, Self::Error> {
        self.sight.state.world_mut().map_err(Error::State)
    }
    fn preflight_membership(&self, destination: PlotCoord) -> Result<(), Self::Error> {
        self.index(destination)?;
        self.actor()?;
        Ok(())
    }
    fn local_plot_visible(&mut self, old: PlotCoord) -> Result<bool, Self::Error> {
        self.plot_visible(old)
    }
    fn local_unit_visible(&mut self) -> Result<bool, Self::Error> {
        self.unit_visible()
    }
    fn plot_mut(&mut self, location: PlotCoord) -> Result<&mut Plot, Self::Error> {
        let index = self.index(location)?;
        self.sight
            .state
            .world_mut()
            .map_err(Error::State)?
            .plot_mut(index as usize)
            .ok_or(Error::Missing("placement plot"))
    }
    fn remove_sight(&mut self, old: PlotCoord) -> Result<(), Self::Error> {
        c6_core::unit_sight::update_map(self, old, false).map_err(|e| {
            Error::Transition(Box::new(
                c6_core::unit_placement_transition::normal_move::Error::Sight(e),
            ))
        })
    }
    fn set_current_formation_location(
        &mut self,
        destination: PlotCoord,
    ) -> Result<(), Self::Error> {
        let unit = self.actor()?;
        let owner = unit.e_owner;
        let id = unit.id;
        let formation = self
            .sight
            .state
            .world()
            .player(owner)
            .map_err(Error::World)?
            .unit_formation(id)
            .map_err(Error::World)?
            .map(|f| f.record.id);
        if let Some(id) = formation {
            self.sight
                .state
                .formation_mut(owner, id)
                .map_err(Error::State)?
                .location = destination;
        }
        Ok(())
    }
    fn destination_is_water(&mut self, destination: PlotCoord) -> Result<bool, Self::Error> {
        self.queries
            .placement_context(self.sight.state, self.definitions.placement)
            .map_err(Error::Placement)?
            .water(self.plot_at_coordinate(destination)?)
            .map_err(Error::Placement)
    }
    fn destination_has_city_attributes(
        &mut self,
        destination: PlotCoord,
    ) -> Result<bool, Self::Error> {
        Ok(self
            .plot_at_coordinate(destination)?
            .city_attributes
            .is_some())
    }
    fn domain(&mut self) -> Result<i32, Self::Error> {
        c6_core::unit_sight::SightContext::domain(&mut self.sight).map_err(Error::Sight)
    }
    fn destination_city_domain_valid(
        &mut self,
        destination: PlotCoord,
        domain: i32,
    ) -> Result<bool, Self::Error> {
        let index = self.index(destination)?;
        self.queries
            .placement_context(self.sight.state, self.definitions.placement)
            .map_err(Error::Placement)?
            .city_domain_valid(index as usize, domain)
            .map_err(Error::Placement)
    }
    fn can_embark(&mut self) -> Result<bool, Self::Error> {
        self.queries
            .placement_context(self.sight.state, self.definitions.placement)
            .map_err(Error::Placement)?
            .can_embark(self.actor()?)
            .map_err(Error::Placement)
    }
    fn placement_layer(&mut self) -> Result<u32, Self::Error> {
        Err(Error::Missing("PlacementMembershipContext.placement_layer"))
    }
}

impl<E: Effects> c6_core::unit_placement_transition::PlacementStateContext
    for Context<'_, '_, '_, E>
{
    type Error = Error<EffectsError<E>>;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        self.actor()
    }
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
        self.edit()
    }
    fn preflight_placement_report(&self, event: u32) -> Result<(), Self::Error> {
        let h = self.handle()?;
        self.sight
            .effects
            .preflight_placement_report(self.sight.state, h, event)
            .map_err(Error::Effect)
    }
    fn send_placement_report(&mut self, report: PlacementReport) -> Result<(), Self::Error> {
        self.sight
            .effects
            .placement_report(self.sight.state, report)
            .map_err(Error::Effect)
    }
}

impl<E: Effects> c6_core::unit_healing::UnitHealingContext for Context<'_, '_, '_, E> {
    type Error = Error<EffectsError<E>>;
    fn unit(&self) -> Result<&Unit, Self::Error> {
        self.actor()
    }
    fn unit_mut(&mut self) -> Result<&mut Unit, Self::Error> {
        self.edit()
    }
    fn healing_world_state(&self) -> Result<HealingWorldState, Self::Error> {
        Err(Error::Missing("UnitHealingContext.healing_world_state"))
    }
    fn healing_rules(&self) -> Result<HealingRules, Self::Error> {
        Err(Error::Missing("UnitHealingContext.healing_rules"))
    }
    fn preflight_healing_effects(&self) -> Result<(), Self::Error> {
        Err(Error::Missing(
            "UnitHealingContext.preflight_healing_effects",
        ))
    }
    fn combat_max_hit_points(&self) -> Result<i32, Self::Error> {
        Ok(self.definitions.mechanics.maximum_hit_points)
    }
    fn maximum_damage_for_report(&self) -> Result<i32, Self::Error> {
        if self
            .actor()?
            .k_properties
            .entries
            .iter()
            .any(|e| e.key == 0x7d3fc711)
        {
            return Err(Error::Missing("dynamic maximum TypedVariant conversion"));
        }
        Ok(self.definitions.mechanics.maximum_hit_points)
    }
    fn send_damage_changed(&mut self, report: DamageChangedReport) -> Result<(), Self::Error> {
        self.sight.effects.damage_changed(self.sight.state, report)
    }
    fn clear_current_operations(&mut self) -> Result<(), Self::Error> {
        self.sight
            .effects
            .clear_operations(self.sight.state, self.sight.actor)
    }
    fn dispatch_unit_healed(&mut self, signal: UnitHealedSignal) -> Result<(), Self::Error> {
        let _ = signal;
        Err(Error::Missing("UnitHealingContext.dispatch_unit_healed"))
    }
}

impl<E: Effects> c6_core::improvement_movement::Context for Context<'_, '_, '_, E> {
    type Map = ();
    type Plot = u32;
    type Diplomacy = i32;
    type Definition = u32;
    type AlivePlayers = ();
    fn preflight_movement_damage(&self) -> Result<(), Self::Error> {
        Ok(()) // Normal Placement effects preflight already admitted this continuation.
    }
    fn barbarian_camp_improvement(&mut self) -> Result<u32, Self::Error> {
        Ok(self.definitions.mechanics.camp)
    }
    fn unit_is_barbarian(&mut self) -> Result<bool, Self::Error> {
        let owner = self.actor()?.e_owner;
        self.queries
            .civilization_levels
            .and_then(|v| v.iter().find(|p| p.0 == owner))
            .map(|p| p.1 == 0xd959ec24)
            .ok_or(Error::Missing("current civilization level"))
    }
    fn clans_active(&mut self) -> Result<bool, Self::Error> {
        Err(Error::Missing("Context.clans_active"))
    }
    fn unit_plot(&mut self) -> Result<Option<Self::Plot>, Self::Error> {
        let p = self.actor()?.k_location;
        Ok(Some(self.index(p)?))
    }
    fn improvement_type(&mut self, plot: Self::Plot) -> Result<u32, Self::Error> {
        Ok(self.plot_by_index(plot)?.improvement_hash)
    }
    fn improvement_owner(&mut self, plot: Self::Plot) -> Result<i32, Self::Error> {
        Ok(i32::from(self.plot_by_index(plot)?.improvement_owner))
    }
    fn improvement_pillaged(&mut self, plot: Self::Plot) -> Result<bool, Self::Error> {
        Ok(self
            .plot_by_index(plot)?
            .flags
            .get(c6_core::map::PlotFlag::ImprovementPillaged))
    }
    fn improvement_at(&mut self, kind: u32) -> Result<Self::Definition, Self::Error> {
        self.definitions
            .mechanics
            .improvements
            .find_index(kind)
            .map(|v| v as u32)
            .ok_or(Error::Missing("Improvement At"))
    }
    fn improvement_find(&mut self, kind: u32) -> Result<Option<Self::Definition>, Self::Error> {
        Ok(self
            .definitions
            .mechanics
            .improvements
            .find_index(kind)
            .map(|v| v as u32))
    }
    fn remove_on_entry(&mut self, definition: Self::Definition) -> Result<bool, Self::Error> {
        self.definitions
            .mechanics
            .remove_on_entry
            .get(definition as usize)
            .copied()
            .ok_or(Error::Missing("RemoveOnEntry"))
    }
    fn map_and_diplomacy(&mut self) -> Result<(Self::Map, Self::Diplomacy), Self::Error> {
        Ok(((), self.actor()?.e_owner))
    }
    fn plot_at(
        &mut self,
        map: Self::Map,
        location: PlotCoord,
    ) -> Result<Option<Self::Plot>, Self::Error> {
        let () = map;
        c6_core::movement_world::plot_at(self.sight.state.world().map(), location)
            .map(|(i, _)| Some(i as u32))
            .map_err(|_| Error::Missing("damage neighborhood GetPlot"))
    }
    fn plot_location(&mut self, plot: Self::Plot) -> Result<PlotCoord, Self::Error> {
        let width = self.sight.state.world().map().header.width;
        if width <= 0 {
            return Err(Error::Missing("map width"));
        }
        self.plot_by_index(plot)?;
        Ok(PlotCoord {
            x: plot as i32 % width,
            y: plot as i32 / width,
        })
    }
    fn plot_index(&mut self, plot: Self::Plot) -> Result<i32, Self::Error> {
        self.plot_by_index(plot)?;
        i32::try_from(plot).map_err(|_| Error::Missing("plot index range"))
    }
    fn at_war(&mut self, diplomacy: Self::Diplomacy, owner: i32) -> Result<bool, Self::Error> {
        if diplomacy == owner {
            return Ok(false);
        }
        self.sight
            .state
            .game_relations()
            .map_err(Error::State)?
            .is_at_war(diplomacy, owner)
            .map_err(|_| Error::Missing("current war relation"))
    }
    fn property(
        &mut self,
        definition: Self::Definition,
        key: u32,
    ) -> Result<Option<i32>, Self::Error> {
        let _ = definition;
        let _ = key;
        Err(Error::Missing("Context.property"))
    }
    fn update_improvement_visibility(
        &mut self,
        plot: Self::Plot,
        direction: i32,
    ) -> Result<(), Self::Error> {
        self.sight.effects.improvement_visibility(
            self.sight.state,
            self.sight.runtime,
            plot,
            direction,
        )
    }
    fn kill(&mut self, delayed: bool, attacker: i32) -> Result<(), Self::Error> {
        self.sight
            .effects
            .kill(self.sight.state, self.sight.actor, delayed, attacker)
    }
    fn alive_players(&mut self) -> Result<Self::AlivePlayers, Self::Error> {
        Err(Error::Missing("Context.alive_players"))
    }
    fn alive_player_count(&mut self, players: Self::AlivePlayers) -> Result<usize, Self::Error> {
        let () = players;
        Err(Error::Missing("Context.alive_player_count"))
    }
    fn alive_player(
        &mut self,
        players: Self::AlivePlayers,
        index: usize,
    ) -> Result<i32, Self::Error> {
        let () = players;
        let _ = index;
        Err(Error::Missing("Context.alive_player"))
    }
    fn signal_improvement_damage(&mut self, owner: i32, amount: i32) -> Result<(), Self::Error> {
        self.sight
            .effects
            .improvement_damage(self.sight.state, self.sight.actor, owner, amount)
    }
    fn set_dynamic_integer(&mut self, key: u32, value: i32) -> Result<(), Self::Error> {
        let _ = key;
        let _ = value;
        Err(Error::Missing("Context.set_dynamic_integer"))
    }
}

impl<E: Effects> c6_core::unit_placement_transition::normal_move::Context
    for Context<'_, '_, '_, E>
{
    type PlacementPlot = u32;
    fn validate_location(
        &mut self,
        destination: PlotCoord,
        flags: u32,
    ) -> Result<Validation, Error<EffectsError<E>>> {
        let q = self
            .queries
            .placement_context(self.sight.state, self.definitions.placement)
            .map_err(Error::Placement)?;
        let r = q
            .unit_record_placement(
                self.sight.state,
                self.actor()?,
                destination,
                c6_core::movement_shore::PlacementRequest::from_validate_options(flags),
            )
            .map_err(Error::Placement)?;
        match r {
            0 => Ok(Validation::Rejected),
            1 => Ok(Validation::Exact),
            _ => Err(Error::Missing("alternate placement coordinate")),
        }
    }
    fn preflight_normal_placement(
        &self,
        destination: PlotCoord,
    ) -> Result<(), Error<EffectsError<E>>> {
        self.handle()?;
        self.sight.effects.preflight_normal(
            self.sight.state,
            self.sight.runtime,
            self.sight.actor,
            destination,
        )
    }
    fn local_player(&mut self) -> Result<i32, Error<EffectsError<E>>> {
        self.queries
            .local_player
            .ok_or(Error::Missing("current local player"))
    }
    fn outgoing_visibility_to(
        &mut self,
        owner: i32,
        viewer: i32,
    ) -> Result<bool, Error<EffectsError<E>>> {
        Ok(
            crate::formation_path_state::mapped_visibility(self.sight.state, owner)
                .map_err(|_| Error::Missing("visibility owner"))?
                .outgoing_visibility
                .iter()
                .any(|v| v.player_id == viewer),
        )
    }
    fn outgoing_visibility_blocked(
        &mut self,
        owner: i32,
        viewer: i32,
    ) -> Result<bool, Error<EffectsError<E>>> {
        Ok(
            crate::formation_path_state::mapped_visibility(self.sight.state, owner)
                .map_err(|_| Error::Missing("visibility owner"))?
                .outgoing_visibility_blockers
                .iter()
                .any(|v| v.player_id == viewer),
        )
    }
    fn local_revealed_state(
        &mut self,
        destination: PlotCoord,
    ) -> Result<i32, Error<EffectsError<E>>> {
        let _ = destination;
        Err(Error::Missing("Context.local_revealed_state"))
    }
    fn visibility_messages_disabled(&mut self) -> Result<bool, Error<EffectsError<E>>> {
        Ok(self.sight.runtime.messages_disabled)
    }
    fn plot_distance(
        &mut self,
        old: PlotCoord,
        new: PlotCoord,
    ) -> Result<i32, Error<EffectsError<E>>> {
        Ok(crate::loaded_actions::city_range_attack::plot_distance(
            self.sight.state,
            (old.x, old.y),
            (new.x, new.y),
        ))
    }
    fn embarked_state_change(
        &mut self,
        target: bool,
        old_valid: bool,
    ) -> Result<(), Error<EffectsError<E>>> {
        self.sight.effects.embarked_state_change(
            self.sight.state,
            self.sight.runtime,
            self.sight.actor,
            target,
            old_valid,
        )
    }
    fn placement_plot(
        &mut self,
        at: PlotCoord,
    ) -> Result<Option<Self::PlacementPlot>, Error<EffectsError<E>>> {
        match c6_core::movement_world::plot_at(self.sight.state.world().map(), at) {
            Ok((i, _)) => Ok(Some(i as u32)),
            Err(_) => Err(Error::Missing("placement GetPlot")),
        }
    }
    fn plot_improvement(
        &mut self,
        plot: Self::PlacementPlot,
    ) -> Result<u32, Error<EffectsError<E>>> {
        Ok(self.plot_by_index(plot)?.improvement_hash)
    }
    fn improvement_fortification(&mut self, kind: u32) -> Result<i32, Error<EffectsError<E>>> {
        let i = self
            .definitions
            .mechanics
            .improvements
            .find_index(kind)
            .ok_or(Error::Missing("Improvement fortification definition"))?;
        self.definitions
            .mechanics
            .improvement_fortification
            .get(i)
            .copied()
            .ok_or(Error::Missing("Improvement fortification"))
    }
    fn plot_improvement_pillaged(
        &mut self,
        plot: Self::PlacementPlot,
    ) -> Result<bool, Error<EffectsError<E>>> {
        Ok(self
            .plot_by_index(plot)?
            .flags
            .get(c6_core::map::PlotFlag::ImprovementPillaged))
    }
    fn buildings_at_plot(
        &mut self,
        plot: Self::PlacementPlot,
    ) -> Result<Vec<u32>, Error<EffectsError<E>>> {
        let _ = plot;
        Err(Error::Missing("Context.buildings_at_plot"))
    }
    fn building_fortification(&mut self, kind: u32) -> Result<i32, Error<EffectsError<E>>> {
        let i = self
            .definitions
            .mechanics
            .buildings
            .find_index(kind)
            .ok_or(Error::Missing("Building fortification definition"))?;
        self.definitions
            .mechanics
            .building_fortification
            .get(i)
            .copied()
            .ok_or(Error::Missing("Building fortification"))
    }
    fn signal_unit_moved(
        &mut self,
        old: PlotCoord,
        new: PlotCoord,
    ) -> Result<(), Error<EffectsError<E>>> {
        self.sight.effects.signal_unit_moved(
            self.sight.state,
            self.sight.runtime,
            self.sight.actor,
            old,
            new,
        )
    }
    fn plot_is_city(&mut self, plot: Self::PlacementPlot) -> Result<bool, Error<EffectsError<E>>> {
        let _ = plot;
        Err(Error::Missing("Context.plot_is_city"))
    }
    fn plot_district_type(
        &mut self,
        plot: Self::PlacementPlot,
    ) -> Result<u32, Error<EffectsError<E>>> {
        let _ = plot;
        Err(Error::Missing("Context.plot_district_type"))
    }
    fn owning_city(
        &mut self,
        plot: Self::PlacementPlot,
    ) -> Result<ComponentId, Error<EffectsError<E>>> {
        let _ = plot;
        Err(Error::Missing("Context.owning_city"))
    }
    fn plot_district(
        &mut self,
        plot: Self::PlacementPlot,
    ) -> Result<ComponentId, Error<EffectsError<E>>> {
        let _ = plot;
        Err(Error::Missing("Context.plot_district"))
    }
    fn city_unit_changed(&mut self, city: ComponentId) -> Result<(), Error<EffectsError<E>>> {
        self.sight.effects.city_unit_changed(self.sight.state, city)
    }
    fn check_siege_near(
        &mut self,
        location: PlotCoord,
        owner: i32,
    ) -> Result<(), Error<EffectsError<E>>> {
        let _ = location;
        let _ = owner;
        Err(Error::Missing("Context.check_siege_near"))
    }
    fn award_post_move_bonuses(
        &mut self,
        destination: PlotCoord,
    ) -> Result<(), Error<EffectsError<E>>> {
        let _ = destination;
        Err(Error::Missing("Context.award_post_move_bonuses"))
    }
    fn notify_barbarian_approach(
        &mut self,
        destination: PlotCoord,
    ) -> Result<(), Error<EffectsError<E>>> {
        let _ = destination;
        Err(Error::Missing("Context.notify_barbarian_approach"))
    }
    fn diplomacy_unit_moved(&mut self, old: PlotCoord) -> Result<(), Error<EffectsError<E>>> {
        let _ = old;
        Err(Error::Missing("Context.diplomacy_unit_moved"))
    }
    fn rules_unit_moved(&mut self, old: PlotCoord) -> Result<(), Error<EffectsError<E>>> {
        let _ = old;
        Err(Error::Missing("Context.rules_unit_moved"))
    }
}

impl<E: Effects> Context<'_, '_, '_, E> {
    fn handle(&self) -> Result<crate::loaded_upkeep::UnitHandle, Error<EffectsError<E>>> {
        crate::loaded_upkeep::UnitHandle::resolve(
            self.sight.state,
            self.sight.actor.0,
            self.sight.actor.1,
        )
        .map_err(Error::World)?
        .ok_or(Error::Missing("placement actor"))
    }
    fn index(&self, p: PlotCoord) -> Result<u32, Error<EffectsError<E>>> {
        c6_core::movement_world::plot_at(self.sight.state.world().map(), p)
            .map(|(i, _)| i as u32)
            .map_err(|_| Error::Missing("placement coordinate"))
    }
    fn plot_at_coordinate(&self, p: PlotCoord) -> Result<&Plot, Error<EffectsError<E>>> {
        self.plot_by_index(self.index(p)?)
    }
    fn plot_by_index(&self, i: u32) -> Result<&Plot, Error<EffectsError<E>>> {
        self.sight
            .state
            .world()
            .map()
            .plot_by_index(i)
            .ok_or(Error::Missing("placement plot index"))
    }
    fn plot_visible(&self, p: PlotCoord) -> Result<bool, Error<EffectsError<E>>> {
        let local = self
            .queries
            .local_player
            .ok_or(Error::Missing("current local player"))?;
        let layers = self.sight.runtime.shared(local).map_err(Error::Missing)?;
        let normal = layers
            .0
            .iter()
            .find(|l| l.type_hash == c6_core::visibility_change::BASE_LAYER_HASH)
            .ok_or(Error::Missing("local normal layer"))?;
        crate::loaded_upkeep::placement_tails::plot_visible(
            self.sight.runtime,
            local,
            normal.grid_width,
            p,
        )
        .map_err(Error::Missing)
    }
    fn unit_visible(&self) -> Result<bool, Error<EffectsError<E>>> {
        let local = self
            .queries
            .local_player
            .ok_or(Error::Missing("current local player"))?;
        let unit = self.actor()?;
        let layers = self.sight.runtime.shared(local).map_err(Error::Missing)?;
        crate::loaded_actions::city_range_attack::unit_visible_with_counts(
            self.sight.state,
            unit,
            local,
            self.definitions.sight,
            |hash, _| {
                let l = layers
                    .0
                    .iter()
                    .find(|l| l.type_hash == hash)
                    .ok_or(Error::Missing("local visibility layer"))?;
                let i = l
                    .grid_width
                    .wrapping_mul(unit.k_location.y as u32)
                    .wrapping_add(unit.k_location.x as u32);
                if i >= l.count {
                    return Ok(0);
                }
                l.values
                    .get(i as usize)
                    .copied()
                    .ok_or(Error::Missing("local visibility cell"))
            },
            || {
                self.sight
                    .runtime
                    .hidden_layer_hashes
                    .first()
                    .copied()
                    .ok_or(Error::Missing("hidden layer0"))
            },
        )
    }
}

/// Static native collection mappings and scalar getters, initialized once.
pub struct Mechanics {
    binding: c6_core::core_player::initialization::WireBinding,
    improvements: crate::registry::LinuxDefinitionRegistry,
    remove_on_entry: Vec<bool>,
    improvement_fortification: Vec<i32>,
    buildings: crate::registry::LinuxDefinitionRegistry,
    building_fortification: Vec<i32>,
    camp: u32,
    maximum_hit_points: i32,
}
impl Mechanics {
    pub fn from_rules(
        state: &LoadedState,
        rules: &crate::Rules,
        bound: &crate::resource_binding::ResourceAuthority<'_>,
    ) -> Result<Self, crate::loaded_actions::Error> {
        use crate::loaded_actions::Error as Q;
        crate::unit_initialization_definitions::Definitions::validate_resources(rules, bound)
            .map_err(|_| Q::Unsupported("Placement admitted rules"))?;
        if state.binding() != bound.destination_binding() {
            return Err(Q::ConfigurationIdentity);
        }
        let improvements = crate::unit_initialization_definitions::collection(
            rules,
            "Improvements",
            "ImprovementType",
        )
        .map_err(|_| Q::Unsupported("Improvements registry"))?;
        let buildings =
            crate::unit_initialization_definitions::collection(rules, "Buildings", "BuildingType")
                .map_err(|_| Q::Unsupported("Buildings registry"))?;
        let values = |registry: &crate::registry::LinuxDefinitionRegistry,
                      table: &str,
                      column: &str|
         -> Result<Vec<i32>, Q> {
            let t = rules
                .gameplay()
                .table(table)
                .ok_or(Q::Unsupported("Placement definition table"))?;
            registry
                .entries()
                .iter()
                .map(|e| {
                    i32::try_from(
                        t.row_by_id(e.database_rowid)
                            .ok_or(Q::Unsupported("Placement row"))?
                            .integer(column)
                            .map_err(|_| Q::Unsupported("Placement scalar"))?,
                    )
                    .map_err(|_| Q::Unsupported("Placement scalar range"))
                })
                .collect()
        };
        let remove = values(&improvements, "Improvements", "RemoveOnEntry")?;
        let remove_on_entry = remove
            .into_iter()
            .map(|v| match v {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(Q::Unsupported("RemoveOnEntry bool")),
            })
            .collect::<Result<_, _>>()?;
        let improvement_fortification =
            values(&improvements, "Improvements", "GrantFortification")?;
        let building_fortification = values(&buildings, "Buildings", "GrantFortification")?;
        let camp = improvements
            .entries()
            .iter()
            .position(|e| e.type_name == b"IMPROVEMENT_BARBARIAN_CAMP")
            .ok_or(Q::Unsupported("BarbarianCamp definition"))? as u32;
        let maximum_hit_points = crate::game_parameters::Parameters::from_rules(rules)
            .and_then(|p| p.integer(crate::game_parameters::IntegerParameter::CombatMaxHitPoints))
            .map_err(|_| Q::Unsupported("COMBAT_MAX_HIT_POINTS"))?;
        Ok(Self {
            binding: state.binding().clone(),
            improvements,
            remove_on_entry,
            improvement_fortification,
            buildings,
            building_fortification,
            camp,
            maximum_hit_points,
        })
    }
}
