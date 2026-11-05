//! Concrete stock government continuations borrowing the current Session owners.
use super::{boosts, history, requirements, stock, Definitions};
use crate::loaded_actions::{
    choose_tech, culture_notifications, notifications::processing::Backend, stock_notifications,
};
use c6_core::loaded_state::LoadedState;

pub struct Current<'s, 'q, H> {
    pub governments: &'s Definitions,
    pub boosts: &'s boosts::Definitions,
    pub influence: &'s requirements::InfluenceDefinitions,
    pub levels: &'s [(i32, u32)],
    pub history: &'s crate::melee_attack::resolve::completion::history::Definitions,
    pub history_inputs: crate::melee_attack::resolve::completion::history::Inputs<'s>,
    pub history_effects: &'s mut H,
    pub culture: &'s stock_notifications::research::CultureInputs<'q>,
    pub technologies: &'s choose_tech::Definitions,
    pub technology_graph: Option<&'s choose_tech::GameTechs>,
}
struct TriggerUnavailable;
impl boosts::Trigger for TriggerUnavailable {
    type Error = String;
    fn trigger(&mut self, _: &mut LoadedState, _: i32, _: usize) -> Result<(), String> {
        Err("positive government-tier boost requires current research continuation".into())
    }
}
struct Influence;
impl requirements::InfluenceEffects for Influence {
    type Error = String;
    fn report(&mut self, _: &mut LoadedState, _: i32, kind: u32) -> Result<(), String> {
        if kind != 0xd35e571d {
            return Err("unknown Influence report".into());
        }
        // Stock report is a numeric/read-only cache refresh. Gameplay queries
        // read the three canonical Influence values just mutated by the kernel.
        Ok(())
    }
    fn notification(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        kind: u32,
    ) -> Result<(), String> {
        if kind != 0x579b28d1 {
            return Err("unknown Influence notification".into());
        }
        if state
            .player_slot_status(owner)
            .map_err(|e| format!("{e:?}"))?
            != 3
        {
            return Ok(());
        }
        let player = state
            .retained()
            .players
            .iter()
            .find(|p| p.header.owner == owner)
            .ok_or("current Influence Player")?;
        // Same stock Influence.IsNeeded order as social::influence_needed:
        // human -> positive tokens -> CanGiveTokens -> considered.
        if player.influence.tokens_to_give <= 0 {
            return Ok(());
        }
        Err("positive Influence notification CanGiveTokens continuation".into())
    }
}
impl<'a, P, H: crate::melee_attack::resolve::completion::history::Effects>
    stock::Continuations<'a, P> for Current<'_, '_, H>
where
    H::Error: std::fmt::Debug,
{
    type Error = String;
    fn gossip(
        &mut self,
        _: &mut LoadedState,
        _: &mut stock_notifications::Runtime<'a, P>,
        _: i32,
        _: i32,
        history: &crate::game_update_state::deactivation::VisibilityHistory,
    ) -> Result<(), String> {
        if !history.history.is_empty() {
            return Err("current government Gossip delivery".into());
        }
        Ok(())
    }
    fn history(
        &mut self,
        state: &mut LoadedState,
        _: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
        government: i32,
        tiers: history::Tiers,
    ) -> Result<(), String> {
        history::enacted(
            state,
            self.governments,
            tiers,
            owner,
            government,
            self.history,
            self.history_inputs,
            self.history_effects,
        )
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
    }
    fn boost(
        &mut self,
        state: &mut LoadedState,
        _: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
        government: i32,
        row_id: i32,
    ) -> Result<(), String> {
        boosts::enacted(
            state,
            self.boosts,
            self.governments,
            owner,
            government,
            row_id,
            &mut TriggerUnavailable,
        )
        .map_err(|e| format!("{e:?}"))
    }
    fn player_event(
        &mut self,
        state: &mut LoadedState,
        _: &mut stock_notifications::Runtime<'a, P>,
        receiver_owner: i32,
        owner: i32,
        government: i32,
        enacted: bool,
    ) -> Result<(), String> {
        requirements::player_event(
            state,
            self.influence,
            receiver_owner,
            Some(owner),
            government,
            enacted,
            self.levels,
            &mut Influence,
        )
        .map_err(|e| format!("{e:?}"))
    }
    fn requirement(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        receiver: requirements::Receiver,
        owner: i32,
    ) -> Result<(), String> {
        backend
            .with_modifier_owners(|_, current| {
                requirements::changed(
                    state,
                    &mut current.requirements,
                    receiver,
                    owner,
                    &mut requirements::UnavailableContinuations,
                )
                .map_err(|e| format!("{e:?}"))
            })
            .map_err(|e| format!("{e:?}"))?
    }
    fn slots_rebuilt(
        &mut self,
        state: &mut LoadedState,
        _: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
    ) -> Result<(), String> {
        // Actual stock +198 is CultureAi.PurchaseAndSlotPolicies20300c0.
        // Its first query returns immediately for a human (20300ed..20300ef).
        if state
            .player_slot_status(owner)
            .map_err(|e| format!("{e:?}"))?
            == 3
        {
            Ok(())
        } else {
            Err("nonhuman policy purchase/slot external decision continuation".into())
        }
    }
    fn notifications(
        &mut self,
        state: &mut LoadedState,
        backend: &mut stock_notifications::Runtime<'a, P>,
        owner: i32,
    ) -> Result<(), String> {
        if !backend
            .manager_initialized(state)
            .map_err(|e| format!("{e:?}"))?
            || backend
                .game_uninitializing(state)
                .map_err(|e| format!("{e:?}"))?
            || state
                .player_slot_status(owner)
                .map_err(|e| format!("{e:?}"))?
                != 3
        {
            return Ok(());
        }
        let technology = self
            .technologies
            .player(state, owner, self.technology_graph)
            .map_err(|e| format!("{e:?}"))?;
        for kind in [
            culture_notifications::Kind::ConsiderGovernmentChange,
            culture_notifications::Kind::FillCivicSlot,
        ] {
            let needed = self
                .culture
                .definitions
                .player(
                    state,
                    owner,
                    self.culture.graph,
                    Some(&technology),
                    self.culture.starting,
                )
                .map_err(|e| format!("{e:?}"))?
                .is_needed(kind, true)
                .map_err(|e| format!("{e:?}"))?;
            if needed {
                backend
                    .publish_decision(state, owner, kind.type_hash(), [-1, -1], None)
                    .map_err(|e| format!("{e:?}"))?;
            }
        }
        Ok(())
    }
}
