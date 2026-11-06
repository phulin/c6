//! Linux XP2 ordinary noncombat capture, joined to the existing Unit factory,
//! delayed death, placement and movement owners. The first supported profile is
//! an unembarked, undamaged Settler; other reached branches remain explicit.
pub mod stock;

use crate::{
    formation_rules, loaded_upkeep, registry::linux_type_hash, unit_creation as creation,
    unit_initialization_definitions::collection, unit_placement_state as placement,
    unit_sight_state as sight, Rules,
};
use c6_core::{
    core_player::initialization::WireBinding,
    loaded_state::LoadedState,
    movement_shore::PlacementRequest,
    unit::{ComponentId, PlotCoord, Unit},
};

/// Called by Formation.MoveTo in the actual post-Placement scan. Admission is
/// read-only; effects execute on the same state and callback owners as movement.
pub trait Capture<E> {
    fn preflight(
        &self,
        state: &LoadedState,
        formation: (i32, u32),
        victim: (i32, u32),
    ) -> Result<(), String>;
    fn capture(
        &mut self,
        state: &mut LoadedState,
        sight: &mut sight::Runtime,
        effects: &mut E,
        formation: (i32, u32),
        victim: (i32, u32),
    ) -> Result<(), String>;
}
pub struct Unavailable;
impl<E> Capture<E> for Unavailable {
    fn preflight(&self, _: &LoadedState, _: (i32, u32), _: (i32, u32)) -> Result<(), String> {
        Err("MoveTo noncombat capture candidate: current capture services unavailable".into())
    }
    fn capture(
        &mut self,
        _: &mut LoadedState,
        _: &mut sight::Runtime,
        _: &mut E,
        _: (i32, u32),
        _: (i32, u32),
    ) -> Result<(), String> {
        Err("MoveTo noncombat capture services unavailable".into())
    }
}

pub struct Definitions {
    binding: WireBinding,
    units: crate::registry::LinuxDefinitionRegistry,
    properties: Vec<(bool, bool)>,
    conversions: Vec<(u32, u32)>,
    mayhem: i32,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, String> {
        let units = collection(rules, "Units", "UnitType").map_err(debug)?;
        let table = rules.gameplay().table("Units").ok_or("capture Units")?;
        let mut properties = Vec::new();
        for definition in units.entries() {
            let row = table
                .row_by_id(definition.database_rowid)
                .ok_or("capture Unit row")?;
            properties.push((
                row.integer("Spy").map_err(debug)? != 0,
                row.integer("CanRetreatWhenCaptured").map_err(debug)? != 0,
            ));
        }
        let mut conversions = Vec::new();
        for row in rules
            .gameplay()
            .table("UnitCaptures")
            .ok_or("UnitCaptures")?
            .rows()
        {
            let from = linux_type_hash(row.text("CapturedUnitType").map_err(debug)?.as_bytes());
            let to = linux_type_hash(row.text("BecomesUnitType").map_err(debug)?.as_bytes());
            if units.find_index(from).is_none() || units.find_index(to).is_none() {
                return Err("capture Unit reference".into());
            }
            // Native GetCaptureCollection().begin(), not a type-based replacement.
            if !conversions.iter().any(|&(hash, _)| hash == from) {
                conversions.push((from, to));
            }
        }
        let mut rows = rules
            .gameplay()
            .table("GlobalParameters")
            .ok_or("GlobalParameters")?
            .rows()
            .filter(|r| r.text("Name").ok() == Some("MAYHEM_CAPTURE_CIVILIAN"));
        let row = rows.next().ok_or("MAYHEM_CAPTURE_CIVILIAN")?;
        let mayhem = match row.get("Value").map_err(debug)? {
            crate::SqlValue::Integer(n) => i32::try_from(*n).map_err(debug)?,
            crate::SqlValue::Text(b) => std::str::from_utf8(b)
                .map_err(debug)?
                .parse::<i32>()
                .map_err(debug)?,
            _ => return Err("capture mayhem integer".into()),
        };
        if rows.next().is_some() {
            return Err("duplicate capture mayhem".into());
        }
        Ok(Self {
            binding: state.binding().clone(),
            units,
            properties,
            conversions,
            mayhem,
        })
    }
    fn candidate(
        &self,
        state: &LoadedState,
        formation: (i32, u32),
        victim: (i32, u32),
        combat: &formation_rules::Definitions,
    ) -> Result<Option<(ComponentId, u32)>, String> {
        if state.binding() != &self.binding {
            return Err("capture definition binding".into());
        }
        let unit = unit(state, victim)?;
        let index = self
            .units
            .find_index(unit.e_type_index)
            .ok_or("capture definition")?;
        if self.properties[index].0 {
            return Ok(None);
        }
        if !state
            .game_relations()
            .map_err(debug)?
            .is_at_war(formation.0, unit.e_owner)
            .map_err(debug)?
        {
            return Ok(None);
        }
        let f = state
            .world()
            .player(formation.0)
            .map_err(debug)?
            .formation(formation.1)
            .map_err(debug)?
            .ok_or("capture Formation")?
            .record;
        let mut selected = None;
        for &id in &f.unit_ids {
            let Some(found) = state
                .world()
                .player(f.owner)
                .map_err(debug)?
                .unit(id)
                .map_err(debug)?
            else {
                continue;
            };
            if combat.combat(state, found.record).map_err(debug)? > 0
                || combat.ranged(state, found.record).map_err(debug)? > 0
                || combat.bombard(state, found.record).map_err(debug)? > 0
            {
                selected = Some(identity(found.record));
                break;
            }
        }
        let Some(captor) = selected else {
            return Ok(None);
        };
        if f.unit_ids.len() != 1 {
            return Err("capture multi-member CanAddUnit".into());
        }
        if self.properties[index].1 {
            return Err("capture retreat continuation".into());
        }
        if unit.e_type_index != linux_type_hash(b"UNIT_SETTLER")
            || unit.i_build_charges != 0
            || unit.i_damage != 0
            || unit.b_embarked
        {
            return Err("capture supported ordinary Settler profile".into());
        }
        let Some((_, to)) = self
            .conversions
            .iter()
            .find(|&&(from, _)| from == unit.e_type_index)
        else {
            return Err("capture absent conversion: destruction continuation".into());
        };
        Ok(Some((captor, *to)))
    }
}
fn debug(e: impl std::fmt::Debug) -> String {
    format!("{e:?}")
}
fn identity(unit: &Unit) -> ComponentId {
    ComponentId {
        owner: unit.e_owner as i16,
        component_type: 1,
        id: unit.id,
    }
}
fn unit(state: &LoadedState, id: (i32, u32)) -> Result<&Unit, String> {
    state
        .world()
        .player(id.0)
        .map_err(debug)?
        .unit(id.1)
        .map_err(debug)?
        .map(|u| u.record)
        .ok_or_else(|| "capture Unit missing".into())
}

