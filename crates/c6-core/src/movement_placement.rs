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

/// Definition state needed by the shore selector's visibility branch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefenderDefinition {
    Unresolved,
    Absent,
    Present { team_visibility: bool },
}

/// An actual resolved candidate from the destination's ordered layer list.
/// The owning context must establish these fields, not synthesize records.
pub struct ShoreDefender<T> {
    pub unit: T,
    pub owner: i32,
    pub delayed_death: bool,
    pub location: Option<crate::unit::PlotCoord>,
    pub definition: DefenderDefinition,
    pub religious_strength: Option<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DefenderError<E> {
    Candidate(E),
    VisibilityContext,
    Location,
    Definition,
    SubjectReligiousStrength,
    CandidateReligiousStrength,
    CombatStrength(E),
}

/// Ordered GetBestDefender selection for the shore placement flags only.
/// `visibility_present` describes the actual visibility context pointer, not
/// whether a plot is visible. Unknown context/records remain errors.
///
/// The caller supplies the actual destination layer list after the subject's
/// normal layer lookup. This does not infer occupancy from Unit locations. Each
/// comparison evaluates candidate GetCombat(false), then incumbent again; ties
/// retain the incumbent. The first eligible candidate needs no combat query.
/// Subject religion is an actual source-closed getter result, not a policy.
/// All potentially effectful query support must be admitted before invocation;
/// this component provides no whole-action Unsupported rollback.
pub fn select_shore_defender<T, E>(
    candidates: impl IntoIterator<Item = Result<Option<ShoreDefender<T>>, E>>,
    subject_owner: i32,
    visibility_present: Option<bool>,
    subject_religious_strength: Option<i32>,
    mut combat_false: impl FnMut(&T) -> Result<i32, E>,
) -> Result<Option<T>, DefenderError<E>> {
    let mut best: Option<ShoreDefender<T>> = None;
    for candidate in candidates {
        let Some(candidate) = candidate.map_err(DefenderError::Candidate)? else {
            continue;
        };
        if candidate.delayed_death || candidate.owner == subject_owner {
            continue;
        }
        if visibility_present.ok_or(DefenderError::VisibilityContext)? {
            let location = candidate.location.ok_or(DefenderError::Location)?;
            if !crate::skip_turn::placement::coordinate_valid(location) {
                continue;
            }
            match candidate.definition {
                DefenderDefinition::Absent => continue,
                DefenderDefinition::Present {
                    team_visibility: true,
                } => continue,
                DefenderDefinition::Present {
                    team_visibility: false,
                } => {}
                DefenderDefinition::Unresolved => return Err(DefenderError::Definition),
            }
        }
        if subject_religious_strength.ok_or(DefenderError::SubjectReligiousStrength)? > 0
            && candidate
                .religious_strength
                .ok_or(DefenderError::CandidateReligiousStrength)?
                > 0
        {
            return Ok(Some(candidate.unit));
        }
        if let Some(incumbent) = &best {
            let strength = combat_false(&candidate.unit).map_err(DefenderError::CombatStrength)?;
            let old_strength =
                combat_false(&incumbent.unit).map_err(DefenderError::CombatStrength)?;
            if strength > old_strength {
                best = Some(candidate);
            }
        } else {
            best = Some(candidate);
        }
    }
    Ok(best.map(|candidate| candidate.unit))
}

#[cfg(test)]
mod defender_tests {
    use super::*;
    fn unit(id: i32, owner: i32) -> ShoreDefender<i32> {
        ShoreDefender {
            unit: id,
            owner,
            delayed_death: false,
            location: Some(crate::unit::PlotCoord { x: 0, y: 0 }),
            definition: DefenderDefinition::Present {
                team_visibility: false,
            },
            religious_strength: None,
        }
    }
    #[test]
    fn actual_empty_and_filtered_occupancy_need_no_strength_or_invented_units() {
        let empty: [Result<Option<ShoreDefender<i32>>, &str>; 0] = [];
        assert_eq!(
            select_shore_defender(empty, 4, None, None, |_| Err("unreached")),
            Ok(None)
        );
        let mut dead = unit(3, 7);
        dead.delayed_death = true;
        let candidates: Vec<Result<_, &str>> = vec![Ok(None), Ok(Some(unit(1, 4))), Ok(Some(dead))];
        assert_eq!(
            select_shore_defender(candidates, 4, None, None, |_| Err("unreached")),
            Ok(None)
        );
    }
    #[test]
    fn single_eligible_candidate_needs_no_definition_strength_proxy() {
        let candidates: Vec<Result<_, &str>> = vec![Ok(Some(unit(1, 7)))];
        assert_eq!(
            select_shore_defender(candidates, 4, Some(true), Some(0), |_| Err(
                "no supported combat"
            )),
            Ok(Some(1))
        );
    }
    #[test]
    fn visibility_uses_location_definition_and_team_flag_not_map_visibility() {
        let mut invalid = unit(1, 7);
        invalid.location = Some(crate::unit::PlotCoord { x: -9999, y: 0 });
        invalid.definition = DefenderDefinition::Unresolved;
        let mut missing = unit(2, 7);
        missing.definition = DefenderDefinition::Absent;
        let mut team = unit(3, 7);
        team.definition = DefenderDefinition::Present {
            team_visibility: true,
        };
        let candidates: Vec<Result<_, &str>> = vec![
            Ok(Some(invalid)),
            Ok(Some(missing)),
            Ok(Some(team)),
            Ok(Some(unit(4, 7))),
        ];
        assert_eq!(
            select_shore_defender(candidates, 4, Some(true), Some(0), |_| Err("unreached")),
            Ok(Some(4))
        );
        let mut unqueried = unit(5, 7);
        unqueried.location = None;
        unqueried.definition = DefenderDefinition::Unresolved;
        assert_eq!(
            select_shore_defender(
                [Ok::<_, &str>(Some(unqueried))],
                4,
                Some(false),
                Some(0),
                |_| Err("unreached")
            ),
            Ok(Some(5))
        );
    }
    #[test]
    fn ordered_strength_queries_keep_first_tie_and_read_incumbent_again() {
        let candidates: Vec<Result<_, &str>> = vec![
            Ok(Some(unit(1, 7))),
            Ok(Some(unit(2, 8))),
            Ok(Some(unit(3, 9))),
        ];
        let mut calls = Vec::new();
        let result = select_shore_defender(candidates, 4, Some(true), Some(-1), |id| {
            calls.push(*id);
            Ok(if *id == 3 { -1 } else { 17 })
        });
        assert_eq!(result, Ok(Some(1)));
        assert_eq!(calls, vec![2, 1, 3, 1]);
    }
    #[test]
    fn first_religious_match_returns_immediately_preserving_order() {
        let mut first = unit(1, 7);
        first.religious_strength = Some(3);
        let candidates = [Ok(Some(first)), Err("unreached later lookup")];
        assert_eq!(
            select_shore_defender(candidates, 4, Some(true), Some(1), |_| Err(
                "unreached combat"
            )),
            Ok(Some(1))
        );
    }
    #[test]
    fn unknown_lookups_visibility_and_required_combat_are_not_absence() {
        assert_eq!(
            select_shore_defender(
                [Err::<Option<ShoreDefender<i32>>, _>("unknown")],
                4,
                None,
                None,
                |_| Ok(0)
            ),
            Err(DefenderError::Candidate("unknown"))
        );
        assert_eq!(
            select_shore_defender([Ok::<_, &str>(Some(unit(1, 7)))], 4, None, Some(0), |_| Ok(
                0
            )),
            Err(DefenderError::VisibilityContext)
        );
        let candidates = [Ok::<_, &str>(Some(unit(1, 7))), Ok(Some(unit(2, 8)))];
        assert_eq!(
            select_shore_defender(candidates, 4, Some(true), Some(0), |_| Err(
                "combat unresolved"
            )),
            Err(DefenderError::CombatStrength("combat unresolved"))
        );
    }
}

/// Actual definition query state for GetCombat(false), distinct from unknown.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatDefinition {
    Absent,
    Present {
        strength: i32,
        use_max_melee_trained: bool,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatMissing {
    Definition,
    MaxMeleeTrained,
    Embarked,
    FightEmbarked,
    FormationClass,
    Hero,
    Era,
    EmbarkedEraStrength,
    Modifier,
    MilitaryFormation,
    FormationBonus,
    ModifiedFormationBonus,
    EraModifier,
}
/// Source getter values. This is arithmetic over actual resolved state, not a
/// substitute for resolving the gameplay getters or admitting their effects.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CombatFacts {
    pub definition: Option<CombatDefinition>,
    pub max_melee_trained: Option<i32>,
    pub embarked: Option<bool>,
    pub fight_while_embarked: Option<bool>,
    pub formation_class: Option<i32>,
    pub hero: Option<bool>,
    pub era: Option<i32>,
    pub embarked_era_strength: Option<i32>,
    pub modifier: Option<i32>,
    pub military_formation: Option<i32>,
    /// Corps/army global parameter selected by military_formation1/2.
    pub formation_bonus: Option<i32>,
    /// Actual owner GetModifiedArmyCorpsStrength(domain!=0, formation==1).
    pub modified_formation_bonus: Option<i32>,
    pub era_modifier: Option<i32>,
}

// ADD followed immediately by JLE uses signed-overflow flags, unlike comparing
// the wrapped value with zero. Two sign-extended i32 addends fit exactly in i64.
fn add_is_positive(a: i32, b: i32) -> bool {
    i64::from(a) + i64::from(b) > 0
}

pub fn combat_strength_false(f: &CombatFacts) -> Result<i32, CombatMissing> {
    let mut base = match f.definition.ok_or(CombatMissing::Definition)? {
        CombatDefinition::Absent => 0,
        CombatDefinition::Present {
            strength,
            use_max_melee_trained,
        } => {
            if use_max_melee_trained {
                strength.max(f.max_melee_trained.ok_or(CombatMissing::MaxMeleeTrained)?)
            } else {
                strength
            }
        }
    };
    if f.embarked.ok_or(CombatMissing::Embarked)?
        && !f.fight_while_embarked.ok_or(CombatMissing::FightEmbarked)?
    {
        let class = f.formation_class.ok_or(CombatMissing::FormationClass)?;
        if class != 2
            && class != 1
            && !f.hero.ok_or(CombatMissing::Hero)?
            && f.era.ok_or(CombatMissing::Era)? != -1
        {
            base = f
                .embarked_era_strength
                .ok_or(CombatMissing::EmbarkedEraStrength)?;
        }
    }
    let modifier = f.modifier.ok_or(CombatMissing::Modifier)?.max(0);
    let mut strength = base.wrapping_add(modifier);
    if !add_is_positive(base, modifier) {
        return Ok(strength);
    }
    let formation = f
        .military_formation
        .ok_or(CombatMissing::MilitaryFormation)?;
    if formation == 1 || formation == 2 {
        let bonus = f.formation_bonus.ok_or(CombatMissing::FormationBonus)?;
        let adjusted = f
            .modified_formation_bonus
            .ok_or(CombatMissing::ModifiedFormationBonus)?;
        let addition = bonus.wrapping_add(adjusted);
        let positive = add_is_positive(strength, addition);
        strength = strength.wrapping_add(addition);
        if !positive {
            return Ok(strength);
        }
    }
    Ok(strength.wrapping_add(f.era_modifier.ok_or(CombatMissing::EraModifier)?))
}

#[cfg(test)]
mod combat_tests {
    use super::*;
    fn base(value: i32) -> CombatFacts {
        CombatFacts {
            definition: Some(CombatDefinition::Present {
                strength: value,
                use_max_melee_trained: false,
            }),
            embarked: Some(false),
            modifier: Some(0),
            ..CombatFacts::default()
        }
    }
    #[test]
    fn nonpositive_strength_skips_unreached_formation_and_era_inputs() {
        for value in [i32::MIN, -1, 0] {
            assert_eq!(combat_strength_false(&base(value)), Ok(value));
        }
        let mut f = base(17);
        f.modifier = Some(-30);
        f.military_formation = Some(0);
        f.era_modifier = Some(2);
        assert_eq!(combat_strength_false(&f), Ok(19));
    }
    #[test]
    fn embarked_override_and_trained_max_are_independent_of_base_definition() {
        let mut f = base(8);
        f.definition = Some(CombatDefinition::Present {
            strength: 8,
            use_max_melee_trained: true,
        });
        f.max_melee_trained = Some(12);
        f.embarked = Some(true);
        f.fight_while_embarked = Some(false);
        f.formation_class = Some(0);
        f.hero = Some(false);
        f.era = Some(3);
        f.embarked_era_strength = Some(7);
        f.military_formation = Some(0);
        f.era_modifier = Some(1);
        assert_eq!(combat_strength_false(&f), Ok(8));
        f.formation_class = Some(2);
        assert_eq!(combat_strength_false(&f), Ok(13));
        f.definition = Some(CombatDefinition::Absent);
        f.formation_class = Some(0);
        assert_eq!(combat_strength_false(&f), Ok(8));
    }
    #[test]
    fn add_overflow_flags_preserve_reached_queries_and_wrapped_results() {
        let mut f = base(i32::MAX);
        f.modifier = Some(1);
        assert_eq!(
            combat_strength_false(&f),
            Err(CombatMissing::MilitaryFormation)
        );
        f.military_formation = Some(0);
        f.era_modifier = Some(3);
        assert_eq!(combat_strength_false(&f), Ok(i32::MIN.wrapping_add(3)));
        let mut f = base(1);
        f.military_formation = Some(1);
        f.formation_bonus = Some(i32::MAX);
        f.modified_formation_bonus = Some(1);
        assert_eq!(combat_strength_false(&f), Ok(i32::MIN.wrapping_add(1)));
    }
    #[test]
    fn formation_addition_order_and_missing_getters_are_not_defaults() {
        let mut f = base(10);
        f.military_formation = Some(2);
        f.formation_bonus = Some(7);
        f.modified_formation_bonus = Some(-3);
        f.era_modifier = Some(-2);
        let before = f;
        assert_eq!(combat_strength_false(&f), Ok(12));
        assert_eq!(f, before);
        f.modified_formation_bonus = None;
        assert_eq!(
            combat_strength_false(&f),
            Err(CombatMissing::ModifiedFormationBonus)
        );
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlacementDefinition {
    Absent,
    Present { spy: bool, trade_route: bool },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlacementMissing {
    TerrainPrefix,
    Definition,
    DistrictType,
    UnitOwner,
    PlotOwner,
    BlockedEntry,
    BestDefender,
    SubjectEmbarked,
    DefenderEmbarked,
    War,
    EnterForeign,
    OpenBorders,
    EnforceBorders,
    DistrictLookup,
    DistrictDefense,
    DistrictAtLocation,
    Defensible,
    RangedCombat,
    MeleeForRanged,
    BombardCombat,
    MeleeForBombard,
    IsCombat,
    Barbarian,
    TribeIndex,
    ClansActive,
    TribeLookup,
    BribeTurns,
    ClanExcludedType,
}
/// Actual getter results for the remaining shore Unit placement stages. The
/// terrain prefix must have been evaluated for this same Unit/destination and
/// request; its rejection does not skip the defender query or later read-set.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShoreUnitFacts {
    pub terrain: Option<TerrainAssessment>,
    pub definition: Option<PlacementDefinition>,
    pub district_type: Option<i32>,
    pub unit_owner: Option<i32>,
    pub plot_owner: Option<i32>,
    pub blocked_entry: Option<bool>,
    pub best_defender_present: Option<bool>,
    pub subject_embarked: Option<bool>,
    pub defender_embarked: Option<bool>,
    pub at_war: Option<bool>,
    pub enter_foreign: Option<bool>,
    pub open_borders: Option<bool>,
    pub enforce_borders: Option<bool>,
    pub district_lookup_present: Option<bool>,
    pub district_defense_raw: Option<i32>,
    pub district_at_location_present: Option<bool>,
    pub district_defensible: Option<bool>,
    pub ranged_combat: Option<i32>,
    pub melee_for_ranged: Option<i32>,
    pub bombard_combat: Option<i32>,
    pub melee_for_bombard: Option<i32>,
    pub is_combat: Option<bool>,
    pub barbarian: Option<bool>,
    pub tribe_index: Option<i32>,
    pub clans_active: Option<bool>,
    pub tribe_present: Option<bool>,
    pub bribe_turns: Option<i32>,
    pub clan_excluded_type: Option<bool>,
}
/// A verified native Map lookup result. Unknown lookup is an upstream error,
/// never KnownAbsent. General placement flags/filters are outside this API.
pub enum ShorePlacementPlot<'a> {
    KnownAbsent,
    Present(&'a ShoreUnitFacts),
}
fn required<T>(value: Option<T>, missing: PlacementMissing) -> Result<T, PlacementMissing> {
    value.ok_or(missing)
}

fn foreign_rejection(f: &ShoreUnitFacts) -> Result<bool, PlacementMissing> {
    let mut rejected = false;
    if !required(f.at_war, PlacementMissing::War)? {
        if !required(f.enter_foreign, PlacementMissing::EnterForeign)?
            && !required(f.open_borders, PlacementMissing::OpenBorders)?
            && required(f.enforce_borders, PlacementMissing::EnforceBorders)?
        {
            rejected = true;
        } else if required(f.district_type, PlacementMissing::DistrictType)? != -1
            && required(f.district_lookup_present, PlacementMissing::DistrictLookup)?
        {
            rejected = required(f.district_defense_raw, PlacementMissing::DistrictDefense)? > 0;
        }
    } else if required(
        f.district_at_location_present,
        PlacementMissing::DistrictAtLocation,
    )? && required(f.district_defensible, PlacementMissing::Defensible)?
    {
        let ranged = required(f.ranged_combat, PlacementMissing::RangedCombat)?;
        let melee = required(f.melee_for_ranged, PlacementMissing::MeleeForRanged)?;
        if ranged > melee {
            rejected = true;
        } else {
            let bombard = required(f.bombard_combat, PlacementMissing::BombardCombat)?;
            let melee = required(f.melee_for_bombard, PlacementMissing::MeleeForBombard)?;
            if bombard > melee {
                rejected = true;
            } else {
                // Native still queries this, but shore flag0 resets its rejection.
                let _ = required(f.is_combat, PlacementMissing::IsCombat)?;
            }
        }
    }
    // Reached after this foreign branch even when it just added rejection.
    if required(f.barbarian, PlacementMissing::Barbarian)?
        && required(f.tribe_index, PlacementMissing::TribeIndex)? >= 0
        && required(f.clans_active, PlacementMissing::ClansActive)?
        && required(f.tribe_present, PlacementMissing::TribeLookup)?
        && required(f.bribe_turns, PlacementMissing::BribeTurns)? > 0
        && !required(f.clan_excluded_type, PlacementMissing::ClanExcludedType)?
    {
        rejected = true;
    }
    Ok(rejected)
}

/// Full shore-specific Unit placement decision arithmetic after its terrain
/// prefix, over actual resolved getter results. This does not resolve World
/// state or admit getter effects. Required evaluation must be supported before
/// advancing gameplay; an error is Unsupported, never native result0.
pub fn shore_unit_result(plot: ShorePlacementPlot<'_>) -> Result<i32, PlacementMissing> {
    let ShorePlacementPlot::Present(f) = plot else {
        return Ok(0);
    };
    let definition = required(f.definition, PlacementMissing::Definition)?;
    let mut rejected = required(f.terrain, PlacementMissing::TerrainPrefix)?.rejected;
    if matches!(definition, PlacementDefinition::Present { spy: true, .. })
        && required(f.district_type, PlacementMissing::DistrictType)? == -1
    {
        rejected = true;
    }
    let plot_owner = required(f.plot_owner, PlacementMissing::PlotOwner)?;
    let unit_owner = required(f.unit_owner, PlacementMissing::UnitOwner)?;
    if plot_owner != unit_owner && !rejected && plot_owner != -1 {
        rejected |= required(f.blocked_entry, PlacementMissing::BlockedEntry)?;
    }
    // GetBestDefender is not skipped for an already rejected present Plot.
    if required(f.best_defender_present, PlacementMissing::BestDefender)?
        && required(f.subject_embarked, PlacementMissing::SubjectEmbarked)?
        && required(f.defender_embarked, PlacementMissing::DefenderEmbarked)?
    {
        rejected = true;
    }
    if !rejected
        && plot_owner != unit_owner
        && plot_owner != -1
        && matches!(
            definition,
            PlacementDefinition::Present {
                spy: false,
                trade_route: false
            }
        )
    {
        rejected |= foreign_rejection(f)?;
    }
    Ok(i32::from(!rejected))
}

#[cfg(test)]
mod unit_result_tests {
    use super::*;
    fn own() -> ShoreUnitFacts {
        ShoreUnitFacts {
            terrain: Some(TerrainAssessment { rejected: false }),
            definition: Some(PlacementDefinition::Present {
                spy: false,
                trade_route: false,
            }),
            unit_owner: Some(4),
            plot_owner: Some(4),
            best_defender_present: Some(false),
            ..ShoreUnitFacts::default()
        }
    }
    fn result(f: &ShoreUnitFacts) -> Result<i32, PlacementMissing> {
        shore_unit_result(ShorePlacementPlot::Present(f))
    }
    #[test]
    fn unresolved_terrain_prefix_never_becomes_false_by_default() {
        let mut f = own();
        f.terrain = None;
        assert_eq!(result(&f), Err(PlacementMissing::TerrainPrefix));
    }

    #[test]
    fn absent_own_and_unowned_paths_are_not_blanket_denials() {
        assert_eq!(shore_unit_result(ShorePlacementPlot::KnownAbsent), Ok(0));
        let mut f = own();
        assert_eq!(result(&f), Ok(1));
        f.plot_owner = Some(-1);
        assert_eq!(result(&f), Ok(1));
    }
    #[test]
    fn prior_rejection_still_needs_defender_and_skips_unreached_territory() {
        let mut f = own();
        f.terrain = Some(TerrainAssessment { rejected: true });
        f.best_defender_present = None;
        assert_eq!(result(&f), Err(PlacementMissing::BestDefender));
        f.best_defender_present = Some(false);
        f.plot_owner = Some(7);
        assert_eq!(result(&f), Ok(0));
    }
    #[test]
    fn spy_and_embarked_coexistence_add_rejection_only_when_reached() {
        let mut f = own();
        f.definition = Some(PlacementDefinition::Present {
            spy: true,
            trade_route: false,
        });
        f.district_type = Some(-1);
        assert_eq!(result(&f), Ok(0));
        let mut f = own();
        f.best_defender_present = Some(true);
        f.subject_embarked = Some(false);
        assert_eq!(result(&f), Ok(1));
        f.subject_embarked = Some(true);
        assert_eq!(result(&f), Err(PlacementMissing::DefenderEmbarked));
        f.defender_embarked = Some(true);
        assert_eq!(result(&f), Ok(0));
    }
    #[test]
    fn foreign_enforcement_and_district_paths_still_query_barbarian_stage() {
        let mut f = own();
        f.plot_owner = Some(7);
        f.blocked_entry = Some(false);
        f.at_war = Some(false);
        f.enter_foreign = Some(false);
        f.open_borders = Some(false);
        f.enforce_borders = Some(true);
        assert_eq!(result(&f), Err(PlacementMissing::Barbarian));
        f.barbarian = Some(false);
        assert_eq!(result(&f), Ok(0));
        f.enforce_borders = Some(false);
        f.district_type = Some(-1);
        assert_eq!(result(&f), Ok(1));
        f.district_type = Some(2);
        f.district_lookup_present = Some(true);
        f.district_defense_raw = Some(1);
        assert_eq!(result(&f), Ok(0));
    }
    #[test]
    fn war_ranged_bombard_and_queried_combat_flag_preserve_shore_option() {
        let mut f = own();
        f.plot_owner = Some(7);
        f.blocked_entry = Some(false);
        f.at_war = Some(true);
        f.district_at_location_present = Some(true);
        f.district_defensible = Some(true);
        f.ranged_combat = Some(12);
        f.melee_for_ranged = Some(10);
        f.barbarian = Some(false);
        assert_eq!(result(&f), Ok(0));
        f.ranged_combat = Some(10);
        f.bombard_combat = Some(10);
        f.melee_for_bombard = Some(10);
        assert_eq!(result(&f), Err(PlacementMissing::IsCombat));
        f.is_combat = Some(true);
        assert_eq!(result(&f), Ok(1));
    }
    #[test]
    fn clan_bribe_depends_on_actual_branch_and_exclusion_without_mutation() {
        let mut f = own();
        f.plot_owner = Some(7);
        f.blocked_entry = Some(false);
        f.at_war = Some(false);
        f.enter_foreign = Some(true);
        f.district_type = Some(-1);
        f.barbarian = Some(true);
        f.tribe_index = Some(0);
        f.clans_active = Some(true);
        f.tribe_present = Some(true);
        f.bribe_turns = Some(1);
        f.clan_excluded_type = Some(false);
        let before = f;
        assert_eq!(result(&f), Ok(0));
        assert_eq!(f, before);
        f.clan_excluded_type = Some(true);
        assert_eq!(result(&f), Ok(1));
    }
}
