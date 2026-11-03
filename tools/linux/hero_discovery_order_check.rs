//! Offline, synthetic-provider checks plus existing typed wire parsing.
//! The Limits shim is a transport harness type, not a gameplay implementation.
#![allow(dead_code)]
mod unit {
    #[derive(Clone, Copy, Debug)]
    pub struct Limits {
        pub max_bytes: usize,
        pub max_items: usize,
    }
}
#[path = "../../crates/c6-core/src/hero_discovery_order.rs"]
mod hero_discovery_order;
#[path = "../../crates/c6-core/src/player_cultural_identity.rs"]
mod player_cultural_identity;
#[path = "../../crates/c6-core/src/player_governors.rs"]
mod player_governors;
#[path = "../../crates/c6-core/src/player_heroes.rs"]
mod player_heroes;
#[path = "../../crates/c6-core/src/rng.rs"]
mod rng;

#[cfg(not(feature = "full-core"))]
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let bytes = std::fs::read(&args[1]).unwrap();
    let limits = unit::Limits {
        max_bytes: bytes.len(),
        max_items: bytes.len(),
    };
    let (_, governors) = player_governors::Governors::decode_prefix(&bytes, limits).unwrap();
    let (_, cultural) =
        player_cultural_identity::CulturalIdentity::decode_prefix(&bytes[governors..], limits)
            .unwrap();
    let start = governors + cultural;
    let (heroes, used) = player_heroes::Heroes::decode_prefix(&bytes[start..], limits).unwrap();
    assert_eq!(heroes.encode(limits).unwrap(), bytes[start..start + used]);
    println!(
        "{} {} {} {} {}",
        start,
        used,
        heroes.discovery_order.len(),
        heroes.discovered.len(),
        heroes.claims.len()
    );
}

#[cfg(feature = "full-core")]
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let bytes = std::fs::read(&args[1]).unwrap();
    let decoded = c6_core::core_stream::ThroughFirstPlayer::decode(
        &bytes,
        c6_core::map::Limits {
            max_bytes: bytes.len(),
            max_items: 20_000_000,
            max_allocation: 1024 * 1024 * 1024,
            max_depth: 64,
        },
        c6_core::unit::Limits {
            max_bytes: bytes.len(),
            max_items: 20_000_000,
        },
    );
    let prefix = match decoded {
        Ok(prefix) => prefix,
        Err(error) => {
            println!(
                "{{\"parsed\":false,\"unsupported_boundary\":{:?}}}",
                format!("{error:?}")
            );
            return;
        }
    };
    let player = prefix.player_prefix.first_player.as_ref().unwrap();
    let heroes = &player.heroes;
    let encoded = heroes
        .encode(c6_core::unit::Limits {
            max_bytes: bytes.len(),
            max_items: 20_000_000,
        })
        .unwrap();
    std::fs::write(&args[2], &encoded).unwrap();
    println!("{{\"parsed\":true,\"player\":{},\"header_ordered_flags\":{:?},\"discovery_order_records\":{},\"discovered_records\":{},\"claims_records\":{},\"encoded_heroes_bytes\":{},\"consumed\":{},\"boundary\":\"{:?}\"}}", player.header.owner, player.header.ordered_flags, heroes.discovery_order.len(), heroes.discovered.len(), heroes.claims.len(), encoded.len(), prefix.consumed, prefix.player_prefix.boundary);
}

