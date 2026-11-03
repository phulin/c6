    // Lifecycle control coverage only: these synthetic lookup definitions and
    // controls do NOT establish authentic native registry/configuration facts.
    struct ReviewLookup(Vec<u32>);
    impl c6_core::movement_world::DefinitionLookup for ReviewLookup {
        fn count(&self) -> usize { self.0.len() }
        fn find_index(&self, h: u32) -> Option<usize> { self.0.iter().position(|v| *v == h) }
        fn type_hash(&self, i: usize) -> Option<u32> { self.0.get(i).copied() }
    }
    fn review_lookup(mut values: Vec<u32>) -> ReviewLookup {
        values.sort_unstable(); values.dedup(); ReviewLookup(values)
    }
    let mut types = Vec::new();
    let mut abilities = Vec::new();
    let mut terrain = Vec::new();
    let mut features = Vec::new();
    let mut classes = Vec::new();
    let mut operations = Vec::new();
    for p in state.world().players() {
        for u in p.unit_records()? {
            let u = &u.value;
            types.push(u.e_type_index);
            abilities.extend(u.ai_abilities_possessed.iter().map(|p| p.definition_hash));
            terrain.extend(u.ab_invalid_terrain_types.iter().chain(&u.ab_ignore_terrain_types).map(|p| p.definition_hash));
            features.extend(u.ab_ignore_feature_types.iter().map(|p| p.definition_hash));
            classes.extend(u.ai_bonus_points_per_kill.iter().chain(&u.ai_bonus_points_per_kill_strength).map(|p| p.definition_hash));
            operations.extend(u.k_operations_disabled.iter().map(|p| p.definition_hash));
        }
    }
    let types = review_lookup(types);
    let abilities = review_lookup(abilities);
    let terrain = review_lookup(terrain);
    let features = review_lookup(features);
    let classes = review_lookup(classes);
    let operations = review_lookup(operations);
    let definitions = vec![c6_core::unit_initialization::UnitDefinition { domain_hash: 0 }; types.0.len()];
    let identities = state.binding().inputs.clone();
    let inputs = c6_core::unit_initialization::InitializationInputs {
        identities: &identities,
        units: c6_core::movement_world::DefinitionTable::new(&types, &definitions)?,
        abilities: &abilities, terrains: &terrain, features: &features,
        great_person_classes: &classes, operations: &operations,
    };
    use c6_core::loaded_state::Error as ReviewError;
    state.initialize_formations()?;
    assert!(state.is_pristine());
    assert!(state.processing_automation().is_err());
    assert!(matches!(state.initialize_formations(), Err(ReviewError::AlreadyInitialized)));
    state.initialize_rosters(&inputs)?;
    assert!(state.is_pristine());
    assert!(matches!(state.initialize_rosters(&inputs), Err(ReviewError::AlreadyInitialized)));
    for p in state.world().players() {
        for u in p.unit_records()? { state.unit_runtime(p.owner(), u.value.id)?; }
        for f in p.formation_records()? { state.formation_runtime(p.owner(), f.value.id)?; }
    }
    let subject = state.world().players().iter().find_map(|p| p.unit_records().ok()?.first().map(|u| (p.owner(), u.value.id))).unwrap();
    let initial_pointer = state.unit_runtime(subject.0, subject.1)? as *const _;
    fn review_controls(s: &LoadedState) -> c6_core::loaded_state::ControlInputs {
        c6_core::loaded_state::ControlInputs { binding: s.binding().clone(),
            player_mapping: [-1;65], player_slots: [false;65], players: vec![],
            processing_automation: None, pause_storage: None }
    }
    state.bind_controls(review_controls(&state))?;
    assert_eq!(initial_pointer, state.unit_runtime(subject.0, subject.1)? as *const _);
    assert!(matches!(state.bind_controls(review_controls(&state)), Err(ReviewError::AlreadyBound)));
    let (u, runtime) = state.unit_with_runtime_mut(subject.0, subject.1)?;
    let old_timer = u.i_operation_timer;
    u.i_operation_timer = 123;
    runtime.abilities.push(0x1234);
    runtime.archive.mark_dirty(32).unwrap();
    assert_eq!(state.unit_runtime(subject.0, subject.1)?.abilities.last(), Some(&0x1234));
    assert!(state.unit_runtime(subject.0, subject.1)?.archive.dirty_calls().contains(&32));
    assert!(matches!(state.initialize_rosters(&inputs), Err(ReviewError::AlreadyInitialized)));
    state.unit_mut(subject.0, subject.1)?.i_operation_timer = old_timer;
    let old_type = state.unit_mut(subject.0, subject.1)?.e_type_index;
    state.unit_mut(subject.0, subject.1)?.e_type_index ^= 1;
    assert!(matches!(state.unit_runtime(subject.0, subject.1), Err(ReviewError::RuntimeDependenciesChanged)));
    state.unit_mut(subject.0, subject.1)?.e_type_index = old_type;
    let old_formation = state.unit_mut(subject.0, subject.1)?.formation_id;
    state.unit_mut(subject.0, subject.1)?.formation_id ^= 1;
    assert!(matches!(state.unit_runtime(subject.0, subject.1), Err(ReviewError::RuntimeDependenciesChanged)));
    state.unit_mut(subject.0, subject.1)?.formation_id = old_formation;
    state.unit_mut(subject.0, subject.1)?.ai_abilities_possessed.push(unit::HashValue { definition_hash: 0, value: 42 });
    assert!(matches!(state.unit_runtime(subject.0, subject.1), Err(ReviewError::RuntimeDependenciesChanged)));
    state.unit_mut(subject.0, subject.1)?.ai_abilities_possessed.pop();
    assert_eq!(state.unit_runtime(subject.0, subject.1)?.abilities.last(), Some(&0x1234));
    assert!(matches!(state.bind_controls(review_controls(&state)), Err(ReviewError::AlreadyBound)));
    assert_eq!(state.unit_runtime(subject.0, subject.1)?.abilities.last(), Some(&0x1234));
    let formation_subject = state.world().players().iter().find_map(|p| p.formation_records().ok()?.first().map(|f| (p.owner(), f.value.id))).unwrap();
    let (record, f) = state.formation_with_runtime_mut(formation_subject.0, formation_subject.1)?;
    f.set_evaluating_operation(record, true).unwrap();
    f.archive.mark_dirty(13).unwrap();
    assert!(matches!(state.initialize_formations(), Err(ReviewError::AlreadyInitialized)));
    assert!(state.formation_runtime(formation_subject.0, formation_subject.1)?.evaluating_operation());
    assert!(state.formation_runtime(formation_subject.0, formation_subject.1)?.archive.dirty_calls().contains(&13));
    let linked = state.world().players().iter().find_map(|p| {
        p.unit_records().ok()?.iter().find_map(|u| {
            let r = state.unit_runtime(p.owner(), u.value.id).ok()?.formation?;
            Some((p.owner(), u.value.id, r))
        })
    }).expect("fixture linked Formation");
    let (target, _) = state.formation_with_runtime_mut(linked.2.owner, linked.2.requested_id)?;
    target.id ^= 0x10000;
    assert!(matches!(state.unit_runtime(linked.0, linked.1), Err(ReviewError::RuntimeDependenciesChanged)),
        "cached resolved Formation identity must track actual target");
    assert!(!state.is_pristine());
    // Test witness replay with explicitly synthetic witness metadata. This
    // exercises admission lifetime, not authenticity or this Read's live facts.
    let fresh = || LoadedState::decode(CoreImage::Unique(review_bytes.clone()), shared, player_limits,
        binding.clone(), &capture.for_binding(&binding), AssemblyLimits::default());
    let mut observed = fresh()?;
    let witness = c6_core::loaded_state::ObservedSnapshotWitness {
        source_role: WireRole::SavedOutputCore,
        snapshot_sha256: observed.binding().core_sha256,
        snapshot_bytes: observed.source_image().len(),
        capture_runtime_sha256: [0;32], capture_facts_sha256: [0;32], capture_configuration_sha256: [0;32],
    };
    observed.bind_observed_snapshot_controls(witness.clone(), review_controls(&observed))?;
    assert_eq!(observed.binding(), &binding);
    assert!(matches!(observed.bind_observed_snapshot_controls(witness.clone(), review_controls(&observed)), Err(ReviewError::AlreadyBound)));
    let mut advanced = fresh()?;
    advanced.unit_mut(subject.0, subject.1)?;
    assert!(matches!(advanced.bind_observed_snapshot_controls(witness, review_controls(&advanced)), Err(ReviewError::NotPristine)));
