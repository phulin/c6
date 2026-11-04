//! Ordinary unit-versus-unit TestAttackInto continuation, no attack application.
use super::{actor, TargetSelection};
use crate::{
    combat_defender, formation_movement_cost, formation_rules, movement_placement_state,
    registry::LinuxDefinitionRegistry, resource_binding::ResourceAuthority,
    unit_initialization_definitions, unit_movement_state as moves, unit_placement_services,
    unit_sight_state, Rules,
};
use c6_core::{
    combat_defender::{Context as DefenderContext, MeleeContext},
    core_player::initialization::WireBinding,
    geometry::plot_valid,
    loaded_state::LoadedState,
    map::UnitReference,
    melee_attack::target as kernel,
    skip_turn::placement,
    unit::{ComponentId, PlotCoord, Unit},
};

#[derive(Debug)]
pub enum Error {
    Stage(Box<super::Error>),
    Defender(Box<combat_defender::Error>),
    Combat(Box<crate::loaded_actions::Error>),
    Moves(Box<moves::Error>),
    World(c6_core::world::MissingInput),
    State(c6_core::loaded_state::Error),
    Relations(c6_core::game_diplomatic_relations::Error),
    Placement(placement::Error),
    Cost(Box<formation_movement_cost::Error>),
    Binding,
    Rules(&'static str),
    Missing(&'static str),
    Unsupported(&'static str),
}
impl From<crate::loaded_actions::Error> for Error {
    fn from(e: crate::loaded_actions::Error) -> Self {
        Self::Combat(Box::new(e))
    }
}
impl From<combat_defender::Error> for Error {
    fn from(e: combat_defender::Error) -> Self {
        Self::Defender(Box::new(e))
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "melee target query: {self:?}")
    }
}
impl std::error::Error for Error {}

pub struct Definitions {
    binding: WireBinding,
    units: LinuxDefinitionRegistry,
    properties: Vec<(bool, i32)>,
}
impl Definitions {
    pub fn from_rules(
        state: &LoadedState,
        rules: &Rules,
        bound: &ResourceAuthority<'_>,
    ) -> Result<Self, Error> {
        if bound.destination_binding() != state.binding() {
            return Err(Error::Binding);
        }
        unit_initialization_definitions::Definitions::validate_resources(rules, bound)
            .map_err(|_| Error::Rules("resource identity"))?;
        let units = unit_initialization_definitions::collection(rules, "Units", "UnitType")
            .map_err(|_| Error::Rules("Units registry"))?;
        let table = rules
            .gameplay()
            .table("Units")
            .ok_or(Error::Rules("Units"))?;
        let properties = units
            .entries()
            .iter()
            .map(|e| {
                let row = table
                    .row_by_id(e.database_rowid)
                    .ok_or(Error::Rules("Unit row"))?;
                let air = match row
                    .integer("CanTargetAir")
                    .map_err(|_| Error::Rules("CanTargetAir"))?
                {
                    0 => false,
                    1 => true,
                    _ => return Err(Error::Rules("CanTargetAir bool")),
                };
                let base = i32::try_from(
                    row.integer("BaseMoves")
                        .map_err(|_| Error::Rules("BaseMoves"))?,
                )
                .map_err(|_| Error::Rules("BaseMoves i32"))?;
                Ok((air, base))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            binding: state.binding().clone(),
            units,
            properties,
        })
    }
    fn unit(&self, state: &LoadedState, unit: &Unit) -> Result<Option<(bool, i32)>, Error> {
        if &self.binding != state.binding() {
            return Err(Error::Binding);
        }
        let actual = state
            .unit_runtime(unit.e_owner, unit.id)
            .map_err(Error::State)?
            .definition_index;
        let expected = self
            .units
            .find_index(unit.e_type_index)
            .and_then(|i| i32::try_from(i).ok())
            .unwrap_or(-1);
        if actual != expected {
            return Err(Error::Missing("current Unit definition identity"));
        }
        if actual == -1 {
            return Ok(None);
        }
        self.properties
            .get(actual as usize)
            .copied()
            .map(Some)
            .ok_or(Error::Missing("Unit definition index"))
    }
}

pub struct Inputs<'a> {
    pub combat: &'a formation_rules::Definitions,
    pub movement: &'a moves::Definitions,
    pub sight: &'a unit_sight_state::Runtime,
    pub services: &'a unit_placement_services::Runtime,
    /// Required only after an actual enemy District has been resolved.
    pub districts: Option<&'a formation_movement_cost::Definitions<'a>>,
}
#[derive(Debug, Eq, PartialEq)]
pub enum Outcome {
    Rejected,
    RequiresAttackInto(AttackInto),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Defender {
    Unit(UnitReference),
    District(ComponentId),
}
/// TestAttackInto returned true. AttackInto must STILL resolve its actual target
/// again (its profile differs), generate/resolve results, RNG and all callbacks.
#[derive(Debug, Eq, PartialEq)]
pub struct AttackInto {
    actor: UnitReference,
    destination: PlotCoord,
    defender: Defender,
}
impl AttackInto {
    pub fn actor(&self) -> UnitReference {
        self.actor
    }
    pub fn destination(&self) -> PlotCoord {
        self.destination
    }
    pub fn tested_defender(&self) -> Defender {
        self.defender
    }
}

fn lookup(state: &LoadedState, id: UnitReference) -> Result<&Unit, Error> {
    actor(state, id).map_err(|e| Error::Stage(Box::new(e)))
}
fn enemy(state: &LoadedState, a: i32, b: i32) -> Result<bool, Error> {
    state.world().player(a).map_err(Error::World)?;
    state
        .game_relations()
        .map_err(Error::State)?
        .is_at_war(a, b)
        .map_err(Error::Relations)
}
struct Selector<'s, 'a> {
    state: &'s LoadedState,
    base: combat_defender::Current<'s, 'a>,
    inputs: &'a Inputs<'a>,
    actor: UnitReference,
}
impl DefenderContext for Selector<'_, '_> {
    type Error = Error;
    type Unit = UnitReference;
    fn candidate_count(&mut self) -> Result<usize, Error> {
        self.base.candidate_count().map_err(Error::from)
    }
    fn resolve(&mut self, i: usize) -> Result<Option<UnitReference>, Error> {
        self.base.resolve(i).map_err(Error::from)
    }
    fn delayed_death(&mut self, u: UnitReference) -> Result<bool, Error> {
        self.base.delayed_death(u).map_err(Error::from)
    }
    fn owner(&mut self, u: UnitReference) -> Result<i32, Error> {
        self.base.owner(u).map_err(Error::from)
    }
    fn combat_false(&mut self, u: UnitReference) -> Result<i32, Error> {
        self.base.combat_false(u).map_err(Error::from)
    }
    fn ranged_combat(&mut self, u: UnitReference) -> Result<i32, Error> {
        self.base.ranged_combat(u).map_err(Error::from)
    }
    fn bombard_combat(&mut self, u: UnitReference) -> Result<i32, Error> {
        self.base.bombard_combat(u).map_err(Error::from)
    }
}
impl MeleeContext for Selector<'_, '_> {
    fn is_enemy(&mut self, a: i32, b: i32) -> Result<bool, Error> {
        enemy(self.state, a, b)
    }
    fn visible(&mut self, id: UnitReference) -> Result<bool, Error> {
        let viewer = lookup(self.state, self.actor)?.e_owner;
        let unit = lookup(self.state, id)?;
        if !movement_placement_state::visibility_context_present(
            &self.state.world().map().visibility,
            self.state.world().player_mapping.as_ref(),
            viewer,
        )
        .map_err(|e| Error::Stage(Box::new(super::Error::Placement(e))))?
        {
            return Ok(true);
        }
        self.inputs.combat.unit_visible_with_counts(
            self.state,
            unit,
            viewer,
            |hash, normal| {
                if self.inputs.services.binding != *self.state.binding() {
                    return Err(Error::Binding);
                }
                let width = self
                    .inputs
                    .services
                    .player::<()>(viewer)
                    .map_err(|_| Error::Missing("current visibility owner"))?
                    .native_visibility_width
                    .ok_or(Error::Missing("current Visibility.Instance width"))?;
                let layers = self.inputs.sight.shared(viewer).map_err(Error::Missing)?;
                let Some(layer) = layers.0.iter().find(|l| l.type_hash == hash) else {
                    return if normal {
                        Err(Error::Missing("current initialized shared normal layer"))
                    } else {
                        Ok(0)
                    };
                };
                let index = width
                    .wrapping_mul(unit.k_location.y as u32)
                    .wrapping_add(unit.k_location.x as u32);
                if index >= layer.count {
                    return Ok(0);
                }
                layer
                    .values
                    .get(index as usize)
                    .copied()
                    .ok_or(Error::Missing("current shared visibility scalar"))
            },
            || {
                self.inputs
                    .sight
                    .hidden_layer_hashes
                    .first()
                    .copied()
                    .ok_or(Error::Missing("hidden enum0 layer identity"))
            },
        )
    }
    fn domain(&mut self, id: UnitReference) -> Result<i32, Error> {
        let u = lookup(self.state, id)?;
        Ok(self
            .state
            .unit_runtime(u.e_owner, u.id)
            .map_err(Error::State)?
            .domain)
    }
    fn attacker_religious_strength(&mut self) -> Result<i32, Error> {
        Ok(lookup(self.state, self.actor)?
            .k_religion
            .i_religious_strength)
    }
    fn religious_strength(&mut self, id: UnitReference) -> Result<i32, Error> {
        Ok(lookup(self.state, id)?.k_religion.i_religious_strength)
    }
}

fn attack_after_moving(
    state: &LoadedState,
    defs: &Definitions,
    inputs: &Inputs<'_>,
    actor: UnitReference,
) -> Result<bool, Error> {
    let u = lookup(state, actor)?;
    if u.b_attack_after_moving {
        return Ok(true);
    }
    let Some((_, base)) = defs.unit(state, u)? else {
        return Ok(false);
    };
    Ok(moves::unit_max_moves(
        state,
        u,
        &moves::Inputs {
            definitions: inputs.movement,
        },
    )
    .map_err(|e| Error::Moves(Box::new(e)))?
        > base)
}
fn can_attack_player(state: &LoadedState, a: i32, b: i32) -> Result<bool, Error> {
    if a == -1 || b == -1 {
        return Ok(false);
    }
    if enemy(state, a, b)? {
        return Ok(true);
    }
    Err(Error::Unsupported(
        "CanDeclareWarOn and both banned diplomatic-action predicates",
    ))
}

struct Target<'s, 'a> {
    state: &'s LoadedState,
    defs: &'a Definitions,
    inputs: &'a Inputs<'a>,
    actor: UnitReference,
    defender: UnitReference,
}
impl kernel::Context for Target<'_, '_> {
    type Error = Error;
    fn attack_after_moving(&mut self) -> Result<bool, Error> {
        attack_after_moving(self.state, self.defs, self.inputs, self.actor)
    }

