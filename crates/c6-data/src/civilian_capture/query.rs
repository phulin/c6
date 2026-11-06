//! Combat.IsEnemyAt28ce9d0 for the bounded noncombat destination. Native
//! GetBestDefender flags0x101 rejects noncombat before visibility/domain/strength
//! ranking. Reached combat and District continuations stay explicit errors.
use super::{debug, unit};
use crate::{combat_defender, formation_rules};
use c6_core::{
    combat_defender as kernel, loaded_state::LoadedState, map::UnitReference, unit::PlotCoord,
};
struct Current<'a, 'd> {
    base: combat_defender::Current<'a, 'd>,
    state: &'a LoadedState,
}
impl kernel::Context for Current<'_, '_> {
    type Error = String;
    type Unit = UnitReference;
    fn candidate_count(&mut self) -> Result<usize, String> {
        self.base.candidate_count().map_err(debug)
    }
    fn resolve(&mut self, index: usize) -> Result<Option<UnitReference>, String> {
        self.base.resolve(index).map_err(debug)
    }
    fn delayed_death(&mut self, unit: UnitReference) -> Result<bool, String> {
        self.base.delayed_death(unit).map_err(debug)
    }
    fn owner(&mut self, unit: UnitReference) -> Result<i32, String> {
        self.base.owner(unit).map_err(debug)
    }
    fn combat_false(&mut self, unit: UnitReference) -> Result<i32, String> {
        self.base.combat_false(unit).map_err(debug)
    }
    fn ranged_combat(&mut self, unit: UnitReference) -> Result<i32, String> {
        self.base.ranged_combat(unit).map_err(debug)
    }
    fn bombard_combat(&mut self, unit: UnitReference) -> Result<i32, String> {
        self.base.bombard_combat(unit).map_err(debug)
    }
}
impl kernel::EnemyContext for Current<'_, '_> {
    fn is_enemy(&mut self, owner: i32, viewer: i32) -> Result<bool, String> {
        if owner == -1 {
            return Ok(false);
        }
        self.state
            .game_relations()
            .map_err(debug)?
            .is_at_war(owner, viewer)
            .map_err(debug)
    }
    fn visible(&mut self, _: UnitReference) -> Result<bool, String> {
        Err("capture IsEnemyAt combat defender visibility/air activity continuation".into())
    }
}
pub(super) fn enemy_at(
    state: &LoadedState,
    definitions: &formation_rules::Definitions,
    actor: (i32, u32),
    destination: PlotCoord,
) -> Result<bool, String> {
    let unit = unit(state, actor)?;
    if unit.k_combat_modifers.b_bypass_combat_units {
        return Err("capture IsEnemyAt combat bypass selection".into());
    }
    let layer = c6_core::skip_turn::placement::unit_layer(state.world(), unit).map_err(debug)?;
    let mut current = Current {
        state,
        base: combat_defender::Current::new(state, definitions, destination, layer),
    };
    let selected = kernel::select_enemy(&mut current, unit.e_owner)?;
    if selected.is_some() {
        return Err("capture IsEnemyAt selected combat defender".into());
    }
    let (_, plot) =
        c6_core::movement_world::plot_at(state.world().map(), destination).map_err(debug)?;
    if plot
        .city_attributes
        .as_ref()
        .is_some_and(|a| a.district != u32::MAX && a.player != -1)
    {
        return Err("capture IsEnemyAt enemy District continuation".into());
    }
    Ok(false)
}
