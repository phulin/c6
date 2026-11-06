//! Current requirement dispatch through the SAME stock runtime owner.
use super::{normal, sight, ComponentId, Error, LoadedState, PlotCoord, Runtime};
use crate::{
    plot_adjacent_friendly_unit_tag_requirement as tag, plot_adjacent_owner_requirement as plot,
    unit_moved_requirements as moved, unit_near_religious_city_requirement as near,
};
struct Plot<'a> {
    state: &'a mut LoadedState,
    runtime: &'a mut plot::Runtime,
    changes: &'a mut Vec<(u64, i32, i32)>,
}
impl plot::Context for Plot<'_> {
    type Error = Error;
    fn state(&self) -> &LoadedState {
        self.state
    }
    fn runtime(&self) -> &plot::Runtime {
        self.runtime
    }
    fn runtime_mut(&mut self) -> &mut plot::Runtime {
        self.runtime
    }
    fn changed(&mut self, receiver: u64, old: i32, new: i32) -> Result<(), Error> {
        self.changes.push((receiver, old, new));
        Err(Error::RequirementChanged { receiver, old, new })
    }
}
struct Moved<'a> {
    state: &'a mut LoadedState,
    runtime: &'a mut moved::Runtime,
    definitions: &'a moved::Definitions,
    changes: &'a mut Vec<(u64, i32, i32)>,
}
impl moved::Context for Moved<'_> {
    type Error = Error;
    fn state(&self) -> &LoadedState {
        self.state
    }
    fn runtime(&self) -> &moved::Runtime {
        self.runtime
    }
    fn runtime_mut(&mut self) -> &mut moved::Runtime {
        self.runtime
    }
    fn definitions(&self) -> Result<&moved::Definitions, moved::QueryError> {
        Ok(self.definitions)
    }
    fn changed(&mut self, receiver: u64, old: i32, new: i32) -> Result<(), Error> {
        self.changes.push((receiver, old, new));
        Err(Error::RequirementChanged { receiver, old, new })
    }
}
struct Near<'a> {
    state: &'a mut LoadedState,
    runtime: &'a mut near::Runtime,
    definitions: &'a near::Definitions,
    changes: &'a mut Vec<(u64, i32, i32)>,
}
impl near::Context for Near<'_> {
    type Error = Error;
    fn state(&self) -> &LoadedState {
        self.state
    }
    fn runtime(&self) -> &near::Runtime {
        self.runtime
    }
    fn runtime_mut(&mut self) -> &mut near::Runtime {
        self.runtime
    }
    fn definitions(&self) -> &near::Definitions {
        self.definitions
    }
    fn changed(&mut self, receiver: u64, old: i32, new: i32) -> Result<(), Error> {
        self.changes.push((receiver, old, new));
        Err(Error::RequirementChanged { receiver, old, new })
    }
}
impl<P> normal::Effects for Runtime<'_, P> {
    fn embarked_state_change(
        &mut self,
        state: &mut LoadedState,
        _: &mut sight::Runtime,
        actor: (i32, u32),
        _: bool,
        _: bool,
    ) -> Result<(), normal::Error<Error>> {
        // Rules.Placement2bef6d0 ignores both flags. Read current remaining
        // before GetMaxMoves, after completion and SetEmbarked subscribers.
        let unit = state
            .world()
            .player(actor.0)
            .and_then(|p| p.unit(actor.1))
            .map_err(normal::Error::World)?
            .ok_or(normal::Error::Missing("shore movement Unit"))?;
        let unit = unit.record;
        let remaining = unit.x_moves_remaining;
        // This immutable definition binding also serves research-triggered
        // formation updates; no captured maximum or former-domain value enters.
        let definitions = self
            .research_unit_updates
            .ok_or(normal::Error::Missing("current Unit movement definitions"))?;
        let inputs = crate::unit_movement_state::Inputs { definitions };
        let max = crate::unit_movement_state::unit_max_moves(state, unit, &inputs)
            .map_err(|e| normal::Error::Effect(Error::ShoreMovement(format!("{e:?}"))))?;
        let excess = remaining.wrapping_sub(max.wrapping_shl(8));
        if excess > 0 {
            crate::loaded_upkeep::change_unit_moves(
                state,
                actor.0,
                actor.1,
                &inputs,
                self,
                excess.wrapping_neg(),
            )
            .map_err(|e| normal::Error::Effect(Error::ShoreMovement(format!("{e:?}"))))?;
        }
        Ok(())
    }

    fn activate_removable_improvement(
        &mut self,
        state: &mut LoadedState,
        runtime: &mut sight::Runtime,
        actor: (i32, u32),
        plot: u32,
    ) -> Result<bool, normal::Error<Error>> {
        self.activate_stock_camp(state, runtime, actor, plot)
            .map_err(normal::Error::Effect)
    }

    fn damage_changed(
        &mut self,
        state: &mut LoadedState,
        report: c6_core::unit_healing::DamageChangedReport,
    ) -> Result<(), normal::Error<Error>> {
        self.current_actor(
            state,
            (i32::from(report.identity.owner), report.identity.id),
        )
        .map_err(normal::Error::Effect)?;
        if self
            .movement()
            .map_err(normal::Error::Effect)?
            .reporting_suspended
        {
            return Ok(());
        }
        self.reporting
            .damage_changed(state, report)
            .map_err(|e| normal::Error::Effect(Error::Rules(e)))
    }
    fn city_unit_changed(
        &mut self,
        state: &mut LoadedState,
        city: ComponentId,
    ) -> Result<(), normal::Error<Error>> {
        self.current_city_unit_changed(state, city)
    }

    fn preflight_normal(
        &self,
        state: &LoadedState,
        runtime: &sight::Runtime,
        actor: (i32, u32),
        destination: PlotCoord,
    ) -> Result<(), normal::Error<Error>> {
        self.current_actor(state, actor)
            .map_err(normal::Error::Effect)?;
        self.visibility_owner(state, runtime, actor.0, None)
            .map_err(normal::Error::Effect)?;
        let map = state.world().map();
        if map.header.width <= 0 || map.header.height <= 0 {
            return Err(normal::Error::Missing("map dimensions"));
        }
        let x = c6_core::movement_world::coord_range(
            destination.x,
            map.header.width,
            map.header.wrap_x,
        );
        let y = c6_core::movement_world::coord_range(
            destination.y,
            map.header.height,
            map.header.wrap_y,
        );
        map.plot_at(x, y)
            .ok_or(normal::Error::Missing("movement destination"))?;
        Ok(())
    }
    fn unit_moved_registrations(
        &self,
    ) -> Result<&c6_core::unit_moved_signal::Registry, normal::Error<Error>> {
        Ok(&self.movement().map_err(normal::Error::Effect)?.signal)
    }
    fn unit_moved_registrations_mut(
        &mut self,
    ) -> Result<&mut c6_core::unit_moved_signal::Registry, normal::Error<Error>> {
        Ok(&mut self
            .movement
            .as_mut()
            .ok_or(normal::Error::Missing("movement services"))?
            .signal)
    }
    fn on_unit_moved_script(
        &mut self,
        state: &mut LoadedState,
        _: &mut sight::Runtime,
        current: ComponentId,
        old: PlotCoord,
        new: PlotCoord,
    ) -> Result<(), normal::Error<Error>> {
        if current.component_type != 1 {
            return Err(normal::Error::Missing("UnitMoved unit component"));
        }
        self.script
            .on_unit_moved(state, (i32::from(current.owner), current.id), old, new)
            .map_err(|e| normal::Error::Effect(Error::Script(e)))
    }
    fn on_unit_moved_achievement(
        &mut self,
        state: &mut LoadedState,
        _: &mut sight::Runtime,
        old: PlotCoord,
        new: PlotCoord,
    ) -> Result<(), normal::Error<Error>> {
        crate::unit_moved_achievement::omit_stock_same_owner(state, old, new)
            .map_err(|e| normal::Error::Effect(Error::Requirement(format!("{e:?}"))))
    }
    fn on_unit_moved_requirement(
        &mut self,
        state: &mut LoadedState,
        _: &mut sight::Runtime,
        receiver: u64,
        current: ComponentId,
        old: PlotCoord,
        new: PlotCoord,
    ) -> Result<(), normal::Error<Error>> {
        let owner = self
            .movement
            .as_mut()
            .ok_or(normal::Error::Missing("movement services"))?;
        owner.requirement_attempts.push(receiver);
        if let Some(runtime) = owner
            .plot_requirements
            .as_mut()
            .filter(|r| r.receiver(receiver).is_ok())
        {
            return plot::invalidate(
                &mut Plot {
                    state,
                    runtime,
                    changes: &mut owner.requirement_changes,
                },
                receiver,
            )
            .map_err(|e| normal::Error::Effect(Error::Requirement(format!("{e:?}"))));
        }
        if let Some(runtime) = owner
            .moved_requirements
            .as_mut()
            .filter(|r| r.receiver(receiver).is_ok())
        {
            let definitions = owner
                .definitions
                .moved_requirements
                .ok_or(normal::Error::Missing("moved requirement definitions"))?;
            return moved::on_unit_moved(
                &mut Moved {
                    state,
                    runtime,
                    definitions,
                    changes: &mut owner.requirement_changes,
                },
                receiver,
                current,
                old,
                new,
            )
            .map_err(|e| normal::Error::Effect(Error::Requirement(format!("{e:?}"))));
        }
        if let Some(runtime) = owner
            .near_requirements
            .as_mut()
            .filter(|r| r.receiver(receiver).is_ok())
        {
            let definitions = owner
                .definitions
                .near_requirements
                .ok_or(normal::Error::Missing("near requirement definitions"))?;
            return near::on_unit_moved(
                &mut Near {
                    state,
                    runtime,
                    definitions,
                    changes: &mut owner.requirement_changes,
                },
                receiver,
                c6_core::map::UnitReference::from_component_id(current),
            )
            .map_err(|e| normal::Error::Effect(Error::Requirement(format!("{e:?}"))));
        }
        if let Some(runtime) = owner
            .tag_requirements
            .as_ref()
            .filter(|r| r.receiver(receiver).is_ok())
        {
            return tag::filter_unit_moved(state, runtime, receiver, current)
                .map_err(|e| normal::Error::Effect(Error::Requirement(format!("{e:?}"))));
        }
        Err(normal::Error::Missing(
            "UnitMoved requirement receiver kind",
        ))
    }
    fn clear_operations(
        &mut self,
        state: &mut LoadedState,
        actor: (i32, u32),
    ) -> Result<(), normal::Error<Error>> {
        crate::loaded_upkeep::clear_unit_operations(state, actor.0, actor.1, self)
            .map_err(|e| normal::Error::Effect(Error::Requirement(format!("{e:?}"))))
    }
}
