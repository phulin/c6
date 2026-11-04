//! Enemy unit reachability and synchronous short-vector threat contributions.
use super::*;
use c6_core::influence_map_update::{threats, Context as _};

impl Current<'_, '_, '_> {
    fn add_threat(&mut self, plot: usize, strength: i32, religious: bool) -> Result<(), Error> {
        threats::add(self.influence()?, plot, strength, religious)
            .ok_or(Error::Missing("influence contribution index"))
    }
    fn topology(&self, at: PlotCoord, dx: i32, dy: i32, range: i32) -> Option<(usize, PlotCoord)> {
        threats::from_delta(self.state.world().map(), at, dx, dy, range)
    }
    fn check_range(&self, range: i32) -> Result<(), Error> {
        // A caller resource bound, never a substituted range or truncated scan.
        if range > 1024 || range == i32::MIN {
            return Err(Error::Missing("influence radius resource limit"));
        }
        Ok(())
    }
    pub(super) fn apply_unit_jobs(&mut self, units: &[UnitReference]) -> Result<(), Error> {
        for &id in units {
            let unit = self.unit(id)?;
            let initial_range = unit.k_attack_range_modifiers.i_attack_range;
            let path = self
                .queries
                .path
                .ok_or(Error::Missing("influence path definitions"))?;
            let _max_moves = crate::unit_movement_state::unit_max_moves(
                self.state,
                unit,
                &crate::unit_movement_state::Inputs {
                    definitions: &path.units,
                },
            )
            .map_err(|e| Error::Path(crate::formation_path_state::Error::UnitMoves(e)))?;
            let bombard = self
                .queries
                .formation
                .bombard(self.state, unit)
                .map_err(Error::Combat)?;
            let melee = self
                .queries
                .formation
                .combat(self.state, unit)
                .map_err(Error::Combat)?;
            let mut strength = if bombard > 0 {
                let candidate = self
                    .queries
                    .formation
                    .bombard(self.state, unit)
                    .map_err(Error::Combat)?
                    .wrapping_sub(self.definitions.bombard_modifier);
                if melee > candidate {
                    self.queries
                        .formation
                        .combat(self.state, unit)
                        .map_err(Error::Combat)?
                } else {
                    self.queries
                        .formation
                        .bombard(self.state, unit)
                        .map_err(Error::Combat)?
                        .wrapping_sub(self.definitions.bombard_modifier)
                }
            } else {
                let ranged = self
                    .queries
                    .formation
                    .ranged(self.state, unit)
                    .map_err(Error::Combat)?;
                if melee > ranged {
                    self.queries
                        .formation
                        .combat(self.state, unit)
                        .map_err(Error::Combat)?
                } else {
                    self.queries
                        .formation
                        .ranged(self.state, unit)
                        .map_err(Error::Combat)?
                }
            };
            let religious = unit.k_religion.i_spread_charges > 0;
            if religious {
                strength = unit.k_religion.i_religious_strength;
            }
            let range = unit.k_attack_range_modifiers.i_attack_range;
            self.check_range(range)?;
            let location = unit.k_location;
            let runtime = self
                .state
                .unit_runtime(unit.e_owner, unit.id)
                .map_err(Error::State)?;
            if runtime.domain == 1 {
                if range >= 0 {
                    for dx in -range..=range {
                        for dy in -range..=range {
                            if let Some((plot, _)) = self.topology(location, dx, dy, range) {
                                self.add_threat(plot, strength, false)?;
                            }
                        }
                    }
                }
                continue;
            }
            let formation = runtime
                .formation
                .ok_or(Error::Missing("influence Unit.GetFormation"))?;
            let source_runtime = self
                .queries
                .path_runtime
                .ok_or(Error::Missing("influence current path runtime"))?;
            // This exclusive rebuild writes only InfluenceMap arrays. Preserve
            // the caller's current city-domain proof while refreshing the broad
            // action revision after our own derived-map writes, not after any
            // unaccounted city/map mutation.
            if source_runtime
                .city_domains
                .as_ref()
                .is_some_and(|domains| domains.stamp != self.input_stamp)
            {
                return Err(Error::Missing(
                    "city domains stale before influence rebuild",
                ));
            }
            let current_runtime = crate::formation_path_state::Runtime {
                visibility: source_runtime.visibility,
                civilization_levels: source_runtime.civilization_levels.as_ref().map(|levels| {
                    crate::movement_placement_state::CurrentCivilizationLevels {
                        binding: levels.binding,
                        values: levels.values,
                    }
                }),
                city_domains: source_runtime.city_domains.as_ref().map(|domains| {
                    crate::formation_path_state::CurrentCityDomains {
                        binding: domains.binding,
                        stamp: self.state.action_stamp(),
                        masks: domains.masks,
                    }
                }),
                execution: source_runtime.execution,
                is_evaluating_operation: source_runtime.is_evaluating_operation,
            };
            let context = crate::formation_path_state::Context::new(
                self.state,
                path,
                current_runtime,
                (formation.owner, formation.resolved_id),
            )
            .map_err(Error::Path)?;
            let positions = crate::formation_path_state::breadth_first::influence_positions(
                context,
                initial_range > 0,
            )
            .map_err(Error::Path)?;
            let mut extra = Vec::new();
            if range >= 0 {
                for &at in &positions {
                    for dx in -range..=range {
                        for dy in -range..=range {
                            if let Some((_, location)) = self.topology(at, dx, dy, range) {
                                if !positions.contains(&location) && !extra.contains(&location) {
                                    extra.push(location);
                                }
                            }
                        }
                    }
                }
            }
            // Raid adds unique LAND ring coordinates to extra, without testing
            // membership in positions. A reachable raid neighbor can contribute
            // twice: once through each source vector.
            if self.unit(id)?.k_combat_modifers.b_conduct_raid {
                let placement = crate::movement_placement_state::Context::from_definitions(
                    self.state,
                    &path.placement,
                )
                .map_err(|e| Error::Path(crate::formation_path_state::Error::Placement(e)))?;
                for &at in &positions {
                    for (index, location) in threats::ring(self.state.world().map(), at, 1) {
                        if !placement
                            .water(&self.state.world().map().plots[index])
                            .map_err(|e| {
                                Error::Path(crate::formation_path_state::Error::Placement(e))
                            })?
                            && !extra.contains(&location)
                        {
                            extra.push(location);
                        }
                    }
                }
            }
            for at in positions {
                let (index, _) = self
                    .topology(at, 0, 0, 0)
                    .ok_or(Error::Missing("reachable plot index"))?;
                self.add_threat(index, strength, religious)?;
            }
            for at in extra {
                let (index, _) = self
                    .topology(at, 0, 0, 0)
                    .ok_or(Error::Missing("extra threat plot index"))?;
                self.add_threat(index, strength, false)?;
            }
        }
        Ok(())
    }
    fn attack(&self, district: &District) -> Result<(i32, i32), Error> {
        let (_, city) = self.owner_city(district)?;
        let strength = city_range_attack::attack_strength_q8(
            self.state,
            city,
            district,
            self.queries.city_range,
        )
        .map_err(Error::Combat)?;
        let range = city_range_attack::attack_range(
            city,
            district,
            strength,
            self.queries.city_range.definitions,
        );
        Ok((strength, range))
    }
    pub(super) fn apply_enemy_district(&mut self, id: DistrictId) -> Result<(), Error> {
        let plot = self.plot(self.district(id)?)?;
        if self.defense(self.district(id)?)? > 0 {
            let strength = self.defense(self.district(id)?)? >> 8;
            self.add_threat(plot, strength, false)?;
        }
        let at = {
            let d = self.district(id)?;
            PlotCoord {
                x: d.location.0,
                y: d.location.1,
            }
        };
        if self.district_can_attack(id)? {
            let (strength, range) = self.attack(self.district(id)?)?;
            self.check_range(range)?;
            if range >= 0 {
                for dx in -range..=range {
                    for dy in -range..=range {
                        if let Some((index, location)) = self.topology(at, dx, dy, range) {
                            if location != at {
                                self.add_threat(index, strength >> 8, false)?;
                            }
                        }
                    }
                }
            }
        } else {
            for (index, _) in threats::ring(self.state.world().map(), at, 1) {
                self.add_threat(index, 1, false)?;
            }
        }
        Ok(())
    }
    pub(super) fn suppress_friendly_district(&mut self, id: DistrictId) -> Result<(), Error> {
        let (strength, range) = self.attack(self.district(id)?)?;
        self.check_range(range)?;
        let at = {
            let d = self.district(id)?;
            PlotCoord {
                x: d.location.0,
                y: d.location.1,
            }
        };
        if range >= 0 {
            // IterateNeighbors includes its center, which the callback rejects.
            for radius in 0..=range as u32 {
                for (index, location) in threats::ring(self.state.world().map(), at, radius) {
                    if location != at {
                        threats::suppress(self.influence()?, index, strength >> 8)
                            .ok_or(Error::Missing("friendly ranged index"))?;
                    }
                }
            }
        }
        Ok(())
    }
}
