//! Bounded Linux geographic discovery kernels (XP2 a0b6611f).
//! Native naming changes retained Map state and consumes the game RNG. Callers
//! supply resolved current rules, ever-alive major civilizations and that stream.
//! This module does not implement the surrounding visibility action or turns.
use crate::map::{
    self, FeatureComponent, GeographicalTerritory, LocalizedText, NamedTerritory, RiverComponent,
    TerrainComponent, Territory,
};

#[derive(Clone, Debug)]
pub struct NameDefinition {
    pub hash: u32,
    pub collection_index: i32,
    pub name: LocalizedText,
    /// Resolved references, in definition-reference order; missing civs omitted.
    pub civilizations: Vec<u32>,
}
#[derive(Clone, Copy, Debug)]
pub struct CivilizationName {
    pub civilization: Option<u32>,
    pub name_hash: Option<u32>,
}
#[derive(Clone, Debug, Default)]
pub struct NameRules {
    pub definitions: Vec<NameDefinition>,
    /// Native Named*Civilizations collection order (duplicates remain weighted).
    pub civilization_names: Vec<CivilizationName>,
}
impl NameRules {
    fn find(&self, hash: u32) -> Option<&NameDefinition> {
        self.definitions.iter().rev().find(|d| d.hash == hash)
    }
    /// Choose*Name2dd3f30/2de7260/2df0820..1020. Exactly one u16-bound
    /// game draw for a nonempty candidate vector, including a single candidate.
    pub fn choose<E>(
        &self,
        civilization: u32,
        used: impl Fn(u32, i32) -> bool,
        ever_alive: &[u32],
        draw: &mut impl FnMut(u16) -> Result<u16, E>,
    ) -> Result<Option<&NameDefinition>, E> {
        let mut candidates: Vec<_> = self
            .civilization_names
            .iter()
            .filter(|a| a.civilization == Some(civilization))
            .filter_map(|a| a.name_hash.and_then(|h| self.find(h)))
            .filter(|d| !used(d.hash, d.collection_index))
            .collect();
        if candidates.is_empty() {
            candidates.extend(self.definitions.iter().filter(|d| {
                !used(d.hash, d.collection_index)
                    && d.civilizations
                        .iter()
                        .all(|c| *c == civilization || !ever_alive.contains(c))
            }));
        }
        if candidates.is_empty() {
            Ok(None)
        } else {
            Ok(Some(
                candidates[usize::from(draw(candidates.len() as u16)?)],
            ))
        }
    }
}

/// DiscoverVolcano2dd3eb0 scans ordinary volcanoes only; natural-wonder
/// volcanoes are not touched. An already-discovered matching plot returns early.
pub fn discover_volcano<E>(
    manager: &mut FeatureComponent,
    plot: i32,
    civilization: u32,
    rules: &NameRules,
    ever_alive: &[u32],
    draw: &mut impl FnMut(u16) -> Result<u16, E>,
) -> Result<(), E> {
    for index in 0..manager.volcanoes.len() {
        if manager.volcanoes[index].plot != plot {
            continue;
        }
        if manager.volcanoes[index].discovering_civilization_hash != 0 {
            break;
        }
        manager.volcanoes[index].discovering_civilization_hash = civilization;
        let chosen = rules.choose(
            civilization,
            |hash, _| manager.volcanoes.iter().any(|v| v.name_hash == hash),
            ever_alive,
            draw,
        )?;
        manager.volcanoes[index].name_hash = chosen.map_or(u32::MAX, |d| d.hash);
    }
    Ok(())
}

/// DiscoverRiver2de71d0 uses sparse plot river-ID order. Failed name selection
/// leaves type -1, so a later discovery may retry even when a civ was recorded.
pub fn discover_river<E>(
    manager: &mut RiverComponent,
    plot: i32,
    civilization: u32,
    rules: &NameRules,
    ever_alive: &[u32],
    draw: &mut impl FnMut(u16) -> Result<u16, E>,
) -> Result<(), RiverError<E>> {
    let ids = manager
        .plot_rivers
        .get(plot)
        .map_err(RiverError::Map)?
        .clone();
    for id in ids {
        let Some(index) = manager.rivers.iter().position(|r| r.id == id) else {
            continue;
        };
        if manager.rivers[index].discovering_civilization_hash != 0
            && manager.rivers[index].name_hash != u32::MAX
        {
            continue;
        }
        manager.rivers[index].discovering_civilization_hash = civilization;
        let chosen = rules
            .choose(
                civilization,
                |hash, _| manager.rivers.iter().any(|r| r.name_hash == hash),
                ever_alive,
                draw,
            )
            .map_err(RiverError::Random)?;
        manager.rivers[index].name_hash = chosen.map_or(u32::MAX, |d| d.hash);
    }
    Ok(())
}

#[derive(Debug)]
pub enum RiverError<E> {
    Map(map::Error),
    Random(E),
}

