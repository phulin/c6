//! Distinct PolicyObsoleted subscribers on the ongoing stock modifier owners.
use super::{Effects, Error, Runtime};
use crate::loaded_actions::{policy_actions::modifiers, stock_notifications::processing};
use c6_core::{core_player::initialization::WireBinding, loaded_state::LoadedState};

#[derive(Clone, Copy, Debug)]
pub enum Listener {
    /// The actual obsolete signal target is the same detach method as retraction.
    DetachGameEffects,
    /// PolicyChanged(owner, policy, false, true); nonempty hooks stay unsupported.
    Script,
}
pub struct Binding {
    binding: WireBinding,
    listeners: Vec<Listener>,
    modifiers: modifiers::Definitions,
    publication_qualified: bool,
}
impl<P> Runtime<'_, P> {
    /// Bind the actual obsolete signal order and qualified stock report closure
    /// from this World's initialization. This does not admit Retracted listeners
    /// as substitutes: the script arguments and native signal identity differ.
    pub fn bind_policy_obsolescence(
        &mut self,
        state: &LoadedState,
        rules: &crate::Rules,
        listeners: Vec<Listener>,
        publication_qualified: bool,
    ) -> Result<(), String> {
        self.check(state).map_err(|e| e.to_string())?;
        if self.policy_obsolescence.is_some() {
            return Err("current obsolete-policy binding already supplied".into());
        }
        self.policy_obsolescence = Some(Binding {
            binding: state.binding().clone(),
            listeners,
            modifiers: modifiers::Definitions::from_rules(state, rules)
                .map_err(|e| format!("{e:?}"))?,
            publication_qualified,
        });
        Ok(())
    }
}
struct YieldReports;
impl crate::city_production_turn::modifier_effects::Effects for YieldReports {
    fn report(
        &mut self,
        _: &mut LoadedState,
        _: i32,
        _: u32,
        _: crate::city_production_turn::modifier_effects::Report,
    ) -> Result<(), crate::city_production_turn::Error> {
        // Same qualified stock ef1e2348 closure as explicit policy removal.
        Ok(())
    }
}
impl<P> Effects<'_, '_, P> {
    pub(super) fn policy_obsoleted(
        &mut self,
        state: &mut LoadedState,
        owner: i32,
        policy: i32,
        hash: u32,
    ) -> Result<(), Error> {
        let binding = self.backend.policy_obsolescence.take().ok_or_else(|| {
            Error::PolicyObsolescence("current PolicyObsoleted subscribers".into())
        })?;
        let result = (|| {
            if binding.binding != *state.binding() {
                return Err("current PolicyObsoleted binding".into());
            }
            for listener in &binding.listeners {
                match listener {
                    Listener::DetachGameEffects => self
                        .backend
                        .policy_modifier_event(
                            state,
                            owner,
                            policy,
                            hash,
                            false,
                            &binding.modifiers,
                            &mut YieldReports,
                        )
                        .map_err(|e| format!("{e:?}"))?,
                    Listener::Script => self
                        .backend
                        .script
                        .call_empty(state, "PolicyChanged", &[owner, policy])
                        .map_err(|e| format!("{e:?}"))?,
                }
            }
            Ok(())
        })();
        self.backend.policy_obsolescence = Some(binding);
        result.map_err(Error::PolicyObsolescence)
    }
    pub(super) fn obsolete_publication_qualified(&self, state: &LoadedState) -> Result<(), Error> {
        if !self
            .backend
            .policy_obsolescence
            .as_ref()
            .is_some_and(|b| b.binding == *state.binding() && b.publication_qualified)
        {
            return Err(Error::Notification(processing::Error::Unsupported(
                "current obsolete-policy stock publication closure",
            )));
        }
        Ok(())
    }
}
