//! Current owned-object member preparation for ordinary melee generation.
//! Callers must perform AttackInto's own target re-selection first. A successful
//! readonly TestAttackInto continuation is not an entitlement to apply combat.
use crate::{formation_movement_cost, movement_placement_state};
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
}

/// Retains the actual current objects through the unfinished generation stage.
/// The shared borrow prevents silently mutating/replacing them between member
/// preparation and this continuation. This does not implement Generate/Resolve.
#[must_use]
pub struct RequiresGenerate<'s> {
    pub attacker: &'s Unit,
    pub defender: Defender<'s>,
    next: kernel::RequiresGenerate,
}
pub enum Defender<'s> {
    Unit(&'s Unit),
    District(&'s District),
}
impl RequiresGenerate<'_> {
    pub fn next(&self) -> &kernel::RequiresGenerate {
        &self.next
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
        attacker,
        defender: Defender::Unit(defender),
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
        attacker,
        defender: Defender::District(defender),
        next,
    })
}
