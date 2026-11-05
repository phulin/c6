//! Ordered improvement plunder and post-pillage effects for Linux XP2.
//! Reached economic, damage, loyalty, reporting and signal continuations are
//! required services. An error preserves all preceding writes and callbacks.
use crate::{registry::LinuxDefinitionRegistry, Rules};
use c6_core::{
    core_player::initialization::WireBinding, loaded_state::LoadedState, map::UnitReference,
    unit::variants::VariantPayload,
};
#[derive(Debug)]
pub enum Error<E = ()> {
    Binding,
    Missing(&'static str),
    State(c6_core::loaded_state::Error),
    Effect(E),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Report {
    Yield {
        actor: UnitReference,
        kind: i32,
        amount: i32,
        status: u32,
    },
    Loyalty {
        actor: UnitReference,
        city: (i32, u32),
        amount: i32,
    },
    HealSignal {
        actor: UnitReference,
        requested: i32,
        actual: i32,
    },
}
pub struct Definitions {
    binding: WireBinding,
    improvements: LinuxDefinitionRegistry,
    plunder: Vec<(i32, i32)>,
}
impl Definitions {
    pub fn from_rules(state: &LoadedState, rules: &Rules) -> Result<Self, Error> {
        let improvements = crate::unit_initialization_definitions::collection(
            rules,
            "Improvements",
            "ImprovementType",
        )
        .map_err(|_| Error::Missing("plunder improvement definitions"))?;
        let table = rules
            .gameplay()
            .table("Improvements")
            .ok_or(Error::Missing("Improvements"))?;
        let mut plunder = Vec::new();
        for e in improvements.entries() {
            let row = table
                .row_by_id(e.database_rowid)
                .ok_or(Error::Missing("plunder improvement row"))?;
            let name_hash = match row
                .get("PlunderType")
                .map_err(|_| Error::Missing("PlunderType"))?
            {
                crate::SqlValue::Null => 0,
                crate::SqlValue::Text(value) => crate::registry::linux_type_hash(value),
                _ => return Err(Error::Missing("PlunderType text")),
            };
            let kind = name_to_plunder_type(name_hash);
            let amount = crate::game_parameters::sql_integer(
                row.get("PlunderAmount")
                    .map_err(|_| Error::Missing("PlunderAmount"))?,
            )
            .ok_or(Error::Missing("integer PlunderAmount"))?;
            plunder.push((kind, amount));
        }
        Ok(Self {
            binding: state.binding().clone(),
            improvements,
            plunder,
        })
    }
}
/// Actual Linux Definition.Utility.NameToPlunderType21f1380. Every other
/// name, including the stock PLUNDER_NONE rows, resolves to NO_PLUNDER (-1).
fn name_to_plunder_type(hash: u32) -> i32 {
    match hash {
        0x0b90a0a5 => 0,
        0x3df735fb => 1,
        0xda2e0de6 => 2,
        0xc3e39b3a => 3,
        0xd3765604 => 4,
        _ => -1,
    }
}
/// Current implementations must perform each reached call; these methods have
/// no success defaults. Amounts passed to ChangeYield and ChangeLoyalty are Q8.
pub trait Services {
    type Error;
    fn scaled_improvement_plunder(
        &mut self,
        state: &LoadedState,
        actor: UnitReference,
        base: i32,
    ) -> Result<i32, Self::Error>;
    fn change_damage(
        &mut self,
        state: &mut LoadedState,
        actor: UnitReference,
        delta: i32,
    ) -> Result<(), Self::Error>;
    fn change_yield(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        kind: i32,
        amount_q8: i32,
    ) -> Result<(), Self::Error>;
    fn change_loyalty(
        &mut self,
        state: &mut LoadedState,
        city: (i32, u32),
        delta_q8: i32,
    ) -> Result<(), Self::Error>;
    fn report(&mut self, state: &mut LoadedState, report: Report) -> Result<(), Self::Error>;
}
pub struct Inputs<'a> {
    pub civilization_levels: &'a [(i32, u32)],
    pub cached_majors: &'a [i32],
}
pub struct Current<'a, E> {
    pub definitions: &'a Definitions,
    pub inputs: Inputs<'a>,
    pub services: &'a mut E,
}
fn player<E>(
    state: &LoadedState,
    owner: i32,
) -> Result<&c6_core::loaded_components::RetainedPlayer, Error<E>> {
    state
        .retained()
        .players
        .iter()
        .find(|p| p.requested_owner == owner)
        .ok_or(Error::Missing("plunder current Player"))
}
fn unit<E>(state: &LoadedState, actor: UnitReference) -> Result<&c6_core::unit::Unit, Error<E>> {
    state
        .world()
        .unit(actor)
        .map_err(|_| Error::Missing("plunder current Unit"))?
        .map(|u| u.record)
        .ok_or(Error::Missing("plunder current Unit"))
}
fn property<E>(state: &LoadedState, actor: UnitReference) -> Result<i32, Error<E>> {
    let mut entries = unit(state, actor)?
        .k_properties
        .entries
        .iter()
        .filter(|e| e.key == 0x2c0247e5);
    let Some(e) = entries.next() else {
        return Ok(0);
    };
    if entries.next().is_some()
        || e.value.type_bits != 2
        || e.value.modifier_bits != 0
        || e.value.value.count_flags != 0
        || e.value.value.element_size != 0
    {
        return Err(Error::Missing(
            "pillage healing property integer conversion",
        ));
    }
    match e.value.value.payload {
        VariantPayload::Inline32(v) => Ok(v as i32),
        _ => Err(Error::Missing("pillage healing property payload")),
    }
}
/// Plunder wrapper2aad2d0 passes its output as the first generic output;
/// experience uses the second output. Both use the last qualifying supplier.
pub fn joint_war_partners<E>(
    state: &LoadedState,
    inputs: &Inputs<'_>,
    enemy: i32,
) -> Result<(Vec<i32>, i32), Error<E>> {
    if !state
        .world()
        .player_is_valid(enemy)
        .map_err(|_| Error::Missing("plunder target player validity"))?
    {
        return Ok((vec![], 0));
    }
    let level = inputs
        .civilization_levels
        .iter()
        .find(|p| p.0 == enemy)
        .ok_or(Error::Missing("plunder target civilization level"))?
        .1;
    if level == 0xd959ec24 {
        return Ok((vec![], 0));
    }
    let relations = state.game_relations().map_err(Error::State)?;
    let mut supplier = None;
    let mut percent = 0;
    for &owner in inputs.cached_majors {
        if !relations
            .is_at_war(owner, enemy)
            .map_err(|_| Error::Missing("plunder supplier war"))?
        {
            continue;
        }
        let values = player(state, owner)?.diplomacy.war_and_alliance_modifiers;
        if values[3] > 0 || values[4] > 0 {
            supplier = Some(owner);
            percent = values[4];
        }
    }
    let Some(supplier) = supplier else {
        return Ok((vec![], percent));
    };
    let mut partners = vec![];
    for &owner in inputs.cached_majors {
        if owner == supplier
            || (relations
                .is_allied(owner, supplier)
                .map_err(|_| Error::Missing("plunder supplier alliance"))?
                && relations
                    .is_at_war(owner, enemy)
                    .map_err(|_| Error::Missing("plunder allied war"))?)
        {
            partners.push(owner);
        }
    }
    Ok((partners, percent))
}
fn positive_sum(a: i32, b: i32) -> bool {
    // Native JLE consumes flags of ADD, so signed overflow does not change the
    // mathematical positivity test even though the resulting amount wraps.
    i64::from(a) + i64::from(b) > 0
}
impl<E: Services> Current<'_, E> {
    fn check(&self, state: &LoadedState) -> Result<(), Error<E::Error>> {
        if state.binding() != &self.definitions.binding {
            Err(Error::Binding)
        } else {
            Ok(())
        }
    }
    fn base(
        &mut self,
        state: &LoadedState,
        actor: UnitReference,
        plot: usize,
        scaled: bool,
    ) -> Result<i32, Error<E::Error>> {
        let p = state
            .world()
            .map()
            .plots
            .get(plot)
            .ok_or(Error::Missing("plunder plot"))?;
        let i = self
            .definitions
            .improvements
            .find_index(p.improvement_hash)
            .ok_or(Error::Missing("plunder improvement"))?;
        let mut amount = self.definitions.plunder[i].1;
        if scaled {
            amount = self
                .services
                .scaled_improvement_plunder(state, actor, amount)
                .map_err(Error::Effect)?;
        }
        let modifier = player(state, actor.player())?.header.ordered_scalars[2];
        if modifier > 0 {
            amount = amount.wrapping_mul(modifier.wrapping_add(100)) / 100;
        }
        Ok(amount)
    }
    pub fn post_pillage_effects(
        &mut self,
        state: &mut LoadedState,
        actor: UnitReference,
        plot: usize,
    ) -> Result<(), Error<E::Error>> {
        self.check(state)?;
        unit(state, actor)?;
        if actor.player() == -1 {
            return Ok(());
        }
        let p = state
            .world()
            .map()
            .plots
            .get(plot)
            .ok_or(Error::Missing("post-pillage plot"))?;
        let Some(a) = p.city_attributes.as_ref() else {
            return Ok(());
        };
        if a.owning_city == u32::MAX {
            return Ok(());
        }
        let city = (i32::from(a.player), a.owning_city);
        // Native EditCity returns null for an absent owning-city identity.
        if !state.retained().players.iter().any(|p| {
            p.requested_owner == city.0
                && p.cities
                    .records
                    .entries
                    .iter()
                    .any(|c| c.index as u32 == city.1)
        }) {
            return Ok(());
        }
        let own = city.0 == actor.player();
        let at_war = |s: &LoadedState| {
            s.game_relations()
                .map_err(Error::State)?
                .is_at_war(actor.player(), city.0)
                .map_err(|_| Error::Missing("post-pillage war relation"))
        };
        let amount = if own {
            player(state, actor.player())?
                .cultural_identity
                .post_pillage_local_loyalty
        } else if at_war(state)? {
            player(state, actor.player())?
                .cultural_identity
                .post_pillage_foreign_loyalty
        } else {
            0
        };
        if amount != 0 {
            self.services
                .change_loyalty(state, city, amount.wrapping_shl(8))
                .map_err(Error::Effect)?;
            self.services
                .report(
                    state,
                    Report::Loyalty {
                        actor,
                        city,
                        amount,
                    },
                )
                .map_err(Error::Effect)?;
        }
        if at_war(state)? {
            let requested = property(state, actor)?;
            if requested != 0 {
                let before = unit(state, actor)?.i_damage;
                self.services
                    .change_damage(state, actor, requested.wrapping_neg())
                    .map_err(Error::Effect)?;
                let actual = before.wrapping_sub(unit(state, actor)?.i_damage);
                self.services
                    .report(
                        state,
                        Report::HealSignal {
                            actor,
                            requested,
                            actual,
                        },
                    )
                    .map_err(Error::Effect)?;
            }
        }
        Ok(())
    }
    pub fn award_plunder(
        &mut self,
        state: &mut LoadedState,
        actor: UnitReference,
        plot: usize,
    ) -> Result<(), Error<E::Error>> {
        self.check(state)?;
        let p = state
            .world()
            .map()
            .plots
            .get(plot)
            .ok_or(Error::Missing("plunder plot"))?;
        if p.city_attributes
            .as_ref()
            .is_some_and(|a| a.district != u32::MAX)
        {
            return Err(Error::Missing("district plunder"));
        }
        let i = self
            .definitions
            .improvements
            .find_index(p.improvement_hash)
            .ok_or(Error::Missing("plunder improvement"))?;
        let kind = self.definitions.plunder[i].0;
        for current in 0..5 {
            let base = if current == kind {
                self.base(state, actor, plot, current != 4)?
            } else {
                0
            };
            let target = state.world().map().plots[plot]
                .city_attributes
                .as_ref()
                .map_or(-1, |a| i32::from(a.player));
            let (partners, percent) = joint_war_partners(state, &self.inputs, target)?;
            let hash = state.world().map().plots[plot].improvement_hash;
            let tail = player(state, actor.player())?
                .tail
                .as_ref()
                .and_then(|t| t.alive.as_ref())
                .ok_or(Error::Missing("current additional pillaging owner"))?;
            let amounts = tail
                .additional_pillaging
                .iter()
                .filter(|a| a.improvement_hash == hash && a.plunder_type == current)
                .map(|a| a.amount)
                .collect::<Vec<_>>();
            let mut extra = 0i32;
            for value in amounts {
                let value = if current == 4 {
                    value
                } else {
                    self.services
                        .scaled_improvement_plunder(state, actor, value)
                        .map_err(Error::Effect)?
                };
                extra = extra.wrapping_add(value);
            }
            let modifier = player(state, actor.player())?.header.ordered_scalars[2];
            if modifier > 0 {
                extra = extra.wrapping_mul(modifier.wrapping_add(100)) / 100;
            }
            if !positive_sum(base, extra) {
                continue;
            }
            let amount = base.wrapping_add(extra);
            if current == 4 {
                let heal = self.base(state, actor, plot, false)?;
                self.services
                    .change_damage(state, actor, heal.wrapping_neg())
                    .map_err(Error::Effect)?;
            } else {
                self.services
                    .change_yield(state, actor.player(), current, amount.wrapping_shl(8))
                    .map_err(Error::Effect)?;
                for partner in partners {
                    if partner != actor.player() {
                        self.services
                            .change_yield(
                                state,
                                partner,
                                current,
                                (percent.wrapping_mul(amount) / 100).wrapping_shl(8),
                            )
                            .map_err(Error::Effect)?;
                    }
                }
            }
            self.services
                .report(
                    state,
                    Report::Yield {
                        actor,
                        kind: current,
                        amount,
                        status: 0x30ae2a3d,
                    },
                )
                .map_err(Error::Effect)?;
            let faith = unit(state, actor)?
                .k_religion
                .i_plunder_improvement_faith_yield;
            if faith != 0 {
                self.services
                    .change_yield(state, actor.player(), 3, faith.wrapping_shl(8))
                    .map_err(Error::Effect)?;
                self.services
                    .report(
                        state,
                        Report::Yield {
                            actor,
                            kind: 3,
                            amount: faith,
                            status: 0x30ae2a3d,
                        },
                    )
                    .map_err(Error::Effect)?;
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::{name_to_plunder_type, positive_sum};
    #[test]
    fn stock_none_plunder_rows_use_the_native_fallback() {
        for value in [
            b"PLUNDER_NONE".as_slice(),
            b"NO_PLUNDER".as_slice(),
            b"".as_slice(),
        ] {
            assert_eq!(
                name_to_plunder_type(crate::registry::linux_type_hash(value)),
                -1
            );
        }
        assert_eq!(
            name_to_plunder_type(crate::registry::linux_type_hash(b"PLUNDER_HEAL")),
            4
        );
    }
    #[test]
    fn award_add_flags_preserve_signed_overflow_condition() {
        assert!(positive_sum(i32::MAX, 1));
        assert!(!positive_sum(i32::MIN, -1));
        assert!(!positive_sum(5, -5));
    }
}