/// Exact synchronous leaves after canonical mechanics. Implementations retain
/// UnitCaptured, AI analysis, reporting and notification consumers. No default
/// success exists for any of these reached branches.
pub trait Events<E> {
    fn preflight(
        &self,
        state: &LoadedState,
        victim: ComponentId,
        captor: ComponentId,
    ) -> Result<(), String>;
    fn captured(
        &mut self,
        state: &mut LoadedState,
        effects: &mut E,
        new: ComponentId,
    ) -> Result<(), String>;
    fn analysis(
        &mut self,
        state: &mut LoadedState,
        effects: &mut E,
        owner: i32,
        units: &[u32],
    ) -> Result<(), String>;
    fn notification(
        &mut self,
        state: &mut LoadedState,
        effects: &mut E,
        victim_owner: i32,
        captor: ComponentId,
        new: ComponentId,
        at: PlotCoord,
    ) -> Result<(), String>;
    fn processed(
        &mut self,
        state: &mut LoadedState,
        effects: &mut E,
        new: ComponentId,
    ) -> Result<(), String>;
}
/// Definition and callback owners only; every invocation borrows the live
/// Session World, sight and movement provider without cloning gameplay state.
pub struct Services<'a, 'd, 'r, C, V> {
    pub definitions: &'a Definitions,
    pub creation: &'a creation::Definitions,
    pub resources: &'a crate::resource_binding::ResourceAuthority<'r>,
    pub movement: &'a crate::unit_movement_state::Inputs<'d>,
    pub combat: &'a formation_rules::Definitions,
    pub runtime: &'a creation::Runtime,
    pub tags: &'a crate::loaded_actions::reporting::hold_cycling::commands::Tags,
    pub placement: &'a placement::Definitions<'d, 'r>,
    pub queries: &'a placement::Queries<'d>,
    pub barbarian_owner: i32,
    pub death: C,
    pub events: V,
}
impl<E, C, V> Capture<E> for Services<'_, '_, '_, C, V>
where
    E: creation::FactoryEffects
        + loaded_upkeep::FormationRestoreEffects<Error = <E as creation::Effects>::Error>
        + sight::Effects<Error = <E as creation::Effects>::Error>,
    <E as creation::Effects>::Error: std::fmt::Debug,
    C: crate::unit_kill::Continuations<E>,
    C::Error: std::fmt::Debug,
    V: Events<E>,
{
    fn preflight(
        &self,
        state: &LoadedState,
        formation: (i32, u32),
        victim: (i32, u32),
    ) -> Result<(), String> {
        if let Some((captor, _)) =
            self.definitions
                .candidate(state, formation, victim, self.combat)?
        {
            if i32::from(captor.owner) == self.barbarian_owner {
                return Err("barbarian captor tribe continuation".into());
            }
            self.events
                .preflight(state, identity(unit(state, victim)?), captor)?;
        }
        Ok(())
    }
    fn capture(
        &mut self,
        state: &mut LoadedState,
        visibility: &mut sight::Runtime,
        effects: &mut E,
        formation: (i32, u32),
        victim: (i32, u32),
    ) -> Result<(), String> {
        let Some((captor, to)) =
            self.definitions
                .candidate(state, formation, victim, self.combat)?
        else {
            return Ok(());
        };
        if i32::from(captor.owner) == self.barbarian_owner {
            return Err("barbarian captor tribe continuation".into());
        }
        self.events
            .preflight(state, identity(unit(state, victim)?), captor)?;
        let old_identity = identity(unit(state, victim)?);
        let at = unit(state, victim)?.k_location;
        // LogCapture is gameplay state even though it belongs to GlobalAi.
        state
            .add_global_mayhem(self.definitions.mayhem as f32)
            .map_err(debug)?;
        crate::unit_kill::kill_with_continuations(
            state,
            visibility,
            effects,
            &mut self.death,
            self.placement,
            self.queries,
            victim,
            true,
            -1,
        )
        .map_err(debug)?;
        let created = creation::PlayerContext {
            state,
            definitions: self.creation,
            resources: self.resources,
            movement: self.movement,
            combat: self.combat,
            runtime: self.runtime,
            tags: self.tags,
            effects,
        }
        .create_without_city(i32::from(captor.owner), to)
        .map_err(debug)?;
        let Some(new) = created else { return Ok(()) };
        let actor = (i32::from(new.owner), new.id);
        state
            .unit_mut(actor.0, actor.1)
            .map_err(debug)?
            .e_original_owner = i32::from(old_identity.owner);
        let query = self
            .queries
            .placement_context(state, self.placement.placement)
            .map_err(debug)?;
        if query
            .unit_placement(
                actor.0,
                actor.1,
                at,
                PlacementRequest::from_validate_options(0),
            )
            .map_err(debug)?
            != 1
        {
            crate::unit_kill::kill_with_continuations(
                state,
                visibility,
                effects,
                &mut self.death,
                self.placement,
                self.queries,
                actor,
                true,
                -1,
            )
            .map_err(debug)?;
            return Ok(());
        }
        // Set flags1 accepts the separately validated location; normal placement
        // and all fresh-unit callback tails remain the existing shared kernel.
        let placed = placement::creation::set(
            state,
            self.placement,
            self.queries,
            visibility,
            effects,
            placement::Request {
                actor,
                destination: at,
                validation_flags: 1,
            },
            self.tags,
        )
        .map_err(debug)?;
        if placed != 1 {
            return Err("capture validated placement failed".into());
        }
        // SetDamage(0) still reaches Heal-operation clearing on a callback-edited
        // new queue, so use the full shared setter even when damage is unchanged.
        let delta = unit(state, actor)?.i_damage.wrapping_neg();
        placement::change_damage(
            state,
            self.placement,
            self.queries,
            visibility,
            effects,
            actor,
            delta,
        )
        .map_err(debug)?;
        if unit(state, actor)?.b_embarked {
            state.unit_mut(actor.0, actor.1).map_err(debug)?.b_embarked = false;
            let report = crate::loaded_actions::IdentityReport {
                event: 0x84c1fdd6,
                subtype: 0,
                identity: identity(unit(state, actor)?),
                first: Some(0),
                second: None,
            };
            let handle = loaded_upkeep::UnitHandle::resolve(state, actor.0, actor.1)
                .map_err(debug)?
                .ok_or("capture embarked report Unit")?;
            loaded_upkeep::OperationReports::preflight_operation_report(
                effects, state, handle, report,
            )
            .map_err(debug)?;
            loaded_upkeep::OperationReports::operation_report(effects, state, report)
                .map_err(debug)?;
        }
        let mut religion = unit(state, victim)?.k_religion.clone();
        // Wire framing is not a gameplay member copied by Religion::operator=.
        religion.wire_version = unit(state, actor)?.k_religion.wire_version;
        state.unit_mut(actor.0, actor.1).map_err(debug)?.k_religion = religion;
        loaded_upkeep::finish_unit_moves(state, actor.0, actor.1, self.movement, effects)
            .map_err(debug)?;
        self.events.processed(state, effects, new)?;
        self.events.captured(state, effects, new)?;
        // Native includes the capturing Unit when it has no AI controller, or
        // is barbarian. Player policy never runs here.
        let captor_unit = unit(state, (i32::from(captor.owner), captor.id))?;
        let mut analyzed = vec![new.id];
        if captor_unit.ai_controller == u32::MAX {
            analyzed.push(captor.id)
        }
        self.events
            .analysis(state, effects, i32::from(captor.owner), &analyzed)?;
        let report = crate::loaded_actions::IdentityReport {
            event: 0x664a37b6,
            subtype: 0,
            identity: old_identity,
            first: Some(i32::from(old_identity.owner)),
            second: Some(i32::from(captor.owner)),
        };
        loaded_upkeep::OperationReports::preflight_operation_report(
            effects,
            state,
            loaded_upkeep::UnitHandle::resolve(state, victim.0, victim.1)
                .map_err(debug)?
                .ok_or("capture old report identity")?,
            report,
        )
        .map_err(debug)?;
        loaded_upkeep::OperationReports::operation_report(effects, state, report).map_err(debug)?;
        self.events.notification(
            state,
            effects,
            i32::from(old_identity.owner),
            captor,
            new,
            at,
        )?;
        Ok(())
    }
}

