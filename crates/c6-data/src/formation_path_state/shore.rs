//! Live shore gates. Facts are read only when the native short circuit reaches them.
use super::*;
use c6_core::movement_shore::{self as shore, MissingFact as F, ShoreFacts, ShoreGate};

pub(super) fn query(
    c: &Context<'_, '_, '_>,
    from: PlotCoord,
    to: PlotCoord,
    disembark: Option<(bool, bool)>,
) -> Result<bool, Error> {
    let mut facts = ShoreFacts::default();
    loop {
        let gate = match disembark {
            Some((allow, flag7)) => shore::disembark_gate(&facts, allow, flag7),
            None => shore::embark_gate(&facts, true),
        };
        let missing = match gate {
            Ok(ShoreGate::Denied) => return Ok(false),
            Ok(ShoreGate::NeedsPlacement(request)) => {
                let result = c
                    .placement()?
                    .formation_placement(c.record().owner, c.record().id, to, request)
                    .map_err(Error::Placement)?;
                return Ok(request.accepts_result(result));
            }
            Err(missing) => missing,
        };
        match missing {
            F::Embarked => facts.embarked = Some(c.record().is_embarked),
            F::CanEmbark => facts.can_embark = Some(c.record().can_embark),
            F::DestinationWater => facts.destination_water = Some(c.water(to)?),
            F::SourceWater => facts.source_water = Some(c.water(from)?),
            F::SourceAdjacentShallowWater => {
                facts.source_adjacent_shallow_water = Some(adjacent(c, from, true)?)
            }
            F::SourceAdjacentLand => facts.source_adjacent_land = Some(adjacent(c, from, false)?),
            F::SourceCliffCrossing => {
                let direction = movement_world::direction_to(c.state.world().map(), from, to)
                    .map_err(|_| Error::Missing("shore direction"))?
                    .map_or(-1, i32::from);
                facts.source_cliff_crossing = Some(shore::cliff_crossing(
                    c.require_plot(from)?.1.cliff_crossing_mask,
                    direction,
                ));
            }
            F::SourceCity => facts.source_city = Some(city(c, from)?),
            F::DestinationCity => facts.destination_city = Some(city(c, to)?),
            F::IgnoreCliffs => facts.ignore_cliffs = Some(ignore_cliffs(c)?),
        }
    }
}
fn adjacent(c: &Context<'_, '_, '_>, from: PlotCoord, shallow: bool) -> Result<bool, Error> {
    for at in unit_placement::query_plots(c.state.world().map(), from)
        .map_err(Error::PlacementQuery)?
        .into_iter()
        .skip(1)
        .flatten()
    {
        let water = c.water(at)?;
        if !shallow {
            if !water {
                return Ok(true);
            }
        } else if water {
            let terrain = c
                .definitions
                .placement
                .terrain_definitions()
                .plot_index(c.require_plot(at)?.1.terrain_hash)
                .map_err(|_| Error::Missing("shore terrain index"))?;
            if usize::try_from(terrain)
                .ok()
                .and_then(|i| c.definitions.shallow_terrains.get(i))
                .copied()
                .unwrap_or(false)
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
fn city(c: &Context<'_, '_, '_>, at: PlotCoord) -> Result<bool, Error> {
    match c
        .definitions
        .movement
        .district_at(c.state, c.require_plot(at)?.1)
        .map_err(Error::Cost)?
    {
        Some(district) => c
            .definitions
            .movement
            .district_is_city(district)
            .map_err(Error::Cost),
        None => Ok(false),
    }
}
fn ignore_cliffs(c: &Context<'_, '_, '_>) -> Result<bool, Error> {
    let world = c.state.world();
    let player = world.player(c.record().owner).map_err(Error::World)?;
    let mut all = true;
    for &id in &c.record().unit_ids {
        let Some(unit) = player.unit(id).map_err(Error::World)? else {
            continue;
        };
        let ignored = unit.record.k_movement_modifiers.b_ignore_cliffs;
        if ignored && unit.record.k_movement_modifiers.b_escort_mobility {
            return Ok(true);
        }
        all &= ignored;
    }
    Ok(all)
}
