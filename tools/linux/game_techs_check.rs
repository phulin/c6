//! Standalone std-only check of the exact owner source file. The two shims are
//! API-shaped test dependencies, not native definition data or a full Core.
mod unit {
    #[derive(Clone, Copy)]
    pub struct Limits {
        pub max_bytes: usize,
        pub max_items: usize,
    }
}
mod movement_world {
    pub trait DefinitionLookup {
        fn count(&self) -> usize;
        fn find_index(&self, index_or_hash: u32) -> Option<usize>;
        fn type_hash(&self, index: usize) -> Option<u32>;
    }
}
#[path = "../../crates/c6-core/src/game_techs.rs"]
mod game_techs;
use game_techs::{Error, Techs};
use unit::Limits;
struct Registry;
impl movement_world::DefinitionLookup for Registry {
    fn count(&self) -> usize {
        3
    }
    fn find_index(&self, value: u32) -> Option<usize> {
        match value {
            0..=2 => Some(value as usize),
            100..=102 => Some((value - 100) as usize),
            _ => None,
        }
    }
    fn type_hash(&self, index: usize) -> Option<u32> {
        (index < 3).then_some(index as u32 + 100)
    }
}
struct BadValidationRegistry;
impl movement_world::DefinitionLookup for BadValidationRegistry {
    fn count(&self) -> usize {
        3
    }
    fn find_index(&self, value: u32) -> Option<usize> {
        match value {
            100..=102 => Some((value - 100) as usize),
            0..=2 => Some(3),
            _ => None,
        }
    }
    fn type_hash(&self, _: usize) -> Option<u32> {
        None
    }
}
fn limits() -> Limits {
    Limits {
        max_bytes: 1_000_000,
        max_items: 10_000,
    }
}
fn synthetic() -> Vec<u8> {
    let mut b = Vec::new();
    fn word(b: &mut Vec<u8>, v: u32) {
        b.extend_from_slice(&v.to_le_bytes());
    }
    word(&mut b, 3);
    word(&mut b, 4);
    // Duplicate type 1 selects positional second record, retaining stable order.
    for (hash, active, prereqs) in [
        (101, 0, vec![102, 999, 102]),
        (101, 1, vec![100]),
        (999, 1, vec![]),
        (100, 1, vec![]),
    ] {
        word(&mut b, 3);
        word(&mut b, hash);
        b.push(active);
        word(&mut b, 0x80000000);
        word(&mut b, prereqs.len() as u32);
        for p in prereqs {
            word(&mut b, p);
        }
    }
    word(&mut b, 4);
    for p in [102, 999, 100, 102] {
        word(&mut b, p);
    }
    b
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        movement_world::DefinitionLookup::type_hash(&Registry, 1),
        Some(101)
    );
    let b = synthetic();
    let (_, used, items) = Techs::decode_prefix_with_items(&b, limits())?;
    assert_eq!(used, b.len());
    assert_eq!(items, 12);
    let graph = Techs::decode(&b, limits())?
        .project_read(&Registry)?
        .post_load_without_rebuild()?;
    assert_eq!(graph.nodes().len(), 3);
    assert_eq!(graph.iteration(), [2, 0, 2]);
    assert!(graph.has_tech(1));
    assert!(graph.has_tech(0));
    assert_eq!(
        Techs::decode(&b, limits())?.project_read(&BadValidationRegistry),
        Err(Error::DefinitionMapping)
    );
    assert!(!graph.has_tech(-1));
    assert!(!graph.has_tech(2));
    assert_eq!(graph.prerequisites(1)?, [0]);
    assert_eq!(graph.prerequisites(99), Err(Error::FallbackUnknown));
    assert_eq!(graph.prerequisites(-1), Err(Error::FallbackUnknown));
    assert_eq!(graph.nodes()[0].prerequisites, [2, 2]);
    assert_eq!(graph.nodes()[0].cost_raw, i32::MIN);
    for n in 0..b.len() {
        assert!(Techs::decode(&b[..n], limits()).is_err());
    }
    let exact = Limits {
        max_bytes: b.len(),
        max_items: items,
    };
    assert!(Techs::decode(&b, exact).is_ok());
    assert_eq!(
        Techs::decode(
            &b,
            Limits {
                max_bytes: b.len() - 1,
                ..exact
            }
        ),
        Err(Error::ResourceLimit)
    );
    assert_eq!(
        Techs::decode(
            &b,
            Limits {
                max_items: items - 1,
                ..exact
            }
        ),
        Err(Error::ResourceLimit)
    );
    let mut bad = b.clone();
    bad.push(0);
    assert_eq!(Techs::decode(&bad, limits()), Err(Error::Trailing));
    bad = b.clone();
    bad[16] = 2;
    assert_eq!(Techs::decode(&bad, limits()), Err(Error::InvalidBool(2)));
    bad = b.clone();
    bad[0] = 2;
    assert_eq!(Techs::decode(&bad, limits()), Err(Error::Version(2)));
    for (nodes, iteration_hashes) in [
        (vec![], vec![100]),
        (
            vec![game_techs::WireNode {
                type_hash: 999,
                active: true,
                cost_raw: 0,
                prerequisite_hashes: vec![],
            }],
            vec![100],
        ),
    ] {
        assert_eq!(
            Techs {
                nodes,
                iteration_hashes
            }
            .project_read(&Registry)?
            .post_load_without_rebuild(),
            Err(Error::BuildTechsRequired)
        );
    }
    let mut empty_iteration = Techs::decode(&b, limits())?;
    empty_iteration.iteration_hashes = vec![999];
    assert_eq!(
        empty_iteration
            .project_read(&Registry)?
            .post_load_without_rebuild(),
        Err(Error::BuildTechsRequired)
    );
    if let Some(path) = std::env::args_os().nth(1) {
        let bytes = std::fs::read(path)?;
        let (wire, used, items) = Techs::decode_prefix_with_items(&bytes, limits())?;
        assert_eq!(used, bytes.len());
        println!(
            "{{\"nodes\":{},\"iteration\":{},\"items\":{},\"bytes\":{}}}",
            wire.nodes.len(),
            wire.iteration_hashes.len(),
            items,
            used
        );
    }
    Ok(())
}
