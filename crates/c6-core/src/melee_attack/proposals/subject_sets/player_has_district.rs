//! Linux PlayerHasDistrict predicate and DistrictComplete subscriber guards.
//! The caller supplies the current subject interface and the SAME shared-store
//! invalidation continuation; no transient requirement state is created here.
#[derive(Debug, PartialEq, Eq)]
pub enum Error<E> {
    Context(E),
    MissingPlayerInterface,
}
pub trait Context {
    type Error;
    type Interface: Copy;
    type Player: Copy;
    type District: Copy;
    fn subject_player_interface(&mut self) -> Result<Option<Self::Interface>, Self::Error>;
    fn player(&mut self, interface: Self::Interface) -> Result<Option<Self::Player>, Self::Error>;
    fn target_type(&mut self) -> Result<i32, Self::Error>;
    fn num_districts_of_type(
        &mut self,
        player: Self::Player,
        kind: i32,
        include_pillaged: bool,
        include_incomplete: bool,
    ) -> Result<i32, Self::Error>;
    fn player_id(&mut self, player: Self::Player) -> Result<i32, Self::Error>;
    fn district_owner(&mut self, district: Self::District) -> Result<i32, Self::Error>;
    fn invalidate(&mut self) -> Result<(), Self::Error>;
}
/// 0x23bcf00. Missing interface/player, invalid cached type, or nonpositive
/// current count gives raw1. Positive complete, unpillaged count gives raw2.
pub fn test<C: Context>(c: &mut C) -> Result<i32, Error<C::Error>> {
    let Some(interface) = c.subject_player_interface().map_err(Error::Context)? else {
        return Ok(1);
    };
    let Some(player) = c.player(interface).map_err(Error::Context)? else {
        return Ok(1);
    };
    let kind = c.target_type().map_err(Error::Context)?;
    if kind == -1 {
        return Ok(1);
    }
    Ok(
        if c.num_districts_of_type(player, kind, false, false)
            .map_err(Error::Context)?
            > 0
        {
            2
        } else {
            1
        },
    )
}
/// 0x2413fa0 -> 0x23bcf70. The actual lambda ignores its completion boolean;
/// it does not filter district type. A valid subject interface is required by
/// this callback, while an actually null GetPlayer result returns immediately.
pub fn on_district_completed<C: Context>(
    c: &mut C,
    district: C::District,
    _completion: bool,
) -> Result<(), Error<C::Error>> {
    let interface = c
        .subject_player_interface()
        .map_err(Error::Context)?
        .ok_or(Error::MissingPlayerInterface)?;
    let Some(player) = c.player(interface).map_err(Error::Context)? else {
        return Ok(());
    };
    let owner = c.player_id(player).map_err(Error::Context)?;
    if owner == c.district_owner(district).map_err(Error::Context)? {
        c.invalidate().map_err(Error::Context)?;
    }
    Ok(())
}
#[derive(Clone, Copy, Debug)]
pub struct DistrictCountRecord {
    pub type_index: i32,
    pub complete: bool,
    pub pillaged: bool,
}
/// 0x2ae2530's false/false path over the canonical current roster. Type remaps
/// are supplied from admitted definitions, never inferred from SQL row IDs.
pub fn count_complete_unpillaged(
    kind: i32,
    records: impl IntoIterator<Item = DistrictCountRecord>,
) -> i32 {
    let mut count = 0_i32;
    for district in records {
        if district.type_index == kind && district.complete && !district.pillaged {
            count = count.wrapping_add(1);
        }
    }
    count
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        interface: bool,
        player: bool,
        target: i32,
        count: i32,
        owner: i32,
        invalidations: usize,
        reads: Vec<&'static str>,
    }
    impl Context for Fixture {
        type Error = &'static str;
        type Interface = ();
        type Player = ();
        type District = ();
        fn subject_player_interface(&mut self) -> Result<Option<()>, Self::Error> {
            self.reads.push("interface");
            Ok(self.interface.then_some(()))
        }
        fn player(&mut self, _: ()) -> Result<Option<()>, Self::Error> {
            self.reads.push("player");
            Ok(self.player.then_some(()))
        }
        fn target_type(&mut self) -> Result<i32, Self::Error> {
            self.reads.push("type");
            Ok(self.target)
        }
        fn num_districts_of_type(
            &mut self,
            _: (),
            t: i32,
            a: bool,
            b: bool,
        ) -> Result<i32, Self::Error> {
            assert_eq!((t, a, b), (21, false, false));
            self.reads.push("count");
            Ok(self.count)
        }
        fn player_id(&mut self, _: ()) -> Result<i32, Self::Error> {
            Ok(0)
        }
        fn district_owner(&mut self, _: ()) -> Result<i32, Self::Error> {
            Ok(self.owner)
        }
        fn invalidate(&mut self) -> Result<(), Self::Error> {
            self.invalidations += 1;
            Ok(())
        }
    }
    fn fixture() -> Fixture {
        Fixture {
            interface: true,
            player: true,
            target: 21,
            count: 0,
            owner: 0,
            invalidations: 0,
            reads: vec![],
        }
    }
    #[test]
    fn completion_false_still_invalidates_matching_owner_without_type_query() {
        let mut f = fixture();
        on_district_completed(&mut f, (), false).unwrap();
        assert_eq!(f.invalidations, 1);
        assert_eq!(f.reads, vec!["interface", "player"]);
        f.owner = 1;
        on_district_completed(&mut f, (), true).unwrap();
        assert_eq!(f.invalidations, 1);
    }
    #[test]
    fn test_is_lazy_and_preserves_raw_states() {
        let mut f = fixture();
        f.interface = false;
        assert_eq!(test(&mut f), Ok(1));
        assert_eq!(f.reads, vec!["interface"]);
        let mut f = fixture();
        f.target = -1;
        assert_eq!(test(&mut f), Ok(1));
        assert!(!f.reads.contains(&"count"));
        let mut f = fixture();
        f.count = 1;
        assert_eq!(test(&mut f), Ok(2));
        f.count = -1;
        assert_eq!(test(&mut f), Ok(1));
        let rows = [
            DistrictCountRecord {
                type_index: 0,
                complete: true,
                pillaged: false,
            },
            DistrictCountRecord {
                type_index: 21,
                complete: false,
                pillaged: false,
            },
            DistrictCountRecord {
                type_index: 21,
                complete: true,
                pillaged: true,
            },
        ];
        assert_eq!(count_complete_unpillaged(21, rows), 0);
    }
}