/// The normal persistent operation handler can use this composition directly;
/// pathfinding, legal queries, transport and queue lifecycle remain unchanged.
pub struct Movement<'a, 'r, C> {
    pub movement: crate::loaded_actions::move_to::ComputedMovement<'a, 'r>,
    pub capture: C,
}
impl<E, C> crate::loaded_actions::move_to::Movement<E> for Movement<'_, '_, C>
where
    E: placement::Effects
        + loaded_upkeep::FormationRestoreEffects<Error = placement::EffectsError<E>>,
    C: Capture<E>,
{
    type Error = crate::loaded_actions::move_to::ComputedError<placement::EffectsError<E>>;
    fn fortify_turn_max(&self) -> i32 {
        self.movement.path.fortify_turn_max
    }
    fn is_fortifyable(&self, state: &LoadedState, actor: (i32, u32)) -> Result<bool, Self::Error> {
        crate::loaded_actions::move_to::Movement::<E>::is_fortifyable(&self.movement, state, actor)
    }
    fn alert_enemy_visible(
        &self,
        state: &LoadedState,
        sight: &sight::Runtime,
        actor: (i32, u32),
    ) -> Result<bool, Self::Error> {
        crate::loaded_actions::move_to::Movement::<E>::alert_enemy_visible(
            &self.movement,
            state,
            sight,
            actor,
        )
    }
    fn city_at(
        &self,
        state: &LoadedState,
        at: PlotCoord,
    ) -> Result<Option<c6_core::trade::ComponentId>, Self::Error> {
        crate::loaded_actions::move_to::Movement::<E>::city_at(&self.movement, state, at)
    }
    fn preflight(
        &self,
        state: &LoadedState,
        actor: (i32, u32),
        request: crate::loaded_actions::move_to::Request,
    ) -> Result<(), Self::Error> {
        crate::loaded_actions::move_to::Movement::<E>::preflight(
            &self.movement,
            state,
            actor,
            request,
        )
    }
    fn test_move_to(
        &mut self,
        state: &mut LoadedState,
        sight: &sight::Runtime,
        formation: (i32, u32),
        at: PlotCoord,
        flags: u32,
    ) -> Result<i32, Self::Error> {
        crate::loaded_actions::move_to::Movement::<E>::test_move_to(
            &mut self.movement,
            state,
            sight,
            formation,
            at,
            flags,
        )
    }
    fn move_to(
        &mut self,
        state: &mut LoadedState,
        sight: &mut sight::Runtime,
        formation: (i32, u32),
        at: PlotCoord,
        flags: u32,
        effects: &mut E,
    ) -> Result<i32, Self::Error> {
        let movement = &mut self.movement;
        crate::formation_move_to::move_to_with_capture(
            state,
            movement.path,
            movement.placement,
            movement.queries,
            sight,
            movement.search,
            formation,
            at,
            flags,
            movement.execution,
            effects,
            &mut self.capture,
        )
        .map_err(crate::loaded_actions::move_to::ComputedError::Step)
    }
    fn valid_location(
        &mut self,
        state: &LoadedState,
        sight: &sight::Runtime,
        actor: (i32, u32),
        at: PlotCoord,
    ) -> Result<i32, Self::Error> {
        crate::loaded_actions::move_to::Movement::<E>::valid_location(
            &mut self.movement,
            state,
            sight,
            actor,
            at,
        )
    }
}
