//! Current-state siege computation shared by placement and delayed death.
use super::*;
use crate::{
    formation_movement_cost as movement, loaded_upkeep::OperationReports, unit_placement_state,
};
use c6_core::unit_placement_transition::PlacementPayload;
use c6_core::{
    combat_defender as defender, map::UnitReference, unit_placement_tails::siege as kernel,
};

type Failure<E> = Error<<E as Services>::Error>;
struct Current<'a, 'd, 'r, E> {
    base: Context<'a, E>,
    definitions: &'a unit_placement_state::Definitions<'d, 'r>,
}

pub fn run<E>(
    state: &mut LoadedState,
    runtime: &mut sight::Runtime,
    effects: &mut E,
    definitions: &unit_placement_state::Definitions<'_, '_>,
    location: PlotCoord,
    owner: i32,
) -> Result<(), Failure<E>>
where
    E: Services + OperationReports<Error = <E as Services>::Error>,
{
    tails::check_for_siege_near(
        &mut Current {
            base: Context {
                state,
                sight: runtime,
                effects,
                actor: (owner, u32::MAX),
                encountered_type: None,
            },
            definitions,
        },
        location,
        owner,
    )
}
impl<E: Services> Current<'_, '_, '_, E> {
    fn district(&self, id: Actor) -> Result<&c6_core::player_districts::District, Failure<E>> {
        self.base
            .state
            .district(id.0, id.1)
            .map_err(Error::District)
    }
    fn members(&self, index: u32) -> Result<&[UnitReference], Failure<E>> {
        let map = self.base.state.world().map();
        let Some(layer) = map.units.layers.iter().find(|l| l.layer_id == 0) else {
            return Ok(&[]);
        };
        if index >= layer.maximum || index as usize >= map.plots.len() {
            return Ok(&[]);
        }
        self.base
            .state
            .world()
            .plot_units(0, index as usize)
            .map_err(Error::World)
    }
    fn unit_index(&self, location: PlotCoord) -> u32 {
        (location.y as u32)
            .wrapping_mul(self.base.state.world().map().units.width)
            .wrapping_add(location.x as u32)
    }
    fn best(&mut self, location: PlotCoord, viewer: i32) -> Result<Option<Actor>, Failure<E>> {
        self.base
            .state
            .world()
            .player(viewer)
            .map_err(Error::World)?;
        // Acquiring the current visibility receiver precedes candidate traversal.
        self.base.sight.shared(viewer).map_err(Error::Missing)?;
        let index = self.unit_index(location);
        defender::select_enemy(
            &mut Candidates {
                current: self,
                index,
                viewer,
            },
            viewer,
        )
    }
}
struct Candidates<'a, 'b, 'd, 'r, E> {
    current: &'a mut Current<'b, 'd, 'r, E>,
    index: u32,
    viewer: i32,
}
impl<E: Services> defender::Context for Candidates<'_, '_, '_, '_, E> {
    type Error = Failure<E>;
    type Unit = Actor;
    fn candidate_count(&mut self) -> Result<usize, Self::Error> {
        Ok(self.current.members(self.index)?.len())
    }
    fn resolve(&mut self, index: usize) -> Result<Option<Actor>, Self::Error> {
        let reference = *self
            .current
            .members(self.index)?
            .get(index)
            .ok_or(Error::Missing("siege candidate index"))?;
        Ok(self
            .current
            .base
            .state
            .world()
            .unit(reference)
            .map_err(Error::World)?
            .map(|u| (u.record.e_owner, u.record.id)))
    }
    fn delayed_death(&mut self, u: Actor) -> Result<bool, Self::Error> {
        Ok(self.current.base.current_unit(u)?.b_delayed_death)
    }
    fn owner(&mut self, u: Actor) -> Result<i32, Self::Error> {
        Ok(self.current.base.current_unit(u)?.e_owner)
    }
    fn combat_false(&mut self, u: Actor) -> Result<i32, Self::Error> {
        city_range_attack::unit_combat(
            self.current.base.state,
            self.current.base.current_unit(u)?,
            self.current.definitions.sight,
        )
        .map_err(Error::Combat)
    }
    fn ranged_combat(&mut self, u: Actor) -> Result<i32, Self::Error> {
        city_range_attack::unit_ranged_combat(
            self.current.base.state,
            self.current.base.current_unit(u)?,
            self.current.definitions.sight,
        )
        .map_err(Error::Combat)
    }
    fn bombard_combat(&mut self, u: Actor) -> Result<i32, Self::Error> {
        city_range_attack::unit_bombard_combat(
            self.current.base.state,
            self.current.base.current_unit(u)?,
            self.current.definitions.sight,
        )
        .map_err(Error::Combat)
    }
}
impl<E: Services> defender::EnemyContext for Candidates<'_, '_, '_, '_, E> {
    fn is_enemy(&mut self, owner: i32, viewer: i32) -> Result<bool, Self::Error> {
        self.current
            .base
            .state
            .game_relations()
            .map_err(Error::State)?
            .is_at_war(owner, viewer)
            .map_err(Error::Relations)
    }
    fn visible(&mut self, u: Actor) -> Result<bool, Self::Error> {
        let current = &self.current;
        let unit = current.base.current_unit(u)?;
        let location = unit.k_location;
        city_range_attack::unit_visible_with_counts(
            current.base.state,
            unit,
            self.viewer,
            current.definitions.sight,
            |hash, normal| {
                let layers = current
                    .base
                    .sight
                    .shared(self.viewer)
                    .map_err(Error::Missing)?;
                let Some(layer) = layers.0.iter().find(|l| l.type_hash == hash) else {
                    return if normal {
                        Err(Error::Missing("shared normal visibility layer"))
                    } else {
                        Ok(0)
                    };
                };
                let index = layer
                    .grid_width
                    .wrapping_mul(location.y as u32)
                    .wrapping_add(location.x as u32) as usize;
                Ok(layer.values.get(index).copied().unwrap_or(0))
            },
            || {
                current
                    .base
                    .sight
                    .hidden_layer_hashes
                    .first()
                    .copied()
                    .ok_or(Error::Missing("hidden visibility layer0"))
            },
        )
    }
}
impl<E> tails::SiegeContext for Current<'_, '_, '_, E>
where
    E: Services + OperationReports<Error = <E as Services>::Error>,
{
    type Error = Failure<E>;
    type Map = ();
    type Plot = u32;
    type District = Actor;
    fn map(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn plot_at(&mut self, _: (), p: PlotCoord) -> Result<Option<u32>, Self::Error> {
        self.base.plot_at_coord(p)
    }
    fn location(&mut self, p: u32) -> Result<PlotCoord, Self::Error> {
        tails::SiegeContext::location(&mut self.base, p)
    }
    fn distance(&mut self, m: (), a: PlotCoord, b: PlotCoord) -> Result<i32, Self::Error> {
        tails::SiegeContext::distance(&mut self.base, m, a, b)
    }
    fn owner(&mut self, p: u32) -> Result<i32, Self::Error> {
        self.base.plot_owner_value(p)
    }
    fn is_city(&mut self, p: u32) -> Result<bool, Self::Error> {
        plot_is_city(self.base.state, self.base.effects, p)
    }
    fn district(&mut self, p: u32) -> Result<Option<Actor>, Self::Error> {
        self.base.district_value(p)
    }
    fn is_besieged(&mut self, d: Actor) -> Result<bool, Self::Error> {
        kernel::is_besieged(self, d)
    }
    fn set_siege_status(&mut self, d: Actor, v: bool) -> Result<(), Self::Error> {
        kernel::set_siege_status(self, d, v)
    }
}
impl<E: Services> kernel::Context for Current<'_, '_, '_, E> {
    type Error = Failure<E>;
    type Map = ();
    type Player = i32;
    type City = Actor;
    type District = Actor;
    type Plot = u32;
    type Unit = Actor;
    type UnitContainer = ();
    type UnitList = u32;
    fn map(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn owner(&mut self, d: Actor) -> Result<i32, Self::Error> {
        self.district(d)?;
        Ok(d.0)
    }
    fn player(&mut self, p: i32) -> Result<i32, Self::Error> {
        self.base.state.world().player(p).map_err(Error::World)?;
        Ok(p)
    }
    fn city(&mut self, d: Actor) -> Result<Option<Actor>, Self::Error> {
        let id = self.district(d)?.city_id;
        if id == u32::MAX {
            return Ok(None);
        }
        self.base.state.city(d.0, id).map_err(Error::State)?;
        Ok(Some((d.0, id)))
    }
    fn siege_protection(&mut self, c: Actor) -> Result<bool, Self::Error> {
        Ok(self
            .base
            .state
            .city(c.0, c.1)
            .map_err(Error::State)?
            .combat
            .siege_protection)
    }
    fn location(&mut self, d: Actor) -> Result<PlotCoord, Self::Error> {
        let (x, y) = self.district(d)?.location;
        Ok(PlotCoord { x, y })
    }
    fn best_defender(
        &mut self,
        p: PlotCoord,
        first: i32,
        second: i32,
        flags: u32,
    ) -> Result<Option<Actor>, Self::Error> {
        if first != -1 || flags != 1 {
            return Err(Error::Missing("siege defender profile"));
        }
        self.best(p, second)
    }
    fn unit_container(&mut self, _: ()) -> Result<(), Self::Error> {
        Ok(())
    }
    fn ability_index(&mut self, name: &str) -> Result<i32, Self::Error> {
        Ok(self
            .base
            .definition()?
            .abilities
            .find_index(crate::registry::linux_type_hash(name.as_bytes()))
            .map_or(-1, |v| v as i32))
    }
    fn adjacent(&mut self, _: (), p: PlotCoord, d: u32) -> Result<Option<u32>, Self::Error> {
        let around = c6_core::skip_turn::placement::query_plots(self.base.state.world().map(), p)
            .map_err(|_| Error::Missing("siege map neighborhood"))?;
        match around.get(d as usize + 1).copied().flatten() {
            Some(p) => self.base.plot_at_coord(p),
            None => Ok(None),
        }
    }
    fn plot_location(&mut self, p: u32) -> Result<PlotCoord, Self::Error> {
        tails::SiegeContext::location(&mut self.base, p)
    }
    fn impassable(&mut self, p: u32) -> Result<bool, Self::Error> {
        Ok(self.base.current_plot(p)?.flags.get(PlotFlag::Impassable))
    }
    fn units(&mut self, _: (), p: PlotCoord, l: u32) -> Result<u32, Self::Error> {
        if l != 0 {
            return Err(Error::Missing("siege unit layer"));
        }
        Ok(self.unit_index(p))
    }
    fn unit_count(&mut self, l: u32) -> Result<usize, Self::Error> {
        Ok(self.members(l)?.len())
    }
    fn unit_id(&mut self, l: u32, i: usize) -> Result<ComponentId, Self::Error> {
        Ok(self
            .members(l)?
            .get(i)
            .ok_or(Error::Missing("siege unit index"))?
            .component_id())
    }
    fn find_unit(&mut self, p: i32, id: u32) -> Result<Option<Actor>, Self::Error> {
        Ok(self
            .base
            .state
            .world()
            .player(p)
            .map_err(Error::World)?
            .unit(id)
            .map_err(Error::World)?
            .map(|u| (u.record.e_owner, u.record.id)))
    }
    fn has_ability(&mut self, u: Actor, index: i32) -> Result<bool, Self::Error> {
        let Ok(index) = usize::try_from(index) else {
            return Ok(false);
        };
        let Some(hash) = self.base.definition()?.abilities.type_hash(index) else {
            return Ok(false);
        };
        Ok(self
            .base
            .current_unit(u)?
            .ai_abilities_possessed
            .iter()
            .find(|v| v.definition_hash == hash)
            .is_some_and(|v| v.value > 0))
    }
    fn player_id(&mut self, p: i32) -> Result<i32, Self::Error> {
        Ok(p)
    }
    fn at_war(&mut self, p: i32, o: i32) -> Result<bool, Self::Error> {
        self.base
            .state
            .game_relations()
            .map_err(Error::State)?
            .is_at_war(p, o)
            .map_err(Error::Relations)
    }
    fn enemy_or_zoc(&mut self, owner: i32, p: PlotCoord) -> Result<bool, Self::Error> {
        if self.best(p, owner)?.is_some() {
            return Ok(true);
        }
        let plot = self
            .base
            .plot_at_coord(p)?
            .ok_or(Error::Missing("siege ZOC plot"))?;
        let context = crate::movement_placement_state::Context::from_definitions(
            self.base.state,
            self.definitions.placement,
        )
        .map_err(Error::Placement)?;
        let water = context
            .water(self.base.current_plot(plot)?)
            .map_err(Error::Placement)?;
        movement::is_in_enemy_zoc(
            self.base.state,
            self.definitions.movement,
            p,
            None,
            movement::ZocIdentity {
                owner,
                domain: if water { 0 } else { 2 },
                religion: u32::MAX,
                layer: 0,
                ignores_zoc: false,
            },
            false,
            Some(self.base.sight),
        )
        .map_err(Error::Zoc)
    }
}
impl<E> kernel::StatusContext for Current<'_, '_, '_, E>
where
    E: Services + OperationReports<Error = <E as Services>::Error>,
{
    type Error = Failure<E>;
    type District = Actor;
    type City = Actor;
    fn siege_status(&self, d: Actor) -> Result<bool, Self::Error> {
        Ok(self.district(d)?.under_siege)
    }
    fn store_siege_status(&mut self, d: Actor, v: bool) -> Result<(), Self::Error> {
        self.base
            .state
            .district_mut(d.0, d.1)
            .map_err(Error::District)?
            .under_siege = v;
        Ok(())
    }
    fn city(&mut self, d: Actor) -> Result<Actor, Self::Error> {
        kernel::Context::city(self, d)?.ok_or(Error::Missing("siege parent city"))
    }
    fn city_identity(&mut self, c: Actor) -> Result<ComponentId, Self::Error> {
        self.base.state.city(c.0, c.1).map_err(Error::State)?;
        Ok(ComponentId {
            owner: c.0 as i16,
            component_type: 2,
            id: c.1,
        })
    }
    fn report(&mut self, event: u32, city: ComponentId, value: i32) -> Result<(), Self::Error> {
        self.base
            .effects
            .placement_report(
                self.base.state,
                PlacementReport {
                    event_hash: event,
                    identity: city,
                    payload: PlacementPayload::Integer(value),
                },
            )
            .map_err(Error::Effect)
    }
}
