//! Single canonical Game.NameManager owner. Player.NameManager is unrelated.
use super::{Error, LoadedState, MissingInput, ObservedSnapshotWitness, WireBinding};
use crate::game_name_manager_wire::NameManager;
#[derive(Debug)]
pub(super) struct Owner {
    pub value: NameManager,
    pub source: Option<ObservedSnapshotWitness>,
}
impl LoadedState {
    /// Supplied initialized state, not a default inferred from an absent field.
    /// Capture adapters authenticate its actual epoch; model-origin construction
    /// uses None without claiming an original-runtime observation.
    pub fn install_game_city_names(
        &mut self,
        binding: &WireBinding,
        names: NameManager,
        source: Option<ObservedSnapshotWitness>,
    ) -> Result<(), Error> {
        if binding != &self.binding {
            return Err(Error::Binding);
        }
        if self.game_city_names.is_some() {
            return Err(Error::AlreadyBound);
        }
        if names.wire_version != 1 {
            return Err(Error::Binding);
        }
        if source
            .as_ref()
            .is_some_and(|s| s.capture_configuration_sha256 != binding.inputs.configuration_sha256)
        {
            return Err(Error::Binding);
        }
        let next = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        self.game_city_names = Some(Owner {
            value: names,
            source,
        });
        self.revision = next;
        Ok(())
    }
    pub fn game_city_names(&self) -> Result<&NameManager, Error> {
        self.game_city_names
            .as_ref()
            .map(|s| &s.value)
            .ok_or(Error::Missing(MissingInput::GameCityNames))
    }
    pub fn game_city_names_source(&self) -> Option<&ObservedSnapshotWitness> {
        self.game_city_names
            .as_ref()
            .and_then(|s| s.source.as_ref())
    }
    pub fn game_city_names_mut(&mut self) -> Result<&mut NameManager, Error> {
        if self.game_city_names.is_none() {
            return Err(Error::Missing(MissingInput::GameCityNames));
        }
        self.revision = self.revision.checked_add(1).ok_or(Error::ResourceLimit)?;
        Ok(&mut self.game_city_names.as_mut().expect("checked owner").value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_are_unknown_until_same_binding_install_and_mutation_advances_revision() {
        let mut state = super::super::tests::state();
        assert!(matches!(
            state.game_city_names(),
            Err(Error::Missing(MissingInput::GameCityNames))
        ));
        let binding = state.binding().clone();
        let mut wrong = binding.clone();
        wrong.inputs.configuration_sha256[0] ^= 1;
        let names = || NameManager {
            wire_version: 1,
            used_names: vec![b"Capital".to_vec()],
        };
        assert!(matches!(
            state.install_game_city_names(&wrong, names(), None),
            Err(Error::Binding)
        ));
        state
            .install_game_city_names(&binding, names(), None)
            .unwrap();
        assert!(matches!(
            state.install_game_city_names(&binding, names(), None),
            Err(Error::AlreadyBound)
        ));
        let revision = state.revision;
        state
            .game_city_names_mut()
            .unwrap()
            .used_names
            .push(b"Next".to_vec());
        assert_eq!(state.revision, revision + 1);
        assert_eq!(state.game_city_names().unwrap().used_names.len(), 2);
    }
}
