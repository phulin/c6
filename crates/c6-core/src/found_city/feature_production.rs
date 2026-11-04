//! General City feature-adjacent district/wonder production modifier effects.
//! These are effect leaves, reached only after the shared modifier/requirement
//! machinery admits the actual subject. No wonder-specific or adjacency answer.
use crate::{loaded_state::LoadedState, unit::ComponentId};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
    District,
    Wonder,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Effect {
    pub target: Target,
    pub native_feature: i32,
    pub percent: i32,
}
#[derive(Debug)]
pub enum Error {
    State(crate::loaded_state::Error),
    FeatureIndex,
    Identity,
    CurrentArrayShape,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Feature production effect: {self:?}")
    }
}
impl std::error::Error for Error {}
/// Native converts percent to f32, multiplies the binary32 0.01 constant, then
/// constructs Q8 (floor/low-fraction byte) and adds wrapping. Remove negates the
/// integer BEFORE conversion; it is not necessarily the inverse in Q8 bits.
pub fn delta_q8(percent: i32, remove: bool) -> i32 {
    let value = if remove {
        percent.wrapping_neg()
    } else {
        percent
    };
    crate::movement::MovementPoints::route_float_bits((value as f32 * 0.01f32).to_bits()).0
}
/// AdjustDistrictAdjacentNaturalWonderProduction23cfda0/23cfdd0 and
/// AdjustWonderAdjacentNaturalWonderProduction23d8940/23d8970. Native -1 is a
/// true no-op before City.BuildQueue access. Other invalid indexes are errors.
pub fn apply(
    state: &mut LoadedState,
    city: ComponentId,
    feature_hashes: &[u32],
    effect: Effect,
    remove: bool,
) -> Result<bool, Error> {
    if effect.native_feature == -1 {
        return Ok(true);
    }
    let index = usize::try_from(effect.native_feature).map_err(|_| Error::FeatureIndex)?;
    let hash = *feature_hashes.get(index).ok_or(Error::FeatureIndex)?;
    if city.component_type != 2 {
        return Err(Error::Identity);
    }
    let queue = &state
        .city(i32::from(city.owner), city.id)
        .map_err(Error::State)?
        .build_queue;
    let values = match effect.target {
        Target::District => &queue.district_adjacent_natural_wonder_production_modifier,
        Target::Wonder => &queue.wonder_adjacent_natural_wonder_production_modifier,
    };
    if values.get(index).is_none_or(|v| v.definition_hash != hash) {
        return Err(Error::CurrentArrayShape);
    }
    let queue = &mut state
        .city_mut(i32::from(city.owner), city.id)
        .map_err(Error::State)?
        .build_queue;
    let values = match effect.target {
        Target::District => &mut queue.district_adjacent_natural_wonder_production_modifier,
        Target::Wonder => &mut queue.wonder_adjacent_natural_wonder_production_modifier,
    };
    let value = &mut values[index];
    value.value = value.value.wrapping_add(delta_q8(effect.percent, remove));
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn float_conversion_and_integer_negation_preserve_native_fractional_asymmetry() {
        assert_eq!(delta_q8(50, false), 128);
        assert_eq!(delta_q8(50, true), -128);
        assert_eq!(delta_q8(1, false), 2);
        assert_eq!(delta_q8(1, true), -3);
        assert_eq!(delta_q8(i32::MIN, false), delta_q8(i32::MIN, true));
    }
}
