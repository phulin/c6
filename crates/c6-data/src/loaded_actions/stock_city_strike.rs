//! District/Unit execution borrowing the persistent canonical combat owners.
use super::Runtime;
use crate::{
    city_strike::{self as strike, error, reports::Effects as _},
    loaded_actions::city_range_attack as city,
    loaded_upkeep as upkeep,
    melee_attack::{self as melee, backend, experience},
    unit_sight_state as sight,
};
use c6_core::{
    loaded_state::LoadedState,
    map::UnitReference,
    unit::{ComponentId, PlotCoord},
};

impl<'a> Runtime<'a> {
    /// Execute an already admitted ordinary city strike against its freshly
    /// selected real defender. Every owner is restored even on partial failure.
    #[allow(clippy::too_many_arguments)]
    pub fn city_strike(
        &mut self,
        state: &mut LoadedState,
        sight: &mut sight::Runtime,
        definitions: &city::Definitions,
        inputs: &melee::transition::Inputs<'_, '_, '_>,
        attacker: ComponentId,
        defender: UnitReference,
        location: PlotCoord,
    ) -> Result<strike::Generated, strike::Error> {
        let mut combat = self
            .combat
            .take()
            .ok_or_else(|| error("stock owners", "combat services"))?;
        if combat.inputs.is_none() || combat.bindings.is_none() {
            self.combat = Some(combat);
            return Err(error(
                "stock owners",
                "city combat unavailable in cleanup context",
            ));
        }
        let Some(owners) = self.modifier_owners.take() else {
            self.combat = Some(combat);
            return Err(error("stock owners", "canonical modifier owners"));
        };
        let super::unit_creation::ModifierOwners {
            effects,
            requirements,
            collections,
            unit_created,
        } = owners;
        let backend_inputs = combat.inputs.take().expect("checked combat inputs");
        let backend = backend::Backend::resume_owned(
            state,
            combat.origin,
            (),
            requirements,
            effects,
            backend_inputs,
        );
        let mut bindings = combat.bindings.take().expect("checked combat bindings");
        self.combat = Some(combat);
        let (backend, result) = match backend {
            Err(backend) => (
                backend,
                Err(error("stock owners", "different combat World")),
            ),
            Ok(backend) => {
                let (_, mut backend) = backend.with_reporting(melee::transition::Reports(self));
                let result = (|| {
                    let members = strike::generate(
                        state,
                        &mut backend,
                        definitions,
                        inputs,
                        attacker,
                        defender,
                        location,
                    )?;
                    backend
                        .reporting
                        .0
                        .combat_mut()
                        .map_err(|e| error("combat outputs", e))?
                        .city_strikes
                        .push(members.clone());
                    upkeep::clear_unit_operations(
                        state,
                        defender.player(),
                        defender.id,
                        backend.reporting.0,
                    )
                    .map_err(|e| error("defender interrupt", e))?;
                    resolve_surviving(
                        state,
                        sight,
                        backend.reporting.0,
                        definitions,
                        inputs,
                        &mut bindings,
                        &members,
                    )?;
                    Ok(members)
                })();
                let (_, backend) = backend.with_reporting(());
                (backend, result)
            }
        };
        let ((), requirements, effects, backend_inputs) = backend.into_parts();
        let combat = self
            .combat
            .as_mut()
            .expect("synchronous combat outputs retained");
        combat.inputs = Some(backend_inputs);
        combat.bindings = Some(bindings);
        self.modifier_owners = Some(super::unit_creation::ModifierOwners {
            effects,
            requirements,
            collections,
            unit_created,
        });
        result
    }
}
#[allow(clippy::too_many_arguments)]
fn resolve_surviving(
    state: &mut LoadedState,
    sight: &mut sight::Runtime,
    effects: &mut Runtime<'_>,
    _definitions: &city::Definitions,
    inputs: &melee::transition::Inputs<'_, '_, '_>,
    bindings: &mut melee::resolve::completion::stock::Bindings,
    members: &strike::Generated,
) -> Result<(), strike::Error> {
    let attacker = members.attacker;
    let defender = members.defender;
    let current = melee::actor(state, defender).map_err(|e| error("resolve defender", e))?;
    let location = if state
        .world()
        .map()
        .plot_at(members.location.x, members.location.y)
        .is_some()
    {
        members.location
    } else {
        current.k_location
    };
    if state
        .world()
        .map()
        .plot_at(location.x, location.y)
        .is_none()
    {
        return Err(error("resolve plot", "missing current combat plot"));
    }
    let district = state
        .district_mut(i32::from(attacker.owner), attacker.id)
        .map_err(|e| error("shot count", e))?;
    // FAutoVariable assignment only dirties transport bookkeeping; no gameplay
    // callbacks or RNG occur between the wrapping addition and clamp-to-zero.
    district.remaining_attacks = district.remaining_attacks.wrapping_sub(1).max(0);
    crate::unit_placement_state::change_damage(
        state,
        inputs.placement,
        inputs.queries,
        sight,
        effects,
        (defender.player(), defender.id),
        members.damage,
    )
    .map_err(|e| error("defender damage", e))?;
    let current = melee::actor(state, defender).map_err(|e| error("defender damage", e))?;
    if current.i_damage
        >= inputs
            .generation
            .maximum_damage
            .maximum_damage(current)
            .map_err(|e| error("maximum", e))?
    {
        return Err(error(
            "defender death",
            "city strike death continuation required",
        ));
    }
    experience::change(
        state,
        inputs.experience,
        inputs.generation.maximum_damage,
        inputs.generation.levels,
        defender,
        members.defender_experience,
        effects,
    )
    .map_err(|e| error("defender experience", e))?;
    let enemy = city::district_owner(
        state,
        state
            .district(i32::from(attacker.owner), attacker.id)
            .map_err(|e| error("joint war district", e))?,
    )
    .map_err(|e| error("joint war owner", e))?;
    let partners = experience::joint_war_partners(
        state,
        inputs.generation.levels,
        inputs.cached_majors,
        enemy,
    )
    .map_err(|e| error("joint war", e))?;
    if !partners.owners.is_empty() {
        return Err(error("joint war", partners));
    }
    effects
        .city_surviving_statuses(state, members, inputs.death_report_status)
        .map_err(|e| error("surviving status", e))?;
    upkeep::clear_unit_operations(state, defender.player(), defender.id, effects)
        .map_err(|e| error("resolve defender clear", e))?;
    effects
        .district_combat_changed(state, attacker)
        .map_err(|e| error("district combat report", e))?;
    strike::completion::finish(state, bindings, inputs.callbacks, effects, members)
}
