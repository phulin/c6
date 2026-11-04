//! Shore-transition gates over actual resolved facts. These do not move units
//! or establish complete movement legality. Missing facts remain errors. A
//! placement request must be evaluated on the same formation and destination;
//! it is never implicit placement success.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MissingFact {
    Embarked,
    CanEmbark,
    DestinationWater,
    SourceWater,
    SourceAdjacentShallowWater,
    SourceAdjacentLand,
    SourceCliffCrossing,
    SourceCity,
    DestinationCity,
    IgnoreCliffs,
}
/// `None` is unresolved. Unreached facts may remain unresolved.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShoreFacts {
    pub embarked: Option<bool>,
    pub can_embark: Option<bool>,
    pub destination_water: Option<bool>,
    pub source_water: Option<bool>,
    pub source_adjacent_shallow_water: Option<bool>,
    pub source_adjacent_land: Option<bool>,
    /// Direction from source toward destination, not its opposite.
    pub source_cliff_crossing: Option<bool>,
    pub source_city: Option<bool>,
    pub destination_city: Option<bool>,
    /// Actual formation aggregate, including member/escort semantics.
    pub ignore_cliffs: Option<bool>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlacementRequest {
    flags: u32,
}
impl PlacementRequest {
    /// Construct the fixed shore placement argument policy. This does not
    /// assert that a shore gate or any placement validity test has passed.
    pub const fn shore_query(flag7: bool) -> Self {
        Self {
            flags: 1 | ((flag7 as u32) << 7),
        }
    }

