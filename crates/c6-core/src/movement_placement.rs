//! Source-closed shore-placement components, not complete location validity.
//! Terrain rejection is an accumulated prefix: even a rejection does not permit
//! skipping later combat/diplomacy queries or their unresolved effects.
use crate::movement_shore::PlacementRequest;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MissingFact {
    Impassable,
    InvalidTerrain,
    Domain,
    Water,
    CanEmbark,
    HasCityAttributes,
    CityDomainValid,
}

/// Actual getter results, with unresolved facts kept separate from false.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TerrainFacts {
    pub impassable: Option<bool>,
    pub invalid_terrain: Option<bool>,
    /// Actual effective domain enum; this is not a definition hash.
    pub domain: Option<i32>,
    pub water: Option<bool>,
    /// Actual Unit CanEmbark result, distinct from Formation CanEmbark.
    pub can_embark: Option<bool>,
    pub has_city_attributes: Option<bool>,
    pub city_domain_valid: Option<bool>,
}

/// An accumulated prefix only. Neither value admits the complete placement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerrainAssessment {
    pub rejected: bool,
}

/// Evaluate the terrain/domain prefix for an actual present destination Plot.
/// A missing Plot takes a separate native path; callers must not fabricate one.
pub fn terrain_prefix(
    facts: &TerrainFacts,
    request: PlacementRequest,
) -> Result<TerrainAssessment, MissingFact> {
    let impassable = facts.impassable.ok_or(MissingFact::Impassable)?;
    let invalid = facts.invalid_terrain.ok_or(MissingFact::InvalidTerrain)?;
    let domain = facts.domain.ok_or(MissingFact::Domain)?;
    let needs_city_domain = match domain {
        0 => {
            // Native queries IsWater even when flag7 bypasses its restriction.
            let water = facts.water.ok_or(MissingFact::Water)?;
            !water && request.flags() & 128 == 0
        }
        2 => {
            let water = facts.water.ok_or(MissingFact::Water)?;
            water && !facts.can_embark.ok_or(MissingFact::CanEmbark)?
        }
        _ => false,
    };
    let domain_invalid = needs_city_domain
        && (!facts
            .has_city_attributes
            .ok_or(MissingFact::HasCityAttributes)?
            || !facts
                .city_domain_valid
                .ok_or(MissingFact::CityDomainValid)?);
    Ok(TerrainAssessment {
        rejected: impassable || invalid || domain_invalid,
    })
}

/// Formation placement over the actual ordered membership lookup. Verified
/// absent members are skipped; unknown lookup/evaluation returns Err. The first
/// native result other than1 propagates unchanged. Empty membership returns1.
/// Neither membership nor evaluator effects are cloned, retried or rolled back.
/// A complete action must admit all effectful evaluation before invoking this
/// query component; this does not provide Unsupported rollback/atomicity.
pub fn formation_result<T, E>(
    members: impl IntoIterator<Item = Result<Option<T>, E>>,
    mut evaluate: impl FnMut(T) -> Result<i32, E>,
) -> Result<i32, E> {
    for member in members {
        if let Some(member) = member? {
            let result = evaluate(member)?;
            if result != 1 {
                return Ok(result);
            }
        }
    }
    Ok(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement_shore::{disembark_gate, ShoreFacts, ShoreGate};
    fn request(flag7: bool) -> PlacementRequest {
        let f = ShoreFacts {
            can_embark: Some(true),
            embarked: Some(true),
            source_water: Some(true),
            source_adjacent_land: Some(true),
            source_cliff_crossing: Some(false),
            ..ShoreFacts::default()
        };
        let ShoreGate::NeedsPlacement(r) = disembark_gate(&f, false, flag7).unwrap() else {
            panic!("request")
        };
        r
    }
    fn base(domain: i32) -> TerrainFacts {
        TerrainFacts {
            impassable: Some(false),
            invalid_terrain: Some(false),
            domain: Some(domain),
            ..TerrainFacts::default()
        }
    }
    #[test]
    fn already_rejected_prefix_still_requires_later_queried_facts() {
        let mut f = base(0);
        f.impassable = Some(true);
        assert_eq!(terrain_prefix(&f, request(true)), Err(MissingFact::Water));
        f.water = Some(false);
        assert_eq!(
            terrain_prefix(&f, request(true)).unwrap(),
            TerrainAssessment { rejected: true }
        );
    }
    #[test]
    fn domain_zero_flag7_bypasses_restriction_but_does_not_need_city_facts() {
        let mut f = base(0);
        f.water = Some(false);
        assert_eq!(
            terrain_prefix(&f, request(false)),
            Err(MissingFact::HasCityAttributes)
        );
        assert!(!terrain_prefix(&f, request(true)).unwrap().rejected);
        f.water = Some(true);
        assert!(!terrain_prefix(&f, request(false)).unwrap().rejected);
    }
    #[test]
    fn domain_two_embark_and_city_exception_do_not_clear_prior_rejection() {
        let mut f = base(2);
        f.water = Some(true);
        assert_eq!(
            terrain_prefix(&f, request(false)),
            Err(MissingFact::CanEmbark)
        );
        f.can_embark = Some(false);
        f.has_city_attributes = Some(false);
        assert!(terrain_prefix(&f, request(false)).unwrap().rejected);
        f.has_city_attributes = Some(true);
        assert_eq!(
            terrain_prefix(&f, request(false)),
            Err(MissingFact::CityDomainValid)
        );
        f.city_domain_valid = Some(true);
        assert!(!terrain_prefix(&f, request(false)).unwrap().rejected);
        f.invalid_terrain = Some(true);
        assert!(terrain_prefix(&f, request(false)).unwrap().rejected);
        f.can_embark = Some(true);
        f.has_city_attributes = None;
        assert!(terrain_prefix(&f, request(false)).unwrap().rejected);
    }
    #[test]
    fn other_domains_do_not_invent_water_or_city_queries_and_inputs_are_immutable() {
        for domain in [i32::MIN, -1, 1, 3, i32::MAX] {
            let f = base(domain);
            let before = f;
            assert!(!terrain_prefix(&f, request(false)).unwrap().rejected);
            assert_eq!(f, before);
        }
    }
    #[test]
    fn formation_preserves_alias_order_missing_members_and_first_failure_effects() {
        let mut calls = Vec::new();
        let members: Vec<Result<Option<i32>, &str>> = vec![
            Ok(None),
            Ok(Some(7)),
            Ok(Some(7)),
            Ok(Some(9)),
            Err("unreached"),
        ];
        let result = formation_result(members, |id| {
            calls.push(id);
            Ok(if calls.len() == 3 { -37 } else { 1 })
        });
        assert_eq!(result, Ok(-37));
        assert_eq!(calls, vec![7, 7, 9]);
        let empty: [Result<Option<i32>, &str>; 0] = [];
        assert_eq!(formation_result(empty, |_| Err("unreached")), Ok(1));
    }
    #[test]
    fn unsupported_member_or_evaluator_is_not_native_failure_or_success() {
        assert_eq!(
            formation_result([Err::<Option<i32>, _>("unknown member")], |_| Ok(1)),
            Err("unknown member")
        );
        assert_eq!(
            formation_result([Ok(Some(1))], |_| Err("unknown validity")),
            Err("unknown validity")
        );
        assert_eq!(formation_result([Ok::<_, &str>(Some(1))], |_| Ok(2)), Ok(2));
    }
}
