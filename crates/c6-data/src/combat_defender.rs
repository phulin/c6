//! Concrete default/null-attacker defender lookup over current canonical layer0.
//! Combat getters reuse the existing admitted rules implementation; errors retain
//! their missing inputs. This does not admit other GetBestDefender flag profiles.
use crate::formation_rules;
use c6_core::{
    combat_defender as kernel,
    loaded_state::LoadedState,
    map::UnitReference,
    unit::{PlotCoord, Unit},
};

#[derive(Debug)]
pub enum Error {
    World(c6_core::world::MissingInput),
    Combat(crate::loaded_actions::Error),
    InvalidatedCandidate(usize),
    MissingUnit(UnitReference),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "default combat defender: {self:?}")
    }
}
impl std::error::Error for Error {}

pub(crate) struct Current<'a, 'd> {
    state: &'a LoadedState,
    definitions: &'d formation_rules::Definitions,
    index: u32,
    layer: u32,
}
impl<'a, 'd> Current<'a, 'd> {
    pub(crate) fn new(
        state: &'a LoadedState,
        definitions: &'d formation_rules::Definitions,
        location: PlotCoord,
        layer: Option<u32>,
    ) -> Self {
        Current {
            state,
            definitions,
            index: (location.y as u32)
                .wrapping_mul(state.world().map().units.width)
                .wrapping_add(location.x as u32),
            layer: layer.unwrap_or(u32::MAX),
        }
    }
    fn members(&self) -> Result<&[UnitReference], Error> {
        let world = self.state.world();
        let map = world.map();
        if self.layer == u32::MAX {
            return Ok(&[]);
        }
        let Some(layer) = map.units.layers.iter().find(|l| l.layer_id == self.layer) else {
            return Ok(&[]);
        };
        // SparseGrid.Get uses wrapping width*y+x and the grid's own maximum,
        // without independent coordinate normalization or x/y validity tests.
        // World validates every nonempty cell within map.plots on admission;
        // an index beyond that range therefore also has no admitted cell.
        if self.index >= layer.maximum || self.index as usize >= map.plots.len() {
            return Ok(&[]);
        }
        world
            .plot_units(self.layer, self.index as usize)
            .map_err(Error::World)
    }
    pub(crate) fn unit(&self, id: UnitReference) -> Result<&'a Unit, Error> {
        self.state
            .world()
            .unit(id)
            .map_err(Error::World)?
            .map(|u| u.record)
            .ok_or(Error::MissingUnit(id))
    }
}
impl kernel::Context for Current<'_, '_> {
    type Error = Error;
    type Unit = UnitReference;
    fn candidate_count(&mut self) -> Result<usize, Error> {
        Ok(self.members()?.len())
    }
    fn resolve(&mut self, index: usize) -> Result<Option<UnitReference>, Error> {
        let id = *self
            .members()?
            .get(index)
            .ok_or(Error::InvalidatedCandidate(index))?;
        Ok(self
            .state
            .world()
            .unit(id)
            .map_err(Error::World)?
            .map(|_| id))
    }
    fn delayed_death(&mut self, id: UnitReference) -> Result<bool, Error> {
        Ok(self.unit(id)?.b_delayed_death)
    }
    fn owner(&mut self, id: UnitReference) -> Result<i32, Error> {
        Ok(self.unit(id)?.e_owner)
    }
    fn combat_false(&mut self, id: UnitReference) -> Result<i32, Error> {
        self.definitions
            .combat(self.state, self.unit(id)?)
            .map_err(Error::Combat)
    }
    fn ranged_combat(&mut self, id: UnitReference) -> Result<i32, Error> {
        self.definitions
            .ranged(self.state, self.unit(id)?)
            .map_err(Error::Combat)
    }
    fn bombard_combat(&mut self, id: UnitReference) -> Result<i32, Error> {
        self.definitions
            .bombard(self.state, self.unit(id)?)
            .map_err(Error::Combat)
    }
}

/// Exact GetBestDefender(location,NO_PLAYER,NO_PLAYER,null,flags0). All owners
/// compete in the current layer0 vector order; equal melee retains the first.
pub fn get_best_defender<'a>(
    state: &'a LoadedState,
    definitions: &formation_rules::Definitions,
    location: PlotCoord,
) -> Result<Option<&'a Unit>, Error> {
    let mut current = Current::new(state, definitions, location, Some(0));
    let selected = kernel::select_default(&mut current)?;
    selected.map(|id| current.unit(id)).transpose()
}

/// The additional IsCombat performed by each District.Has/GetGarrisonedUnit,
/// after the selector returns. Do not cache this across the two district calls.
pub fn is_combat(
    state: &LoadedState,
    definitions: &formation_rules::Definitions,
    unit: &Unit,
) -> Result<bool, Error> {
    Ok(definitions.combat(state, unit).map_err(Error::Combat)? > 0
        || definitions.ranged(state, unit).map_err(Error::Combat)? > 0
        || definitions.bombard(state, unit).map_err(Error::Combat)? > 0)
}

/// The caller owns the preceding District.IsComplete/definition.InternalOnly
/// guards, including their source query order. This is its shared lookup tail.
pub fn garrison_lookup<'a>(
    state: &'a LoadedState,
    definitions: &formation_rules::Definitions,
    location: PlotCoord,
) -> Result<Option<&'a Unit>, Error> {
    match get_best_defender(state, definitions, location)? {
        Some(unit) if is_combat(state, definitions, unit)? => Ok(Some(unit)),
        _ => Ok(None),
    }
}
