// Injected only into the private committed-source review snapshot's existing
// loaded_state::tests module. No public native factory or production edit.
mod independent_diplomacy_review {
    use super::*;
    use crate::diplomacy_manager::{EmptyOpenSessions, Manager, PendingAction, PendingStateSnapshot,
        PlayerHandle, ProcessStatement, StatementHandle};
    use crate::unit_operation_aftermath::{Error as DrainError, Pending, Session};

    fn carrier() -> LoadedState { carrier_owners(&[1,7]) }
    fn carrier_owners(owners: &[i32]) -> LoadedState {
        let mut s = state();
        let map = s.world.map().clone();
        let mut players = vec![];
        s.retained.players.clear();
        // Owned World order differs from the supplied native allocation order.
        for &owner in owners {
            let mut raw = crate::core_player::tests::source_first_player();
            raw.requested_owner = owner;
            raw.header.owner = owner;
            raw.header.ordered_flags[1] = true;
            raw.formations.entries.clear();
            raw.units.records.entries.clear();
            let (player, retained) = loaded_components::split_player(raw).unwrap();
            players.push(player);
            s.retained.players.push(retained);
        }
        s.world = world::World::new(map, players).unwrap();
        s
    }
    fn bind(s: &mut LoadedState, status7: i32, status1: i32) -> ObservedSnapshotWitness {
        let w = witness(s);
        let mut c = controls(s);
        c.player_slots = [false;65];
        c.player_mapping = [-1;65];
        c.players.clear();
        for (index, (owner, status)) in [(7, status7), (1, status1)].into_iter().enumerate() {
            c.player_slots[owner as usize] = true;
            c.player_mapping[owner as usize] = index as i32;
            c.players.push(PlayerControls { owner, slot_status:status, team:owner,
                is_alive:false, turn_active:false });
        }
        s.bind_observed_snapshot_controls(w.clone(),c).unwrap();
        w
    }
    fn snapshot(w: ObservedSnapshotWitness) -> PendingStateSnapshot {
        PendingStateSnapshot { witness:w,/* REVIEW_ALLOCATION_FIELD */ever_alive:vec![PlayerHandle{owner:7},PlayerHandle{owner:1}],
            open_sessions:EmptyOpenSessions::from_header(0x1000,0x1000,0x1000,0).unwrap(),
            pending_actions:vec![PendingAction{id:u32::MAX,from:i32::MIN,to:i32::MAX}] }
    }
    fn ready(status7:i32,status1:i32) -> LoadedState {
        let mut s=carrier();
        let w=bind(&mut s,status7,status1);
        s.install_diplomacy_manager(snapshot(w)).unwrap();
        s
    }
    fn session(owner:i32, id:u64) -> Session<StatementHandle> {
        Session{from:owner,to:3,pending:vec![Pending{from:owner,to:3,statement:StatementHandle(id)}]}
    }
    #[test]
    fn equal_save_hash_cannot_replace_runtime_facts_configuration_or_role() {
        for field in 0..4 {
            let mut s=carrier();let w=bind(&mut s,2,2);let mut altered=w.clone();
            match field {0=>altered.capture_runtime_sha256[0]^=1,
                1=>altered.capture_facts_sha256[0]^=1,
                2=>altered.capture_configuration_sha256[0]^=1,
                _=>altered.source_role=core_player::initialization::WireRole::ReadInputCore}
            assert_eq!(altered.snapshot_sha256,w.snapshot_sha256);
            assert!(matches!(s.install_diplomacy_manager(snapshot(altered)),Err(Error::Binding)));
            assert!(s.diplomacy_manager().is_err());assert_eq!(s.revision(),0);
            s.install_diplomacy_manager(snapshot(w)).unwrap();
        }
    }
    #[test]
    fn bad_membership_is_atomic_and_cannot_sort_or_duplicate_handles() {
        for owners in [vec![7],vec![7,7],vec![7,63]] {
            let mut s=carrier();let w=bind(&mut s,2,2);let mut snap=snapshot(w.clone());
            snap.ever_alive=owners.into_iter().map(|owner|PlayerHandle{owner}).collect();
            assert!(s.install_diplomacy_manager(snap).is_err());
            assert!(s.diplomacy_manager().is_err());assert!(s.diplomacy_manager_source().is_none());
            s.install_diplomacy_manager(snapshot(w.clone())).unwrap();
            assert_eq!(s.diplomacy_manager().unwrap().ever_alive(),[PlayerHandle{owner:7},PlayerHandle{owner:1}]);
            assert!(matches!(s.install_diplomacy_manager(snapshot(w)),Err(Error::AlreadyBound)));
        }
    }
    struct Record(Vec<u64>);
    impl ProcessStatement<LoadedState> for Record {
        fn process_statement(&mut self,s:&mut LoadedState,h:StatementHandle)->Result<(),DrainError>{
            // Removal must already be committed before synchronous callbacks.
            assert!(!s.diplomacy_manager().unwrap().open_sessions().iter().any(|x|x.pending.iter().any(|p|p.statement==h)));
            self.0.push(h.0);Ok(())
        }
    }
    #[test]
    fn allocation_order_not_id_order_or_alive_human_filter() {
        let mut s=ready(2,2);let actions=s.diplomacy_manager().unwrap().pending_actions().to_vec();
        s.diplomacy_manager_mut().unwrap().open_sessions_mut().extend([session(1,1),session(7,7)]);
        let before=s.revision();let mut p=Record(vec![]);let mut budget=9;
        s.process_pending_diplomacy(&mut p,&mut budget).unwrap();
        assert_eq!(p.0,[7,1]);assert_eq!(s.revision(),before+2);assert_eq!(budget,7);
        assert_eq!(s.diplomacy_manager().unwrap().pending_actions(),actions);
        assert_eq!(s.world.players()[0].owner(),1);
    }
    struct Reentrant { seen:Vec<u64> }
    impl ProcessStatement<LoadedState> for Reentrant {
        fn process_statement(&mut self,s:&mut LoadedState,h:StatementHandle)->Result<(),DrainError>{
            assert!(!s.diplomacy_manager().unwrap().open_sessions().iter().any(|x|x.pending.iter().any(|p|p.statement==h)));
            self.seen.push(h.0);
            if h.0==70 {
                // PRIVATE source-only model of missing configuration mutation
                // effects: production has no setter and cannot admit such a
                // Statement subtype yet. This only checks read timing.
                for c in &mut s.controls.as_mut().unwrap().players {
                    c.slot_status=if c.owner==7 {3}else{2};
                }
                let actions=s.diplomacy_manager().unwrap().pending_actions().to_vec();
                let mut replacement=Manager::from_observed_empty_sessions(
                    EmptyOpenSessions::from_has_pending_session_statements(false).unwrap(),
                    vec![PlayerHandle{owner:7}],actions).unwrap();
                replacement.open_sessions_mut().extend([session(7,71),session(1,10)]);
                // A live owner-inventory replacement must not replace the
                // already-copied iteration handles for this drain.
                *s.diplomacy_manager_mut().unwrap()=replacement;
            }
            Ok(())
        }
    }
    #[test]
    fn callbacks_reread_head_and_later_is_ai_but_not_current_player_is_ai() {
        let mut s=ready(2,3);
        s.diplomacy_manager_mut().unwrap().open_sessions_mut().push(session(7,70));
        let before=s.revision();let provenance=s.diplomacy_manager_source().unwrap().clone();
        let mut p=Reentrant{seen:vec![]};let mut budget=10;
        s.process_pending_diplomacy(&mut p,&mut budget).unwrap();
        assert_eq!(p.seen,[70,71,10]);assert_eq!(budget,7);
        assert_eq!(s.revision(),before+4); // three removals + callback owner write
        assert_eq!(s.diplomacy_manager_source(),Some(&provenance));
        assert_eq!(s.diplomacy_manager().unwrap().ever_alive(),[PlayerHandle{owner:7}]);
    }
    #[test]
    fn human_outgoing_waits_and_external_decision_is_an_explicit_partial_diagnostic() {
        let mut s=ready(3,2);
        s.diplomacy_manager_mut().unwrap().open_sessions_mut().extend([session(7,7),session(1,1)]);
        struct Decide;
        impl ProcessStatement<LoadedState> for Decide {
            fn process_statement(&mut self,s:&mut LoadedState,h:StatementHandle)->Result<(),DrainError>{
                assert_eq!(h,StatementHandle(1));
                assert!(s.diplomacy_manager().unwrap().open_sessions()[1].pending.is_empty());
                Err(DrainError::ExternalDecisionRequired("review external policy boundary"))
            }
        }
        let before=s.revision();let mut budget=8;
        assert_eq!(s.process_pending_diplomacy(&mut Decide,&mut budget),Err(DrainError::ExternalDecisionRequired("review external policy boundary")));
        assert_eq!(budget,7);assert_eq!(s.revision(),before+1);
        assert_eq!(s.diplomacy_manager().unwrap().open_sessions()[0].pending[0].statement,StatementHandle(7));
    }
    #[test]
    fn diagnostic_budget_removal_is_not_an_accepted_action_or_native_rule() {
        let mut s=ready(2,2);let mut p=Record(vec![]);
        s.diplomacy_manager_mut().unwrap().open_sessions_mut().extend([session(7,7),session(1,1)]);
        let before=s.revision();let mut budget=0;
        assert_eq!(s.process_pending_diplomacy(&mut p,&mut budget),Err(DrainError::DiagnosticBudgetExceeded));
        assert!(p.0.is_empty());assert_eq!(s.revision(),before+1);
        assert!(s.diplomacy_manager().unwrap().open_sessions()[0].pending.is_empty());
        assert_eq!(s.diplomacy_manager().unwrap().open_sessions()[1].pending.len(),1);
    }
    #[test]
    fn pending_removal_exhaustion_must_leave_statement_and_revision_intact() {
        let mut s=ready(2,2);
        s.diplomacy_manager_mut().unwrap().open_sessions_mut().push(session(7,7));
        s.revision=u64::MAX;let mut p=Record(vec![]);let mut budget=1;
        assert!(s.process_pending_diplomacy(&mut p,&mut budget).is_err());
        assert!(!s.is_pristine());assert_eq!(s.revision(),u64::MAX);
        assert!(p.0.is_empty());assert_eq!(budget,1);
        assert_eq!(s.diplomacy_manager().unwrap().open_sessions()[0].pending.len(),1);
    }
    #[test]
    fn empty_drain_at_exhaustion_remains_read_only() {
        let mut s=ready(2,2);s.revision=u64::MAX;let mut p=Record(vec![]);let mut budget=0;
        s.process_pending_diplomacy(&mut p,&mut budget).unwrap();
        assert_eq!(s.revision(),u64::MAX);assert!(p.0.is_empty());
    }
    fn owner64_snapshot(omit:bool)->(LoadedState,PendingStateSnapshot) {
        // Conditional source boundary: slot count64 does not imply ownerID<64.
        // This is not evidence of an actual initialized native owner64 capture.
        let mut s=carrier_owners(&[1,64]);let w=witness(&s);let mut c=controls(&s);
        c.player_slots=[false;65];c.player_mapping=[-1;65];c.players.clear();
        for (i,owner) in [64,1].into_iter().enumerate() {
            c.player_slots[owner as usize]=true;c.player_mapping[owner as usize]=i as i32;
            c.players.push(PlayerControls{owner,slot_status:2,team:owner,is_alive:false,turn_active:false});
        }
        s.bind_observed_snapshot_controls(w.clone(),c).unwrap();
        let mut snap=snapshot(w);snap.ever_alive=vec![PlayerHandle{owner:1}];
        if !omit {snap.ever_alive.insert(0,PlayerHandle{owner:64});}
        /* REVIEW_OWNER64_ALLOCATION */
        (s,snap)
    }
    #[test]
    fn owner64_is_not_silently_omitted_from_complete_inventory() {
        let (mut s,snap)=owner64_snapshot(true);
        assert!(s.install_diplomacy_manager(snap).is_err(),"incomplete owner64 inventory admitted");
        assert!(s.diplomacy_manager().is_err());
    }
    #[test]
    fn owner64_can_occupy_an_observed_first64_allocation_slot() {
        let (mut s,snap)=owner64_snapshot(false);
        s.install_diplomacy_manager(snap).expect("allocation-slot bound is not an owner-ID bound");
        assert_eq!(s.diplomacy_manager().unwrap().ever_alive(),[PlayerHandle{owner:64},PlayerHandle{owner:1}]);
    }
    /* REVIEW_ALLOCATION_TESTS */
    #[test]
    fn revision_exhaustion_must_not_restore_pristine_admission() {
        let mut s=ready(2,2);s.revision=u64::MAX;
        let _result=s.diplomacy_manager_mut();
        assert!(!s.is_pristine(),"mutation overflow reopened pristine admission");
    }
}
