//! Linux research turn entry and externally requested single-item queues.
//! Progress, overflow, boosts and acquisition reuse the shared research kernel.
//! No native AI research choice is performed by this module.
use crate::relic_creation::boost::{self, Context as BoostContext, Error, Report};
pub use crate::yields::PrerequisiteKind as Kind;

pub trait Context: BoostContext {
    /// Compute the current native yield, including reached current modifiers.
    fn research_yield_q8(&mut self, player: i32, kind: Kind) -> Result<i32, Self::Error>;
    /// Notification592df4c8, sent even when research is already selected.
    fn notify_choose_technology(&mut self, player: i32) -> Result<(), Self::Error>;
}
fn definitions<C: BoostContext>(c: &C, kind: Kind) -> &[Option<boost::ResearchDefinition>] {
    match kind {
        Kind::Technology => &c.boost_rules().technologies,
        Kind::Civic => &c.boost_rules().civics,
    }
}
fn index<C: BoostContext>(c: &C, kind: Kind, raw: u32) -> i32 {
    let definitions = definitions(c, kind);
    if (raw as usize) < definitions.len() {
        return raw as i32;
    }
    definitions
        .iter()
        .flatten()
        .rfind(|d| d.hash == raw)
        .map_or(-1, |d| d.index)
}
fn queue<C: BoostContext>(c: &C, player: i32, kind: Kind) -> Result<&[u32], C::Error> {
    Ok(match kind {
        Kind::Technology => &c.techs(player)?.research_queue_hashes,
        Kind::Civic => &c.culture(player)?.civic_queue_hashes,
    })
}
fn queue_mut<C: BoostContext>(
    c: &mut C,
    player: i32,
    kind: Kind,
) -> Result<&mut Vec<u32>, C::Error> {
    Ok(match kind {
        Kind::Technology => &mut c.techs_mut(player)?.research_queue_hashes,
        Kind::Civic => &mut c.culture_mut(player)?.civic_queue_hashes,
    })
}
pub fn current<C: BoostContext>(c: &C, player: i32, kind: Kind) -> Result<i32, C::Error> {
    Ok(queue(c, player, kind)?
        .first()
        .map_or(-1, |raw| index(c, kind, *raw)))
}
/// DoResearch2b8a950. Old overflow is cleared before ChangeResearchProgress.
/// With no current technology, yield plus old overflow is banked instead.
pub fn do_research<C: Context>(c: &mut C, player: i32) -> Result<(), Error<C::Error>> {
    let yield_q8 = c
        .research_yield_q8(player, Kind::Technology)
        .map_err(Error::Context)?;
    let total = yield_q8.wrapping_add(
        c.techs(player)
            .map_err(Error::Context)?
            .research_overflow_q8,
    );
    c.techs_mut(player)
        .map_err(Error::Context)?
        .research_overflow_q8 = 0;
    let tech = current(c, player, Kind::Technology).map_err(Error::Context)?;
    if tech != -1 {
        boost::change_research_progress(c, player, Kind::Technology, tech, total)?;
    } else {
        c.techs_mut(player)
            .map_err(Error::Context)?
            .research_overflow_q8 = total;
    }
    c.notify_choose_technology(player).map_err(Error::Context)
}
/// The progress region of Culture.DoCulture. Its anarchy, tourism and other
/// before/after work belongs to the full culture-turn entry, not this helper.
pub fn do_civic_progress<C: Context>(c: &mut C, player: i32) -> Result<(), Error<C::Error>> {
    let yield_q8 = c
        .research_yield_q8(player, Kind::Civic)
        .map_err(Error::Context)?;
    let total = yield_q8.wrapping_add(
        c.culture(player)
            .map_err(Error::Context)?
            .cultural_overflow_bits,
    );
    c.culture_mut(player)
        .map_err(Error::Context)?
        .cultural_overflow_bits = 0;
    let civic = current(c, player, Kind::Civic).map_err(Error::Context)?;
    if civic != -1 {
        boost::change_research_progress(c, player, Kind::Civic, civic, total)?;
    } else {
        c.culture_mut(player)
            .map_err(Error::Context)?
            .cultural_overflow_bits = total;
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InsertMode {
    Replace,
    Front,
    Back,
}
fn acquired<C: BoostContext>(
    c: &C,
    player: i32,
    kind: Kind,
    hash: u32,
) -> Result<bool, Error<C::Error>> {
    let pairs = match kind {
        Kind::Technology => &c.techs(player).map_err(Error::Context)?.has_tech.pairs,
        Kind::Civic => &c.culture(player).map_err(Error::Context)?.has_civic.pairs,
    };
    pairs
        .iter()
        .rfind(|pair| pair.definition_hash == hash)
        .map(|pair| pair.value)
        .ok_or(Error::MissingBool(hash))
}
/// Native SetResearchingTech/SetProgressingCivic single-item overload. Queue
/// inline capacity4 is not a limit. This does not select prerequisites or make
/// an AI choice; the policy supplies the requested definition and insert mode.
pub fn choose<C: BoostContext>(
    c: &mut C,
    player: i32,
    kind: Kind,
    input: i32,
    mode: InsertMode,
) -> Result<(), Error<C::Error>> {
    let raw = input as u32;
    let chosen = if raw <= 0x7ff && raw as usize >= definitions(c, kind).len() {
        -1
    } else {
        index(c, kind, raw)
    };
    let Some(definition) = definitions(c, kind)
        .get(chosen as u32 as usize)
        .and_then(Option::as_ref)
        .cloned()
    else {
        return Ok(());
    };
    if !c.globally_available(kind, chosen).map_err(Error::Context)? {
        return Ok(());
    }
    let eligible = !acquired(c, player, kind, definition.hash)? || definition.repeatable;
    let old: Vec<_> = queue(c, player, kind)
        .map_err(Error::Context)?
        .iter()
        .map(|raw| index(c, kind, *raw))
        .collect();
    let head_changed = match mode {
        InsertMode::Back => {
            if !eligible || old.contains(&chosen) {
                return Ok(());
            }
            queue_mut(c, player, kind)
                .map_err(Error::Context)?
                .push(definition.hash);
            old.is_empty()
        }
        InsertMode::Front => {
            if !eligible || old.first() == Some(&chosen) {
                return Ok(());
            }
            let q = queue_mut(c, player, kind).map_err(Error::Context)?;
            if let Some(position) = old.iter().position(|value| *value == chosen) {
                q.remove(position);
            }
            q.insert(0, definition.hash);
            true
        }
        InsertMode::Replace => {
            if old.first() == Some(&chosen) {
                if old.len() == 1 {
                    return Ok(());
                }
            } else if !eligible {
                return Ok(());
            }
            let q = queue_mut(c, player, kind).map_err(Error::Context)?;
            q.clear();
            q.push(definition.hash);
            true
        }
    };
    if head_changed {
        c.boost_report(Report {
            event: match kind {
                Kind::Technology => 0x48062f97,
                Kind::Civic => 0xc5613042,
            },
            specialization: None,
            player,
            index: Some(chosen),
            old_progress: None,
            source: None,
        })
        .map_err(Error::Context)?;
    }
    c.boost_report(Report {
        event: match kind {
            Kind::Technology => 0x4378a305,
            Kind::Civic => 0xac2769a3,
        },
        specialization: None,
        player,
        index: None,
        old_progress: None,
        source: None,
    })
    .map_err(Error::Context)
}

/// GameAttribute.Value: Base plus its separately quantized percentage term.
/// ScaleByPercentage truncates signed wide division and retains low32 bits.
pub fn yield_attribute_value(base: i32, modifier: i32) -> i32 {
    base.wrapping_add(((i64::from(base) * i64::from(modifier)) / 25_600) as i32)
}

/// Effectful numeric reads made by the stock PlayerTechs client publication.
/// The first GetScienceYield belongs to current GetTurnsLeft and is skipped
/// for an empty queue. Values.Fill then calculates science independently.
/// Pure output buffers and eligibility/cost DTO queries are not gameplay state.
/// The caller must separately qualify its actual synchronous consumer route.
pub fn technology_publication_yields<C: Context>(
    c: &mut C,
    player: i32,
) -> Result<Vec<i32>, Error<C::Error>> {
    let mut reads = Vec::with_capacity(2);
    if current(c, player, Kind::Technology).map_err(Error::Context)? != -1 {
        reads.push(
            c.research_yield_q8(player, Kind::Technology)
                .map_err(Error::Context)?,
        );
    }
    reads.push(
        c.research_yield_q8(player, Kind::Technology)
            .map_err(Error::Context)?,
    );
    Ok(reads)
}

/// RecomputeCost's empty-district stock branches. The caller establishes an
/// actually empty current District roster, making both its count and average
/// count zero. Other progression cases remain explicit errors.
pub fn empty_district_cost(
    base: i32,
    model_hash: u32,
    parameter: i32,
    research_percent: i32,
    game_escalation: i32,
    speed_multiplier: i32,
    starting_era_rate: Option<i32>,
) -> Result<i32, i32> {
    use crate::research_acquisition::units;
    let model = units::progression_model(model_hash);
    let progress = |parameter: i32| {
        parameter
            .wrapping_mul(base)
            .wrapping_div(100)
            .wrapping_sub(base)
            .wrapping_mul(research_percent)
            .wrapping_div(100)
            .wrapping_add(base)
    };
    let escalated = match model {
        1 => progress(parameter),
        5 => progress(game_escalation),
        -1 | 6..=8 => base,
        _ => return Err(model),
    };
    let factor = starting_era_rate.map_or(256, |rate| {
        let numerator = 100i32.wrapping_shl(8).wrapping_sub(rate.wrapping_shl(8));
        ((i64::from(numerator) << 8) / 25_600) as i32
    });
    let speed = units::adjust_for_game_speed(escalated, speed_multiplier);
    let raw = ((i64::from(speed.wrapping_shl(8)) * i64::from(factor)) >> 8) as i32;
    Ok((raw >> 8).max(1))
}
#[cfg(test)]
mod district_cost_tests {
    use super::*;
    #[test]
    fn district_research_scaling_then_speed_and_starting_era_quantization() {
        assert_eq!(
            empty_district_cost(54, 0xf594ad38, 40, 1, 1000, 100, Some(0)),
            Ok(58)
        );
        assert_eq!(
            empty_district_cost(54, 0x12897359, 1000, 100, 1000, 100, Some(50)),
            Ok(270)
        );
        assert_eq!(
            empty_district_cost(54, 0x12897359, 1000, 0, 1000, 50, Some(25)),
            Ok(20)
        );
        assert_eq!(
            empty_district_cost(54, 0xf594ad38, 40, 1, 1000, 100, Some(120)),
            Ok(1)
        );
        assert_eq!(empty_district_cost(-3, 0, 0, 0, 1000, 100, None), Ok(1));
        assert_eq!(
            empty_district_cost(54, 0x33b005a1, 40, 1, 1000, 100, None),
            Err(0)
        );
    }
}

/// OnPlayerTechResearched requires at least half the living majors, rounded
/// upwards, with a minimum of one. Native intermediate arithmetic is signed32.
pub fn minor_technology_catchup_threshold(alive_majors: i32) -> i32 {
    (alive_majors.wrapping_mul(50).wrapping_add(50) / 100).max(1)
}
#[cfg(test)]
mod catchup_threshold_tests {
    use super::*;
    #[test]
    fn odd_major_counts_round_up_without_granting_an_empty_game_threshold() {
        assert_eq!(
            (0..=6)
                .map(minor_technology_catchup_threshold)
                .collect::<Vec<_>>(),
            vec![1, 1, 1, 2, 2, 3, 3]
        );
        assert_eq!(minor_technology_catchup_threshold(64), 32);
        assert_eq!(minor_technology_catchup_threshold(i32::MAX), 1);
    }
}
