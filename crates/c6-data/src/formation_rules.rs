//! Linux Formation's effective Unit getters over admitted rules and live fields.
//! Shared combat arithmetic remains in the existing CityRangeAttack provider;
//! this module supplies the additional upkeep queries, not a parallel combat
//! subsystem. Native RestoreMovement/Formation acceptance remains pending.
use crate::{
    loaded_actions::{city_range_attack, Error},
    registry::LinuxDefinitionRegistry,
    resource_binding::ResourceAuthority,
    unit_initialization_definitions::collection,
    Rules, SqlValue,
};
use c6_core::{
    core_player::initialization::WireBinding, loaded_state::LoadedState,
    movement_world::DefinitionLookup, unit::Unit,
};

/// Existing shared combat definitions plus the source-proven CanCapture bit and
/// configured maximum-attack input. Native collection index never means rowid.
pub struct Definitions {
    binding: WireBinding,
    combat: city_range_attack::Definitions,
    units: LinuxDefinitionRegistry,
    capture: Vec<bool>,
    max_attacks: i32,
}
impl Definitions {
    /// Source-closed CanEmbark prefix when current owner configuration is absent.
    /// This reuses placement's lazy decision, so proven domain0 returns false;
    /// reaching a Player capability query returns the actual missing dependency.
    /// Use the full placement Context for other configured gameplay branches.
    pub fn can_embark_without_configuration(
        &self,
        state: &LoadedState,
        unit: &Unit,
    ) -> Result<bool, crate::movement_placement_state::Error> {
        use crate::movement_placement_state::{self, Capability, Error};
        if state.binding() != &self.binding {
            return Err(Error::Binding);
        }
        let runtime = state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?;
        movement_placement_state::unit_embark_decision(
            runtime.domain,
            unit.e_owner,
            unit.e_original_owner,
            |owner| {
                Err(Error::Capability(Capability::PlayerCivilizationLevel(
                    owner,
                )))
            },
            |owner| state.world().player_is_valid(owner).map_err(Error::World),
        )
    }

    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        let combat = city_range_attack::Definitions::from_rules(state, rules, bound)?;
        let units = collection(rules, "Units", "UnitType")
            .map_err(|_| Error::Unsupported("Formation Units registry"))?;
        let native = bound
            .unit_definitions()
            .map_err(|_| Error::Unsupported("Formation bound Units"))?;
        if native.lookup.count() != units.count()
            || (0..units.count()).any(|i| native.lookup.type_hash(i) != units.type_hash(i))
        {
            return Err(Error::Unsupported("Formation native Units order"));
        }
        let table = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Unsupported("Units"))?;
        let capture = units
            .entries()
            .iter()
            .map(|entry| {
                let row = table
                    .row_by_id(entry.database_rowid)
                    .ok_or(Error::Unsupported("Units rowid"))?;
                match row
                    .get("CanCapture")
                    .map_err(|_| Error::Unsupported("CanCapture"))?
                {
                    SqlValue::Integer(0) => Ok(false),
                    SqlValue::Integer(1) => Ok(true),
                    _ => Err(Error::Unsupported("CanCapture scalar")),
                }
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let max_attacks = crate::game_parameters::Parameters::from_rules(rules)
            .and_then(|parameters| {
                parameters.integer(crate::game_parameters::IntegerParameter::CombatMaxNumAttacks)
            })
            .map_err(|_| Error::Unsupported("COMBAT_MAX_NUM_ATTACKS parameter"))?;
        Ok(Self {
            binding: state.binding().clone(),
            combat,
            units,
            capture,
            max_attacks,
        })
    }
    pub(crate) fn check(&self, state: &LoadedState) -> Result<(), Error> {
        if state.binding() != &self.binding {
            return Err(Error::ConfigurationIdentity);
        }
        Ok(())
    }
    /// Actual DefinitionCollection.At(index)->IsCanCapture; an absent definition
    /// is distinct from a present definition whose CanCapture is false.
    pub fn definition_can_capture(
        &self,
        state: &LoadedState,
        index: u32,
    ) -> Result<Option<bool>, Error> {
        self.check(state)?;
        Ok(self.capture.get(index as usize).copied())
    }
    pub fn movement_disabled(&self, state: &LoadedState, unit: &Unit) -> Result<bool, Error> {
        self.check(state)?;
        // HasAttacked captures current attacks before querying max; these reads
        // have no callbacks. The zero parameter fallback precedes wrapping bonus.
        let attacks = unit.i_attacks_remaining;
        let base = if self.max_attacks == 0 {
            1
        } else {
            self.max_attacks
        };
        Ok(
            attacks < base.wrapping_add(unit.k_combat_modifers.i_bonus_attacks)
                && unit.i_move_after_attacking <= 0,
        )
    }
    /// Actual configured rule, including zero before the native getter fallback.
    pub fn combat_max_num_attacks(&self, state: &LoadedState) -> Result<i32, Error> {
        self.check(state)?;
        Ok(self.max_attacks)
    }
    pub(crate) fn unit_visible_with_counts<E: From<Error>>(
        &self,
        state: &LoadedState,
        unit: &Unit,
        viewer: i32,
        count: impl FnMut(u32, bool) -> Result<i16, E>,
        hidden_layer0: impl FnOnce() -> Result<u32, E>,
    ) -> Result<bool, E> {
        self.check(state).map_err(E::from)?;
        city_range_attack::unit_visible_with_counts(
            state,
            unit,
            viewer,
            &self.combat,
            count,
            hidden_layer0,
        )
    }
    pub fn combat(&self, state: &LoadedState, unit: &Unit) -> Result<i32, Error> {
        city_range_attack::unit_combat(state, unit, &self.combat)
    }
    pub fn formation_class(&self, state: &LoadedState, unit: &Unit) -> Result<i32, Error> {
        city_range_attack::unit_formation_class(state, unit, &self.combat)
    }
    pub fn ranged(&self, state: &LoadedState, unit: &Unit) -> Result<i32, Error> {
        city_range_attack::unit_ranged_combat(state, unit, &self.combat)
    }
    pub fn bombard(&self, state: &LoadedState, unit: &Unit) -> Result<i32, Error> {
        city_range_attack::unit_bombard_combat(state, unit, &self.combat)
    }
    pub fn can_attack(&self, state: &LoadedState, unit: &Unit) -> Result<bool, Error> {
        Ok((self.combat(state, unit)? > 0
            || self.ranged(state, unit)? > 0
            || self.bombard(state, unit)? > 0
            || unit.k_religion.i_religious_strength > 0)
            && !unit.k_combat_modifers.b_cannot_attack)
    }
    pub fn native_type_index(&self, state: &LoadedState, unit: &Unit) -> Result<u32, Error> {
        self.check(state)?;
        let runtime = state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?;
        let expected = self
            .units
            .find_index(unit.e_type_index)
            .map(|n| n as i32)
            .unwrap_or(-1);
        if runtime.definition_index != expected {
            return Err(Error::Unsupported("Formation initialized Unit type index"));
        }
        Ok(runtime.definition_index as u32)
    }
}
