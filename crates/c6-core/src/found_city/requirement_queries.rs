//! Exact current-age and national-park City requirement queries (Linux XP2).
use crate::game_national_parks::ParkData;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Age {
    Golden,
    Dark,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    NativeNegativePlayer(i32),
    VectorSize,
}
/// HasGoldenAge29966f0 / HasDarkAge29967a0. Bytes are tested against zero;
/// the serializer's raw flags need not be normalized to 0/1.
pub fn has_age(values: &[u8], owner: i32) -> Result<bool, Error> {
    if owner == -1 {
        return Ok(false);
    }
    let size = i32::try_from(values.len()).map_err(|_| Error::VectorSize)?;
    if size <= owner {
        return Ok(false);
    }
    if owner < 0 {
        return Err(Error::NativeNegativePlayer(owner));
    }
    Ok(values[owner as usize] != 0)
}
pub trait AgeTestContext {
    type Error;
    type Player: Copy;
    type Game: Copy;
    type Eras: Copy;
    /// Source requires a real IPlayer interface. None represents GetPlayer's
    /// null result, not a missing interface or unobserved interface profile.
    fn subject_player(&mut self) -> Result<Option<Self::Player>, Self::Error>;
    fn context_game_interface(&mut self) -> Result<Option<Self::Game>, Self::Error>;
    fn current_game_present(&mut self, game: Self::Game) -> Result<bool, Self::Error>;
    fn eras(&mut self) -> Result<Self::Eras, Self::Error>;
    fn player_id(&mut self, player: Self::Player) -> Result<i32, Self::Error>;
    fn has_age(&mut self, eras: Self::Eras, owner: i32, age: Age) -> Result<bool, Self::Error>;
}
/// PlayerHasGoldenAge23bd690 / PlayerHasDarkAge23bcc40 return 0 met,1 unmet.
pub fn test_age<C: AgeTestContext>(c: &mut C, age: Age) -> Result<i32, C::Error> {
    let player = c.subject_player()?;
    let game = c.context_game_interface()?;
    let (Some(player), Some(game)) = (player, game) else {
        return Ok(1);
    };
    if !c.current_game_present(game)? {
        return Ok(1);
    }
    let eras = c.eras()?;
    let owner = c.player_id(player)?;
    Ok(i32::from(!c.has_age(eras, owner, age)?))
}
pub trait ParkTestContext {
    type Error;
    type Interface: Copy;
    type City: Copy;
    fn city_interface(&mut self) -> Result<Option<Self::Interface>, Self::Error>;
    fn current_city(
        &mut self,
        interface: Self::Interface,
    ) -> Result<Option<Self::City>, Self::Error>;
    fn tourism_for_city(&mut self, city: Self::City) -> Result<i32, Self::Error>;
}
/// CityHasNationalPark23766e0 tests positive computed tourism, not park count.
pub fn test_national_park<C: ParkTestContext>(c: &mut C) -> Result<i32, C::Error> {
    let Some(interface) = c.city_interface()? else {
        return Ok(1);
    };
    let Some(city) = c.current_city(interface)? else {
        return Ok(1);
    };
    Ok(i32::from(c.tourism_for_city(city)? <= 0))
}
pub trait TourismContext {
    type Error;
    fn plot_appeal(&mut self, plot: i32) -> Result<Option<i32>, Self::Error>;
    fn national_park_modifier_q8(&mut self, owner: i32) -> Result<i32, Self::Error>;
}
/// GetTourismNationalParksModifier2a79a80 constructs 1.0 and applies only a
/// strictly positive current Q8 percent, then the caller uses arithmetic AsInt.
pub fn park_multiplier_as_int(modifier_q8: i32) -> i32 {
    if modifier_q8 <= 0 {
        return 1;
    }
    let numerator = 25600_i32.wrapping_add(modifier_q8);
    ((i64::from(numerator) * 256 / 25600) as i32) >> 8
}
/// GetTourismFromPark2a06990: null plot entries skip; signed sum and final
/// integer multiply wrap. Modifier is queried only after positive sum/founder.
pub fn tourism_from_park<C: TourismContext>(c: &mut C, park: &ParkData) -> Result<i32, C::Error> {
    let mut sum = 0_i32;
    for &plot in &park.plot_indices {
        if let Some(appeal) = c.plot_appeal(plot)? {
            sum = sum.wrapping_add(appeal);
        }
    }
    if sum > 0 && park.founder_ordinal != -1 {
        sum = sum.wrapping_mul(park_multiplier_as_int(
            c.national_park_modifier_q8(park.founder_ordinal)?,
        ));
    }
    Ok(sum)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn age_boundaries_and_raw_nonzero_flags() {
        assert!(!has_age(&[0, 0x80], -1).unwrap());
        assert!(!has_age(&[0, 0x80], 0).unwrap());
        assert!(has_age(&[0, 0x80], 1).unwrap());
        assert!(!has_age(&[0, 0x80], 2).unwrap());
        assert_eq!(has_age(&[0], -2), Err(Error::NativeNegativePlayer(-2)));
    }
    struct AgeC {
        calls: Vec<&'static str>,
        player: Option<i32>,
        game: Option<i32>,
        present: bool,
    }
    impl AgeTestContext for AgeC {
        type Error = ();
        type Player = i32;
        type Game = i32;
        type Eras = ();
        fn subject_player(&mut self) -> Result<Option<i32>, ()> {
            self.calls.push("subject");
            Ok(self.player)
        }
        fn context_game_interface(&mut self) -> Result<Option<i32>, ()> {
            self.calls.push("context");
            Ok(self.game)
        }
        fn current_game_present(&mut self, _: i32) -> Result<bool, ()> {
            self.calls.push("game");
            Ok(self.present)
        }
        fn eras(&mut self) -> Result<(), ()> {
            self.calls.push("eras");
            Ok(())
        }
        fn player_id(&mut self, p: i32) -> Result<i32, ()> {
            self.calls.push("id");
            Ok(p)
        }
        fn has_age(&mut self, _: (), _: i32, _: Age) -> Result<bool, ()> {
            self.calls.push("age");
            Ok(true)
        }
    }
    #[test]
    fn test_reads_context_even_after_null_player_and_eras_before_id() {
        let mut c = AgeC {
            calls: vec![],
            player: None,
            game: Some(1),
            present: true,
        };
        assert_eq!(test_age(&mut c, Age::Dark), Ok(1));
        assert_eq!(c.calls, ["subject", "context"]);
        c.player = Some(0);
        c.calls.clear();
        assert_eq!(test_age(&mut c, Age::Golden), Ok(0));
        assert_eq!(c.calls, ["subject", "context", "game", "eras", "id", "age"]);
    }
    struct Tourism {
        calls: Vec<i32>,
        modifier: i32,
    }
    impl TourismContext for Tourism {
        type Error = ();
        fn plot_appeal(&mut self, p: i32) -> Result<Option<i32>, ()> {
            self.calls.push(p);
            Ok((p >= 0).then_some(p))
        }
        fn national_park_modifier_q8(&mut self, o: i32) -> Result<i32, ()> {
            self.calls.push(o + 100);
            Ok(self.modifier)
        }
    }
    #[test]
    fn tourism_sums_all_plots_and_truncates_multiplier_before_final_multiply() {
        let mut c = Tourism {
            calls: vec![],
            modifier: 99 * 256,
        };
        let p = ParkData {
            wire_version: 1,
            founder_ordinal: 4,
            turn_founded: 1,
            plot_indices: vec![3, -1, 5],
            name_bytes: vec![],
        };
        assert_eq!(tourism_from_park(&mut c, &p), Ok(8));
        assert_eq!(c.calls, [3, -1, 5, 104]);
        c.modifier = 100 * 256;
        assert_eq!(tourism_from_park(&mut c, &p), Ok(16));
        assert_eq!(park_multiplier_as_int(-1), 1);
        assert!(park_multiplier_as_int(i32::MAX) < 0);
    }
    struct ParkC {
        calls: Vec<&'static str>,
        interface: bool,
        city: bool,
        tourism: i32,
    }
    impl ParkTestContext for ParkC {
        type Error = ();
        type Interface = ();
        type City = ();
        fn city_interface(&mut self) -> Result<Option<()>, ()> {
            self.calls.push("interface");
            Ok(self.interface.then_some(()))
        }
        fn current_city(&mut self, _: ()) -> Result<Option<()>, ()> {
            self.calls.push("city");
            Ok(self.city.then_some(()))
        }
        fn tourism_for_city(&mut self, _: ()) -> Result<i32, ()> {
            self.calls.push("tourism");
            Ok(self.tourism)
        }
    }
    #[test]
    fn national_park_test_uses_positive_tourism_and_lazy_interface_returns() {
        let mut c = ParkC {
            calls: vec![],
            interface: false,
            city: true,
            tourism: 99,
        };
        assert_eq!(test_national_park(&mut c), Ok(1));
        assert_eq!(c.calls, ["interface"]);
        c.interface = true;
        c.tourism = 0;
        c.calls.clear();
        assert_eq!(test_national_park(&mut c), Ok(1));
        assert_eq!(c.calls, ["interface", "city", "tourism"]);
        c.tourism = 1;
        assert_eq!(test_national_park(&mut c), Ok(0));
        c.tourism = -1;
        assert_eq!(test_national_park(&mut c), Ok(1));
    }
}
