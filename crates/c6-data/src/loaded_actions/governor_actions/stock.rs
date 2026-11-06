//! Stock governor subscribers over the Session's persistent modifier owners.
use super::{Definitions, Effects, Event};
use crate::{found_city::cultural_identity as identity, loaded_actions::stock_notifications};
use c6_core::loaded_state::LoadedState;

#[derive(Clone, Copy, Debug)]
pub enum Scope {
    /// Stock cache/summary/achievement consumers, empty secret societies, no
    /// active governor requirements and no custom consumers of summary data.
    /// Player policy selection is external, including governor/city selection.
    /// A different live subscriber graph requires its own Effects implementation.
    StockPlainHeadlessExternalPolicies,
}
pub struct Current<'s, 'a, P> {
    pub backend: &'s mut stock_notifications::Runtime<'a, P>,
    pub scope: Scope,
}
struct Reports;
impl crate::city_production_turn::modifier_effects::Effects for Reports {
    fn report(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
        report: crate::city_production_turn::modifier_effects::Report,
    ) -> Result<(), crate::city_production_turn::Error> {
        match report {
            crate::city_production_turn::modifier_effects::Report::YieldChange { .. } => Ok(()),
            _ => Err(crate::city_production_turn::Error::Unsupported(
                "governor modifier report",
            )),
        }
    }
}
impl<P> Effects for Current<'_, '_, P> {
    type Error = String;
    fn preflight(
        &mut self,
        state: &LoadedState,
        _: &Definitions,
        owner: i32,
    ) -> Result<(), String> {
        self.backend
            .check(state)
            .map_err(|e| format!("governor stock binding: {e:?}"))?;
        self.backend
            .modifier_owners()
            .map_err(|e| format!("governor persistent modifier owners: {e:?}"))?;
        if state.governor_current(owner).is_none() {
            return Err("current governor maximum absent".into());
        }
        Ok(())
    }
    fn event(
        &mut self,
        state: &mut LoadedState,
        defs: &Definitions,
        event: Event,
    ) -> Result<(), String> {
        let (owner, report) = match event {
            Event::Promoted { owner, .. } => (owner, 0x8c6b2399),
            Event::Appointed { owner, .. } => (owner, 0x69356374),
            Event::Assigned { owner, .. } => (owner, 0xcf9c73f1),
            Event::Turn { owner, .. } => (owner, 0xcb6c9e05),
            Event::Established { owner, city, .. } => {
                let id = state
                    .retained()
                    .players
                    .iter()
                    .find(|p| p.header.owner == owner)
                    .and_then(|p| {
                        p.governors
                            .entries
                            .iter()
                            .find(|s| s.governor.definition_hash == defs.governor_hash)
                    })
                    .map(|s| s.governor.id)
                    .ok_or("established Governor instance")?;
                let recipes = defs
                    .recipes
                    .iter()
                    .map(
                        |r| crate::melee_attack::backend::city_creation::governors::Recipe {
                            id: r.id.clone(),
                            yield_hash: r.yield_hash,
                            amount: r.amount,
                        },
                    )
                    .collect::<Vec<_>>();
                self.backend
                    .with_modifier_owners(|_, owners| {
                        owners.effects.attach_governor_modifiers(
                            state,
                            &mut owners.requirements,
                            owner,
                            id,
                            &recipes,
                            &mut Reports,
                        )
                    })
                    .map_err(|e| format!("governor modifier owner: {e:?}"))?
                    .map_err(|e| format!("governor establishment modifiers: {e:?}"))?;
                if city.owner as i32 != owner {
                    return Err("governor established foreign city".into());
                }
                (owner, 0xdb0807b6)
            }
            Event::CityGovernorChanged { .. } => {
                return Err("governor assignment requires current loyalty services".into())
            }
        };
        // These stock reporting registrations publish derived City/Player cache
        // DTOs. Gameplay Message signals above execute before publication.
        self.backend
            .turn_report_records
            .push(stock_notifications::turn_reports::Record::Report {
                event: report,
                player: owner,
            });
        Ok(())
    }
}
pub struct Assignment<'s, 'a, 'q, P, Q> {
    pub current: Current<'s, 'a, P>,
    pub identity: &'q identity::Definitions,
    pub queries: &'q mut Q,
}
impl<P, Q: identity::Context> Effects for Assignment<'_, '_, '_, P, Q> {
    type Error = String;
    fn preflight(
        &mut self,
        state: &LoadedState,
        defs: &Definitions,
        owner: i32,
    ) -> Result<(), String> {
        self.current.preflight(state, defs, owner)
    }
    fn event(
        &mut self,
        state: &mut LoadedState,
        defs: &Definitions,
        event: Event,
    ) -> Result<(), String> {
        if let Event::CityGovernorChanged { city } = event {
            return identity::do_city_created(
                state,
                self.identity,
                self.queries,
                i32::from(city.owner),
                city.id,
            )
            .map_err(|e| format!("governor assignment loyalty: {e:?}"));
        }
        self.current.event(state, defs, event)
    }
}
