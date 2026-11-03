//! Read-only native-record differential adapter for the bounded operation phase.
use c6_core::messages::NullParameterUnitOperationBody;
use c6_core::skip_turn::{self, Effect, ReplacementInputs, StartData, StartEffects};
use c6_core::unit::{ComponentId, Formation, Limits, Unit};
use std::{env, fs, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = env::args().collect();
    if a.len() != 9 {
        return Err("usage: skip_turn_kernel UNIT FORMATION COMMAND TURN LOCAL UNIT_OUT FORMATION_OUT ITERATIONS".into());
    }
    let limits = Limits::default();
    let u = Unit::decode(&fs::read(&a[1])?, limits)?;
    let f = Formation::decode(&fs::read(&a[2])?, limits)?;
    let command = fs::read(&a[3])?;
    let body = NullParameterUnitOperationBody::decode(command.get(4..).ok_or("short command")?)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(body.mode, 0, "only exact USE_REQUEST mode supported");
    let i = ReplacementInputs {
        identity: ComponentId {
            owner: body.unit.owner as i16,
            component_type: body.unit.object_type as i16,
            id: body.unit.id,
        },
        skip_turn_operation_id: body.handler.0,
        current_turn: a[4].parse()?,
        local_player: a[5].parse()?,
        directive: body.secondary,
        flags: body.flags as i32,
        append: false,
        from_user: false,
        busy: false,
        can_move: true,
        activation_can_start: true,
        activity_archive_bound: true,
        timer_archive_bound: true,
        queue_archive_bound: true,
        path_archive_bound: true,
        cache_archive_bound: true,
    };
    let r = skip_turn::replace_operation(&u, &f, i)?;
    fs::write(&a[6], r.unit.encode(limits)?)?;
    fs::write(&a[7], r.formation.encode(limits)?)?;
    for effect in &r.effects {
        match effect {
            Effect::UnitDirty(index) => println!("{{\"kind\":\"unit_dirty\",\"index\":{index}}}"),
            Effect::FormationDirty(index) => {
                println!("{{\"kind\":\"formation_dirty\",\"index\":{index}}}")
            }
            Effect::Report {
                event_hash,
                subtype_hash,
                identity,
                first,
                second,
            } => {
                let bits = (u64::from(identity.id) << 32)
                    | (u64::from(identity.component_type as u16) << 16)
                    | u64::from(identity.owner as u16);
                let ty = (u64::from(*subtype_hash) << 32) | u64::from(*event_hash);
                let v = |x: &Option<i32>| x.map_or("null".to_string(), |v| v.to_string());
                println!("{{\"kind\":\"report\",\"type_bits\":\"{ty:016x}\",\"component_bits\":\"{bits:016x}\",\"first\":{},\"second\":{}}}",v(first),v(second));
            }
        }
    }
    // Native-derived branch contracts; these modified inputs have no native fixtures.
    let mut cases = 0;
    for modify in [
        |v: &mut ReplacementInputs| v.append = true,
        |v: &mut ReplacementInputs| v.from_user = true,
        |v: &mut ReplacementInputs| v.flags = 1,
        |v: &mut ReplacementInputs| v.directive = 0,
        |v: &mut ReplacementInputs| v.identity.id ^= 1,
        |v: &mut ReplacementInputs| v.busy = true,
        |v: &mut ReplacementInputs| v.activation_can_start = false,
    ] {
        let mut unsupported = i;
        modify(&mut unsupported);
        assert!(skip_turn::replace_operation(&u, &f, unsupported).is_err());
        cases += 1;
    }
    let mut pending = u.clone();
    pending.b_needs_movement_completion = true;
    assert_eq!(
        skip_turn::replace_operation(&pending, &f, i),
        Err(skip_turn::Error::MovementCompletionPending)
    );
    cases += 1;
    let mut unsupported_queue = u.clone();
    unsupported_queue
        .k_operations
        .nodes
        .push(unsupported_queue.k_operations.nodes[0].clone());
    assert_eq!(
        skip_turn::replace_operation(&unsupported_queue, &f, i),
        Err(skip_turn::Error::UnsupportedQueue)
    );
    let mut automated = u.clone();
    automated.e_automation = 0;
    assert_eq!(
        skip_turn::replace_operation(&automated, &f, i),
        Err(skip_turn::Error::UnsupportedAutomation)
    );
    let mut wrong_formation = f.clone();
    wrong_formation.unit_ids.push(u.id);
    assert_eq!(
        skip_turn::replace_operation(&u, &wrong_formation, i),
        Err(skip_turn::Error::FormationMismatch)
    );
    cases += 3;
    let mut empty = u.clone();
    empty.k_operations.nodes.clear();
    empty.e_activity = 0x6b25012e;
    let e = skip_turn::replace_operation(&empty, &f, i)?;
    assert_eq!(
        e.effects
            .iter()
            .filter(|e| matches!(e, Effect::FormationDirty(14)))
            .count(),
        1
    );
    assert_eq!(
        r.effects
            .iter()
            .filter(|e| matches!(e, Effect::FormationDirty(14)))
            .count(),
        3
    );
    cases += 1;
    let mut no_move = i;
    no_move.can_move = false;
    let n = skip_turn::replace_operation(&u, &f, no_move)?;
    assert!(!n.effects.iter().any(|e| matches!(
        e,
        Effect::Report {
            event_hash: 0xe23b112d,
            first: Some(0x490cc8e1),
            ..
        }
    )));
    cases += 1;
    let mut detached = i;
    detached.activity_archive_bound = false;
    detached.timer_archive_bound = false;
    detached.queue_archive_bound = false;
    detached.path_archive_bound = false;
    detached.cache_archive_bound = false;
    let d = skip_turn::replace_operation(&u, &f, detached)?;
    assert!(!d
        .effects
        .iter()
        .any(|e| matches!(e, Effect::UnitDirty(_) | Effect::FormationDirty(_))));
    assert_eq!(d.unit, r.unit);
    assert_eq!(d.formation, r.formation);
    cases += 1;
    // A synchronous start report can change activity before the setter reads it.
    struct Subscriber {
        dirty: usize,
        reports: usize,
    }
    impl StartEffects for Subscriber {
        fn mark_activity_dirty(&mut self, _: &Unit, _: u32) {
            self.dirty += 1;
        }
        fn send(&mut self, u: &mut Unit, r: skip_turn::IdentityIntReport) -> i32 {
            self.reports += 1;
            if r.event_hash == 0x3d4006f2 {
                u.e_activity = skip_turn::HOLD;
            }
            37
        }
    }
    let mut changed = u.clone();
    let mut sub = Subscriber {
        dirty: 0,
        reports: 0,
    };
    let mut data = StartData {
        delete: false,
        action: true,
    };
    skip_turn::start(&mut changed, i.identity, &mut data, true, &mut sub)?;
    assert_eq!(
        (sub.dirty, sub.reports, data.delete, data.action),
        (0, 1, true, true)
    );
    cases += 1;
    // Deactivate sets literal AWAKE, regardless of the old node's saved activity.
    let mut saved = u.clone();
    saved.k_operations.nodes[0].previous_activity = i32::MIN;
    assert_eq!(skip_turn::replace_operation(&saved, &f, i)?.unit, r.unit);
    cases += 1;
    let iterations: u32 = a[8].parse()?;
    let start = Instant::now();
    for _ in 0..iterations {
        std::hint::black_box(skip_turn::replace_operation(
            std::hint::black_box(&u),
            std::hint::black_box(&f),
            i,
        )?);
    }
    println!("{{\"kind\":\"checks\",\"synthetic_no_native_fixture_cases\":{cases},\"iterations\":{iterations},\"elapsed_ns\":{}}}",start.elapsed().as_nanos());
    Ok(())
}
