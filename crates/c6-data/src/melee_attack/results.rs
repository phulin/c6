//! Current owned-object member preparation for ordinary melee generation.
//! Callers must perform AttackInto's own target re-selection first. A successful
//! readonly TestAttackInto continuation is not an entitlement to apply combat.
use crate::{formation_movement_cost, formation_rules, movement_placement_state};
use c6_core::{
    loaded_state::LoadedState,
    map::UnitReference,
    melee_attack::results as kernel,
    player_districts::District,
    unit::{ComponentId, PlotCoord, Unit},
};

#[derive(Debug)]
pub enum Error {
    Actor(super::Error),
    Cost(formation_movement_cost::Error),
    Plot(c6_core::movement_world::MissingInput),
    Owner(movement_placement_state::Error),
    MissingDistrict,
    OwnerRange,
    Combat(crate::loaded_actions::Error),
    Unsupported(&'static str),
}

/// Retains the actual current objects through the unfinished generation stage.
/// The shared borrow prevents silently mutating/replacing them between member
/// preparation and this continuation. This does not implement Generate/Resolve.
#[must_use]
pub struct RequiresGenerate<'s> {
    state: &'s LoadedState,
    attacker: &'s Unit,
    defender: Defender<'s>,
    location: PlotCoord,
    next: kernel::RequiresGenerate,
}
pub enum Defender<'s> {
    Unit(&'s Unit),
    District(&'s District),
}
impl RequiresGenerate<'_> {
    pub fn attacker(&self) -> &Unit {
        self.attacker
    }
    pub fn defender(&self) -> &Defender<'_> {
        &self.defender
    }
    pub fn next(&self) -> &kernel::RequiresGenerate {
        &self.next
    }
    /// Compute the first Unit/Unit generation stage on the retained objects.
    /// Reaching a proposal is still unfinished: its registry writes and ordered
    /// synchronous modifier callbacks must run before strength modifiers.
    pub fn prepare_unit_strengths(
        &self,
        definitions: &formation_rules::Definitions,
    ) -> Result<kernel::UnitGeneration, Error> {
        let Defender::Unit(defender) = self.defender else {
            return Err(Error::Unsupported("Unit/District strength generation"));
        };
        kernel::prepare_unit_strengths(&mut CurrentStrengths {
            state: self.state,
            definitions,
            attacker: self.attacker,
            defender,
            location: self.location,
        })
    }
}

pub(crate) fn current_unit_strengths(
    state: &LoadedState,
    definitions: &formation_rules::Definitions,
    attacker: UnitReference,
    defender: UnitReference,
    location: PlotCoord,
) -> Result<kernel::UnitGeneration, Error> {
    kernel::prepare_unit_strengths(&mut CurrentStrengths {
        state,
        definitions,
        attacker: super::actor(state, attacker).map_err(Error::Actor)?,
        defender: super::actor(state, defender).map_err(Error::Actor)?,
        location,
    })
}

struct CurrentStrengths<'a> {
    state: &'a LoadedState,
    definitions: &'a formation_rules::Definitions,
    attacker: &'a Unit,
    defender: &'a Unit,
    location: PlotCoord,
}
impl kernel::StrengthContext for CurrentStrengths<'_> {
    type Error = Error;
    fn plot_exists(&mut self) -> Result<bool, Error> {
        // The bounded adapter requires a canonical resident plot. It does not
        // invent a null Map service result for missing imported map contents.
        c6_core::movement_world::plot_at(self.state.world().map(), self.location)
            .map_err(Error::Plot)?;
        Ok(true)
    }
    fn attacker_embarked(&mut self) -> Result<bool, Error> {
        Ok(crate::unit_movement_state::unit_is_embarked(self.attacker))
    }
    fn plot_is_coastal_land(&mut self) -> Result<bool, Error> {
        Err(Error::Unsupported(
            "current Plot.IsCoastalLand(-1) for embarked melee",
        ))
    }
    fn attacker_combat(&mut self, coastal: bool) -> Result<i32, Error> {
        if coastal {
            return Err(Error::Unsupported("GetCombat(true)"));
        }
        self.definitions
            .combat(self.state, self.attacker)
            .map_err(Error::Combat)
    }
    fn defender_combat(&mut self) -> Result<i32, Error> {
        self.definitions
            .combat(self.state, self.defender)
            .map_err(Error::Combat)
    }
    fn defender_formation_class(&mut self) -> Result<i32, Error> {
        self.definitions
            .formation_class(self.state, self.defender)
            .map_err(Error::Combat)
    }
}

fn unit_member(unit: &Unit) -> Result<kernel::Member, Error> {
    Ok(kernel::Member {
        identity: ComponentId {
            owner: i16::try_from(unit.e_owner).map_err(|_| Error::OwnerRange)?,
            component_type: 1,
            id: unit.id,
        },
        location: unit.k_location,
    })
}

pub fn prepare_units<'s>(
    state: &'s LoadedState,
    results: &mut kernel::Members,
    attacker: UnitReference,
    defender: UnitReference,
    flags: u32,
) -> Result<RequiresGenerate<'s>, Error> {
    let attacker = super::actor(state, attacker).map_err(Error::Actor)?;
    let defender = super::actor(state, defender).map_err(Error::Actor)?;
    let next = kernel::prepare_units(
        results,
        unit_member(attacker)?,
        unit_member(defender)?,
        flags,
    );
    Ok(RequiresGenerate {
        state,
        attacker,
        defender: Defender::Unit(defender),
        location: results.location,
        next,
    })
}

/// Resolves the actual District at the requested plot, then snapshots its
/// GetComponentID using CURRENT Plot.GetOwner, not the containing player's ID.
pub fn prepare_district<'s>(
    state: &'s LoadedState,
    definitions: &formation_movement_cost::Definitions<'_>,
    results: &mut kernel::Members,
    attacker: UnitReference,
    at: PlotCoord,
    flags: u32,
) -> Result<RequiresGenerate<'s>, Error> {
    let attacker = super::actor(state, attacker).map_err(Error::Actor)?;
    let (_, plot) =
        c6_core::movement_world::plot_at(state.world().map(), at).map_err(Error::Plot)?;
    let defender = definitions
        .district_at(state, plot)
        .map_err(Error::Cost)?
        .ok_or(Error::MissingDistrict)?;
    let location = PlotCoord {
        x: defender.location.0,
        y: defender.location.1,
    };
    let (_, current_plot) =
        c6_core::movement_world::plot_at(state.world().map(), location).map_err(Error::Plot)?;
    let owner = movement_placement_state::plot_owner(current_plot).map_err(Error::Owner)?;
    let member = kernel::Member {
        identity: ComponentId {
            owner: i16::try_from(owner).map_err(|_| Error::OwnerRange)?,
            component_type: 3,
            id: defender.id,
        },
        location,
    };
    let next = kernel::prepare_district(results, unit_member(attacker)?, member, flags);
    Ok(RequiresGenerate {
        state,
        attacker,
        defender: Defender::District(defender),
        location: results.location,
        next,
    })
}
