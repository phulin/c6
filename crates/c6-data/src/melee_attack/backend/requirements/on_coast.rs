//! UnitOnCoast on a ProposedCombat subject. The proposal exposes IPlot but no
//! IUnit, so initialization never binds UnitMoved; release has no bound handle.
use super::{registry, sets, Error};
use crate::{registry::linux_type_hash, resource_binding::ResourceAuthority, Rules};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    map::{Plot, PlotFlag},
    simple_requirement::{self, Requirement},
};

pub const TYPE_HASH: u32 = 0x91c0_8f7e;
struct Definition {
    id: Vec<u8>,
    water: bool,
    triggered: bool,
}
pub struct Definitions {
    binding: WireBinding,
    requirements: Vec<Definition>,
    terrains: Vec<(u32, bool, bool)>,
    ocean_minimum: i32,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        authority: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        crate::unit_initialization_definitions::Definitions::validate_resources(rules, authority)
            .map_err(|_| Error::Missing("coast rules binding"))?;
        if authority.destination_binding() != state.binding() {
            return Err(Error::Missing("coast state binding"));
        }
        let table = |name| rules.gameplay().table(name).ok_or(Error::Missing(name));
        let mut requirements = Vec::new();
        for row in table("Requirements")?
            .rows()
            .filter(|r| r.text("RequirementType").ok() == Some("REQUIREMENT_UNIT_ON_COAST"))
        {
            let id = row
                .text("RequirementId")
                .map_err(|_| Error::Missing("coast ID"))?;
            let mut water = false;
            let mut seen = false;
            for argument in table("RequirementArguments")?.rows().filter(|a| {
                a.text("RequirementId").ok() == Some(id) && a.text("Name").ok() == Some("Water")
            }) {
                if seen || argument.text("Type").ok() != Some("ARGTYPE_IDENTITY") {
                    return Err(Error::Missing("coast Water argument form"));
                }
                seen = true;
                // The admitted scalar forms; other parse branches remain explicit.
                water = match argument
                    .text("Value")
                    .map_err(|_| Error::Missing("coast Water"))?
                {
                    "1" => true,
                    "0" => false,
                    _ => return Err(Error::Missing("unsupported coast Water text")),
                };
            }
            requirements.push(Definition {
                id: id.as_bytes().to_vec(),
                water,
                triggered: row
                    .integer("Triggered")
                    .map_err(|_| Error::Missing("coast Triggered"))?
                    != 0,
            });
        }
        let terrains = table("Terrains")?
            .rows()
            .map(|row| {
                Ok((
                    linux_type_hash(
                        row.text("TerrainType")
                            .map_err(|_| Error::Missing("coast TerrainType"))?
                            .as_bytes(),
                    ),
                    row.integer("Water")
                        .map_err(|_| Error::Missing("coast Water terrain"))?
                        != 0,
                    row.integer("ShallowWater")
                        .map_err(|_| Error::Missing("coast ShallowWater"))?
                        != 0,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let ocean_minimum = table("GlobalParameters")?
            .rows()
            .find(|r| r.text("Name").ok() == Some("OCEAN_MIN_WATER_SIZE"))
            .and_then(|r| r.get("Value").ok())
            .and_then(crate::game_parameters::sql_integer)
            .ok_or(Error::Missing("OCEAN_MIN_WATER_SIZE"))?;
        Ok(Self {
            binding: state.binding().clone(),
            requirements,
            terrains,
            ocean_minimum,
        })
    }
    fn terrain(&self, plot: &Plot) -> Result<(bool, bool), Error> {
        self.terrains
            .iter()
            .find(|r| r.0 == plot.terrain_hash)
            .map(|r| (r.1, r.2))
            .ok_or(Error::Missing("coast current terrain definition"))
    }
    fn accepted(
        &self,
        state: &LoadedState,
        plot: &Plot,
        location: c6_core::unit::PlotCoord,
    ) -> Result<bool, Error> {
        let (water, shallow) = self.terrain(plot)?;
        if water {
            return Ok(shallow);
        }
        for direction in 0..6 {
            let Some(index) = crate::barbarian_camps::queries::adjacent(state, location, direction)
                .map_err(|_| Error::Missing("coast adjacent plot"))?
            else {
                continue;
            };
            let adjacent = state
                .world()
                .map()
                .plots
                .get(index)
                .ok_or(Error::Missing("coast adjacent resident"))?;
            if !self.terrain(adjacent)?.0 || adjacent.flags.get(PlotFlag::Impassable) {
                continue;
            }
            if self.ocean_minimum <= 0 {
                return Ok(true);
            }
            if adjacent.landmass_id != -1
                && state
                    .world()
                    .map()
                    .landmasses
                    .entries
                    .iter()
                    .find(|r| r.index == (adjacent.landmass_id as u16) as i32)
                    .is_some_and(|r| r.value.plot_count >= self.ocean_minimum)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
struct Initial(Requirement);
impl simple_requirement::Context for Initial {
    type Error = Error;
    fn active(&mut self) -> Result<bool, Error> {
        Ok(self.0.active)
    }
    fn requirement(&mut self) -> Result<&mut Requirement, Error> {
        Ok(&mut self.0)
    }
    fn changed(&mut self, _: i32, _: i32) -> Result<(), Error> {
        Err(Error::Missing(
            "initial coast child has no changed subscribers",
        ))
    }
}
pub(super) fn initialize(
    state: &LoadedState,
    definitions: &Definitions,
    subject: registry::Handle,
    definition: &sets::RequirementDefinition,
) -> Result<Requirement, Error> {
    if definitions.binding != *state.binding() {
        return Err(Error::Missing("coast current binding"));
    }
    let parameters = definitions
        .requirements
        .iter()
        .find(|r| r.id == definition.id)
        .ok_or(Error::Missing("coast definition flags/arguments"))?;
    // SimpleRequirement.Initialize calls Activate, then ForceState(1) for
    // IsTriggered; only ordinary children enter virtual Test/SetState(false).
    if parameters.triggered {
        return Ok(Requirement {
            active: true,
            state: 1,
            inverse: definition.inverse,
            persistent: definition.persistent,
            triggered: true,
        });
    }
    let proposal = state
        .combat_proposals()
        .map_err(Error::Access)?
        .proposal(subject)
        .map_err(Error::Registry)?;
    // The IPlot interface exists even when its current GetPlot returns NULL.
    let tested = match proposal.plot.and_then(|at| {
        state
            .world()
            .map()
            .plot_at(at.x, at.y)
            .map(|plot| (at, plot))
    }) {
        Some((at, plot)) if parameters.water => i32::from(!definitions.accepted(state, plot, at)?),
        _ => 1,
    };
    let mut initial = Initial(Requirement {
        active: true,
        state: 3,
        inverse: definition.inverse,
        persistent: definition.persistent,
        triggered: parameters.triggered,
    });
    simple_requirement::set_state(&mut initial, tested, false)?;
    Ok(initial.0)
}
