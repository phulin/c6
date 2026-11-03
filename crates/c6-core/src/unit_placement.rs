//! Exact Linux XP2 layered friendly-count query over the existing typed World.
//! This is a post-load query projection, not full action admission or live import.
use crate::map::UnitReference;
use crate::movement_world::DefinitionTable;
use crate::unit::{ComponentId, Formation, Unit};
use crate::world::{MissingInput as WorldError, World};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    World(WorldError),
    SlotStatus(i32),
    Alive(i32),
    Team(i32),
    Definitions,
    DefinitionIndex,
    WarRelations,
    WarIndex,
    Formation { owner: i32, id: u32 },
    PlotUnitLimit,
}
impl From<WorldError> for Error {
    fn from(value: WorldError) -> Self {
        Self::World(value)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Linux placement input: {self:?}")
    }
}
impl std::error::Error for Error {}

/// Actual native getter values in the existing native-order DefinitionTable.
/// These are definition hashes/scalars, never SQL row IDs or effective Unit stats.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnitProperties {
    pub domain_hash: u32,
    pub formation_class_hash: u32,
    pub make_trade_route: bool,
    pub spy: bool,
    pub combat: i32,
    pub range: i32,
}

/// Exact queried values from Game.DiplomaticRelations, not Player.Diplomacy.
/// The complete native WarTypes vector retains its triangular storage order.
/// Each value is the original relationship's first i32; -1 is NO_WAR.
pub struct WarRelations<'a> {
    pub player_count: i32,
    pub war_types: &'a [i32],
}
impl WarRelations<'_> {
    pub fn at_war(&self, a: i32, b: i32) -> Result<bool, Error> {
        if a == b {
            return Ok(false);
        }
        let n = self.player_count;
        // Native helper's signed wrapping arithmetic and arithmetic right shift.
        let total = n.wrapping_sub(1).wrapping_mul(n) >> 1;
        if total < 0 || self.war_types.len() != total as usize {
            return Err(Error::WarRelations);
        }
        let high = a.max(b);
        let low = a.min(b);
        let triangle = high.wrapping_add(1).wrapping_mul(high) >> 1;
        let index = low.wrapping_sub(triangle).wrapping_add(total);
        let value = self.war_types.get(index as usize).ok_or(Error::WarIndex)?;
        Ok(*value != -1)
    }
}

/// Exact actual-manager result. The otherwise redundant combat/range reads are
/// retained from the native unequal-formation-class branch; no host type policy.
pub fn same_type(a: Option<&UnitProperties>, b: Option<&UnitProperties>) -> bool {
    let (Some(a), Some(b)) = (a, b) else {
        return false;
    };
    let mut x = a.domain_hash;
    let mut y = b.domain_hash;
    if y == 0x44c22cdb {
        y = x;
    }
    if x == 0x44c22cdb {
        x = y;
        y = b.domain_hash;
    }
    if x != y || x == 0x5397d46f || a.make_trade_route || b.make_trade_route {
        return false;
    }
    if a.formation_class_hash == b.formation_class_hash {
        return true;
    }
    let first_combat = a.combat > 0 || a.range > 0;
    let second_combat = b.combat > 0 || b.range > 0;
    first_combat == second_combat && a.formation_class_hash == b.formation_class_hash
}

/// Original actual IsCargo body at 0x28e04f0 is literally false for this target.
pub fn is_cargo(_: &Unit) -> bool {
    false
}

/// Native sparse-grid lookup wraps the signed linear arithmetic; it does not
/// wrap coordinates geometrically or require each axis independently in bounds.
pub fn layered_units<'a>(world: &'a World, unit: &Unit) -> Result<&'a [UnitReference], Error> {
    let coord = unit.k_location;
    if coord.x < -9998 || coord.y < -9998 {
        return Ok(&[]);
    }
    let index = coord
        .y
        .wrapping_mul(world.map().units.width as i32)
        .wrapping_add(coord.x) as u32;
    let identity = ComponentId {
        owner: unit.e_owner as i16,
        component_type: 1,
        id: unit.id,
    };
    let mut layer_id = None;
    for layer in &world.map().units.layers {
        if index >= layer.maximum || index as usize >= world.map().plots.len() {
            continue;
        }
        let candidates = world.plot_units(layer.layer_id, index as usize)?;
        if candidates.iter().any(|r| r.component_id() == identity) {
            layer_id = Some(layer.layer_id);
            break;
        }
    }
    let Some(id) = layer_id.filter(|id| *id != u32::MAX) else {
        return Ok(&[]);
    };
    Ok(world.plot_units(id, index as usize)?)
}

