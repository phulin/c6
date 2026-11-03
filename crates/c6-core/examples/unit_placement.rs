//! Native serialized occupancy replay and synthetic source-derived query branches.
//! No original code executes; modified cases are not native parity fixtures.
use c6_core::map::{Map, Recycled, Slot, UnitCell, UnitContainer, UnitLayer, UnitReference};
use c6_core::movement_world::{DefinitionLookup, DefinitionTable};
use c6_core::skip_turn::{
    legality,
    placement::{self, Inputs, UnitProperties, WarRelations},
};
use c6_core::unit::{ComponentId, Formation, Unit};
use c6_core::world::{Player, World};
use std::{env, fs};
fn recycled<T>(entries: Vec<T>, id: impl Fn(&T) -> u32) -> Recycled<T> {
    let capacity = entries.iter().map(|v| id(v) & 65535).max().unwrap_or(0) + 1;
    Recycled {
        capacity,
        entries: entries
            .into_iter()
            .map(|value| Slot {
                index: (id(&value) & 65535) as i32,
                value,
            })
            .collect(),
        current_id_bits: 0x10000,
        free_head: 0,
        free_tail: 0,
        free_links: vec![],
    }
}
fn reference(owner: i16, ty: i16, id: u32) -> UnitReference {
    UnitReference::from_component_id(ComponentId {
        owner,
        component_type: ty,
        id,
    })
}
struct Registry;
impl DefinitionLookup for Registry {
    fn count(&self) -> usize {
        4
    }
    fn find_index(&self, value: u32) -> Option<usize> {
        if value < 4 {
            Some(value as usize)
        } else {
            None
        }
    }
    fn type_hash(&self, index: usize) -> Option<u32> {
        (index < 4).then_some(index as u32)
    }
}
fn world(map: &Map, unit: &Unit, formation: &Formation, religion: bool) -> World {
    let mut map = map.clone();
    let mut a = unit.clone();
    a.id = 0x10001;
    a.e_owner = 0;
    a.e_type_index = 0;
    a.formation_id = 1;
    a.k_location.x = 0;
    a.k_location.y = 0;
    a.i_fortify_turns = 0;
    a.e_activity = 0x6b25012e;
    a.k_espionage.e_operation = u32::MAX;
    a.b_delayed_death = false;
    a.k_religion.i_religious_strength = if religion { 1 } else { 0 };
    a.k_religion.e_religion_type = 101;
    let mut b = a.clone();
    b.id = 0x20002;
    b.e_owner = 1;
    b.e_type_index = 1;
    b.k_religion.e_religion_type = 102;
    let mut c = a.clone();
    c.id = 0x30003;
    c.e_type_index = 2;
    let mut d = a.clone();
    d.id = 0x40004;
    d.e_type_index = 3;
    let mut f = formation.clone();
    f.id = 1;
    f.owner = 0;
    f.unit_ids = vec![a.id, c.id, u32::MAX];
    map.units = UnitContainer {
        width: map.header.width as u32,
        height: map.header.height as u32,
        layers: vec![
            // Wrong component type must not win layer discovery.
            UnitLayer {
                layer_id: 9,
                maximum: map.plots.len() as u32,
                cells: vec![UnitCell {
                    plot_index: 0,
                    units: vec![reference(0, 7, a.id)],
                }],
            },
            UnitLayer {
                layer_id: 3,
                maximum: map.plots.len() as u32,
                cells: vec![UnitCell {
                    plot_index: 0,
                    units: vec![
                        reference(0, 1, a.id),
                        reference(1, 1, b.id),
                        reference(0, 1, c.id),
                        reference(0, 1, d.id),
                        reference(0, 1, u32::MAX),
                    ],
                }],
            },
            UnitLayer {
                layer_id: 1,
                maximum: map.plots.len() as u32,
                cells: vec![UnitCell {
                    plot_index: 0,
                    units: vec![reference(0, 1, a.id)],
                }],
            },
        ],
    };
    let mut p = Player::new(
        0,
        Some(recycled(vec![a, c, d], |u| u.id)),
        Some(recycled(vec![f], |f| f.id)),
    )
    .unwrap();
    p.slot_status = Some(3);
    p.team = Some(4);
    p.is_alive = Some(true);
    let mut q = Player::new(
        1,
        Some(recycled(vec![b], |u| u.id)),
        Some(recycled(vec![], |f: &Formation| f.id)),
    )
    .unwrap();
    q.slot_status = Some(1);
    q.team = Some(5);
    q.is_alive = Some(true);
    let mut w = World::new(map, vec![p, q]).unwrap();
    let mut mapping = [-1; 65];
    mapping[0] = 0;
    mapping[1] = 1;
    w.player_mapping = Some(mapping);
    w
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = env::args().collect();
    if a.len() != 4 {
        return Err("usage: unit_placement MAP UNIT FORMATION".into());
    }
    let map = Map::decode(&fs::read(&a[1])?, Default::default())?;
    let unit = Unit::decode(&fs::read(&a[2])?, Default::default())?;
    let formation = Formation::decode(&fs::read(&a[3])?, Default::default())?;
    let p = Player::new(
        unit.e_owner,
        Some(recycled(vec![unit.clone()], |u| u.id)),
        Some(recycled(vec![formation.clone()], |f| f.id)),
    )?;
    let native = World::new(map.clone(), vec![p])?;
    let candidates = placement::layered_units(&native, &unit)?;
    println!(
        "{{\"kind\":\"native_occupancy\",\"unit\":{},\"owner\":{},\"references\":[{}]}}",
        unit.id,
        unit.e_owner,
        candidates
            .iter()
            .map(|r| format!("[{},{}]", r.owner, r.id))
            .collect::<Vec<_>>()
            .join(",")
    );
    let properties = [
        Some(UnitProperties {
            domain_hash: 101,
            formation_class_hash: 10,
            make_trade_route: false,
            spy: false,
            combat: 1,
            range: 0,
        }),
        Some(UnitProperties {
            domain_hash: 102,
            formation_class_hash: 20,
            make_trade_route: false,
            spy: false,
            combat: 0,
            range: 1,
        }),
        Some(UnitProperties {
            domain_hash: 101,
            formation_class_hash: 20,
            make_trade_route: false,
            spy: false,
            combat: 1,
            range: 0,
        }),
        Some(UnitProperties {
            domain_hash: 101,
            formation_class_hash: 10,
            make_trade_route: false,
            spy: true,
            combat: 0,
            range: 0,
        }),
    ];
    let defs = DefinitionTable::new(&Registry, &properties)?;
    let relations = WarRelations {
        player_count: 2,
        war_types: &[-1],
    };
    let mut w = world(&map, &unit, &formation, false);
    let u = w.player(0)?.unit(1)?.unwrap().record.clone();
    macro_rules! inputs {
        ($world:expr) => {
            Inputs {
                world: $world,
                definitions: Some(&defs),
                relations: Some(&relations),
                plot_unit_limit: Some(2),
            }
        };
    }
    let mut checks = 0;
    assert_eq!(placement::layered_units(&w, &u)?.len(), 5);
    checks += 1;
    // Wrong component type cannot select a layer, but original GetUnit ignores it.
    assert_eq!(
        w.unit(reference(0, 7, 0x90001))?.unwrap().resolved_id,
        0x10001
    );
    checks += 1;
    assert!(w.unit(reference(-1, 1, 1))?.is_none());
    checks += 1;
    assert!(w.unit(reference(-2, 1, 1))?.is_none());
    checks += 1;
    let x = reference(-2, -7, 0xabcdef01).component_id();
    assert_eq!((x.owner, x.component_type, x.id), (-2, -7, 0xabcdef01));
    checks += 1;
    assert_eq!(inputs!(&w).friendly_count(&u)?, 2);
    checks += 1; // max(member counts), not sum
    let mut copy = u.clone();
    assert!(legality::skip_turn_can_start(&mut copy, &mut inputs!(&w))?);
    checks += 1;
    w.player_mut(0)?.slot_status = None;
    assert_eq!(
        inputs!(&w).is_human(0),
        Err(placement::Error::SlotStatus(0))
    );
    checks += 1;
    w.player_mut(0)?.slot_status = Some(2);
    let mut absent = Inputs {
        world: &w,
        definitions: None,
        relations: None,
        plot_unit_limit: None,
    };
    assert!(legality::skip_turn_can_start(&mut copy, &mut absent)?);
    checks += 1; // nonhuman skips placement
    w.player_mut(0)?.slot_status = Some(3);
    assert!(!inputs!(&w).at_war_with(0, 0)?);
    checks += 1;
    let war = WarRelations {
        player_count: 2,
        war_types: &[0],
    };
    let hostile = Inputs {
        relations: Some(&war),
        ..inputs!(&w)
    };
    assert_eq!(hostile.friendly_count(&u)?, 1);
    checks += 1;
    w.player_mut(1)?.is_alive = Some(false);
    assert!(!hostile_from(&w, &defs, &war).at_war_with(0, 1)?);
    checks += 1;
    w.player_mut(1)?.is_alive = None;
    assert_eq!(
        inputs!(&w).at_war_with(0, 1),
        Err(placement::Error::Alive(1))
    );
    checks += 1;
    w.player_mut(1)?.is_alive = Some(true);
    w.player_mut(0)?.unit_mut(1)?.unwrap().b_delayed_death = true;
    w.player_mut(0)?.unit_mut(3)?.unwrap().b_delayed_death = true;
    assert_eq!(
        inputs!(&w).formation_count(
            w.player(0)?.formation(1)?.unwrap().record,
            placement::layered_units(&w, &u)?
        )?,
        0
    );
    checks += 1;
    let mut religious = world(&map, &unit, &formation, true);
    let mut religious_u = religious.player(0)?.unit(1)?.unwrap().record.clone();
    assert_eq!(inputs!(&religious).friendly_count(&religious_u)?, 1);
    checks += 1;
    religious.player_mut(1)?.team = Some(4);
    assert_eq!(inputs!(&religious).friendly_count(&religious_u)?, 2);
    checks += 1;
    religious.player_mut(1)?.team = Some(-1);
    assert_eq!(inputs!(&religious).friendly_count(&religious_u)?, 1);
    checks += 1;
    religious
        .player_mut(0)?
        .unit_mut(1)?
        .unwrap()
        .k_religion
        .i_religious_strength = -1;
    religious
        .player_mut(0)?
        .unit_mut(3)?
        .unwrap()
        .k_religion
        .i_religious_strength = -1;
    religious.player_mut(1)?.team = None;
    religious_u.k_religion.i_religious_strength = -1;
    assert_eq!(inputs!(&religious).friendly_count(&religious_u)?, 2);
    checks += 1; // no team read for nonreligious subject
    religious.player_mapping = None;
    assert!(matches!(
        religious.unit(reference(0, 1, 1)),
        Err(c6_core::world::MissingInput::PlayerRegistry)
    ));
    checks += 1;
    let mut defs_changed = properties;
    defs_changed[1] = None; // known missing definition still counts for foreign nonrival
    let d = DefinitionTable::new(&Registry, &defs_changed)?;
    assert_eq!(
        Inputs {
            world: &world(&map, &unit, &formation, false),
            definitions: Some(&d),
            relations: Some(&relations),
            plot_unit_limit: Some(2)
        }
        .friendly_count(&u)?,
        2
    );
    checks += 1;
    for (left, right, expect) in [
        (101, 101, true),
        (101, 102, false),
        (0x44c22cdb, 101, true),
        (101, 0x44c22cdb, true),
        (0x5397d46f, 0x5397d46f, false),
    ] {
        let mut a = properties[0].unwrap();
        let mut b = a;
        a.domain_hash = left;
        b.domain_hash = right;
        assert_eq!(placement::same_type(Some(&a), Some(&b)), expect);
        checks += 1;
    }
    let table = WarRelations {
        player_count: 4,
        war_types: &[0, -1, 0, -1, 0, -1],
    };
    for (a, b, i) in [
        (0, 1, 5),
        (0, 2, 3),
        (1, 2, 4),
        (0, 3, 0),
        (1, 3, 1),
        (2, 3, 2),
    ] {
        assert_eq!(table.at_war(a, b)?, table.war_types[i] != -1);
        assert_eq!(table.at_war(b, a)?, table.war_types[i] != -1);
        checks += 1;
    }
    println!(
        "{{\"kind\":\"synthetic_checks\",\"count\":{checks},\"native_boundary_fixtures\":false}}"
    );
    Ok(())
}
fn hostile_from<'a, 'd>(
    world: &'a World,
    definitions: &'a DefinitionTable<'d, Option<UnitProperties>>,
    relations: &'a WarRelations<'a>,
) -> Inputs<'a, 'd> {
    Inputs {
        world,
        definitions: Some(definitions),
        relations: Some(relations),
        plot_unit_limit: Some(2),
    }
}
