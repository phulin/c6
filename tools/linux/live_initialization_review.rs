//! Independent public-API checks; the private Unit input is only a decode seed.
use c6_core::core_player::initialization::{DatabaseIdentity, InputIdentities};
use c6_core::formation_initialization::{Error as ArchiveError, FormationRuntime};
use c6_core::map::Slot;
use c6_core::movement_world::{DefinitionLookup, DefinitionTable};
use c6_core::unit::{Formation, HashValue, PlotCoord, Unit};
use c6_core::unit_initialization::{Error, InitializationInputs, UnitDefinition, UnitRuntime};
use c6_core::world::{CompleteSlots, MissingInput, Player};
use std::cell::Cell;

struct Lookup;
impl DefinitionLookup for Lookup {
    fn count(&self) -> usize {
        3
    }
    fn find_index(&self, hash: u32) -> Option<usize> {
        match hash {
            0xaaaa => Some(2),
            0xbbbb => Some(0),
            0xcccc => Some(9),
            _ => None,
        }
    }
    fn type_hash(&self, index: usize) -> Option<u32> {
        [0xbbbb, 0xdddd, 0xaaaa].get(index).copied()
    }
}
fn pair<T>(definition_hash: u32, value: T) -> HashValue<T> {
    HashValue {
        definition_hash,
        value,
    }
}
fn formation() -> Formation {
    Formation {
        id: 0xabcd0007,
        owner: 2,
        location: PlotCoord { x: 5, y: 8 },
        moves_remaining_raw: -99,
        domain: 4,
        is_embarked: false,
        can_embark: false,
        can_attack: true,
        is_ranged_combat: true,
        can_capture: false,
        has_moved: false,
        has_moved_into_zoc: false,
        ignores_zoc: false,
        last_path: vec![],
        last_path_cache_location: PlotCoord { x: 5, y: 8 },
        unit_ids: vec![0x12340003, 0xffff0003, 0x12340003],
    }
}
fn main() {
    // An &self lookup can change through interior mutation. Validate each
    // answer once, then use that exact answer for writes, including duplicates.
    struct ChangingLookup(Cell<usize>);
    impl DefinitionLookup for ChangingLookup {
        fn count(&self) -> usize {
            3
        }
        fn find_index(&self, _: u32) -> Option<usize> {
            let calls = self.0.get();
            self.0.set(calls + 1);
            Some(if calls < 2 { calls } else { 9 })
        }
        fn type_hash(&self, _: usize) -> Option<u32> {
            None
        }
    }
    let changing = ChangingLookup(Cell::new(0));
    let mut destination = [7, 8, 9];
    c6_core::unit_initialization::overlay(
        &mut destination,
        &[pair(0xaaaa, 11), pair(0xbbbb, 22)],
        &changing,
    )
    .unwrap();
    assert_eq!(destination, [11, 22, 9]);
    assert_eq!(changing.0.get(), 2);
    let seed = std::fs::read(std::env::args().nth(1).expect("Unit seed path")).unwrap();
    let mut unit = Unit::decode(&seed, Default::default()).unwrap();
    unit.id = 0x12340003;
    unit.e_owner = 2;
    unit.e_type_index = 0xaaaa;
    unit.formation_id = 0xeeee0007;
    unit.ai_abilities_possessed = vec![pair(0xaaaa, 8), pair(0x1111, 99), pair(0xaaaa, -3)];
    unit.ab_invalid_terrain_types = vec![pair(0xbbbb, true)];
    unit.ab_ignore_terrain_types = vec![];
    unit.ab_ignore_feature_types = vec![pair(0xaaaa, true)];
    unit.ai_bonus_points_per_kill = vec![pair(0xbbbb, -7)];
    unit.ai_bonus_points_per_kill_strength = vec![pair(0xaaaa, 12)];
    unit.k_operations_disabled = vec![pair(0xbbbb, true), pair(0xaaaa, true), pair(0xbbbb, false)];
    let identities = InputIdentities {
        configuration_sha256: [1; 32],
        rules_inputs_sha256: [2; 32],
        database: DatabaseIdentity::LiveSnapshotUnavailable,
        dlc_inputs_sha256: [3; 32],
        mods_inputs_sha256: [4; 32],
        content_inputs_sha256: [5; 32],
    };
    let definitions = [
        UnitDefinition { domain_hash: 0 },
        UnitDefinition { domain_hash: 0 },
        UnitDefinition {
            domain_hash: 0xc03bbd2f,
        },
    ];
    let inputs = InitializationInputs {
        identities: &identities,
        units: DefinitionTable::new(&Lookup, &definitions).unwrap(),
        abilities: &Lookup,
        terrains: &Lookup,
        features: &Lookup,
        great_person_classes: &Lookup,
        operations: &Lookup,
    };
    // Move records into the owner once. Runtime creation borrows this owner.
    let owner = Player::from_lookup_slots(
        2,
        Some(CompleteSlots {
            entries: vec![Slot {
                index: 3,
                value: unit,
            }],
        }),
        Some(CompleteSlots {
            entries: vec![Slot {
                index: 7,
                value: formation(),
            }],
        }),
    )
    .unwrap();
    let owned = owner.unit(0x99990003).unwrap().unwrap().record;
    let original = owned.clone(); // one record snapshot for nonmutation assertion
    let runtime = UnitRuntime::from_read(owned, &identities, &inputs, &owner).unwrap();
    assert_eq!((runtime.definition_index, runtime.domain), (2, 3));
    let reference = runtime.formation.unwrap();
    assert_eq!(
        (
            reference.owner,
            reference.requested_id,
            reference.resolved_id
        ),
        (2, 0xeeee0007, 0xabcd0007)
    );
    assert_eq!(runtime.abilities, [0, 0, -3]);
    assert_eq!(runtime.invalid_terrain, [true, false, false]);
    assert_eq!(runtime.ignore_terrain, [false; 3]);
    assert_eq!(runtime.ignore_feature, [false, false, true]);
    assert_eq!(runtime.bonus_points_per_kill, [-7, 0, 0]);
    assert_eq!(runtime.bonus_points_per_kill_strength, [0, 0, 12]);
    assert_eq!(runtime.operations_disabled, [false, false, true]);
    assert_eq!(runtime.archive.identity_bits(), 0x12340003_00010002);
    assert_eq!(
        runtime.archive.dirty_indices(),
        Err(ArchiveError::DirtyBaselineUnavailable)
    );
    assert_eq!(owned, &original);
    assert_eq!(owned.e_type_index, 0xaaaa);
    let mut other = identities.clone();
    other.content_inputs_sha256[0] ^= 1;
    assert_eq!(
        UnitRuntime::from_read(owned, &other, &inputs, &owner),
        Err(Error::Profile)
    );
    let mut changed = owned.clone();
    changed.e_type_index = 0x1111;
    assert_eq!(
        UnitRuntime::from_read(&changed, &identities, &inputs, &owner),
        Err(Error::Definition)
    );
    changed.e_type_index = 0xcccc;
    assert_eq!(
        UnitRuntime::from_read(&changed, &identities, &inputs, &owner),
        Err(Error::DefinitionIndex)
    );
    changed = owned.clone();
    changed.e_owner = 3;
    assert_eq!(
        UnitRuntime::from_read(&changed, &identities, &inputs, &owner),
        Err(Error::Owner)
    );
    changed = owned.clone();
    changed.ai_abilities_possessed.push(pair(0xcccc, 1));
    assert_eq!(
        UnitRuntime::from_read(&changed, &identities, &inputs, &owner),
        Err(Error::DefinitionIndex)
    );
    assert_eq!(owned, &original);
    let missing = Player::from_lookup_slots(2, None, None).unwrap();
    assert_eq!(
        UnitRuntime::from_read(owned, &identities, &inputs, &missing),
        Err(Error::World(MissingInput::Formations { owner: 2 }))
    );
    let empty =
        Player::from_lookup_slots(2, None, Some(CompleteSlots { entries: vec![] })).unwrap();
    assert!(UnitRuntime::from_read(owned, &identities, &inputs, &empty)
        .unwrap()
        .formation
        .is_none());
    changed = owned.clone();
    changed.formation_id = u32::MAX;
    assert!(
        UnitRuntime::from_read(&changed, &identities, &inputs, &owner)
            .unwrap()
            .formation
            .is_none()
    );
    changed.formation_id = 0xffff0008;
    assert!(
        UnitRuntime::from_read(&changed, &identities, &inputs, &owner)
            .unwrap()
            .formation
            .is_none()
    );
    let mut f = formation();
    let mut fr = FormationRuntime::from_read(&f, |owner| Ok::<_, ()>(Some(owner))).unwrap();
    assert_eq!(fr.archive.dirty_calls(), [4, 13]);
    fr.clear_path_cache(&mut f).unwrap();
    assert_eq!(fr.archive.dirty_calls(), [4, 13, 14, 15]);
    let members = f.unit_ids.clone();
    let trace = fr.archive.dirty_calls().to_vec();
    f.id ^= 0x10000;
    assert_eq!(
        fr.set_evaluating_operation(&f, true),
        Err(ArchiveError::IdentityMismatch)
    );
    assert_eq!(
        fr.clear_path_cache(&mut f),
        Err(ArchiveError::IdentityMismatch)
    );
    assert_eq!(fr.archive.dirty_calls(), trace);
    assert_eq!(f.unit_ids, members);
    println!("public API integration checks passed; no native execution or action acceptance");
}
