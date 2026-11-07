//! Linux XP2 tribal-village reward selection, before reward application.
//! All definition and subtype ordering is supplied in native collection order.
//! Eligibility must query current gameplay state; unsupported effects must never
//! be removed from the selection pool. This alone is not a complete hut action.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Family {
    pub hash: u32,
    pub improvement: Option<u32>,
    pub weight: i32,
    pub rewards: Vec<Reward>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reward {
    pub hash: u32,
    pub weight: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Draw {
    Family,
    Subtype,
}
pub trait Context {
    type Error;
    fn eligible(&mut self, reward: u32) -> Result<bool, Self::Error>;
    fn prior_rewards(&mut self, family: u32) -> Result<i32, Self::Error>;
    fn random(&mut self, bound: u16, draw: Draw) -> Result<u16, Self::Error>;
    fn increment_rewards(&mut self, family: u32) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub family: u32,
    pub reward: u32,
}

// FixedPointT<8> constructors, multiplication and Round preserve the target's
// i32 carriers. The count is ALL families, not eligible rewards or subtypes.
fn family_weight(count: usize, weight: i32, prior: i32) -> i32 {
    let count = (count as i32).wrapping_shl(8);
    let weight = weight.wrapping_shl(8);
    let mut raw = ((i64::from(count) * i64::from(weight)) >> 8) as i32;
    for _ in 0..prior {
        raw = if raw < 512 { 256 } else { raw / 2 };
    }
    raw.wrapping_add(128) >> 8
}
fn choose<C: Context>(
    context: &mut C,
    values: &[(usize, i32)],
    draw: Draw,
) -> Result<usize, C::Error> {
    let total = values
        .iter()
        .fold(0i32, |sum, (_, weight)| sum.wrapping_add(*weight));
    let mut roll = i32::from(context.random(total as u16, draw)?);
    // Native falls through to the last copied element if a truncated u16 total
    // cannot exhaust the vector; retain that behavior, without normalization.
    let mut selected = values[0].0;
    for &(index, weight) in values {
        selected = index;
        roll = roll.wrapping_sub(weight);
        if roll < 0 {
            break;
        }
    }
    Ok(selected)
}
/// GetNewType: two weighted GameSync draws and then the selected family count
/// write. A subtype's eligibility is queried again after the family draw.
/// If a query/draw fails, callers retain all preceding gameplay/RNG changes.
pub fn select<C: Context>(
    context: &mut C,
    families: &[Family],
    improvement: u32,
) -> Result<Option<Selection>, C::Error> {
    let mut candidates = Vec::new();
    for (index, family) in families.iter().enumerate() {
        if family.improvement.is_some_and(|value| value != improvement) {
            continue;
        }
        let mut eligible = 0;
        for reward in &family.rewards {
            if reward.weight > 0 && context.eligible(reward.hash)? {
                eligible += 1;
            }
        }
        // The positivity test precedes prior-reward attenuation.
        let initial = family_weight(families.len(), family.weight, 0);
        if eligible > 0 && initial > 0 {
            candidates.push((
                index,
                family_weight(
                    families.len(),
                    family.weight,
                    context.prior_rewards(family.hash)?,
                ),
            ));
        }
    }
    if candidates.is_empty() {
        return Ok(None);
    }
    let family = &families[choose(context, &candidates, Draw::Family)?];
    let mut candidates = Vec::new();
    for (index, reward) in family.rewards.iter().enumerate() {
        if reward.weight > 0 && context.eligible(reward.hash)? {
            candidates.push((index, reward.weight));
        }
    }
    if candidates.is_empty() {
        return Ok(None);
    }
    let reward = family.rewards[choose(context, &candidates, Draw::Subtype)?];
    context.increment_rewards(family.hash)?;
    Ok(Some(Selection {
        family: family.hash,
        reward: reward.hash,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Fixture {
        draws: Vec<(u16, Draw)>,
        rolls: Vec<u16>,
        queries: Vec<u32>,
        rewarded: Vec<u32>,
        prior: i32,
    }
    impl Context for Fixture {
        type Error = &'static str;
        fn eligible(&mut self, hash: u32) -> Result<bool, Self::Error> {
            self.queries.push(hash);
            Ok(hash != 99)
        }
        fn prior_rewards(&mut self, _: u32) -> Result<i32, Self::Error> {
            Ok(self.prior)
        }
        fn random(&mut self, bound: u16, draw: Draw) -> Result<u16, Self::Error> {
            self.draws.push((bound, draw));
            if self.rolls.is_empty() {
                Err("missing draw")
            } else {
                Ok(self.rolls.remove(0))
            }
        }
        fn increment_rewards(&mut self, hash: u32) -> Result<(), Self::Error> {
            self.rewarded.push(hash);
            Ok(())
        }
    }
    fn family(hash: u32, rewards: &[(u32, i32)]) -> Family {
        Family {
            hash,
            improvement: None,
            weight: 100,
            rewards: rewards
                .iter()
                .map(|&(hash, weight)| Reward { hash, weight })
                .collect(),
        }
    }
    #[test]
    fn native_order_and_two_draws_requery_selected_subtypes() {
        let mut c = Fixture {
            rolls: vec![200, 10],
            ..Fixture::default()
        };
        assert_eq!(
            select(
                &mut c,
                &[
                    family(1, &[(10, 50)]),
                    family(2, &[(20, 10), (21, 20), (99, 70)])
                ],
                7
            ),
            Ok(Some(Selection {
                family: 2,
                reward: 21
            }))
        );
        assert_eq!(c.draws, [(400, Draw::Family), (30, Draw::Subtype)]);
        assert_eq!(c.queries, [10, 20, 21, 99, 20, 21, 99]);
        assert_eq!(c.rewarded, [2]);
    }
    #[test]
    fn history_halving_uses_q8_and_floor_at_one_before_round() {
        assert_eq!(family_weight(6, 100, 1), 300);
        assert_eq!(family_weight(6, 100, 4), 38);
        assert_eq!(family_weight(6, 100, 20), 1);
    }
    #[test]
    fn unavailable_family_or_zero_weight_subtype_does_not_draw() {
        let mut c = Fixture::default();
        let mut f = family(1, &[(10, 0), (99, 5)]);
        f.improvement = Some(8);
        assert_eq!(select(&mut c, &[f], 7), Ok(None));
        assert!(c.queries.is_empty());
        assert!(c.draws.is_empty());
    }
    #[test]
    fn failed_second_draw_preserves_first_draw_without_reward_increment() {
        let mut c = Fixture {
            rolls: vec![0],
            ..Fixture::default()
        };
        assert_eq!(
            select(&mut c, &[family(1, &[(10, 5)])], 7),
            Err("missing draw")
        );
        assert_eq!(c.draws, [(100, Draw::Family), (5, Draw::Subtype)]);
        assert!(c.rewarded.is_empty());
    }
}
