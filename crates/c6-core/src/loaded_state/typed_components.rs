//! Consume typed original component exports without running a whole Core parser.
//! This API checks consistency; the importing adapter authenticates each source.
use super::*;
use crate::core_player::initialization::WireRole;
use sha2::{Digest, Sha256};

impl LoadedState {
    /// Assemble independently decoded components into one canonical owner.
    /// `image` is the exact attributed component bundle, not a fabricated Save.
    /// Every retained component must be supplied; absent components fail with
    /// Missing rather than receiving native-looking defaults. Runtime arrays,
    /// controls and other observations remain unbound until explicitly installed.
    /// Decode counters are zero because this method does not decode the bundle.
    pub fn from_typed_components(
        image: CoreImage,
        binding: WireBinding,
        world: world::World,
        retained: Option<RetainedComponents>,
        limits: AssemblyLimits,
    ) -> Result<Self, Error> {
        if binding.role != WireRole::SavedOutputComponents
            || binding.core_sha256 != <[u8; 32]>::from(Sha256::digest(image.as_ref()))
            || binding.occurrence_id.is_empty()
            || binding.output_id.as_ref().is_none_or(Vec::is_empty)
        {
            return Err(Error::Binding);
        }
        let retained = retained.ok_or(Error::Missing(MissingInput::RetainedComponents))?;
        if world.players().len() != retained.players.len() {
            return Err(Error::IncompletePlayers);
        }
        let mut units = 0usize;
        let mut formations = 0usize;
        for player in world.players() {
            let mut owners = retained
                .players
                .iter()
                .filter(|p| p.header.owner == player.owner());
            let metadata = owners
                .next()
                .ok_or(Error::Missing(MissingInput::Player(player.owner())))?;
            if owners.next().is_some()
                || metadata.requested_owner != player.owner()
                || metadata
                    .initialization_snapshot
                    .as_ref()
                    .is_some_and(|s| s.facts().binding != binding)
            {
                return Err(Error::Binding);
            }
            units = units
                .checked_add(player.unit_records().map_err(Error::World)?.len())
                .ok_or(Error::ResourceLimit)?;
            formations = formations
                .checked_add(player.formation_records().map_err(Error::World)?.len())
                .ok_or(Error::ResourceLimit)?;
        }
        if world.players().len() > limits.max_roster_records
            || units
                .checked_add(formations)
                .is_none_or(|n| n > limits.max_roster_records)
        {
            return Err(Error::ResourceLimit);
        }
        let statistics = ImportStatistics {
            players: world.players().len(),
            units,
            formations,
            consumed: 0,
            items_used: 0,
        };
        Ok(Self {
            image,
            binding,
            world,
            retained,
            statistics,
            controls: None,
            runtime: None,
            formation_runtime: None,
            combat_proposals: None,
            game_sync_runtime: None,
            game_city_names: None,
            limits,
            revision: 0,
            instance_identity: fresh_state_identity()?,
            admission_generation: 0,
            observed_snapshot: None,
            game_relations: None,
            game_relations_source: None,
            object_pairing: None,
            object_pairing_source: None,
            diplomacy_manager: None,
            diplomacy_manager_source: None,
            player_notifications: None,
            player_notifications_source: None,
            notification_runtime_projection_installed: false,
            notification_pools: Vec::new(),
            notification_pool_projection_installed: false,
            game_techs: None,
            game_techs_source: None,
            trade_graph: None,
            trade_graph_source: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inputs() -> (CoreImage, WireBinding, world::World, RetainedComponents) {
        let LoadedState {
            image,
            mut binding,
            world,
            mut retained,
            ..
        } = super::super::tests::state();
        // Synthetic typed records are reused, not their unrelated source-only
        // Military snapshot. This constructor must not invent that metadata.
        for player in &mut retained.players {
            player.initialization_snapshot = None;
        }
        binding.role = WireRole::SavedOutputComponents;
        binding.output_id = Some(b"actual-component-export-occurrence".to_vec());
        (image, binding, world, retained)
    }
    #[test]
    fn consumes_typed_components_without_inventing_runtime_or_wire_decode() {
        let (image, binding, world, retained) = inputs();
        let expected = world.players()[0].unit_records().unwrap().len();
        let state = LoadedState::from_typed_components(
            image,
            binding,
            world,
            Some(retained),
            AssemblyLimits::default(),
        )
        .unwrap();
        assert_eq!(state.statistics().units, expected);
        assert_eq!(state.statistics().consumed, 0);
        assert!(
            state.controls.is_none()
                && state.runtime.is_none()
                && state.formation_runtime.is_none()
        );
        assert!(state.observed_snapshot().is_none());
        assert_eq!(state.revision(), 0);
        assert!(matches!(
            state.trade_graph(),
            Err(Error::Missing(MissingInput::TradeGraph))
        ));
    }
    #[test]
    fn missing_components_bad_identity_and_limits_are_explicit() {
        let (image, binding, world, _) = inputs();
        assert!(matches!(
            LoadedState::from_typed_components(
                image,
                binding,
                world,
                None,
                AssemblyLimits::default()
            ),
            Err(Error::Missing(MissingInput::RetainedComponents))
        ));
        let (image, mut binding, world, retained) = inputs();
        binding.core_sha256[0] ^= 1;
        assert!(matches!(
            LoadedState::from_typed_components(
                image,
                binding,
                world,
                Some(retained),
                AssemblyLimits::default()
            ),
            Err(Error::Binding)
        ));
        let (image, binding, world, retained) = inputs();
        assert!(matches!(
            LoadedState::from_typed_components(
                image,
                binding,
                world,
                Some(retained),
                AssemblyLimits {
                    max_roster_records: 0,
                    ..AssemblyLimits::default()
                }
            ),
            Err(Error::ResourceLimit)
        ));
        let (image, binding, world, mut retained) = inputs();
        retained.players.clear();
        assert!(matches!(
            LoadedState::from_typed_components(
                image,
                binding,
                world,
                Some(retained),
                AssemblyLimits::default()
            ),
            Err(Error::IncompletePlayers)
        ));
    }
}
