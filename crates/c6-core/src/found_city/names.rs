//! Source Game.NameManager capital selection and the reached weighted RNG leaf.
//! These edit the ONE supplied canonical NameManager; no new seed or stream.
use crate::game_name_manager_wire::NameManager;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapitalRow {
    pub civilization: Vec<u8>,
    pub leader: Vec<u8>,
    pub capital: Vec<u8>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    EmbeddedNul,
    AmbiguousQueryOrder,
    ResourceLimit,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "city naming: {self:?}")
    }
}
impl std::error::Error for Error {}
fn check(bytes: &[u8]) -> Result<(), Error> {
    if bytes.contains(&0) {
        Err(Error::EmbeddedNul)
    } else {
        Ok(())
    }
}
pub fn was_used(names: &NameManager, name: &[u8]) -> bool {
    // CityNameWasUsed returns false for empty even if empty strings are stored.
    !name.is_empty() && names.used_names.iter().any(|x| x == name)
}
pub fn mark_used(names: &mut NameManager, name: &[u8]) -> Result<(), Error> {
    check(name)?;
    if !was_used(names, name) {
        names
            .used_names
            .try_reserve(1)
            .map_err(|_| Error::ResourceLimit)?;
        let mut value = Vec::new();
        value
            .try_reserve_exact(name.len())
            .map_err(|_| Error::ResourceLimit)?;
        value.extend_from_slice(name);
        names.used_names.push(value);
    }
    Ok(())
}
/// FindCapitalCityName29fff70. Exact pair is queried first; only an empty result
/// falls back to civilization-only LIMIT1. An ambiguous unordered fallback is
/// explicitly unsupported until its actual source database plan/order is known.
/// A genuinely absent query row leaves the native temporary empty, which can
/// still be selected (CityNameWasUsed(empty) is false).
pub fn capital(
    names: &NameManager,
    first: bool,
    civilization: &[u8],
    leader: &[u8],
    rows: &[CapitalRow],
) -> Result<Option<Vec<u8>>, Error> {
    if !first || civilization.is_empty() || leader.is_empty() {
        return Ok(None);
    }
    check(civilization)?;
    check(leader)?;
    let pair = rows
        .iter()
        .filter(|r| r.civilization == civilization && r.leader == leader)
        .collect::<Vec<_>>();
    if pair.len() > 1 {
        return Err(Error::AmbiguousQueryOrder);
    }
    let mut value = pair.first().map(|r| r.capital.as_slice()).unwrap_or(&[]);
    if value.is_empty() {
        let fallback = rows
            .iter()
            .filter(|r| r.civilization == civilization)
            .collect::<Vec<_>>();
        if fallback.len() > 1 {
            return Err(Error::AmbiguousQueryOrder);
        }
        value = fallback
            .first()
            .map(|r| r.capital.as_slice())
            .unwrap_or(&[]);
    }
    check(value)?;
    Ok((!was_used(names, value)).then(|| value.to_vec()))
}
/// PickWeightedRandomName2a00500/2a00b40: positive depth truncates the mutable
/// candidate vector, then weights are n,n-1,...,1. Even one candidate draws.
/// The original delegate receives the LOW16 of wrapping i32 total weight.
/// `draw` must execute Game.GetRandNum on its canonical owner, including its
/// archive/logging effects; this API never substitutes a host PRNG.
pub fn weighted_name<E>(
    names: &mut Vec<Vec<u8>>,
    depth: i32,
    mut draw: impl FnMut(u16, &'static str) -> Result<u16, E>,
) -> Result<Option<Vec<u8>>, E> {
    if depth <= 0 {
        return Ok(None);
    }
    if names.len() > depth as usize {
        names.truncate(depth as usize);
    }
    if names.is_empty() {
        return Ok(None);
    }
    let n = names.len() as i32;
    let total = (1..=n).fold(0i32, i32::wrapping_add);
    let mut remaining = i32::from(draw(total as u16, "Choosing a City Name")?);
    let mut index = names.len() - 1;
    for (i, weight) in (1..=n).rev().enumerate() {
        remaining = remaining.wrapping_sub(weight);
        if remaining < 0 {
            index = i;
            break;
        }
    }
    Ok(Some(names[index].clone()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_capital_and_used_name_state_are_lazy_and_empty_is_special() {
        let mut owner = NameManager {
            wire_version: 1,
            used_names: vec![],
        };
        let rows = [CapitalRow {
            civilization: b"C".to_vec(),
            leader: b"L".to_vec(),
            capital: b"CAP".to_vec(),
        }];
        assert_eq!(
            capital(&owner, true, b"C", b"L", &rows).unwrap(),
            Some(b"CAP".to_vec())
        );
        mark_used(&mut owner, b"CAP").unwrap();
        assert_eq!(capital(&owner, true, b"C", b"L", &rows).unwrap(), None);
        assert_eq!(capital(&owner, false, b"C", b"L", &[]).unwrap(), None);
        assert_eq!(
            capital(&owner, true, b"missing", b"leader", &[]).unwrap(),
            Some(vec![])
        );
        mark_used(&mut owner, b"").unwrap();
        mark_used(&mut owner, b"").unwrap();
        assert_eq!(owner.used_names.len(), 3);
    }
    #[test]
    fn weighted_order_depth_and_one_candidate_draw_are_preserved() {
        let mut names = vec![b"A".to_vec(), b"B".to_vec(), b"C".to_vec()];
        let mut calls = vec![];
        let chosen = weighted_name(&mut names, 2, |bound, label| {
            calls.push((bound, label));
            Ok::<_, ()>(2)
        })
        .unwrap();
        assert_eq!(chosen, Some(b"B".to_vec()));
        assert_eq!(names.len(), 2);
        assert_eq!(calls, [(3, "Choosing a City Name")]);
        weighted_name(&mut names, 1, |bound, _| {
            assert_eq!(bound, 1);
            Ok::<_, ()>(0)
        })
        .unwrap();
        assert_eq!(
            weighted_name(&mut names, 0, |_, _| Err::<u16, _>("unexpected")),
            Ok(None)
        );
    }
}
