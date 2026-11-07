//! Concrete Linux first-contact opinion leaves over the canonical live state.
//! Positive pledge accumulation and contributing third-party relations remain
//! explicit unsupported branches. Outer modifier membership and affected lists
//! are owned by declare_war::requirement_modifiers, including early returns.
use crate::game_update_state::activation::trade_relations_opinion as trade;
use c6_core::loaded_state::LoadedState;

#[derive(Debug)]
pub enum Effect {
    /// Raw derived fields: native single-precision rank fractions and Q8 values.
    Bushido {
        military_fraction: f32,
        culture_fraction: f32,
        positive: i32,
        negative: i32,
    },
    UnsupportedDerived(&'static str),
    SettledCities {
        penalty: i32,
    },
    Archaeology {
        penalty: i32,
    },
    ThirdParty {
        kind: i32,
        maximum: i32,
    },
    BrokenPledge,
    KeptPledge,
    NoPledge,
    ThirdPartyWarmonger {
        maximum: i32,
        percent: i32,
    },
    Warmonger {
        maximum: i32,
        percent: i32,
    },
    TradeRelations {
        effect: trade::Effect,
        statement_flags: [bool; 2],
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Preparation {
    /// The concrete Apply returns true before calling SimpleEffect.Apply.
    Skip,
    /// SetModifierValue supplies both initial and maximum to BaseApply.
    Value(i32),
    /// No SetModifierValue was reached: retain the actual BEFORE base fields.
    Unchanged,
}
pub trait CurrentYields {
    fn faith_culture_strength(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
    ) -> Result<i32, &'static str>;
}
impl Effect {
    /// These concrete Remove bodies are literal true returns; the outer owner
    /// still clears applied membership and affected instances before dispatch.
    pub fn remove_is_inert(&self) -> bool {
        matches!(
            self,
            Self::Bushido { .. }
                | Self::SettledCities { .. }
                | Self::Archaeology { .. }
                | Self::ThirdParty { .. }
                | Self::KeptPledge
                | Self::NoPledge
        )
    }

    /// Concrete GetEffectNameTextKey overrides SimpleEffect's stored name.
    /// Linux Bushido2433d30 and NewWarmonger2438a70 are fixed-key returns.
    pub fn effect_name(&self) -> Option<&'static [u8]> {
        match self {
            Self::Bushido { .. } => Some(b"LOC_TOOLTIP_SAMPLE_DIPLOMACY_BUSHIDO"),
            Self::Warmonger { .. } => Some(b"LOC_TOOLTIP_SAMPLE_DIPLOMACY_GRIEVANCES"),
            _ => None,
        }
    }
    pub fn prepare(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        subject: i32,
        instance: i32,
        alive_majors: Option<&[i32]>,
        yields: Option<&mut dyn CurrentYields>,
    ) -> Result<Preparation, &'static str> {
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or("opinion owner diplomacy")?;
        let index = usize::try_from(subject)
            .ok()
            .filter(|i| *i < 64)
            .ok_or("opinion subject index")?;
        let incursions = |promise: i32| -> i32 {
            player
                .diplomacy
                .incursions_made
                .iter()
                .filter(|(p, _)| *p == subject)
                .flat_map(|(_, types)| &types.entries)
                .filter(|(kind, _)| *kind == promise)
                .flat_map(|(_, turns)| turns)
                .fold(0i32, |count, turn| count.wrapping_add(i32::from(*turn > 0)))
        };
        Ok(match self {
            Self::UnsupportedDerived(reason) => return Err(reason),
            Self::Bushido {
                military_fraction,
                culture_fraction,
                positive,
                negative,
            } => {
                let majors = current_majors(state, alive_majors)?;
                if !majors.contains(&subject) {
                    Preparation::Value(0)
                } else {
                    let military = |state: &LoadedState, who| {
                        state
                            .retained()
                            .players
                            .iter()
                            .find(|p| p.header.owner == who)
                            .map(|p| p.stats.military_strength)
                            .ok_or("Bushido current military stats")
                    };
                    let strength = military(state, subject)?;
                    let yields = yields.ok_or("Bushido current faith/culture queries")?;
                    let faith_culture = yields.faith_culture_strength(state, subject)?;
                    let (mut total, mut military_rank, mut culture_rank) = (1i32, 1i32, 1i32);
                    for &other in majors {
                        if other == subject {
                            continue;
                        }
                        total = total.wrapping_add(1);
                        military_rank = military_rank
                            .wrapping_add(i32::from(military(state, other)? > strength));
                        culture_rank = culture_rank.wrapping_add(i32::from(
                            yields.faith_culture_strength(state, other)? > faith_culture,
                        ));
                    }
                    Preparation::Value(bushido_value(
                        total,
                        military_rank,
                        culture_rank,
                        *military_fraction,
                        *culture_fraction,
                        *positive,
                        *negative,
                    ))
                }
            }
            Self::SettledCities { penalty } => {
                Preparation::Value(settled_value(incursions(0xfadf5055u32 as i32), *penalty))
            }
            Self::Archaeology { penalty } => {
                let count = incursions(0x1347b09f);
                if count > 0 {
                    Preparation::Value(count.wrapping_mul(*penalty))
                } else {
                    Preparation::Unchanged
                }
            }
            Self::BrokenPledge | Self::KeptPledge | Self::NoPledge => {
                let board = &player.diplomacy.blackboard;
                let count = match self {
                    Self::BrokenPledge => board.broken_pledges[index],
                    Self::KeptPledge => board.kept_pledges[index],
                    _ => board.no_pledges[index],
                };
                pledge_guard(count)?
            }
            Self::ThirdParty { kind, maximum } => {
                let value = if !(1..=8).contains(kind) {
                    0
                } else {
                    let relations = state
                        .game_relations()
                        .map_err(|_| "opinion current relations")?;
                    third_party_zero(
                        *kind,
                        owner,
                        subject,
                        current_majors(state, alive_majors)?,
                        |who, other, query| match query {
                            RelationQuery::Friendship => relations
                                .has_declared_friendship(who, other)
                                .map_err(|_| "opinion friendship"),
                            RelationQuery::Alliance => relations
                                .is_allied(who, other)
                                .map_err(|_| "opinion alliance"),
                            RelationQuery::War => {
                                relations.is_at_war(who, other).map_err(|_| "opinion war")
                            }
                            RelationQuery::Denounced => {
                                let diplomacy = &state
                                    .retained()
                                    .players
                                    .iter()
                                    .find(|p| p.header.owner == who)
                                    .ok_or("opinion third-party diplomacy")?
                                    .diplomacy;
                                let index = usize::try_from(other)
                                    .ok()
                                    .filter(|i| *i < 64)
                                    .ok_or("opinion player index")?;
                                Ok(diplomacy.denounced[index] >= 0)
                            }
                        },
                    )?
                };
                Preparation::Value(clamp_third_party(value, *maximum))
            }
            Self::Warmonger { maximum, percent } => {
                let score = state
                    .game_relations()
                    .map_err(|_| "opinion grievance relations")?
                    .grievance_score(owner, subject)
                    .map_err(|_| "opinion grievance score")?;
                Preparation::Value(if score > 0 {
                    (score.wrapping_mul(*percent) / -100).max(*maximum)
                } else {
                    0
                })
            }
            Self::ThirdPartyWarmonger { .. } => {
                // Zero on every reached pair proves both alive-major sums zero.
                let relations = state
                    .game_relations()
                    .map_err(|_| "opinion grievance relations")?;
                for &other in current_majors(state, alive_majors)? {
                    if other == owner || other == subject {
                        continue;
                    }
                    if relations
                        .grievance_score(other, owner)
                        .map_err(|_| "opinion third-party grievance")?
                        != 0
                        || relations
                            .grievance_score(other, subject)
                            .map_err(|_| "opinion third-party grievance")?
                            != 0
                    {
                        return Err("opinion nonzero third-party grievances");
                    }
                }
                Preparation::Value(0)
            }
            Self::TradeRelations {
                effect,
                statement_flags,
            } => {
                trade::apply(state, effect, owner, subject, instance, *statement_flags)?;
                Preparation::Skip
            }
        })
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RelationQuery {
    Friendship,
    Alliance,
    War,
    Denounced,
}

/// Linux 23684f0..23693e9 iterates every alive major (including owner and
/// subject). An owner's enemy alone contributes nothing: the subject must
/// also pass the concrete relation predicate against that same third party.
/// Only the reached contribution/decay branch requires the effect's additional
/// amount and duration fields, which this bounded implementation cannot apply.
fn third_party_zero(
    kind: i32,
    owner: i32,
    subject: i32,
    majors: &[i32],
    mut query: impl FnMut(i32, i32, RelationQuery) -> Result<bool, &'static str>,
) -> Result<i32, &'static str> {
    use RelationQuery::*;
    let (first, second) = if kind <= 4 {
        (Friendship, Alliance)
    } else {
        (Denounced, War)
    };
    let subject_query = match kind {
        1 | 5 => War,
        2 | 6 => Denounced,
        3 | 7 => Friendship,
        4 | 8 => Alliance,
        _ => return Ok(0),
    };
    for &other in majors {
        if (query(owner, other, first)? || query(owner, other, second)?)
            && query(subject, other, subject_query)?
        {
            return Err("opinion contributing third-party relation calculation");
        }
    }
    Ok(0)
}

fn current_majors<'a>(
    state: &LoadedState,
    roster: Option<&'a [i32]>,
) -> Result<&'a [i32], &'static str> {
    let roster = roster.ok_or("opinion current alive-major roster")?;
    for &owner in roster {
        if !state
            .player_alive(owner)
            .map_err(|_| "opinion alive-major player")?
        {
            return Err("opinion alive-major roster changed");
        }
    }
    Ok(roster)
}
fn settled_value(count: i32, penalty: i32) -> i32 {
    if count > 0 {
        count.wrapping_neg().wrapping_mul(penalty)
    } else {
        0
    }
}
fn clamp_third_party(value: i32, maximum: i32) -> i32 {
    value.max(maximum.wrapping_neg()).min(maximum)
}
fn pledge_guard(count: i32) -> Result<Preparation, &'static str> {
    if count <= 0 {
        Ok(Preparation::Skip)
    } else {
        Err("opinion positive pledge accumulation")
    }
}
fn bushido_value(
    total: i32,
    military_rank: i32,
    culture_rank: i32,
    military_fraction: f32,
    culture_fraction: f32,
    positive: i32,
    negative: i32,
) -> i32 {
    let count = total as f32;
    let military_qualifies = military_fraction * count >= military_rank as f32;
    if !military_qualifies {
        return 0;
    }
    if culture_fraction * count >= culture_rank as f32 {
        return positive;
    }
    if culture_fraction * count >= 1i32.wrapping_sub(culture_rank).wrapping_add(total) as f32 {
        negative
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bushido_rank_boundaries_preserve_ties_and_single_precision_thresholds() {
        assert_eq!(bushido_value(2, 1, 1, 0.5, 0.5, 2048, -2048), 2048);
        assert_eq!(bushido_value(2, 2, 1, 0.5, 0.5, 2048, -2048), 0);
        assert_eq!(bushido_value(2, 1, 2, 0.5, 0.5, 2048, -2048), -2048);
        assert_eq!(bushido_value(2, 1, 1, f32::NAN, 0.5, 2048, -2048), 0);
    }
    #[test]
    fn settled_count_guard_and_product_wrap() {
        assert_eq!(settled_value(-1, 3), 0);
        assert_eq!(settled_value(0, 3), 0);
        assert_eq!(settled_value(2, 3), -6);
        assert_eq!(settled_value(2, i32::MAX), 2);
    }
    #[test]
    fn pledge_nonpositive_returns_before_simple_activation() {
        assert_eq!(pledge_guard(0), Ok(Preparation::Skip));
        assert_eq!(pledge_guard(-1), Ok(Preparation::Skip));
        assert!(pledge_guard(1).is_err());
    }
    #[test]
    fn third_party_war_with_subject_does_not_contribute_without_subject_relation() {
        for kind in 1..=8 {
            let value = third_party_zero(kind, 0, 1, &[0, 1], |who, other, query| {
                Ok(query == RelationQuery::War && who != other)
            });
            assert_eq!(value, Ok(0), "kind {kind}");
        }
    }
    #[test]
    fn third_party_uses_concrete_subject_predicate_and_preserves_unknown_contributions() {
        use RelationQuery::*;
        for kind in 1..=8 {
            let subject_query = [War, Denounced, Friendship, Alliance][(kind - 1) as usize % 4];
            let owner_query = if kind <= 4 { Friendship } else { War };
            for actual in [War, Denounced, Friendship, Alliance] {
                let value = third_party_zero(kind, 0, 1, &[2], |who, other, query| {
                    assert_eq!(other, 2);
                    Ok((who == 0 && query == owner_query) || (who == 1 && query == actual))
                });
                assert_eq!(
                    value.is_err(),
                    actual == subject_query,
                    "kind {kind}, {actual:?}"
                );
            }
        }
    }
    #[test]
    fn third_party_native_clamp_preserves_negative_limit_branch() {
        assert_eq!(clamp_third_party(0, 10), 0);
        assert_eq!(clamp_third_party(0, -3), -3);
        assert_eq!(clamp_third_party(0, i32::MIN), i32::MIN);
    }
}
