//! Linux District constructor/Initialize291e020 and SetComplete291e780.
//! These operate on the canonical District record. Native archive bookkeeping
//! is omitted; completion subscribers and founding rewards remain mandatory.
use crate::{
    player_districts::District,
    unit::{variants::TypedVariantMap, HashValue},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    ResourceLimit,
}

/// Actual admitted native definition order, including zero-valued cells.
#[derive(Clone, Copy)]
pub struct Definitions<'a> {
    pub yields: &'a [u32],
    pub great_person_classes: &'a [u32],
}
fn zeroes(hashes: &[u32]) -> Result<Vec<HashValue<i32>>, Error> {
    let mut values = Vec::new();
    values
        .try_reserve(hashes.len())
        .map_err(|_| Error::ResourceLimit)?;
    values.extend(hashes.iter().map(|&hash| HashValue {
        definition_hash: hash,
        value: 0,
    }));
    Ok(values)
}
/// The native recycled Add placement-constructs a District on EVERY allocation,
/// including reuse. Constructor calls Initialize before assigning ID/plot/type.
pub fn construct(definitions: Definitions<'_>) -> Result<District, Error> {
    Ok(District {
        wire_version: 15,
        id: u32::MAX,
        location: (-9999, -9999),
        city_id: u32::MAX,
        district_hash: u32::MAX,
        garrison_damage: 0,
        outer_defenses_damage: 0,
        last_turn_damaged: -100,
        remaining_attacks: 0,
        cost: 0,
        is_complete: false,
        marked_for_delete: Some(false),
        properties: TypedVariantMap::default(),
        yield_changes: zeroes(definitions.yields)?,
        base_yield_changes: zeroes(definitions.yields)?,
        yield_adjacency_modifiers: zeroes(definitions.yields)?,
        extra_regional_range: 0,
        extra_regional_entertainment: 0,
        extra_entertainment: 0,
        extra_regional_yields: zeroes(definitions.yields)?,
        tourism_adjacency_yield_modifier: zeroes(definitions.yields)?,
        tourism_change: 0,
        yield_adjacency_mirrors: Vec::new(),
        appeal_yield_changes: Some(Vec::new()),
        great_person_point_changes_q8: zeroes(definitions.great_person_classes)?,
        great_person_point_multiplier: 0,
        great_person_point_adder: 0,
        air_slots_modifier: 0,
        within_one_hex_espionage_defense_bonus: 0,
        under_siege: false,
        pillaged: false,
    })
}