    fn has_moved(&mut self) -> Result<bool, Error> {
        moves::unit_has_moved(
            self.state,
            lookup(self.state, self.actor)?,
            &moves::Inputs {
                definitions: self.inputs.movement,
            },
        )
        .map_err(|e| Error::Moves(Box::new(e)))
    }
    fn attacker_religious_strength(&mut self) -> Result<i32, Error> {
        Ok(lookup(self.state, self.actor)?
            .k_religion
            .i_religious_strength)
    }
    fn is_combat(&mut self) -> Result<bool, Error> {
        combat_defender::is_combat(
            self.state,
            self.inputs.combat,
            lookup(self.state, self.actor)?,
        )
        .map_err(Error::from)
    }
    fn can_target_air(&mut self) -> Result<bool, Error> {
        self.defs
            .unit(self.state, lookup(self.state, self.actor)?)?
            .map(|p| p.0)
            .ok_or(Error::Missing("dereferenced attacker Unit definition"))
    }
    fn defender_domain(&mut self) -> Result<i32, Error> {
        let u = lookup(self.state, self.defender)?;
        Ok(self
            .state
            .unit_runtime(u.e_owner, u.id)
            .map_err(Error::State)?
            .domain)
    }
    fn ranged(&mut self) -> Result<i32, Error> {
        self.inputs
            .combat
            .ranged(self.state, lookup(self.state, self.actor)?)
            .map_err(Error::from)
    }
    fn combat_false(&mut self) -> Result<i32, Error> {
        self.inputs
            .combat
            .combat(self.state, lookup(self.state, self.actor)?)
            .map_err(Error::from)
    }
    fn bombard(&mut self) -> Result<i32, Error> {
        self.inputs
            .combat
            .bombard(self.state, lookup(self.state, self.actor)?)
            .map_err(Error::from)
    }
    fn attacker_domain(&mut self) -> Result<i32, Error> {
        let u = lookup(self.state, self.actor)?;
        Ok(self
            .state
            .unit_runtime(u.e_owner, u.id)
            .map_err(Error::State)?
            .domain)
    }
    fn defender_embarked(&mut self) -> Result<bool, Error> {
        Ok(moves::unit_is_embarked(lookup(self.state, self.defender)?))
    }
    fn can_attack_player(&mut self) -> Result<bool, Error> {
        let a = lookup(self.state, self.actor)?.e_owner;
        let b = lookup(self.state, self.defender)?.e_owner;
        can_attack_player(self.state, a, b)
    }
}

struct DistrictTarget<'s, 'a> {
    state: &'s LoadedState,
    defs: &'a Definitions,
    inputs: &'a Inputs<'a>,
    actor: UnitReference,
    district: &'s c6_core::player_districts::District,
}
impl kernel::DistrictContext for DistrictTarget<'_, '_> {
    type Error = Error;
    fn is_combat(&mut self) -> Result<bool, Error> {
        combat_defender::is_combat(
            self.state,
            self.inputs.combat,
            lookup(self.state, self.actor)?,
        )
        .map_err(Error::from)
    }
    fn attacker_owner(&mut self) -> Result<i32, Error> {
        Ok(lookup(self.state, self.actor)?.e_owner)
    }
    fn district_owner(&mut self) -> Result<i32, Error> {
        district_owner(self.state, self.district)
    }
    fn embarked(&mut self) -> Result<bool, Error> {
        Ok(moves::unit_is_embarked(lookup(self.state, self.actor)?))
    }
    fn fight_while_embarked(&mut self) -> Result<bool, Error> {
        Ok(lookup(self.state, self.actor)?
            .k_combat_modifers
            .b_fight_while_embarked)
    }
    fn domain(&mut self) -> Result<i32, Error> {
        let u = lookup(self.state, self.actor)?;
        Ok(self
            .state
            .unit_runtime(u.e_owner, u.id)
            .map_err(Error::State)?
            .domain)
    }
    fn attack_after_moving(&mut self) -> Result<bool, Error> {
        attack_after_moving(self.state, self.defs, self.inputs, self.actor)
    }
    fn has_moved(&mut self) -> Result<bool, Error> {
        moves::unit_has_moved(
            self.state,
            lookup(self.state, self.actor)?,
            &moves::Inputs {
                definitions: self.inputs.movement,
            },
        )
        .map_err(|e| Error::Moves(Box::new(e)))
    }
    fn district_defensible(&mut self) -> Result<bool, Error> {
        self.inputs
            .districts
            .ok_or(Error::Missing("current district definitions"))?
            .district_defensible(self.district)
            .map_err(|e| Error::Cost(Box::new(e)))
    }
    fn can_attack_player(&mut self, a: i32, b: i32) -> Result<bool, Error> {
        can_attack_player(self.state, a, b)
    }
}
fn district_owner(
    state: &LoadedState,
    d: &c6_core::player_districts::District,
) -> Result<i32, Error> {
    let (_, plot) = c6_core::movement_world::plot_at(
        state.world().map(),
        PlotCoord {
            x: d.location.0,
            y: d.location.1,
        },
    )
    .map_err(|_| Error::Missing("District.GetPlot"))?;
    movement_placement_state::plot_owner(plot)
        .map_err(|e| Error::Stage(Box::new(super::Error::Placement(e))))
}
fn enemy_defensible_district<'s>(
    state: &'s LoadedState,
    inputs: &Inputs<'_>,
    owner: i32,
    at: PlotCoord,
) -> Result<Option<&'s c6_core::player_districts::District>, Error> {
    if !plot_valid(at.x, at.y) {
        return Ok(None);
    }
    let map = state.world().map();
    let index = (at.y as u32)
        .wrapping_mul(map.units.width)
        .wrapping_add(at.x as u32);
    let Some(plot) = map.plots.get(index as usize) else {
        return Ok(None);
    };
    let Some(attrs) = &plot.city_attributes else {
        return Ok(None);
    };
    if attrs.district == u32::MAX
        || attrs.player == -1
        || !enemy(state, i32::from(attrs.player), owner)?
    {
        return Ok(None);
    }
    let defs = inputs
        .districts
        .ok_or(Error::Missing("current district definitions"))?;
    match defs
        .district_at(state, plot)
        .map_err(|e| Error::Cost(Box::new(e)))?
    {
        Some(d)
            if defs
                .district_defensible(d)
                .map_err(|e| Error::Cost(Box::new(e)))? =>
        {
            Ok(Some(d))
        }
        _ => Ok(None),
    }
}

