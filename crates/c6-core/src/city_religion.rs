//! Current City religious pressure and follower allocation used by population
//! growth and founding. Ordered rows are canonical City.Religion storage.
use crate::city_services::ReligionInCity;

pub const NO_RELIGION: u32 = u32::MAX;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    ZeroPressurePerFollower,
}

/// Original Find-and-add updates only the first matching row; unknown religion
/// identity is preserved, including NO_RELIGION. New rows start uninitiated.
pub fn add_pressure(rows: &mut Vec<ReligionInCity>, religion: u32, raw_q8: i32) {
    if let Some(row) = rows.iter_mut().find(|r| r.religion_hash == religion) {
        row.pressure_raw_q8 = row.pressure_raw_q8.wrapping_add(raw_q8);
    } else {
        rows.push(ReligionInCity {
            religion_hash: religion,
            followers: 0,
            pressure_raw_q8: raw_q8,
            initiation_bonus_paid: false,
        });
    }
}
fn q8_div(a: i32, b: i32) -> Result<i32, Error> {
    if b == 0 {
        return Err(Error::ZeroPressurePerFollower);
    }
    Ok((((a as i64) << 8) / (b as i64)) as i32)
}
fn q8_mul(a: i32, b: i32) -> i32 {
    (((a as i64) * (b as i64)) >> 8) as i32
}

/// 28c2da0's allocation body. Zero/nonpositive total pressure resets to the
/// original one-row atheism baseline before allocating the current population.
/// Remainder ties go to the LAST row, and each selected remainder is cleared.
/// The native working remainder is recomputed here, never supplied by a save.
pub fn recompute_followers(
    rows: &mut Vec<ReligionInCity>,
    population: i32,
    atheism_pressure_per_pop: i32,
) -> Result<(), Error> {
    if population <= 0 {
        return Ok(());
    }
    let mut total = rows
        .iter()
        .fold(0i32, |n, r| n.wrapping_add(r.pressure_raw_q8));
    if total <= 0 {
        rows.clear();
        total = atheism_pressure_per_pop.wrapping_shl(8);
        rows.push(ReligionInCity {
            religion_hash: NO_RELIGION,
            followers: 1,
            pressure_raw_q8: total,
            initiation_bonus_paid: false,
        });
    }
    let per_follower = q8_div(total, population.wrapping_shl(8))?;
    if per_follower == 0 {
        return Err(Error::ZeroPressurePerFollower);
    }
    let mut remaining = population;
    let mut remainders = Vec::with_capacity(rows.len());
    for row in rows.iter_mut() {
        row.followers = q8_div(row.pressure_raw_q8, per_follower)? >> 8;
        remaining = remaining.wrapping_sub(row.followers);
        remainders.push(
            row.pressure_raw_q8
                .wrapping_sub(q8_mul(row.followers.wrapping_shl(8), per_follower)),
        );
    }
    while remaining > 0 {
        let mut best = None;
        let mut amount = 0;
        for (index, &value) in remainders.iter().enumerate() {
            if value >= amount {
                best = Some(index);
                amount = value;
            }
        }
        if amount <= 0 {
            break;
        }
        let index = best.expect("positive remainder has a row");
        rows[index].followers = rows[index].followers.wrapping_add(1);
        remainders[index] = 0;
        remaining = remaining.wrapping_sub(1);
    }
    Ok(())
}

