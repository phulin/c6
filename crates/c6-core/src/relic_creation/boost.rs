//! Linux BoostManager, technology eureka and civic inspiration transitions.
//! Uses existing typed Techs/Culture fields and actual runtime rules. Complete
//! acquisition, reports, notifications and synchronous handlers are required.
use crate::player_prerequisites::IndexedBools;
use crate::unit::HashValue;
use crate::yields::PrerequisiteKind as Kind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResearchDefinition {
    pub index: i32,
    pub hash: u32,
    pub type_name: Vec<u8>,
    pub repeatable: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CostDefinition {
    pub cost: i32,
    pub era: Option<i32>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reference {
    pub index: i32,
    pub type_name: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Definition {
    pub technology: Option<Reference>,
    pub civic: Option<Reference>,
    pub percent: i32,
    pub num_items: i32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rules {
    pub technologies: Vec<Option<ResearchDefinition>>,
    pub civics: Vec<Option<ResearchDefinition>>,
    pub boosts: Vec<Definition>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Report {
    pub event: u32,
    /// Event::Type(hash,index) specialization, if native supplies one.
    pub specialization: Option<i32>,
    pub player: i32,
    pub index: Option<i32>,
    pub old_progress: Option<i32>,
    pub source: Option<i32>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Notification {
    pub kind: Kind,
    pub notification_type: u32,
    pub player: i32,
    pub index: i32,
    pub old_progress: Option<i32>,
    pub source: Option<i32>,
    pub percent: Option<i32>,
}
impl Notification {
    pub fn variants(&self) -> Vec<(u32, i32)> {
        let (index, old, source) = match self.kind {
            Kind::Technology => (0x772a_31a3, 0xf9ea_b12f, 0xb5a7_0b6c),
            Kind::Civic => (0xe4d9_4f18, 0xa5e9_5793, 0xe987_9216),
        };
        let mut result = vec![(index, self.index)];
        if let Some(value) = self.old_progress {
            result.push((old, value));
        }
        if let Some(value) = self.source {
            result.push((source, value));
        }
        result
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error<E> {
    Context(E),
    Definition,
    MissingBool(u32),
    MissingProgress(u32),
    DivideByZero,
    Acquisition(crate::research_acquisition::Error<E>),
}

/// All state is current typed state, not inferred defaults for missing Save
/// pairs. Preflight must ensure complete arrays, rules and all reached effect
/// providers (including research acquisition) before any flag/progress writes.
pub trait Context: crate::research_acquisition::Context {
    fn preflight_boost(&self, player: i32) -> Result<(), Self::Error>;
    fn boost_rules(&self) -> &Rules;
    fn runtime_civic_count(&self, player: i32) -> Result<usize, Self::Error>;
    /// Actual GetTechNode/GetCivicNode cost field, even on an inactive node.
    /// None is a captured missing node, not an unavailable export.
    fn cost_node_raw(&mut self, kind: Kind, index: i32) -> Result<Option<i32>, Self::Error>;
    /// Actual native Find(unsigned int) lookup: index first, then hash.
    fn cost_definition(
        &self,
        kind: Kind,
        index: i32,
    ) -> Result<Option<CostDefinition>, Self::Error>;
    fn cost_game_present(&mut self) -> Result<bool, Self::Error>;
    fn current_game_era(&mut self) -> Result<i32, Self::Error>;
    fn era_chronology(&self, era: i32) -> Result<Option<i32>, Self::Error>;
    fn era_cost_percent(&mut self, kind: Kind, after_game_era: bool) -> Result<i32, Self::Error>;
    /// None only for actual GameSpeedTypes=-1 (native 100 fallback).
    fn game_speed_cost_multiplier(&mut self) -> Result<Option<i32>, Self::Error>;
    fn scratch(&mut self, player: i32) -> Result<bool, Self::Error>;
    fn barbarian(&mut self, player: i32) -> Result<bool, Self::Error>;
    fn boost_report(&mut self, report: Report) -> Result<(), Self::Error>;
    fn boost_notification(&mut self, notification: Notification) -> Result<(), Self::Error>;
    /// Actual EditGame null fact gates native signal dispatch.
    fn edit_game_present(&mut self) -> Result<bool, Self::Error>;
    fn signal_boost(&mut self, player: i32, kind: Kind, index: i32) -> Result<(), Self::Error>;
    fn tourism_culture_per_citizen(&mut self) -> Result<i32, Self::Error>;
    fn total_tourists_from(&mut self, player: i32) -> Result<i32, Self::Error>;
    fn government_notifications(&mut self, player: i32) -> Result<(), Self::Error>;
    fn research_agreement_tech(&mut self, player: i32, other: i32) -> Result<i32, Self::Error>;
    fn manager_logging(&mut self) -> Result<bool, Self::Error>;
    fn manager_log(
        &mut self,
        player: i32,
        kind: Kind,
        index: i32,
        had_progress: bool,
        acquired: bool,
    ) -> Result<(), Self::Error>;
    fn publish_boost_telemetry(&mut self, player: i32, type_name: &[u8])
        -> Result<(), Self::Error>;
}

fn integer(v: i32) -> i32 {
    v.wrapping_shl(8)
}
fn as_int(v: i32) -> i32 {
    v >> 8
}
fn mul(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 8) as i32
}
fn divide<E>(a: i32, b: i32) -> Result<i32, Error<E>> {
    if b == 0 {
        return Err(Error::DivideByZero);
    }
    Ok(((i64::from(a) << 8) / i64::from(b)) as i32)
}
fn percentage(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) / 25_600) as i32
}
fn cstring(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.iter().position(|&v| v == 0).unwrap_or(bytes.len())]
}
fn definition<C: Context>(c: &C, kind: Kind, index: i32) -> Option<&ResearchDefinition> {
    match kind {
        Kind::Technology => &c.boost_rules().technologies,
        Kind::Civic => &c.boost_rules().civics,
    }
    .get(index as usize)
    .and_then(Option::as_ref)
}
fn normalize<C: Context>(c: &C, kind: Kind, input: i32) -> i32 {
    if kind != Kind::Civic {
        return input;
    }
    let raw = input as u32;
    if (raw as usize) < c.boost_rules().civics.len() {
        return input;
    }
    if raw <= 0x7ff {
        return -1;
    }
    c.boost_rules()
        .civics
        .iter()
        .flatten()
        .rfind(|d| d.hash == raw)
        .map_or(-1, |d| d.index)
}
fn bool_value<E>(v: &IndexedBools, hash: u32) -> Result<bool, Error<E>> {
    v.pairs
        .iter()
        .rfind(|p| p.definition_hash == hash)
        .map(|p| p.value)
        .ok_or(Error::MissingBool(hash))
}
fn set_bool<E>(v: &mut IndexedBools, hash: u32, value: bool) -> Result<(), Error<E>> {
    v.pairs
        .iter_mut()
        .rfind(|p| p.definition_hash == hash)
        .ok_or(Error::MissingBool(hash))?
        .value = value;
    Ok(())
}
fn raw_progress<E>(v: &[HashValue<i32>], hash: u32) -> Result<i32, Error<E>> {
    v.iter()
        .rfind(|p| p.definition_hash == hash)
        .map(|p| p.value)
        .ok_or(Error::MissingProgress(hash))
}
fn write_progress<E>(v: &mut [HashValue<i32>], hash: u32, value: i32) -> Result<(), Error<E>> {
    v.iter_mut()
        .rfind(|p| p.definition_hash == hash)
        .ok_or(Error::MissingProgress(hash))?
        .value = value;
    Ok(())
}
fn flag<C: Context>(
    c: &mut C,
    player: i32,
    kind: Kind,
    index: i32,
    boosted: bool,
) -> Result<bool, Error<C::Error>> {
    let Some(d) = definition(c, kind, index) else {
        return Ok(false);
    };
    let hash = d.hash;
    if !c.globally_available(kind, index).map_err(Error::Context)? {
        return Ok(false);
    }
    match kind {
        Kind::Technology => {
            let p = c.techs(player).map_err(Error::Context)?;
            bool_value(
                if boosted {
                    &p.boost_triggered
                } else {
                    &p.has_tech
                },
                hash,
            )
        }
        Kind::Civic => {
            let p = c.culture(player).map_err(Error::Context)?;
            bool_value(
                if boosted {
                    &p.boost_triggered
                } else {
                    &p.has_civic
                },
                hash,
            )
        }
    }
}
fn progress<C: Context>(c: &C, player: i32, kind: Kind, hash: u32) -> Result<i32, Error<C::Error>> {
    match kind {
        Kind::Technology => raw_progress(
            &c.techs(player)
                .map_err(Error::Context)?
                .research_progress_q8,
            hash,
        ),
        Kind::Civic => raw_progress(
            &c.culture(player)
                .map_err(Error::Context)?
                .cultural_progress_bits,
            hash,
        ),
    }
}
fn set_progress_raw<C: Context>(
    c: &mut C,
    player: i32,
    kind: Kind,
    hash: u32,
    value: i32,
) -> Result<(), Error<C::Error>> {
    match kind {
        Kind::Technology => write_progress(
            &mut c
                .techs_mut(player)
                .map_err(Error::Context)?
                .research_progress_q8,
            hash,
            value,
        ),
        Kind::Civic => write_progress(
            &mut c
                .culture_mut(player)
                .map_err(Error::Context)?
                .cultural_progress_bits,
            hash,
            value,
        ),
    }
}

fn max_era<C: Context>(c: &C, first: i32, second: i32) -> Result<i32, Error<C::Error>> {
    let first_order = c.era_chronology(first).map_err(Error::Context)?;
    let second_order = c.era_chronology(second).map_err(Error::Context)?;
    Ok(match (first_order, second_order) {
        (Some(a), Some(b)) => {
            if a < b {
                second
            } else {
                first
            }
        }
        _ => -1,
    })
}

/// Actual global GetTechCost/GetCivicCost. Node costs are first truncated to
/// integer (raw AsInt==-1 selects the definition cost), followed by era scaling
/// and clamped game-speed scaling. Missing node/definition returns -1.
pub fn research_cost<C: Context>(
    c: &mut C,
    kind: Kind,
    index: i32,
    base_only: bool,
) -> Result<i32, Error<C::Error>> {
    use Error::Context;
    let node = c.cost_node_raw(kind, index).map_err(Context)?;
    let definition = c.cost_definition(kind, index).map_err(Context)?;
    let (Some(node), Some(definition)) = (node, definition) else {
        return Ok(-1);
    };
    let value = as_int(node);
    let mut cost = integer(if value == -1 { definition.cost } else { value });
    if !base_only {
        let game_present = c.cost_game_present().map_err(Context)?;
        if let Some(era) = definition.era.filter(|_| game_present) {
            let current = c.current_game_era().map_err(Context)?;
            if current != -1 && era != current {
                let percent = if max_era(c, era, current)? == era {
                    Some(c.era_cost_percent(kind, true).map_err(Context)?)
                } else if max_era(c, era, current)? == current {
                    Some(c.era_cost_percent(kind, false).map_err(Context)?)
                } else {
                    None
                };
                if let Some(percent) = percent {
                    cost = percentage(cost, integer(percent.wrapping_add(100)));
                }
            }
        }
        let speed = c
            .game_speed_cost_multiplier()
            .map_err(Context)?
            .unwrap_or(100);
        let scaled = (i64::from(cost) * i64::from(integer(speed))) / 25_600;
        cost = scaled.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    }
    if cost <= 0 {
        cost = integer(1);
    }
    Ok(as_int(cost))
}

pub fn boost_value<C: Context>(c: &mut C, boost: &Definition) -> Result<i32, Error<C::Error>> {
    let mut cost = 0;
    if let Some(t) = &boost.technology {
        cost = research_cost(c, Kind::Technology, t.index, false)?;
    }
    if let Some(t) = &boost.civic {
        cost = research_cost(c, Kind::Civic, t.index, false)?;
    }
    Ok(as_int(percentage(integer(cost), integer(boost.percent))))
}
/// Preserve all intermediate Q8 truncation/wrapping stages before final integer
/// rounding. The cost and the separately rounded global boost award retain
/// their native query/rounding stages.
pub fn boost_amount<E>(
    player_cost: i32,
    global_boost_value: i32,
    modified_boost: i32,
    old_progress: i32,
) -> Result<i32, Error<E>> {
    let cost = integer(player_cost);
    let ratio = divide(integer(global_boost_value), cost)?;
    let percent = mul(ratio, integer(100)).wrapping_add(integer(modified_boost));
    let amount = integer(as_int(percentage(cost, percent)));
    Ok(cost.wrapping_sub(old_progress).min(amount))
}

fn clean_queue<C: Context>(
    c: &mut C,
    player: i32,
    kind: Kind,
    completed: u32,
) -> Result<(), Error<C::Error>> {
    let front = match kind {
        Kind::Technology => c
            .techs(player)
            .map_err(Error::Context)?
            .research_queue_hashes
            .first(),
        Kind::Civic => c
            .culture(player)
            .map_err(Error::Context)?
            .civic_queue_hashes
            .first(),
    }
    .copied();
    if front != Some(completed) {
        return Ok(());
    }
    loop {
        let next = match kind {
            Kind::Technology => {
                let q = &mut c
                    .techs_mut(player)
                    .map_err(Error::Context)?
                    .research_queue_hashes;
                q.remove(0);
                q.first().copied()
            }
            Kind::Civic => {
                let q = &mut c
                    .culture_mut(player)
                    .map_err(Error::Context)?
                    .civic_queue_hashes;
                q.remove(0);
                q.first().copied()
            }
        };
        let Some(next) = next else {
            break;
        };
        let defs = match kind {
            Kind::Technology => &c.boost_rules().technologies,
            Kind::Civic => &c.boost_rules().civics,
        };
        let index = defs
            .iter()
            .flatten()
            .find(|d| d.hash == next)
            .ok_or(Error::Definition)?
            .index;
        if !flag(c, player, kind, index, false)? {
            break;
        }
    }
    Ok(())
}

pub fn set_research_progress<C: Context>(
    c: &mut C,
    player: i32,
    kind: Kind,
    index: i32,
    mut value: i32,
) -> Result<bool, Error<C::Error>> {
    use Error::Context;
    let index = normalize(c, kind, index);
    if !c.globally_available(kind, index).map_err(Context)? {
        return Ok(false);
    }
    let hash = definition(c, kind, index).ok_or(Error::Definition)?.hash;
    if kind == Kind::Civic {
        let cost = integer(research_cost(c, kind, index, false)?);
        if value >= cost {
            value = integer(research_cost(c, kind, index, false)?);
        }
        let delta = value.wrapping_sub(progress(c, player, kind, hash)?);
        set_progress_raw(c, player, kind, hash, value)?;
        if delta > 0 {
            let p = c.culture_mut(player).map_err(Context)?;
            p.lifetime_culture_bits = p.lifetime_culture_bits.wrapping_add(delta);
            let per_citizen = c.tourism_culture_per_citizen().map_err(Context)?;
            let staycationers = if per_citizen > 0 {
                let total =
                    as_int(c.culture(player).map_err(Context)?.lifetime_culture_bits) / per_citizen;
                total.wrapping_sub(c.total_tourists_from(player).map_err(Context)?)
            } else {
                0
            };
            c.culture_mut(player).map_err(Context)?.staycationers = staycationers;
        }
    } else {
        set_progress_raw(c, player, kind, hash, value)?;
    }
    let completed =
        progress(c, player, kind, hash)? >= integer(research_cost(c, kind, index, false)?);
    if completed {
        crate::research_acquisition::set(c, player, kind, index, true)
            .map_err(Error::Acquisition)?;
        set_progress_raw(c, player, kind, hash, 0)?;
        c.boost_report(Report {
            event: if kind == Kind::Technology {
                0xf621_c5ac
            } else {
                0x8766_7464
            },
            specialization: None,
            player,
            index: Some(index),
            old_progress: None,
            source: None,
        })
        .map_err(Context)?;
        c.boost_notification(Notification {
            kind,
            notification_type: if kind == Kind::Technology {
                0x715a_6581
            } else {
                0xb738_26b0
            },
            player,
            index,
            old_progress: None,
            source: None,
            percent: None,
        })
        .map_err(Context)?;
        if kind == Kind::Civic {
            let p = c.culture_mut(player).map_err(Context)?;
            p.civic_completed_this_turn = true;
            p.civic_completed_this_turn_hash = hash;
        }
        clean_queue(c, player, kind, hash)?;
    } else {
        c.boost_report(Report {
            event: if kind == Kind::Technology {
                0x4806_2f97
            } else {
                0xc561_3042
            },
            specialization: None,
            player,
            index: Some(index),
            old_progress: None,
            source: None,
        })
        .map_err(Context)?;
    }
    if kind == Kind::Civic {
        c.government_notifications(player).map_err(Context)?;
    }
    Ok(completed)
}

pub fn change_research_progress<C: Context>(
    c: &mut C,
    player: i32,
    kind: Kind,
    index: i32,
    delta: i32,
) -> Result<bool, Error<C::Error>> {
    use Error::Context;
    let index = normalize(c, kind, index);
    let d = definition(c, kind, index).cloned();
    let active =
        index != -1 && d.is_some() && c.globally_available(kind, index).map_err(Context)?;
    let valid = active && (!flag(c, player, kind, index, false)? || d.as_ref().unwrap().repeatable);
    if !valid {
        match kind {
            Kind::Technology => {
                let p = c.techs_mut(player).map_err(Context)?;
                p.research_overflow_q8 = p.research_overflow_q8.wrapping_add(delta);
            }
            Kind::Civic => {
                let p = c.culture_mut(player).map_err(Context)?;
                p.cultural_overflow_bits = p.cultural_overflow_bits.wrapping_add(delta);
            }
        }
        c.boost_report(Report {
            event: if kind == Kind::Technology {
                0xdca9_6e74
            } else {
                0x8aba_2360
            },
            specialization: None,
            player,
            index: None,
            old_progress: None,
            source: None,
        })
        .map_err(Context)?;
        return Ok(false);
    }
    let sum = progress(c, player, kind, d.unwrap().hash)?.wrapping_add(delta);
    let cost = integer(research_cost(c, kind, index, false)?);
    if sum > cost {
        match kind {
            Kind::Technology => {
                c.techs_mut(player).map_err(Context)?.research_overflow_q8 = sum.wrapping_sub(cost)
            }
            Kind::Civic => {
                c.culture_mut(player)
                    .map_err(Context)?
                    .cultural_overflow_bits = sum.wrapping_sub(cost)
            }
        }
    }
    set_research_progress(c, player, kind, index, sum)
}

pub fn trigger_player_boost<C: Context>(
    c: &mut C,
    player: i32,
    kind: Kind,
    index: i32,
    source: i32,
) -> Result<(), Error<C::Error>> {
    use Error::Context;
    let index = normalize(c, kind, index);
    c.preflight_boost(player).map_err(Context)?;
    if kind == Kind::Civic
        && index as u32 as usize >= c.runtime_civic_count(player).map_err(Context)?
    {
        return Ok(());
    }
    if !c.globally_available(kind, index).map_err(Context)?
        || flag(c, player, kind, index, true)?
        || flag(c, player, kind, index, false)?
    {
        return Ok(());
    }
    let definition = definition(c, kind, index).ok_or(Error::Definition)?.clone();
    // Validate the complete typed storage before any mutation. The real native
    // old-progress read remains below the boost-flag write.
    progress(c, player, kind, definition.hash)?;
    if c.definitions(kind).get(index as usize).map(|d| d.hash) != Some(definition.hash) {
        return Err(Error::Definition);
    }
    match kind {
        Kind::Technology => set_bool(
            &mut c.techs_mut(player).map_err(Context)?.boost_triggered,
            definition.hash,
            true,
        )?,
        Kind::Civic => set_bool(
            &mut c.culture_mut(player).map_err(Context)?.boost_triggered,
            definition.hash,
            true,
        )?,
    }
    let old_progress = progress(c, player, kind, definition.hash)?;
    let boost = c
        .boost_rules()
        .boosts
        .iter()
        .find(|b| {
            let reference = if kind == Kind::Technology {
                &b.technology
            } else {
                &b.civic
            };
            reference
                .as_ref()
                .is_some_and(|r| cstring(&r.type_name) == cstring(&definition.type_name))
        })
        .cloned();
    if let Some(boost) = boost {
        c.boost_report(Report {
            event: if kind == Kind::Technology {
                0xbf48_fda8
            } else {
                0x3fea_ad02
            },
            specialization: Some(index),
            player,
            index: None,
            old_progress: Some(as_int(old_progress)),
            source: Some(source),
        })
        .map_err(Context)?;
        c.boost_notification(Notification {
            kind,
            notification_type: if kind == Kind::Technology {
                0x223f_be46
            } else {
                0xf94c_970d
            },
            player,
            index,
            old_progress: Some(as_int(old_progress)),
            source: Some(source),
            percent: Some(boost.percent),
        })
        .map_err(Context)?;
        if c.edit_game_present().map_err(Context)? {
            c.signal_boost(player, kind, index).map_err(Context)?;
        }
        let cost = research_cost(c, kind, index, false)?;
        if kind == Kind::Civic || integer(cost) > 0 {
            let value = boost_value(c, &boost)?;
            let modified = match kind {
                Kind::Technology => c.techs(player).map_err(Context)?.modified_boost,
                Kind::Civic => c.culture(player).map_err(Context)?.modified_boost,
            };
            let delta = boost_amount(cost, value, modified, old_progress)?;
            change_research_progress(c, player, kind, index, delta)?;
        }
    }
    if kind == Kind::Technology {
        early_research_agreement(c, player, index)?;
    }
    Ok(())
}

/// Concrete Diplomacy::CheckForEarlyResearchAgreementTrigger, including
/// recursive source-5 boosts in the retained native alive-majors order.
pub fn early_research_agreement<C: Context>(
    c: &mut C,
    player: i32,
    index: i32,
) -> Result<(), Error<C::Error>> {
    let players = c.alive_majors().map_err(Error::Context)?;
    for other in players {
        if c.research_agreement_tech(player, other)
            .map_err(Error::Context)?
            == index
            && !flag(c, other, Kind::Technology, index, true)?
        {
            trigger_player_boost(c, other, Kind::Technology, index, 5)?;
        }
    }
    Ok(())
}

/// Boost::Manager::TriggerBoost, including tech-first/civic-second ordering.
pub fn trigger_boost<C: Context>(
    c: &mut C,
    player: Option<i32>,
    boost_index: usize,
) -> Result<(), Error<C::Error>> {
    use Error::Context;
    let logging = c.manager_logging().map_err(Context)?;
    let boost = c
        .boost_rules()
        .boosts
        .get(boost_index)
        .ok_or(Error::Definition)?
        .clone();
    let Some(player) = player else {
        return Ok(());
    };
    c.preflight_boost(player).map_err(Context)?;
    if c.scratch(player).map_err(Context)? || c.barbarian(player).map_err(Context)? {
        return Ok(());
    }
    let mut telemetry = None;
    for (kind, reference) in [
        (Kind::Technology, boost.technology),
        (Kind::Civic, boost.civic),
    ] {
        let Some(reference) = reference else {
            continue;
        };
        if flag(c, player, kind, reference.index, true)?
            || flag(c, player, kind, reference.index, false)?
        {
            continue;
        }
        let hash = definition(c, kind, reference.index)
            .ok_or(Error::Definition)?
            .hash;
        let had_progress = progress(c, player, kind, hash)? != 0;
        trigger_player_boost(c, player, kind, reference.index, 0)?;
        let acquired = flag(c, player, kind, reference.index, false)?;
        telemetry = Some(
            definition(c, kind, reference.index)
                .ok_or(Error::Definition)?
                .type_name
                .clone(),
        );
        if logging {
            c.manager_log(player, kind, reference.index, had_progress, acquired)
                .map_err(Context)?;
        }
    }
    if let Some(name) = telemetry {
        c.publish_boost_telemetry(player, &name).map_err(Context)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player_culture::{Culture, GovernmentSlots};
    use crate::player_techs::Techs;
    fn bools(hash: u32, value: bool) -> IndexedBools {
        IndexedBools {
            pairs: vec![HashValue {
                definition_hash: hash,
                value,
            }],
        }
    }
    fn techs() -> Techs {
        Techs {
            wire_version: 5,
            research_queue_hashes: vec![101],
            research_overflow_q8: 0,
            science_rate_change_q8: 0,
            modified_boost: 0,
            has_tech: bools(101, false),
            boost_triggered: bools(101, false),
            research_progress_q8: vec![HashValue {
                definition_hash: 101,
                value: 0,
            }],
            num_random_tech_boosts_on_new_era: 0,
            num_techs_repeated: 0,
            free_tech_boost_wonder_era: 0,
        }
    }
    // Fully specified synthetic state; these literals are not native defaults.
    fn culture() -> Culture {
        Culture {
            wire_version: 45,
            current_government_hash: 0,
            post_anarchy_government_hash: 0,
            anarchy_end_turn: 0,
            turn_adopted: 0,
            modified_boost: 0,
            government_change_considered: false,
            government_unlocked: IndexedBools { pairs: vec![] },
            government_usage_count: vec![],
            last_government_used: vec![],
            policy_unlocked: IndexedBools { pairs: vec![] },
            policy_purchased: IndexedBools { pairs: vec![] },
            policy_has_been_enacted: IndexedBools { pairs: vec![] },
            policy_is_in_place: IndexedBools { pairs: vec![] },
            policy_banned: IndexedBools { pairs: vec![] },
            active_policies: GovernmentSlots { entries: vec![] },
            modified_slots: GovernmentSlots { entries: vec![] },
            modified_removed_slots: GovernmentSlots { entries: vec![] },
            has_civic: bools(202, false),
            boost_triggered: bools(202, false),
            cultural_progress_bits: vec![HashValue {
                definition_hash: 202,
                value: 0,
            }],
            civic_queue_hashes: vec![202],
            cultural_overflow_bits: 0,
            culture_rate_change_bits: 0,
            civic_completed_this_turn: false,
            civic_completed_this_turn_hash: 0,
            turn_last_policy_change: 0,
            num_random_civic_boosts_on_new_era: 0,
            free_civic_boost_wonder_era: 0,
            tourism_on_bits: vec![],
            dominant_over: vec![],
            turns_until_dominant: vec![],
            tourism_modifier: 0,
            cached_alliance_tourism_bonus_bits: 0,
            cached_alliance_culture_bonus_bits: 0,
            religious_tourism_reduction: 0,
            overall_tourism_reduction: 0,
            always_full_religious_tourism: 0,
            lifetime_culture_bits: 0,
            staycationers: 0,
            last_turn_staycationers: 0,
            last_turn_tourists_to: 0,
            tourists_from: vec![],
            extra_trade_route_tourism_modifier: 0,
            auto_themed_building_hash: 0,
            tourism_locations: vec![],
            culture_bomb_triggers: vec![],
            tourism_into_favor: vec![],
            culture_bomb_converts_city: false,
            unlock_features: vec![],
            tourism_national_parks_modifier_bits: 0,
            great_work_object_tourism_modifiers: vec![],
            policy_favor_list: vec![],
            building_favor_list: vec![],
            other_government_intolerance_multiplier_bits: 0,
            other_government_intolerance_modifier: 0,
            previous_government_policies: GovernmentSlots { entries: vec![] },
            num_civics_repeated: 0,
            auto_themed_buildings_with_x_slots: 0,
            auto_themed_wonders_with_x_slots: 0,
            all_great_works_tourism_modifier: 0,
            all_great_works_yield_modifier: 0,
            replaced_slots: vec![],
            government_type_modified_slots: vec![],
        }
    }
    struct Fixture {
        rules: Rules,
        techs: Vec<Techs>,
        culture: Vec<Culture>,
        log: Vec<String>,
        node_cost: Option<i32>,
        cost_definition: Option<CostDefinition>,
        game_era: i32,
        speed: Option<i32>,
        signal_progress: Option<i32>,
        agreement: bool,
    }
    impl Fixture {
        fn new() -> Self {
            Self {
                rules: Rules {
                    technologies: vec![Some(ResearchDefinition {
                        index: 0,
                        hash: 101,
                        type_name: b"TECH_A".to_vec(),
                        repeatable: false,
                    })],
                    civics: vec![Some(ResearchDefinition {
                        index: 0,
                        hash: 202,
                        type_name: b"CIVIC_A".to_vec(),
                        repeatable: false,
                    })],
                    boosts: vec![Definition {
                        technology: Some(Reference {
                            index: 0,
                            type_name: b"TECH_A".to_vec(),
                        }),
                        civic: Some(Reference {
                            index: 0,
                            type_name: b"CIVIC_A".to_vec(),
                        }),
                        percent: 50,
                        num_items: 1,
                    }],
                },
                techs: vec![techs(), techs()],
                culture: vec![culture(), culture()],
                log: vec![],
                node_cost: Some(integer(100)),
                cost_definition: Some(CostDefinition {
                    cost: 100,
                    era: None,
                }),
                game_era: -1,
                speed: Some(100),
                signal_progress: None,
                agreement: false,
            }
        }
    }
    impl Context for Fixture {
        fn preflight_boost(&self, _: i32) -> Result<(), Self::Error> {
            Ok(())
        }
        fn boost_rules(&self) -> &Rules {
            &self.rules
        }
        fn runtime_civic_count(&self, _: i32) -> Result<usize, Self::Error> {
            Ok(1)
        }
        fn cost_node_raw(&mut self, _: Kind, _: i32) -> Result<Option<i32>, Self::Error> {
            Ok(self.node_cost)
        }
        fn cost_definition(&self, _: Kind, _: i32) -> Result<Option<CostDefinition>, Self::Error> {
            Ok(self.cost_definition)
        }
        fn cost_game_present(&mut self) -> Result<bool, Self::Error> {
            Ok(true)
        }
        fn current_game_era(&mut self) -> Result<i32, Self::Error> {
            Ok(self.game_era)
        }
        fn era_chronology(&self, i: i32) -> Result<Option<i32>, Self::Error> {
            Ok(Some(i))
        }
        fn era_cost_percent(&mut self, _: Kind, after: bool) -> Result<i32, Self::Error> {
            Ok(if after { 20 } else { -20 })
        }
        fn game_speed_cost_multiplier(&mut self) -> Result<Option<i32>, Self::Error> {
            Ok(self.speed)
        }
        fn scratch(&mut self, _: i32) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn barbarian(&mut self, _: i32) -> Result<bool, Self::Error> {
            Ok(false)
        }
        fn boost_report(&mut self, e: Report) -> Result<(), Self::Error> {
            self.log.push(format!(
                "report {:x} {} {:?} {:?}",
                e.event, e.player, e.old_progress, e.source
            ));
            Ok(())
        }
        fn boost_notification(&mut self, e: Notification) -> Result<(), Self::Error> {
            self.log.push(format!("notify {:x}", e.notification_type));
            Ok(())
        }
        fn edit_game_present(&mut self) -> Result<bool, Self::Error> {
            Ok(true)
        }
        fn signal_boost(&mut self, p: i32, k: Kind, _: i32) -> Result<(), Self::Error> {
            let flag = match k {
                Kind::Technology => self.techs[p as usize].boost_triggered.pairs[0].value,
                Kind::Civic => self.culture[p as usize].boost_triggered.pairs[0].value,
            };
            assert!(flag);
            self.log.push(format!("signal {k:?} {p}"));
            if k == Kind::Technology {
                if let Some(v) = self.signal_progress {
                    self.techs[p as usize].research_progress_q8[0].value = v;
                }
            }
            Ok(())
        }
        fn tourism_culture_per_citizen(&mut self) -> Result<i32, Self::Error> {
            Ok(10)
        }
        fn total_tourists_from(&mut self, _: i32) -> Result<i32, Self::Error> {
            Ok(3)
        }
        fn government_notifications(&mut self, _: i32) -> Result<(), Self::Error> {
            self.log.push("government".into());
            Ok(())
        }
        fn research_agreement_tech(&mut self, p: i32, o: i32) -> Result<i32, Self::Error> {
            Ok(if self.agreement && p != o { 0 } else { -1 })
        }
        fn manager_logging(&mut self) -> Result<bool, Self::Error> {
            Ok(true)
        }
        fn manager_log(
            &mut self,
            _: i32,
            k: Kind,
            _: i32,
            had: bool,
            done: bool,
        ) -> Result<(), Self::Error> {
            self.log.push(format!("log {k:?} {had} {done}"));
            Ok(())
        }
        fn publish_boost_telemetry(&mut self, _: i32, name: &[u8]) -> Result<(), Self::Error> {
            self.log
                .push(format!("telemetry {}", String::from_utf8_lossy(name)));
            Ok(())
        }
    }
    impl crate::research_acquisition::Context for Fixture {
        type Error = &'static str;
        fn techs(&self, p: i32) -> Result<&Techs, Self::Error> {
            Ok(&self.techs[p as usize])
        }
        fn techs_mut(&mut self, p: i32) -> Result<&mut Techs, Self::Error> {
            Ok(&mut self.techs[p as usize])
        }
        fn culture(&self, p: i32) -> Result<&Culture, Self::Error> {
            Ok(&self.culture[p as usize])
        }
        fn culture_mut(&mut self, p: i32) -> Result<&mut Culture, Self::Error> {
            Ok(&mut self.culture[p as usize])
        }
        fn globally_available(&self, _: Kind, i: i32) -> Result<bool, Self::Error> {
            Ok(i == 0)
        }
        fn alive_majors(&self) -> Result<Vec<i32>, Self::Error> {
            Ok(vec![0, 1])
        }
        fn preflight_acquisition(
            &self,
            _: i32,
            _: Kind,
            _: i32,
            _: bool,
        ) -> Result<(), Self::Error> {
            Ok(())
        }
        fn definitions(&self, k: Kind) -> &[crate::research_acquisition::Definition] {
            const T: [crate::research_acquisition::Definition; 1] =
                [crate::research_acquisition::Definition {
                    hash: 101,
                    repeatable: Some(false),
                }];
            const C: [crate::research_acquisition::Definition; 1] =
                [crate::research_acquisition::Definition {
                    hash: 202,
                    repeatable: Some(false),
                }];
            match k {
                Kind::Technology => &T,
                Kind::Civic => &C,
            }
        }
        fn civic_acquired_count(&self, _: i32) -> Result<usize, Self::Error> {
            Ok(1)
        }
        fn signal_player_acquired(&mut self, _: i32, k: Kind, _: i32) -> Result<(), Self::Error> {
            self.log.push(format!("acquire {k:?}"));
            Ok(())
        }
        fn signal_gain_analysis(&mut self, _: i32, _: Kind) -> Result<(), Self::Error> {
            Ok(())
        }
        fn recompute_unit_costs(&mut self, _: i32) -> Result<(), Self::Error> {
            Ok(())
        }
        fn update_units_for_tech(&mut self, _: i32) -> Result<(), Self::Error> {
            Ok(())
        }
        fn recompute_district_costs(&mut self, _: i32) -> Result<(), Self::Error> {
            Ok(())
        }
        fn early_research_agreement(&mut self, p: i32, i: i32) -> Result<(), Self::Error> {
            early_research_agreement(self, p, i).map_err(|_| "agreement")
        }
        fn team(&self, p: i32) -> Result<i32, Self::Error> {
            Ok(p)
        }
        fn trigger_boost(&mut self, p: i32, k: Kind, i: i32, s: i32) -> Result<(), Self::Error> {
            trigger_player_boost(self, p, k, i, s).map_err(|_| "boost")
        }
        fn signal_game_acquired(&mut self, _: i32, _: Kind, _: i32) -> Result<(), Self::Error> {
            Ok(())
        }
    }
    #[test]
    fn boost_amount_preserves_q8_ratio_rounding_before_integer_award() {
        assert_eq!(boost_amount::<()>(101, 40, 0, 0), Ok(integer(39)));
        assert_eq!(boost_amount::<()>(101, 40, 10, 0), Ok(integer(49)));
        assert_eq!(boost_amount::<()>(100, 50, 0, integer(80)), Ok(integer(20)));
        assert_eq!(boost_amount::<()>(0, 50, 0, 0), Err(Error::DivideByZero));
    }
    #[test]
    fn manager_runs_actual_tech_then_civic_state_and_reports_in_native_order() {
        let mut f = Fixture::new();
        trigger_boost(&mut f, Some(0), 0).unwrap();
        assert!(f.techs[0].boost_triggered.pairs[0].value);
        assert!(f.culture[0].boost_triggered.pairs[0].value);
        assert_eq!(f.techs[0].research_progress_q8[0].value, integer(50));
        assert_eq!(f.culture[0].cultural_progress_bits[0].value, integer(50));
        assert_eq!(f.culture[0].lifetime_culture_bits, integer(50));
        assert_eq!(f.culture[0].staycationers, 2);
        assert_eq!(
            f.log,
            [
                "report bf48fda8 0 Some(0) Some(0)",
                "notify 223fbe46",
                "signal Technology 0",
                "report 48062f97 0 None None",
                "log Technology false false",
                "report 3feaad02 0 Some(0) Some(0)",
                "notify f94c970d",
                "signal Civic 0",
                "report c5613042 0 None None",
                "government",
                "log Civic false false",
                "telemetry CIVIC_A"
            ]
        );
    }
    #[test]
    fn progress_award_rereads_state_after_synchronous_signal() {
        let mut f = Fixture::new();
        f.techs[0].research_progress_q8[0].value = integer(20);
        f.signal_progress = Some(integer(30));
        trigger_player_boost(&mut f, 0, Kind::Technology, 0, 0).unwrap();
        assert_eq!(f.techs[0].research_progress_q8[0].value, integer(80));
        assert_eq!(f.log[0], "report bf48fda8 0 Some(20) Some(0)");
    }
    #[test]
    fn civic_completion_updates_actual_flags_queue_and_tourism_in_order() {
        let mut f = Fixture::new();
        f.culture[0].cultural_progress_bits[0].value = integer(80);
        f.culture[0].lifetime_culture_bits = integer(200);
        trigger_player_boost(&mut f, 0, Kind::Civic, 0, 0).unwrap();
        let p = &f.culture[0];
        assert!(p.has_civic.pairs[0].value);
        assert_eq!(p.cultural_progress_bits[0].value, 0);
        assert!(p.civic_queue_hashes.is_empty());
        assert!(p.civic_completed_this_turn);
        assert_eq!(p.civic_completed_this_turn_hash, 202);
        assert_eq!(p.lifetime_culture_bits, integer(220));
        assert_eq!(p.staycationers, 19);
        assert_eq!(
            &f.log[3..],
            [
                "acquire Civic",
                "report 87667464 0 None None",
                "notify b73826b0",
                "government"
            ]
        );
    }
    #[test]
    fn early_research_agreements_recursively_trigger_other_player_source_five() {
        let mut f = Fixture::new();
        f.agreement = true;
        trigger_player_boost(&mut f, 0, Kind::Technology, 0, 0).unwrap();
        assert!(f.techs[0].boost_triggered.pairs[0].value);
        assert!(f.techs[1].boost_triggered.pairs[0].value);
        assert_eq!(f.techs[1].research_progress_q8[0].value, integer(50));
        assert!(f
            .log
            .iter()
            .any(|v| v == "report bf48fda8 1 Some(0) Some(5)"));
    }
    #[test]
    fn missing_matching_boost_still_sets_flag_and_runs_agreement_tail() {
        let mut f = Fixture::new();
        f.rules.boosts.clear();
        f.agreement = true;
        trigger_player_boost(&mut f, 0, Kind::Technology, 0, 0).unwrap();
        assert!(f.techs[0].boost_triggered.pairs[0].value);
        assert!(f.techs[1].boost_triggered.pairs[0].value);
        assert!(f.log.is_empty());
        assert_eq!(f.techs[0].research_progress_q8[0].value, 0);
    }
    #[test]
    fn source_cost_uses_node_integer_sentinel_era_then_game_speed_and_floor() {
        let mut f = Fixture::new();
        f.node_cost = Some(-1); // raw -1 AsInt is -1: use actual definition cost.
        f.cost_definition = Some(CostDefinition {
            cost: 101,
            era: Some(3),
        });
        f.game_era = 2;
        f.speed = Some(150);
        assert_eq!(research_cost(&mut f, Kind::Technology, 0, false), Ok(181));
        assert_eq!(research_cost(&mut f, Kind::Technology, 0, true), Ok(101));
        f.game_era = 4;
        assert_eq!(research_cost(&mut f, Kind::Civic, 0, false), Ok(121));
        f.node_cost = None;
        assert_eq!(research_cost(&mut f, Kind::Technology, 0, false), Ok(-1));
        f.node_cost = Some(0);
        f.speed = None;
        assert_eq!(research_cost(&mut f, Kind::Technology, 0, false), Ok(1));
    }
    #[test]
    fn incomplete_progress_is_rejected_before_boost_flag_mutation() {
        let mut f = Fixture::new();
        f.techs[0].research_progress_q8.clear();
        assert_eq!(
            trigger_player_boost(&mut f, 0, Kind::Technology, 0, 0),
            Err(Error::MissingProgress(101))
        );
        assert!(!f.techs[0].boost_triggered.pairs[0].value);
        assert!(f.log.is_empty());
    }
}