fn finish(
    state: &LoadedState,
    defs: &Definitions,
    inputs: &Inputs<'_>,
    next: TargetSelection,
) -> Result<Outcome, Error> {
    let actor_id = next.actor();
    let at = next.destination();
    let u = lookup(state, actor_id)?;
    if u.k_religion.i_religious_strength > 0 {
        return Err(Error::Unsupported(
            "GetBestReligiousDefender and religious CanAttackTarget",
        ));
    }
    if let Some(district) = enemy_defensible_district(state, inputs, u.e_owner, at)? {
        let mut target = DistrictTarget {
            state,
            defs,
            inputs,
            actor: actor_id,
            district,
        };
        if !kernel::can_attack_district(&mut target)? {
            return Ok(Outcome::Rejected);
        }
        let owner = district_owner(state, district)?;
        return Ok(Outcome::RequiresAttackInto(AttackInto {
            actor: actor_id,
            destination: at,
            defender: Defender::District(ComponentId {
                owner: owner as i16,
                component_type: 3,
                id: district.id,
            }),
        }));
    }
    let exclude_air = !super::can_attack_domain(state, inputs.combat, actor_id, 1, 0)
        .map_err(|e| Error::Stage(Box::new(e)))?;
    // GetBestDefender captures bypass/layer before scanning; these profiles do
    // not use bypass, but the current actual location layer remains mandatory.
    let _bypass = u.k_combat_modifers.b_bypass_combat_units;
    let layer = placement::unit_layer(state.world(), u).map_err(Error::Placement)?;
    let mut selector = Selector {
        state,
        base: combat_defender::Current::new(state, inputs.combat, at, layer),
        inputs,
        actor: actor_id,
    };
    let Some(defender) =
        c6_core::combat_defender::select_melee(&mut selector, u.e_owner, exclude_air)?
    else {
        return Ok(Outcome::Rejected);
    };
    let mut target = Target {
        state,
        defs,
        inputs,
        actor: actor_id,
        defender,
    };
    if !kernel::can_attack_unit(&mut target)? {
        return Ok(Outcome::Rejected);
    }
    let d = lookup(state, defender)?;
    let defender = UnitReference::from_component_id(ComponentId {
        owner: d.e_owner as i16,
        component_type: 1,
        id: d.id,
    });
    Ok(Outcome::RequiresAttackInto(AttackInto {
        actor: actor_id,
        destination: at,
        defender: Defender::Unit(defender),
    }))
}
/// Complete ordinary unit/district TestAttackInto query. Religious selection
/// remains an explicit error, never a false result or fabricated target.
/// All stages borrow ONE placement state's current objects for the whole call.
pub fn test_attack_into_with_placement(
    defs: &Definitions,
    inputs: &Inputs<'_>,
    placement: &movement_placement_state::Context<'_, '_, '_>,
    actor: UnitReference,
    destination: PlotCoord,
) -> Result<Outcome, Error> {
    match super::test_attack_into_prefix_with_placement(
        inputs.combat,
        placement,
        actor,
        destination,
    )
    .map_err(|e| Error::Stage(Box::new(e)))?
    {
        super::Prefix::Rejected(_) => Ok(Outcome::Rejected),
        super::Prefix::RequiresTargetSelection(next) => {
            finish(placement.current_state(), defs, inputs, next)
        }
    }
}