pub trait CompleteContext {
    type Error;
    type City: Copy;
    fn assign_complete(&mut self, value: bool) -> Result<(), Self::Error>;
    fn game_present(&mut self) -> Result<bool, Self::Error>;
    fn city(&mut self) -> Result<Option<Self::City>, Self::Error>;
    fn plot_owner(&mut self) -> Result<i32, Self::Error>;
    fn edit_city(&mut self) -> Result<Self::City, Self::Error>;
    fn current_type_index(&mut self) -> Result<i32, Self::Error>;
    fn city_id(&mut self, city: Self::City) -> Result<u32, Self::Error>;
    fn signal_player_completed(
        &mut self,
        owner: i32,
        district_type: i32,
        city_id: u32,
    ) -> Result<(), Self::Error>;
    fn signal_game_completed(&mut self, bought: bool) -> Result<(), Self::Error>;
    fn current_type_hash(&mut self) -> Result<u32, Self::Error>;
    /// Source continuation after both signals, using CURRENT Player.Units.
    /// Implementations must execute rewards or return an explicit unsupported
    /// leaf; a successful whole-handler callback cannot replace this work.
    fn founding_rewards(
        &mut self,
        owner: i32,
        district_hash: u32,
        city: Self::City,
    ) -> Result<(), Self::Error>;
}
/// Repeated true assignments dispatch again. The completion write and successful
/// earlier callbacks remain visible if a later required leaf is unavailable.
pub fn set_complete<C: CompleteContext>(
    c: &mut C,
    complete: bool,
    bought: bool,
) -> Result<(), C::Error> {
    c.assign_complete(complete)?;
    if !complete || !c.game_present()? {
        return Ok(());
    }
    if c.city()?.is_none() {
        return Ok(());
    }
    let owner = c.plot_owner()?;
    let city = c.edit_city()?;
    let kind = c.current_type_index()?;
    let city_id = c.city_id(city)?;
    c.signal_player_completed(owner, kind, city_id)?;
    c.signal_game_completed(bought)?;
    let hash = c.current_type_hash()?;
    c.founding_rewards(owner, hash, city)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn constructor_has_real_dense_cells_and_source_sentinels() {
        let d = construct(Definitions {
            yields: &[8, 3],
            great_person_classes: &[27],
        })
        .unwrap();
        assert_eq!(
            (d.id, d.city_id, d.district_hash),
            (u32::MAX, u32::MAX, u32::MAX)
        );
        assert_eq!(d.location, (-9999, -9999));
        assert_eq!(d.last_turn_damaged, -100);
        assert_eq!(
            d.yield_changes,
            vec![
                HashValue {
                    definition_hash: 8,
                    value: 0
                },
                HashValue {
                    definition_hash: 3,
                    value: 0
                }
            ]
        );
        assert_eq!(
            d.great_person_point_changes_q8,
            vec![HashValue {
                definition_hash: 27,
                value: 0
            }]
        );
        assert!(!d.is_complete && !d.under_siege && !d.pillaged);
        assert_eq!(d.marked_for_delete, Some(false));
    }
    struct C {
        log: Vec<&'static str>,
        complete: bool,
        game: bool,
        city: bool,
        fail: bool,
    }
    impl CompleteContext for C {
        type Error = &'static str;
        type City = u32;
        fn assign_complete(&mut self, v: bool) -> Result<(), Self::Error> {
            self.complete = v;
            self.log.push("write");
            Ok(())
        }
        fn game_present(&mut self) -> Result<bool, Self::Error> {
            self.log.push("game");
            Ok(self.game)
        }
        fn city(&mut self) -> Result<Option<u32>, Self::Error> {
            self.log.push("city");
            Ok(self.city.then_some(9))
        }
        fn plot_owner(&mut self) -> Result<i32, Self::Error> {
            Ok(4)
        }
        fn edit_city(&mut self) -> Result<u32, Self::Error> {
            self.log.push("edit city");
            Ok(9)
        }
        fn current_type_index(&mut self) -> Result<i32, Self::Error> {
            Ok(3)
        }
        fn city_id(&mut self, v: u32) -> Result<u32, Self::Error> {
            Ok(v)
        }
        fn signal_player_completed(&mut self, o: i32, t: i32, id: u32) -> Result<(), Self::Error> {
            assert_eq!((o, t, id), (4, 3, 9));
            assert!(self.complete);
            self.log.push("player");
            Ok(())
        }
        fn signal_game_completed(&mut self, b: bool) -> Result<(), Self::Error> {
            assert!(!b);
            self.log.push("game signal");
            if self.fail {
                Err("subscriber")
            } else {
                Ok(())
            }
        }
        fn current_type_hash(&mut self) -> Result<u32, Self::Error> {
            self.log.push("reread type");
            Ok(17)
        }
        fn founding_rewards(&mut self, o: i32, h: u32, c: u32) -> Result<(), Self::Error> {
            assert_eq!((o, h, c), (4, 17, 9));
            self.log.push("rewards");
            Ok(())
        }
    }
    fn context() -> C {
        C {
            log: vec![],
            complete: false,
            game: true,
            city: true,
            fail: false,
        }
    }
    #[test]
    fn completion_order_repeats_and_preserves_prefix_on_error() {
        let mut c = context();
        set_complete(&mut c, true, false).unwrap();
        assert_eq!(
            c.log,
            [
                "write",
                "game",
                "city",
                "edit city",
                "player",
                "game signal",
                "reread type",
                "rewards"
            ]
        );
        c.log.clear();
        c.fail = true;
        assert_eq!(set_complete(&mut c, true, false), Err("subscriber"));
        assert!(c.complete);
        assert_eq!(
            c.log,
            [
                "write",
                "game",
                "city",
                "edit city",
                "player",
                "game signal"
            ]
        );
    }
    #[test]
    fn false_and_missing_game_or_city_return_in_source_order() {
        let mut c = context();
        set_complete(&mut c, false, false).unwrap();
        assert_eq!(c.log, ["write"]);
        c.log.clear();
        c.game = false;
        set_complete(&mut c, true, false).unwrap();
        assert_eq!(c.log, ["write", "game"]);
        c.log.clear();
        c.game = true;
        c.city = false;
        set_complete(&mut c, true, false).unwrap();
        assert_eq!(c.log, ["write", "game", "city"]);
    }
}
