//! Linux Religion::CreateRelic and Game::Culture::AddRelic state changes.
//! Runtime definition order, complete GreatWork state and the actual Game RNG
//! are required. Reporting, theming and synchronous signals retain required
//! providers; this is not complete action/turn equivalence.
use crate::city_production::Buildings;
use crate::rng::RandomState;
use crate::unit::{ComponentId, PlotCoord};

pub const GOVERNOR_SOURCE: u32 = 0xb8cd_57a7;
pub const NATURAL_WONDER_SOURCE: u32 = 0x28c7_ddf5;
pub const CREATED_EVENT: u32 = 0x1a71_e9f0;
pub const RELIC_NOTIFICATION: u32 = 0xae27_8cfa;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GreatWork {
    pub work_type: i32,
    pub player: i32,
    pub created_turn: i32,
    pub creator_name: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Governor {
    pub index: i32,
    pub name: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkDefinition {
    pub index: i32,
    pub object_type: i32,
    /// Resolved GreatWork_MODE then required Governor definition, if both exist.
    pub required_governor: Option<Governor>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Building {
    pub index: i32,
    pub hash: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rules {
    /// Actual runtime GREATWORKOBJECT_RELIC enum value.
    pub relic_object_type: i32,
    pub works: Vec<WorkDefinition>,
    /// None means the actual object definition is absent. Null references in
    /// its native BuildingCollection remain None and are skipped in place.
    pub relic_buildings: Option<Vec<Option<Building>>>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Slot<C> {
    pub city: C,
    pub building: Option<Building>,
    pub index: i32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RelicNotification<C> {
    pub city: C,
    pub player: i32,
    pub location: PlotCoord,
    pub building: Option<Building>,
    pub global_work_index: i32,
    pub work_type: i32,
}
impl<C> RelicNotification<C> {
    /// Original TypedVariantVectorMap insertion order and keys.
    pub fn variants(&self) -> [(u32, i32); 4] {
        [
            (0xa815_4907, self.location.x),
            (0xdf12_7991, self.location.y),
            (0x7e90_2faf, self.building.map_or(-1, |b| b.index)),
            (0x8f3d_7aef, self.global_work_index),
        ]
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CreatedReport {
    pub event_type: u32,
    pub source: ComponentId,
    pub location: PlotCoord,
    pub building: i32,
    pub global_work_index: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error<E> {
    Context(E),
    MissingDefinition(i32),
    InvalidRandomIndex,
    TooManyGreatWorks,
}

pub trait Context {
    type Error;
    type City: Copy;
    fn preflight_relic(&self, player: i32, source: u32) -> Result<(), Self::Error>;
    fn rules(&self) -> &Rules;
    fn great_works(&self) -> &[GreatWork];
    fn great_works_mut(&mut self) -> &mut Vec<GreatWork>;
    /// Actual live Game::Instance Random at wrapper+0xe8, with all logical
    /// fields retained. Other RNG streams may not be substituted here.
    fn game_random_mut(&mut self) -> &mut RandomState;
    fn current_turn(&mut self) -> Result<i32, Self::Error>;
    fn city_count(&mut self, player: i32) -> Result<usize, Self::Error>;
    fn city_at(&mut self, player: i32, index: usize) -> Result<Self::City, Self::Error>;
    fn city_owner(&mut self, city: Self::City) -> Result<i32, Self::Error>;
    fn city_location(&mut self, city: Self::City) -> Result<PlotCoord, Self::Error>;
    fn buildings(&self, city: Self::City) -> Result<&Buildings, Self::Error>;
    fn buildings_mut(&mut self, city: Self::City) -> Result<&mut Buildings, Self::Error>;
    /// Actual applied runtime building location array, not absent save pairs.
    fn building_location(
        &mut self,
        city: Self::City,
        building: Building,
    ) -> Result<i32, Self::Error>;
    fn plot_xy(&mut self, index: i32) -> Result<PlotCoord, Self::Error>;
    fn plot_distance(&mut self, first: PlotCoord, second: PlotCoord) -> Result<i32, Self::Error>;
    fn has_governor(&mut self, player: i32, governor: i32) -> Result<bool, Self::Error>;
    /// Full CheckForThemedBuilding after the slot write, including callbacks.
    fn check_themed(&mut self, city: Self::City, building: Building) -> Result<(), Self::Error>;
    /// GameSignals+0x1260 dispatch after slot/theming effects.
    fn signal_relic(&mut self, player: i32, work_type: i32) -> Result<(), Self::Error>;
    /// Full Notification::Send(RELIC_NOTIFICATION) with x/y/building/global-
    /// work-index variants (event.variants() retains native insertion order).
    fn notify_relic(&mut self, event: RelicNotification<Self::City>) -> Result<(), Self::Error>;
    fn report_created(&mut self, event: CreatedReport) -> Result<(), Self::Error>;
}

pub fn free_slot(buildings: &Buildings, building_hash: u32) -> i32 {
    for building in &buildings.great_work_buildings {
        if building.building_hash == building_hash {
            if let Some(index) = building.slots.iter().position(|s| s.great_work_index == -1) {
                return index as i32;
            }
        }
    }
    -1
}

/// Concrete native AddGreatWorkToSlot write. A negative requested slot selects
/// the first empty slot; it does not reject an occupied nonnegative slot.
pub fn assign_slot(buildings: &mut Buildings, building_hash: u32, slot: i32, work: i32) -> bool {
    for building in &mut buildings.great_work_buildings {
        if building.building_hash != building_hash {
            continue;
        }
        let slot = if slot < 0 {
            let Some(index) = building.slots.iter().position(|s| s.great_work_index == -1) else {
                return false;
            };
            index
        } else {
            slot as usize
        };
        let Some(entry) = building.slots.get_mut(slot) else {
            return false;
        };
        entry.great_work_index = work;
        return true;
    }
    false
}

fn work_in_slot(buildings: &Buildings, building_hash: u32, slot: i32) -> i32 {
    buildings
        .great_work_buildings
        .iter()
        .find(|b| b.building_hash == building_hash)
        .and_then(|b| b.slots.get(slot as usize))
        .map_or(-1, |s| s.great_work_index)
}

pub fn nearest_slot<C: Context>(
    c: &mut C,
    player: i32,
    origin: PlotCoord,
) -> Result<Option<Slot<C::City>>, C::Error> {
    let Some(buildings) = c.rules().relic_buildings.clone() else {
        return Ok(None);
    };
    let mut nearest = None;
    let mut distance = i32::MAX;
    for building in buildings.into_iter().flatten() {
        let mut index = 0;
        while index < c.city_count(player)? {
            let city = c.city_at(player, index)?;
            index += 1;
            if building.index == -1 {
                continue;
            }
            let location = c.building_location(city, building)?;
            if location < 0 {
                continue;
            }
            let slot = free_slot(c.buildings(city)?, building.hash);
            if slot == -1 {
                continue;
            }
            let location = c.building_location(city, building)?;
            let location = c.plot_xy(location)?;
            // Native invalid-origin shortcut returns the current city without
            // assigning outparams, which CreateRelic initialized to -1.
            if !crate::geometry::plot_valid(origin.x, origin.y) {
                return Ok(Some(Slot {
                    city,
                    building: None,
                    index: -1,
                }));
            }
            let current = c.plot_distance(location, origin)?;
            if current < distance {
                nearest = Some(Slot {
                    city,
                    building: Some(building),
                    index: slot,
                });
            }
            distance = distance.min(current);
        }
    }
    Ok(nearest)
}

fn creator_name(source: u32, governor: Option<&Governor>) -> Vec<u8> {
    match source {
        0x6e49_0dd0 => b"LOC_RELIGION_MARTYRED_APOSTLE".to_vec(),
        0xb9d4_96f5 => b"LOC_RELIC_TRIBAL_VILLAGE".to_vec(),
        0xe914_de96 => b"LOC_RELIC_CITY_STATE".to_vec(),
        NATURAL_WONDER_SOURCE => b"LOC_NATURAL_WONDER_NAME".to_vec(),
        GOVERNOR_SOURCE => {
            governor.map_or_else(|| b"LOC_RELIC_GOVERNOR".to_vec(), |g| g.name.clone())
        }
        _ => Vec::new(),
    }
}

/// Game::Culture::AddRelic returns the selected GreatWorkTypes enum, not its
/// appended global list index. Empty eligible sets consume no RNG draw.
pub fn add_relic<C: Context>(
    c: &mut C,
    city: C::City,
    building: Option<Building>,
    slot: i32,
    source: u32,
) -> Result<i32, Error<C::Error>> {
    use Error::Context;
    let player = c.city_owner(city).map_err(Context)?;
    c.preflight_relic(player, source).map_err(Context)?;
    let rules = c.rules().clone();
    let mut candidates = Vec::new();
    for definition in &rules.works {
        if definition.object_type != rules.relic_object_type
            || c.great_works()
                .iter()
                .any(|w| w.work_type == definition.index)
        {
            continue;
        }
        if let Some(governor) = &definition.required_governor {
            if source != GOVERNOR_SOURCE
                || !c.has_governor(player, governor.index).map_err(Context)?
            {
                continue;
            }
        } else if source == GOVERNOR_SOURCE {
            continue;
        }
        candidates.push(definition.index);
    }
    if candidates.is_empty() {
        return Ok(-1);
    }
    if c.great_works().len() >= i32::MAX as usize {
        return Err(Error::TooManyGreatWorks);
    }
    // WeightedVector::ChooseAtRandom ignores weights in this overload and
    // draws a uniform index from its u16-truncated size, including bound zero.
    let choice = c.game_random_mut().draw(candidates.len() as u16) as usize;
    let work_type = *candidates.get(choice).ok_or(Error::InvalidRandomIndex)?;
    let definition = rules
        .works
        .iter()
        .find(|d| d.index == work_type)
        .ok_or(Error::MissingDefinition(work_type))?;
    let creator_name = creator_name(source, definition.required_governor.as_ref());
    let player = c.city_owner(city).map_err(Context)?;
    let turn = c.current_turn().map_err(Context)?;
    c.great_works_mut().push(GreatWork {
        work_type,
        player,
        created_turn: turn,
        creator_name,
    });
    let list_index = (c.great_works().len() as i32).wrapping_sub(1);
    if let Some(building) = building {
        if assign_slot(
            c.buildings_mut(city).map_err(Context)?,
            building.hash,
            slot,
            list_index,
        ) {
            c.check_themed(city, building).map_err(Context)?;
        }
    }
    let player = c.city_owner(city).map_err(Context)?;
    c.signal_relic(player, work_type).map_err(Context)?;
    // Notification variants retain the list index calculated before callbacks.
    // Native city owner and location are queried again after signal dispatch.
    let player = c.city_owner(city).map_err(Context)?;
    let location = c.city_location(city).map_err(Context)?;
    c.notify_relic(RelicNotification {
        city,
        player,
        location,
        building,
        global_work_index: list_index,
        work_type,
    })
    .map_err(Context)?;
    Ok(work_type)
}

pub fn create_relic<C: Context>(
    c: &mut C,
    player: i32,
    origin: PlotCoord,
    source: u32,
) -> Result<(), Error<C::Error>> {
    use Error::Context;
    c.preflight_relic(player, source).map_err(Context)?;
    let Some(slot) = nearest_slot(c, player, origin).map_err(Context)? else {
        return Ok(());
    };
    if add_relic(c, slot.city, slot.building, slot.index, source)? != -1 {
        let work = if let Some(building) = slot.building {
            work_in_slot(
                c.buildings(slot.city).map_err(Context)?,
                building.hash,
                slot.index,
            )
        } else {
            -1
        };
        let location = c.city_location(slot.city).map_err(Context)?;
        c.report_created(CreatedReport {
            event_type: CREATED_EVENT,
            source: ComponentId {
                owner: player as i16,
                component_type: 1,
                id: u32::MAX,
            },
            location,
            building: slot.building.map_or(-1, |b| b.index),
            global_work_index: work,
        })
        .map_err(Context)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::city_production::{GreatWorkBuilding, GreatWorkSlot};
    struct Fixture {
        rules: Rules,
        works: Vec<GreatWork>,
        cities: Vec<Buildings>,
        rng: RandomState,
        log: Vec<String>,
        change_slot_on_theme: bool,
        governor: bool,
    }
    fn buildings() -> Buildings {
        Buildings {
            wire_version: 8,
            building_locations: vec![],
            pillaged: vec![],
            era_created: vec![],
            num_units_supported: vec![],
            yield_modifiers: vec![],
            yield_changes: vec![],
            yield_changes_city_states: vec![],
            wonder_yield_changes: vec![],
            great_work_buildings: vec![GreatWorkBuilding {
                building_hash: 123,
                slots: vec![GreatWorkSlot {
                    wire_version: 8,
                    slot_type: 3,
                    great_work_index: -1,
                }],
            }],
            extra_great_works: vec![],
            yield_change_from_features: vec![],
            building_name_overrides: vec![],
        }
    }
    impl Fixture {
        fn new() -> Self {
            Self {
                rules: Rules {
                    relic_object_type: 7,
                    works: vec![WorkDefinition {
                        index: 9,
                        object_type: 7,
                        required_governor: None,
                    }],
                    relic_buildings: Some(vec![Some(Building {
                        index: 4,
                        hash: 123,
                    })]),
                },
                works: vec![],
                cities: vec![buildings(), buildings()],
                rng: RandomState::new(1),
                log: vec![],
                change_slot_on_theme: false,
                governor: false,
            }
        }
    }
    impl Context for Fixture {
        type Error = &'static str;
        type City = usize;
        fn preflight_relic(&self, _: i32, _: u32) -> Result<(), Self::Error> {
            Ok(())
        }
        fn rules(&self) -> &Rules {
            &self.rules
        }
        fn great_works(&self) -> &[GreatWork] {
            &self.works
        }
        fn great_works_mut(&mut self) -> &mut Vec<GreatWork> {
            &mut self.works
        }
        fn game_random_mut(&mut self) -> &mut RandomState {
            &mut self.rng
        }
        fn current_turn(&mut self) -> Result<i32, Self::Error> {
            Ok(23)
        }
        fn city_count(&mut self, _: i32) -> Result<usize, Self::Error> {
            Ok(self.cities.len())
        }
        fn city_at(&mut self, _: i32, i: usize) -> Result<usize, Self::Error> {
            Ok(i)
        }
        fn city_owner(&mut self, _: usize) -> Result<i32, Self::Error> {
            Ok(0)
        }
        fn city_location(&mut self, city: usize) -> Result<PlotCoord, Self::Error> {
            Ok(PlotCoord {
                x: city as i32,
                y: 2,
            })
        }
        fn buildings(&self, city: usize) -> Result<&Buildings, Self::Error> {
            Ok(&self.cities[city])
        }
        fn buildings_mut(&mut self, city: usize) -> Result<&mut Buildings, Self::Error> {
            Ok(&mut self.cities[city])
        }
        fn building_location(&mut self, city: usize, _: Building) -> Result<i32, Self::Error> {
            Ok(city as i32)
        }
        fn plot_xy(&mut self, index: i32) -> Result<PlotCoord, Self::Error> {
            Ok(PlotCoord { x: index, y: 0 })
        }
        fn plot_distance(&mut self, _: PlotCoord, _: PlotCoord) -> Result<i32, Self::Error> {
            Ok(3)
        }
        fn has_governor(&mut self, _: i32, _: i32) -> Result<bool, Self::Error> {
            Ok(self.governor)
        }
        fn check_themed(&mut self, city: usize, _: Building) -> Result<(), Self::Error> {
            assert_eq!(
                self.cities[city].great_work_buildings[0].slots[0].great_work_index,
                0
            );
            self.log.push("theme".into());
            if self.change_slot_on_theme {
                self.cities[city].great_work_buildings[0].slots[0].great_work_index = 42;
            }
            Ok(())
        }
        fn signal_relic(&mut self, _: i32, kind: i32) -> Result<(), Self::Error> {
            self.log.push(format!("signal {kind}"));
            Ok(())
        }
        fn notify_relic(&mut self, e: RelicNotification<usize>) -> Result<(), Self::Error> {
            self.log
                .push(format!("notify {} {}", e.work_type, e.global_work_index));
            Ok(())
        }
        fn report_created(&mut self, e: CreatedReport) -> Result<(), Self::Error> {
            assert_eq!(e.event_type, CREATED_EVENT);
            assert_eq!(e.source.component_type, 1);
            assert_eq!(e.source.id, u32::MAX);
            self.log.push(format!("report {}", e.global_work_index));
            Ok(())
        }
    }
    #[test]
    fn single_eligible_relic_consumes_real_rng_and_mutates_work_and_slot() {
        let mut f = Fixture::new();
        let mut expected = f.rng;
        assert_eq!(expected.draw(1), 0);
        create_relic(&mut f, 0, PlotCoord { x: 1, y: 2 }, NATURAL_WONDER_SOURCE).unwrap();
        assert_eq!(f.rng, expected);
        assert_eq!(f.rng.calls, 1);
        assert_eq!(
            f.works,
            [GreatWork {
                work_type: 9,
                player: 0,
                created_turn: 23,
                creator_name: b"LOC_NATURAL_WONDER_NAME".to_vec()
            }]
        );
        assert_eq!(
            f.cities[0].great_work_buildings[0].slots[0].great_work_index,
            0
        );
        assert_eq!(
            f.cities[1].great_work_buildings[0].slots[0].great_work_index,
            -1
        );
        assert_eq!(f.log, ["theme", "signal 9", "notify 9 0", "report 0"]);
    }
    #[test]
    fn empty_candidate_set_does_not_draw_and_created_types_cannot_repeat() {
        let mut f = Fixture::new();
        let definition = GreatWork {
            work_type: 9,
            player: 1,
            created_turn: 4,
            creator_name: vec![],
        };
        f.works.push(definition);
        let before = f.rng;
        assert_eq!(
            add_relic(
                &mut f,
                0,
                Some(Building {
                    index: 4,
                    hash: 123
                }),
                0,
                NATURAL_WONDER_SOURCE
            ),
            Ok(-1)
        );
        assert_eq!(f.rng, before);
        assert_eq!(f.works.len(), 1);
        assert!(f.log.is_empty());
    }
    #[test]
    fn callback_slot_change_is_reread_for_outer_report() {
        let mut f = Fixture::new();
        f.change_slot_on_theme = true;
        create_relic(&mut f, 0, PlotCoord { x: 0, y: 0 }, NATURAL_WONDER_SOURCE).unwrap();
        assert_eq!(f.log, ["theme", "signal 9", "notify 9 0", "report 42"]);
    }
    #[test]
    fn governor_source_requires_resolved_owned_governor_and_uses_its_name() {
        let mut f = Fixture::new();
        f.rules.works[0].required_governor = Some(Governor {
            index: 3,
            name: b"LOC_GOVERNOR_NAME".to_vec(),
        });
        assert_eq!(
            add_relic(&mut f, 0, None, -1, NATURAL_WONDER_SOURCE),
            Ok(-1)
        );
        assert_eq!(add_relic(&mut f, 0, None, -1, GOVERNOR_SOURCE), Ok(-1));
        assert_eq!(f.rng.calls, 0);
        f.governor = true;
        assert_eq!(add_relic(&mut f, 0, None, -1, GOVERNOR_SOURCE), Ok(9));
        assert_eq!(f.works[0].creator_name, b"LOC_GOVERNOR_NAME");
        assert_eq!(f.rng.calls, 1);
    }
    #[test]
    fn no_actual_slot_means_no_candidate_selection_or_rng() {
        let mut f = Fixture::new();
        for city in &mut f.cities {
            city.great_work_buildings[0].slots[0].great_work_index = 99;
        }
        create_relic(&mut f, 0, PlotCoord { x: 0, y: 0 }, NATURAL_WONDER_SOURCE).unwrap();
        assert_eq!(f.rng.calls, 0);
        assert!(f.works.is_empty());
        assert!(f.log.is_empty());
    }
}
