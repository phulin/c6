//! UnitOnHomeContinent2398280/23983d0 for an actual ProposedCombat subject.
//! ProposedCombat has IPlot and no IUnit: OnInitialize therefore creates no
//! UnitMoved subscription. It still owns a real SimpleRequirement state.
use super::{registry, sets, ContextObject, Error};
use crate::{
    registry::{linux_type_hash, LinuxDefinitionRegistry},
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::collection,
    Rules,
};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    simple_requirement::{self, Requirement},
};
pub struct Definitions {
    binding: WireBinding,
    continents: LinuxDefinitionRegistry,
    flags: Vec<(Vec<u8>, bool)>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        authority: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        crate::unit_initialization_definitions::Definitions::validate_resources(rules, authority)
            .map_err(|_| Error::Missing("home rules binding"))?;
        if authority.destination_binding() != state.binding() {
            return Err(Error::Missing("home state binding"));
        }
        let continents = collection(rules, "Continents", "ContinentType")
            .map_err(|_| Error::Missing("continent definitions"))?;
        let table = rules
            .gameplay()
            .table("Requirements")
            .ok_or(Error::Missing("requirement definitions"))?;
        let mut flags = Vec::new();
        for row in table.rows().filter(|row| {
            row.text("RequirementType").ok() == Some("REQUIREMENT_UNIT_ON_HOME_CONTINENT")
        }) {
            let id = row
                .text("RequirementId")
                .map_err(|_| Error::Missing("home requirement ID"))?
                .as_bytes()
                .to_vec();
            let triggered = row
                .get("Triggered")
                .ok()
                .and_then(|v| v.integer().ok())
                .ok_or(Error::Missing("home Triggered flag"))?
                != 0;
            flags.push((id, triggered));
        }
        Ok(Self {
            binding: state.binding().clone(),
            continents,
            flags,
        })
    }
}
fn tested(
    state: &LoadedState,
    definitions: &Definitions,
    context: ContextObject,
    subject: registry::Handle,
) -> Result<i32, Error> {
    let ContextObject::Unit(unit) = context else {
        return Ok(1);
    };
    let proposal = state
        .combat_proposals()
        .map_err(Error::Access)?
        .proposal(subject)
        .map_err(Error::Registry)?;
    let Some(location) = proposal.plot else {
        return Ok(1);
    };
    let Some(plot) = state.world().map().plot_at(location.x, location.y) else {
        return Ok(1);
    };
    let unit = crate::melee_attack::actor(state, unit).map_err(|e| Error::Actor(Box::new(e)))?;
    let player = state
        .retained()
        .players
        .iter()
        .find(|p| p.header.owner == unit.e_owner)
        .ok_or(Error::Missing("home context player"))?;
    // Native Cities.Read remaps absent hashes to NO_CONTINENT(-1). The complete
    // admitted domain is required; raw hash equality would mishandle misses.
    let home = definitions
        .continents
        .find_index(player.cities.home_continent_hash)
        .map_or(-1, |i| i as i32);
    let current = definitions
        .continents
        .find_index(plot.continent_hash)
        .map_or(-1, |i| i as i32);
    Ok(i32::from(home != current))
}
struct Initial {
    control: Requirement,
}
impl simple_requirement::Context for Initial {
    type Error = Error;
    fn active(&mut self) -> Result<bool, Error> {
        Ok(self.control.active)
    }
    fn requirement(&mut self) -> Result<&mut Requirement, Error> {
        Ok(&mut self.control)
    }
    fn changed(&mut self, _: i32, _: i32) -> Result<(), Error> {
        Err(Error::Missing(
            "initial home child has no changed subscribers",
        ))
    }
}
pub(super) fn initialize(
    state: &LoadedState,
    definitions: &Definitions,
    context: ContextObject,
    subject: registry::Handle,
    definition: &sets::RequirementDefinition,
) -> Result<Requirement, Error> {
    if definitions.binding != *state.binding() {
        return Err(Error::Missing("home current binding"));
    }
    if definition.predicate
        != sets::Predicate::Unsupported(linux_type_hash(b"REQUIREMENT_UNIT_ON_HOME_CONTINENT"))
    {
        return Err(Error::Missing("home factory identity"));
    }
    let triggered = definitions
        .flags
        .iter()
        .find(|r| r.0 == definition.id)
        .map(|r| r.1)
        .ok_or(Error::Missing("home definition flags"))?;
    let value = tested(state, definitions, context, subject)?;
    let mut initial = Initial {
        control: Requirement {
            active: true,
            state: 3,
            inverse: definition.inverse,
            persistent: definition.persistent,
            triggered,
        },
    };
    // Parent attaches child->set listeners only after all child initializers.
    simple_requirement::set_state(&mut initial, value, false)?;
    Ok(initial.control)
}

struct Release<'a>(&'a mut super::Runtime);
impl sets::changes::Context for Release<'_> {
    type Error = Error;
    fn subject_sets(&self) -> Result<&sets::Runtime, Error> {
        Ok(&self.0.sets)
    }
    fn subject_sets_mut(&mut self) -> Result<&mut sets::Runtime, Error> {
        Ok(&mut self.0.sets)
    }
    fn test_requirement(&mut self, _: u64) -> Result<i32, sets::Error<Error>> {
        Err(sets::Error::Context(Error::Missing(
            "release cannot Test requirement",
        )))
    }
    fn modifier_changed(
        &mut self,
        _: sets::SetListener,
        _: u64,
        _: i32,
        _: i32,
    ) -> Result<(), sets::Error<Error>> {
        Err(sets::Error::Context(Error::Missing(
            "release cannot notify modifier",
        )))
    }
    fn destroy_requirement(&mut self, label: u64) -> Result<(), sets::Error<Error>> {
        let definition = self
            .0
            .sets
            .requirement_definition(label)
            .ok_or(sets::Error::MissingDefinitionInput)?;
        if definition.predicate
            == sets::Predicate::Unsupported(linux_type_hash(b"REQUIREMENT_UNIT_ON_HOME_CONTINENT"))
            && matches!(
                self.0.sets.requirement_context(label),
                Some((_, sets::Subject::Proposal(_)))
            )
        {
            // This factory's Proposal has no IUnit interface, so its native
            // message handle was never bound; derived destructor has no event.
            Ok(())
        } else {
            Err(sets::Error::Context(Error::Missing(
                "derived Simple requirement destructor",
            )))
        }
    }
    fn destroy_set(&mut self, _: u64) -> Result<(), sets::Error<Error>> {
        // Shared release has already detached/destroyed every child handle.
        // SimpleRequirementSet's remaining destruction only releases storage.
        Ok(())
    }
    fn reset_trigger(&mut self, _: u64) -> Result<(), sets::Error<Error>> {
        Err(sets::Error::Context(Error::Missing(
            "release cannot reset trigger",
        )))
    }
}
pub(super) fn release(runtime: &mut super::Runtime, set: u64) -> Result<(), sets::Error<Error>> {
    sets::changes::release(&mut Release(runtime), set)
}