#[derive(Clone, Debug)]
pub struct TerritoryRules {
    pub mountains: NameRules,
    pub deserts: NameRules,
    pub lakes: NameRules,
    pub seas: NameRules,
    pub oceans: NameRules,
}
#[derive(Clone, Copy, Debug)]
pub struct LabelThresholds {
    pub mountain: i32,
    pub desert: i32,
    pub lake: i32,
    pub sea: i32,
    pub ocean: i32,
}
/// DiscoverTerritory2df0340. None means absent Map/Maps_XP2 definition and
/// produces the native early return. The visibility caller owns its empty-name
/// guard. Non-label classes are supported early returns, not missing mechanics.
#[allow(clippy::too_many_arguments)]
pub fn discover_territory<E>(
    manager: &mut TerrainComponent,
    territory: &mut Territory,
    civilization: u32,
    rules: &TerritoryRules,
    thresholds: Option<LabelThresholds>,
    ever_alive: &[u32],
    draw: &mut impl FnMut(u16) -> Result<u16, E>,
) -> Result<(), E> {
    let Some(thresholds) = thresholds else {
        return Ok(());
    };
    let (names, threshold) = match territory.terrain_class_hash {
        0x08ce9142 => (&rules.mountains, thresholds.mountain),
        0x6f69cd12 => (&rules.deserts, thresholds.desert),
        0x74d79f02 if territory.is_lake => (&rules.lakes, thresholds.lake),
        0x74d79f02 if territory.is_sea => (&rules.seas, thresholds.sea),
        0x74d79f02 => (&rules.oceans, thresholds.ocean),
        _ => return Ok(()),
    };
    if territory.plot_count < threshold {
        return Ok(());
    }
    let chosen = names.choose(
        civilization,
        |hash, index| match territory.terrain_class_hash {
            0x08ce9142 => manager.mountain_ranges.iter().any(|v| v.type_hash == hash),
            0x6f69cd12 => manager.deserts.iter().any(|v| v.type_hash == hash),
            _ if territory.is_lake => manager.named_lakes.iter().any(|v| v.kind == index),
            _ if territory.is_sea => manager.named_seas.iter().any(|v| v.kind == index),
            _ => manager.named_oceans.iter().any(|v| v.kind == index),
        },
        ever_alive,
        draw,
    )?;
    let Some(chosen) = chosen else {
        return Ok(());
    };
    let geographical = GeographicalTerritory {
        type_hash: chosen.hash,
        discovering_civilization_hash: civilization,
        territory_id: territory.id,
        custom_name: vec![],
    };
    let named = NamedTerritory {
        kind: chosen.collection_index,
        discovering_civilization_hash: civilization,
        territory_id: territory.id,
        custom_name: vec![],
    };
    match territory.terrain_class_hash {
        0x08ce9142 => manager.mountain_ranges.push(geographical),
        0x6f69cd12 => manager.deserts.push(geographical),
        _ if territory.is_lake => manager.named_lakes.push(named),
        _ if territory.is_sea => manager.named_seas.push(named),
        _ => manager.named_oceans.push(named),
    }
    territory.name = chosen.name.clone();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn definition(hash: u32, collection_index: i32, civilizations: Vec<u32>) -> NameDefinition {
        NameDefinition {
            hash,
            collection_index,
            civilizations,
            name: LocalizedText {
                base: vec![],
                arguments: vec![],
                named_arguments: vec![],
            },
        }
    }
    #[test]
    fn discovery_retains_civilization_when_rng_is_unavailable() {
        use crate::map::{SparsePlotData, Volcano};
        let mut manager = FeatureComponent {
            extra_yields: SparsePlotData {
                default: vec![],
                indices: vec![],
                values: vec![],
            },
            volcanoes: vec![Volcano {
                name_hash: u32::MAX,
                plot: 7,
                discovering_civilization_hash: 0,
                turn_active: -1,
                custom_name: vec![],
                eruptions: vec![],
                affected_plots: vec![],
                periods_raw: vec![],
                chances_raw: vec![],
            }],
            natural_wonder_volcanoes: vec![],
            plot_volcanoes: SparsePlotData {
                default: vec![],
                indices: vec![],
                values: vec![],
            },
            ice: vec![],
        };
        let rules = NameRules {
            definitions: vec![definition(10, 0, vec![])],
            civilization_names: vec![],
        };
        assert_eq!(
            discover_volcano(&mut manager, 7, 99, &rules, &[], &mut |_| Err("unbound")),
            Err("unbound")
        );
        assert_eq!(manager.volcanoes[0].discovering_civilization_hash, 99);
        assert_eq!(manager.volcanoes[0].name_hash, u32::MAX);
        // The native already-discovered volcano guard does not retry name selection.
        assert_eq!(
            discover_volcano(&mut manager, 7, 100, &rules, &[], &mut |_| Err(
                "must not draw"
            )),
            Ok(())
        );
    }
    #[test]
    fn native_candidate_order_fallback_and_singleton_draw() {
        let mut rules = NameRules {
            definitions: vec![
                definition(10, 0, vec![2]),
                definition(11, 1, vec![1]),
                definition(12, 2, vec![3]),
            ],
            civilization_names: vec![CivilizationName {
                civilization: Some(1),
                name_hash: Some(11),
            }],
        };
        let mut bounds = vec![];
        let mut draw = |b| {
            bounds.push(b);
            Ok::<_, ()>(0)
        };
        assert_eq!(
            rules
                .choose(1, |_, _| false, &[1, 2], &mut draw)
                .unwrap()
                .unwrap()
                .hash,
            11
        );
        rules.civilization_names.clear();
        assert_eq!(
            rules
                .choose(1, |h, _| h == 11, &[1, 2], &mut draw)
                .unwrap()
                .unwrap()
                .hash,
            12
        );
        assert!(rules
            .choose(1, |_, _| true, &[1, 2], &mut draw)
            .unwrap()
            .is_none());
        assert_eq!(bounds, [1, 1]);
    }
}