/// 28c32f0. The caller resolves the actual Player.Religion adoption flag,
/// current suzerain and that player's founded religion BEFORE this row scan.
/// `Some` is a real founded-religion identity, never a captured majority answer.
pub fn majority(rows: &[ReligionInCity], ally_founded_religion: Option<u32>) -> u32 {
    if let Some(religion) = ally_founded_religion {
        return religion;
    }
    let (mut total, mut best_followers, mut best_pressure, mut chosen) =
        (0i32, -1i32, 0i32, NO_RELIGION);
    for row in rows {
        // Native requires BOTH comparisons, rather than lexicographic sort.
        if row.followers >= best_followers && row.pressure_raw_q8 >= best_pressure {
            best_followers = row.followers;
            best_pressure = row.pressure_raw_q8;
            chosen = row.religion_hash;
        }
        total = total.wrapping_add(row.followers);
    }
    if best_followers.wrapping_mul(2) < total {
        NO_RELIGION
    } else {
        chosen
    }
}
pub fn followers(rows: &[ReligionInCity], religion: u32) -> i32 {
    rows.iter()
        .find(|r| r.religion_hash == religion)
        .map_or(0, |r| r.followers)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Recomputed {
    pub previous_majority: u32,
    pub majority: u32,
    pub previous_followers: i32,
    pub followers: i32,
    pub previously_initiated: bool,
    pub changed: bool,
}
/// Canonical pressure/follower/majority writes before synchronous conversion
/// effects. The caller must finish the source-ordered signals/initiation/report
/// continuation; this value alone is not a completed religion transition.
pub fn recompute(
    religion: &mut crate::city_services::Religion,
    population: i32,
    previous_majority: u32,
    atheism_pressure_per_pop: i32,
    ally_founded_religion: Option<u32>,
) -> Result<Recomputed, Error> {
    let previous_followers = followers(&religion.religion_status, previous_majority);
    if population <= 0 {
        return Ok(Recomputed {
            previous_majority,
            majority: religion.majority_religion_hash,
            previous_followers,
            followers: previous_followers,
            previously_initiated: false,
            changed: false,
        });
    }
    recompute_followers(
        &mut religion.religion_status,
        population,
        atheism_pressure_per_pop,
    )?;
    let new_majority = majority(&religion.religion_status, ally_founded_religion);
    if religion.majority_religion_hash != NO_RELIGION {
        religion.previous_real_religion_hash = religion.majority_religion_hash;
    }
    religion.majority_religion_hash = new_majority;
    let new_followers = followers(&religion.religion_status, new_majority);
    let previously_initiated = religion
        .religion_status
        .iter()
        .find(|r| r.religion_hash == new_majority)
        .is_some_and(|r| r.initiation_bonus_paid);
    Ok(Recomputed {
        previous_majority,
        majority: new_majority,
        previous_followers,
        followers: new_followers,
        previously_initiated,
        changed: new_majority != previous_majority || new_followers != previous_followers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(religion: u32, pressure: i32) -> ReligionInCity {
        ReligionInCity {
            religion_hash: religion,
            followers: 0,
            pressure_raw_q8: pressure,
            initiation_bonus_paid: false,
        }
    }
    #[test]
    fn insertion_preserves_order_and_updates_only_first_duplicate() {
        let mut v = vec![row(7, 256), row(7, 512)];
        add_pressure(&mut v, 7, 128);
        add_pressure(&mut v, 9, -256);
        assert_eq!(
            v.iter()
                .map(|r| (r.religion_hash, r.pressure_raw_q8))
                .collect::<Vec<_>>(),
            [(7, 384), (7, 512), (9, -256)]
        );
    }
    #[test]
    fn equal_remainder_goes_to_last_row() {
        let mut v = vec![row(7, 100 * 256), row(9, 100 * 256)];
        recompute_followers(&mut v, 3, 50).unwrap();
        assert_eq!(v.iter().map(|r| r.followers).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(majority(&v, None), 9);
    }
    #[test]
    fn no_positive_pressure_replaces_old_rows_with_native_atheism() {
        let mut v = vec![row(7, -1)];
        recompute_followers(&mut v, 2, 50).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(
            (v[0].religion_hash, v[0].followers, v[0].pressure_raw_q8),
            (NO_RELIGION, 2, 50 * 256)
        );
    }
    #[test]
    fn zero_population_retains_prior_rows_and_missing_divisor_is_error() {
        let mut v = vec![row(7, 0)];
        let before = v.clone();
        recompute_followers(&mut v, 0, 50).unwrap();
        assert_eq!(v, before);
        assert_eq!(
            recompute_followers(&mut v, 2, 0),
            Err(Error::ZeroPressurePerFollower)
        );
    }
    #[test]
    fn majority_uses_both_native_comparisons_and_last_exact_tie() {
        let mut a = row(7, 100);
        a.followers = 1;
        let mut b = row(9, 99);
        b.followers = 2;
        assert_eq!(majority(&[a.clone(), b], None), NO_RELIGION);
        assert_eq!(
            majority(
                &[
                    a.clone(),
                    ReligionInCity {
                        religion_hash: 11,
                        ..a
                    }
                ],
                None
            ),
            11
        );
        assert_eq!(majority(&[], Some(13)), 13);
    }
}