    pub const fn flags(self) -> u32 {
        self.flags
    }
    /// An unresolved evaluation must not be replaced with a default result.
    pub const fn accepts_result(self, result: i32) -> bool {
        result == 1
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShoreGate {
    Denied,
    NeedsPlacement(PlacementRequest),
}
fn fact(value: Option<bool>, missing: MissingFact) -> Result<bool, MissingFact> {
    value.ok_or(missing)
}
fn cliff_allows(f: &ShoreFacts) -> Result<bool, MissingFact> {
    if !fact(f.source_cliff_crossing, MissingFact::SourceCliffCrossing)?
        || fact(f.source_city, MissingFact::SourceCity)?
        || fact(f.destination_city, MissingFact::DestinationCity)?
    {
        return Ok(true);
    }
    fact(f.ignore_cliffs, MissingFact::IgnoreCliffs)
}
/// No separate source-land or edge-adjacency test is added.
pub fn embark_gate(f: &ShoreFacts, allow_already_embarked: bool) -> Result<ShoreGate, MissingFact> {
    if fact(f.embarked, MissingFact::Embarked)? && !allow_already_embarked {
        return Ok(ShoreGate::Denied);
    }
    if !fact(f.destination_water, MissingFact::DestinationWater)?
        || !fact(f.can_embark, MissingFact::CanEmbark)?
        || !fact(
            f.source_adjacent_shallow_water,
            MissingFact::SourceAdjacentShallowWater,
        )?
        || !cliff_allows(f)?
    {
        return Ok(ShoreGate::Denied);
    }
    Ok(ShoreGate::NeedsPlacement(PlacementRequest { flags: 1 }))
}
/// No separate destination-land test is added. `flag7` forwards the independent
/// placement option without assigning an unproven meaning to it.
pub fn disembark_gate(
    f: &ShoreFacts,
    allow_unembarked: bool,
    flag7: bool,
) -> Result<ShoreGate, MissingFact> {
    if !fact(f.can_embark, MissingFact::CanEmbark)?
        || (!fact(f.embarked, MissingFact::Embarked)? && !allow_unembarked)
        || !fact(f.source_water, MissingFact::SourceWater)?
        || !fact(f.source_adjacent_land, MissingFact::SourceAdjacentLand)?
        || !cliff_allows(f)?
    {
        return Ok(ShoreGate::Denied);
    }
    Ok(ShoreGate::NeedsPlacement(PlacementRequest {
        flags: 1 | (u32::from(flag7) << 7),
    }))
}
/// Exact directional mask query: -1 is the no-direction sentinel. Other values
/// preserve signed-byte promotion and 32-bit bit-test indexing. Hex directions
/// ordinarily range from zero through five.
pub const fn cliff_crossing(mask: u8, direction: i32) -> bool {
    direction != -1 && ((mask as i8 as i32 as u32) >> (direction as u32 & 31)) & 1 != 0
}
#[cfg(test)]
mod tests {
    use super::*;
    fn open() -> ShoreFacts {
        ShoreFacts {
            embarked: Some(false),
            can_embark: Some(true),
            destination_water: Some(true),
            source_water: Some(true),
            source_adjacent_shallow_water: Some(true),
            source_adjacent_land: Some(true),
            source_cliff_crossing: Some(false),
            ..ShoreFacts::default()
        }
    }
    #[test]
    fn order_and_no_mutation() {
        let f = ShoreFacts {
            embarked: Some(true),
            ..ShoreFacts::default()
        };
        let before = f;
        assert_eq!(embark_gate(&f, false), Ok(ShoreGate::Denied));
        assert_eq!(f, before);
        assert_eq!(embark_gate(&f, true), Err(MissingFact::DestinationWater));
        let f = ShoreFacts {
            can_embark: Some(false),
            ..ShoreFacts::default()
        };
        assert_eq!(disembark_gate(&f, false, false), Ok(ShoreGate::Denied));
        assert_eq!(embark_gate(&f, false), Err(MissingFact::Embarked));
    }
    #[test]
    fn water_and_capability_gates() {
        for change in 0..3 {
            let mut f = open();
            match change {
                0 => f.destination_water = Some(false),
                1 => f.can_embark = Some(false),
                _ => f.source_adjacent_shallow_water = Some(false),
            };
            assert_eq!(embark_gate(&f, false), Ok(ShoreGate::Denied));
        }
        let mut f = open();
        assert_eq!(disembark_gate(&f, false, false), Ok(ShoreGate::Denied));
        assert!(matches!(
            disembark_gate(&f, true, false),
            Ok(ShoreGate::NeedsPlacement(_))
        ));
        f.source_water = Some(false);
        assert_eq!(disembark_gate(&f, true, false), Ok(ShoreGate::Denied));
        f.source_water = Some(true);
        f.source_adjacent_land = Some(false);
        assert_eq!(disembark_gate(&f, true, false), Ok(ShoreGate::Denied));
    }
    #[test]
    fn cliff_city_and_ignore_order() {
        let mut f = open();
        f.source_cliff_crossing = Some(true);
        assert_eq!(embark_gate(&f, false), Err(MissingFact::SourceCity));
        f.source_city = Some(true);
        assert!(embark_gate(&f, false).is_ok());
        f.source_city = Some(false);
        assert_eq!(embark_gate(&f, false), Err(MissingFact::DestinationCity));
        f.destination_city = Some(true);
        assert!(embark_gate(&f, false).is_ok());
        f.destination_city = Some(false);
        assert_eq!(embark_gate(&f, false), Err(MissingFact::IgnoreCliffs));
        f.ignore_cliffs = Some(false);
        assert_eq!(embark_gate(&f, false), Ok(ShoreGate::Denied));
        f.ignore_cliffs = Some(true);
        assert!(matches!(
            embark_gate(&f, false),
            Ok(ShoreGate::NeedsPlacement(_))
        ));
    }
    #[test]
    fn exact_placement_flags_and_result() {
        let f = open();
        for (gate, flags) in [
            (embark_gate(&f, false), 1),
            (disembark_gate(&f, true, false), 1),
            (disembark_gate(&f, true, true), 129),
        ] {
            let ShoreGate::NeedsPlacement(r) = gate.unwrap() else {
                panic!("placement required")
            };
            assert_eq!(r.flags(), flags);
            for v in [i32::MIN, -1, 0, 2, i32::MAX] {
                assert!(!r.accepts_result(v));
            }
            assert!(r.accepts_result(1));
        }
    }
    #[test]
    fn cliff_sentinel_signed_mask_and_index_wrap() {
        for d in 0..6 {
            assert!(cliff_crossing(1 << d, d));
            assert!(!cliff_crossing(0, d));
        }
        assert!(!cliff_crossing(255, -1));
        assert!(cliff_crossing(128, 31));
        assert!(cliff_crossing(1, 32));
        assert!(!cliff_crossing(64, 31));
        assert!(cliff_crossing(128, -2));
    }
}