#[cfg(test)]
mod checks {
    use super::hero_discovery_order::*;
    use super::player_heroes::{DiscoveredHero, DiscoveryWeight, Heroes};
    use super::rng::RandomState;
    use std::collections::BTreeSet;
    #[derive(Clone, Debug, Eq, PartialEq)]
    struct SyntheticOwner {
        random: RandomState,
        dirty: BTreeSet<u64>,
        wrapper: u64,
    }
    impl GameRandomOwner for SyntheticOwner {
        fn draw(&mut self, bound: u16) -> Result<u16, Error> {
            self.dirty.insert(self.wrapper);
            Ok(self.random.draw(bound))
        }
    }
    #[derive(Clone)]
    struct SyntheticInputs {
        definitions: Vec<HeroDefinition>,
        active: bool,
        major: bool,
        multiplayer: bool,
        human: bool,
        local: i32,
        allowance: u32,
        era: Option<i32>,
        missing_claim: bool,
        claim: ClaimInputs,
    }
    impl Inputs for SyntheticInputs {
        fn is_major(&self) -> Result<bool, Error> {
            Ok(self.major)
        }
        fn capability_present(&self, c: u32) -> Result<bool, Error> {
            assert_eq!(c, CAPABILITY);
            Ok(self.active)
        }
        fn any_multiplayer(&self) -> Result<bool, Error> {
            Ok(self.multiplayer)
        }
        fn is_human(&self) -> Result<bool, Error> {
            Ok(self.human)
        }
        fn local_player(&self) -> Result<i32, Error> {
            Ok(self.local)
        }
        fn allowance(&self, c: u32) -> Result<u32, Error> {
            assert_eq!(c, CAPABILITY);
            Ok(self.allowance)
        }
        fn current_era_chronology(&self) -> Result<Option<i32>, Error> {
            Ok(self.era)
        }
        fn ordered_hero_definitions(&self) -> Result<Vec<HeroDefinition>, Error> {
            Ok(self.definitions.clone())
        }
        fn claim_inputs(&self, _: i32) -> Result<ClaimInputs, Error> {
            if self.missing_claim {
                Err(Error::Missing("city domain eligibility"))
            } else {
                Ok(self.claim.clone())
            }
        }
    }
    fn fixture() -> (Heroes, SyntheticInputs, SyntheticOwner) {
        let heroes = Heroes {
            discovered: vec![],
            goody_hut_discovery_chance: 0,
            player_influence_chances: vec![],
            player_suzerain_count: vec![],
            discovery_order: vec![DiscoveryWeight {
                hero_class_hash: 77,
                weight: 55,
            }],
            claims: vec![],
        };
        let inputs = SyntheticInputs {
            definitions: vec![HeroDefinition {
                native_index: 9,
                type_hash: 10,
                discovery_min_era_chronology: None,
            }],
            active: true,
            major: true,
            multiplayer: false,
            human: true,
            local: 0,
            allowance: 1,
            era: Some(5),
            missing_claim: false,
            claim: ClaimInputs {
                valid_class: true,
                claim_player: -1,
                creation_project_exists: true,
                unit_domain: Some(2),
                prerequisite_district_exists: false,
                city_queries: vec![],
            },
        };
        let owner = SyntheticOwner {
            random: RandomState {
                seed: 0,
                calls: u32::MAX,
                resets: 7,
                lock_count: 8,
                synchronous: true,
            },
            dirty: BTreeSet::from([300]),
            wrapper: 200,
        };
        (heroes, inputs, owner)
    }
    #[test]
    fn draw_plan_preserves_inputs_and_complete_synthetic_owner() {
        let (h, p, r) = fixture();
        let before_h = h.clone();
        let before_r = r.clone();
        let out = plan_rebuild(&h, &p, &r).unwrap();
        assert_eq!(h, before_h);
        assert_eq!(r, before_r);
        assert_eq!(
            out.discovery_order,
            vec![DiscoveryWeight {
                hero_class_hash: 10,
                weight: 90
            }]
        );
        assert_eq!(
            out.random_after.random,
            RandomState {
                seed: 12345,
                calls: 0,
                ..r.random
            }
        );
        assert_eq!(out.random_after.dirty, BTreeSet::from([200, 300]));
        assert_eq!(
            out.effects,
            vec![
                Effect::EditAndClearDiscoveryOrder,
                Effect::EditGameRandom,
                Effect::GameRandomDraw {
                    native_index: 9,
                    bound: 20
                },
                Effect::Append {
                    native_index: 9,
                    type_hash: 10,
                    weight: 90
                },
                Effect::SortDescending,
                Effect::LogRebuiltOrder
            ]
        );
    }
    #[test]
    fn zero_draw_gates_clear_and_archive_edit_still_planned() {
        let (h, mut p, r) = fixture();
        for gate in 0..4 {
            p.major = gate != 0;
            p.active = gate != 1;
            p.allowance = if gate == 2 { 0 } else { 1 };
            p.era = if gate == 3 { None } else { Some(5) };
            let out = plan_rebuild(&h, &p, &r).unwrap();
            assert!(out.discovery_order.is_empty());
            assert_eq!(out.random_after, r);
            assert_eq!(out.effects, vec![Effect::EditAndClearDiscoveryOrder]);
        }
    }
    #[test]
    fn discovered_claim_and_minimum_era_gates_do_not_draw() {
        let (mut h, mut p, r) = fixture();
        h.discovered.push(DiscoveredHero {
            hero_class_hash: 10,
            discovered: true,
        });
        assert!(plan_rebuild(&h, &p, &r).unwrap().discovery_order.is_empty());
        h.discovered.clear();
        for claim_gate in 0..5 {
            let mut input = p.clone();
            match claim_gate {
                0 => input.claim.valid_class = false,
                1 => input.claim.claim_player = 0,
                2 => input.claim.creation_project_exists = false,
                3 => input.claim.unit_domain = None,
                _ => input.claim.prerequisite_district_exists = true,
            }
            let out = plan_rebuild(&h, &input, &r).unwrap();
            assert_eq!(out.random_after, r);
            assert_eq!(out.discovery_order[0].weight, 0);
        }
        p.definitions[0].discovery_min_era_chronology = Some(6);
        assert_eq!(plan_rebuild(&h, &p, &r).unwrap().random_after, r);
        p.definitions[0].discovery_min_era_chronology = Some(5);
        assert_ne!(plan_rebuild(&h, &p, &r).unwrap().random_after, r);
    }
    #[test]
    fn city_witnesses_can_be_in_different_cities() {
        let (_, mut p, _) = fixture();
        p.claim.prerequisite_district_exists = true;
        p.claim.unit_domain = Some(0);
        p.claim.city_queries = vec![
            CityClaimQueries {
                has_prerequisite_district: true,
                can_build_unit_domain: false,
            },
            CityClaimQueries {
                has_prerequisite_district: false,
                can_build_unit_domain: true,
            },
        ];
        assert!(p.claim.could_claim_now());
        p.claim.city_queries.pop();
        assert!(!p.claim.could_claim_now());
    }
    #[test]
    fn allowances_match_native_unsigned_arithmetic() {
        let (h, mut p, r) = fixture();
        for multiplayer in [false, true] {
            for human in [false, true] {
                for allowance in [0, 1, 2, 3, u32::MAX] {
                    p.multiplayer = multiplayer;
                    p.human = human;
                    p.allowance = allowance;
                    let allowed = if multiplayer {
                        !human || [1, 2].contains(&allowance)
                    } else {
                        allowance == 1
                    };
                    assert_eq!(
                        plan_rebuild(&h, &p, &r).unwrap().discovery_order.len(),
                        usize::from(allowed)
                    );
                }
            }
        }
        p.multiplayer = false;
        p.local = -1;
        p.allowance = 0;
        assert_eq!(plan_rebuild(&h, &p, &r).unwrap().discovery_order.len(), 1);
    }
    #[test]
    fn bounded_ties_keep_native_registry_order_and_unknowns_are_atomic() {
        let (h, mut p, r) = fixture();
        p.claim.valid_class = false;
        p.definitions = (0..28)
            .map(|i| HeroDefinition {
                native_index: i,
                type_hash: 100 - i as u32,
                discovery_min_era_chronology: None,
            })
            .collect();
        let out = plan_rebuild(&h, &p, &r).unwrap();
        assert_eq!(
            out.discovery_order
                .iter()
                .map(|x| x.hero_class_hash)
                .collect::<Vec<_>>(),
            (0..28).map(|i| 100 - i).collect::<Vec<_>>()
        );
        assert_eq!(out.random_after, r);
        p.definitions.push(HeroDefinition {
            native_index: 28,
            type_hash: 72,
            discovery_min_era_chronology: None,
        });
        assert!(matches!(
            plan_rebuild(&h, &p, &r),
            Err(Error::SortUnsupported { records: 29 })
        ));
        p.definitions.truncate(1);
        p.missing_claim = true;
        assert_eq!(
            plan_rebuild(&h, &p, &r),
            Err(Error::Missing("city domain eligibility"))
        );
        p.missing_claim = false;
        p.definitions.push(p.definitions[0]);
        assert_eq!(
            plan_rebuild(&h, &p, &r),
            Err(Error::InvalidDefinitionIdentity)
        );
        assert_eq!(r.random.calls, u32::MAX);
        assert_eq!(h.discovery_order[0].weight, 55);
    }
    #[test]
    fn existing_order_validate_false_is_preserved() {
        let (mut h, _, _) = fixture();
        assert!(!needs_rebuild(&h, false));
        assert!(needs_rebuild(&h, true));
        h.discovery_order.clear();
        assert!(needs_rebuild(&h, false));
    }

    #[test]
    fn commit_changes_actual_owned_heroes_and_game_random_and_failure_is_atomic() {
        let (mut h, mut p, r) = fixture();
        let untouched = h.clone();
        let mut random = r.random;
        p.missing_claim = true;
        assert!(rebuild(&mut h, &p, &mut random).is_err());
        assert_eq!(h, untouched);
        assert_eq!(random, r.random);
        p.missing_claim = false;
        rebuild(&mut h, &p, &mut random).unwrap();
        assert_eq!(h.discovery_order[0].weight, 90);
        assert_eq!(random.seed, 12345);
        assert_eq!(random.calls, 0);
        assert_eq!(random.lock_count, 8);
        assert_eq!(random.resets, 7);
        assert!(random.synchronous);
        h.discovery_order = untouched.discovery_order.clone();
        assert_eq!(
            Heroes {
                discovery_order: untouched.discovery_order.clone(),
                ..h
            },
            untouched
        );
    }
}