/// Borrowed genuine inputs; absence is explicit and evaluated only when used.
/// Unit wire hashes are reconstructed through the actual native registry's
/// HashToIndex/InitializePostLoad contract. Live records must retain that binding
/// until their next exact type change; arbitrary live pointer edits are excluded.
pub struct Inputs<'a, 'd> {
    pub world: &'a World,
    pub definitions: Option<&'a DefinitionTable<'d, Option<UnitProperties>>>,
    pub relations: Option<&'a WarRelations<'a>>,
    pub plot_unit_limit: Option<i32>,
}
impl Inputs<'_, '_> {
    fn definition(&self, unit: &Unit) -> Result<Option<&UnitProperties>, Error> {
        let table = self.definitions.ok_or(Error::Definitions)?;
        let runtime_index = table
            .lookup
            .find_index(unit.e_type_index)
            .map(|i| i as u32)
            .unwrap_or(u32::MAX);
        // Post-load calls Find again on the remapped runtime index, including -1.
        let Some(index) = table.lookup.find_index(runtime_index) else {
            return Ok(None);
        };
        table
            .at(index as i32)
            .ok_or(Error::DefinitionIndex)
            .map(Option::as_ref)
    }
    pub fn is_human(&self, owner: i32) -> Result<bool, Error> {
        Ok(self
            .world
            .player(owner)?
            .slot_status
            .ok_or(Error::SlotStatus(owner))?
            == 3)
    }
    pub fn at_war_with(&self, owner: i32, target: i32) -> Result<bool, Error> {
        if !self.world.player_is_valid(target)? {
            return Ok(false);
        }
        if !self
            .world
            .player(target)?
            .is_alive
            .ok_or(Error::Alive(target))?
        {
            return Ok(false);
        }
        if owner == target {
            return Ok(false);
        }
        self.relations
            .ok_or(Error::WarRelations)?
            .at_war(owner, target)
    }
    pub fn same_team(&self, a: i32, b: i32) -> Result<bool, Error> {
        // Native resolves both actual GetValidPlayer pointers before testing
        // either for null or taking the same-player shortcut. A valid mapping
        // alone does not prove an imported Player record is available.
        let first = if self.world.player_is_valid(a)? {
            Some(self.world.player(a)?)
        } else {
            None
        };
        let second = if self.world.player_is_valid(b)? {
            Some(self.world.player(b)?)
        } else {
            None
        };
        let (Some(first), Some(second)) = (first, second) else {
            return Ok(false);
        };
        if a == b {
            return Ok(true);
        }
        let x = first.team.ok_or(Error::Team(a))?;
        let y = second.team.ok_or(Error::Team(b))?;
        Ok(x != -1 && y != -1 && x == y)
    }
    pub fn rivals(&self, a: &Unit, b: &Unit) -> Result<bool, Error> {
        if a.e_owner == b.e_owner
            || a.k_religion.i_religious_strength <= 0
            || b.k_religion.i_religious_strength <= 0
            || a.k_religion.e_religion_type == b.k_religion.e_religion_type
        {
            return Ok(false);
        }
        Ok(!self.same_team(a.e_owner, b.e_owner)?)
    }
    pub fn unit_count(&self, subject: &Unit, candidates: &[UnitReference]) -> Result<u32, Error> {
        self.unit_count_with_live(subject, candidates, Some(subject))
    }
    fn unit_count_with_live(
        &self,
        subject: &Unit,
        candidates: &[UnitReference],
        live: Option<&Unit>,
    ) -> Result<u32, Error> {
        // Original obtains this owner's actual Player/Diplomacy before iteration,
        // even when the candidate vector is empty. No relation values read yet.
        self.world.player(subject.e_owner)?;
        let mut count = 0u32;
        for &reference in candidates {
            let Some(found) = self.world.unit(reference)? else {
                continue;
            };
            // Recycled aliases of this exact logical object share its live
            // candidate state, even when a member/reference uses stale ID bits.
            let other = live
                .filter(|u| u.e_owner == found.record.e_owner && u.id == found.record.id)
                .unwrap_or(found.record);
            if self.at_war_with(subject.e_owner, other.e_owner)? {
                continue;
            }
            if other.e_owner == subject.e_owner
                && !same_type(self.definition(subject)?, self.definition(other)?)
            {
                continue;
            }
            if is_cargo(other) {
                continue;
            }
            if self
                .definition(other)?
                .is_some_and(|d| d.make_trade_route || d.spy)
            {
                continue;
            }
            if !self.rivals(subject, other)? {
                count = count.wrapping_add(1);
            }
        }
        Ok(count)
    }
    pub fn formation_count(
        &self,
        formation: &Formation,
        candidates: &[UnitReference],
    ) -> Result<u32, Error> {
        self.formation_count_with_subject(formation, candidates, None)
    }
    fn formation_count_with_subject(
        &self,
        formation: &Formation,
        candidates: &[UnitReference],
        live: Option<&Unit>,
    ) -> Result<u32, Error> {
        let player = self.world.player(formation.owner)?;
        let mut maximum = 0u32;
        for member in player.members(formation) {
            if let Some(found) = member? {
                let unit = live
                    .filter(|u| u.e_owner == found.record.e_owner && u.id == found.record.id)
                    .unwrap_or(found.record);
                if !unit.b_delayed_death {
                    maximum = maximum.max(self.unit_count_with_live(unit, candidates, live)?);
                }
            }
        }
        Ok(maximum)
    }
    pub fn friendly_count(&self, unit: &Unit) -> Result<u32, Error> {
        // Handler gets actual layered candidates before resolving Formation.
        let candidates = layered_units(self.world, unit)?;
        let formation = self
            .world
            .player(unit.e_owner)?
            .formation(unit.formation_id)?
            .ok_or(Error::Formation {
                owner: unit.e_owner,
                id: unit.formation_id,
            })?;
        self.formation_count_with_subject(formation.record, candidates, Some(unit))
    }
}
impl super::legality::SkipTurnQueries for Inputs<'_, '_> {
    type Error = Error;
    fn is_human(&mut self, unit: &mut Unit) -> Result<bool, Self::Error> {
        Inputs::is_human(self, unit.e_owner)
    }
    fn friendly_units_of_type_count(&mut self, unit: &mut Unit) -> Result<u32, Self::Error> {
        self.friendly_count(unit)
    }
    fn plot_unit_limit(&mut self, _: &mut Unit) -> Result<i32, Self::Error> {
        self.plot_unit_limit.ok_or(Error::PlotUnitLimit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn property(domain: u32, class: u32) -> UnitProperties {
        UnitProperties {
            domain_hash: domain,
            formation_class_hash: class,
            make_trade_route: false,
            spy: false,
            combat: 0,
            range: 0,
        }
    }
    #[test]
    fn same_type_uses_actual_definition_categories_and_nulls() {
        let a = property(101, 3);
        let mut b = property(101, 3);
        assert!(same_type(Some(&a), Some(&b)));
        b.formation_class_hash = 4;
        assert!(!same_type(Some(&a), Some(&b)));
        b.combat = 1;
        b.range = 1;
        assert!(!same_type(Some(&a), Some(&b)));
        assert!(!same_type(None, Some(&b)));
        b = a;
        b.make_trade_route = true;
        assert!(!same_type(Some(&a), Some(&b)));
        b = a;
        b.spy = true;
        assert!(same_type(Some(&a), Some(&b))); // Spy is a later count filter.
    }
    #[test]
    fn domain_wildcard_and_excluded_domain_are_symmetric() {
        for (a, b, expected) in [
            (101, 101, true),
            (101, 102, false),
            (0x44c22cdb, 101, true),
            (101, 0x44c22cdb, true),
            (0x44c22cdb, 0x44c22cdb, true),
            (0x5397d46f, 0x5397d46f, false),
            (0x5397d46f, 0x44c22cdb, false),
            (0x44c22cdb, 0x5397d46f, false),
        ] {
            assert_eq!(
                same_type(Some(&property(a, 1)), Some(&property(b, 1))),
                expected
            );
        }
    }
    #[test]
    fn relationship_storage_is_native_triangle_and_minus_one_sentinel() {
        let relations = WarRelations {
            player_count: 4,
            war_types: &[0, -1, i32::MIN, -1, i32::MAX, -1],
        };
        for (a, b, index) in [
            (0, 1, 5),
            (0, 2, 3),
            (1, 2, 4),
            (0, 3, 0),
            (1, 3, 1),
            (2, 3, 2),
        ] {
            assert_eq!(
                relations.at_war(a, b).unwrap(),
                relations.war_types[index] != -1
            );
            assert_eq!(
                relations.at_war(b, a).unwrap(),
                relations.war_types[index] != -1
            );
        }
        assert!(!WarRelations {
            player_count: 0,
            war_types: &[]
        }
        .at_war(2, 2)
        .unwrap());
        assert_eq!(
            WarRelations {
                player_count: 4,
                war_types: &[]
            }
            .at_war(0, 1),
            Err(Error::WarRelations)
        );
    }
    #[test]
    fn packed_references_preserve_signed_halves_and_full_generation() {
        for (owner, ty, id) in [(7, 1, 0x12340001), (-2, -7, 0xabcdef01), (-1, 1, u32::MAX)] {
            let identity = ComponentId {
                owner,
                component_type: ty,
                id,
            };
            let reference = UnitReference::from_component_id(identity);
            assert_eq!(reference.component_id(), identity);
            assert_eq!(reference.player(), i32::from(owner));
        }
    }
}
