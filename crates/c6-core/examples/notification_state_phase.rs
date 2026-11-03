//! Bounded notification phase adapter. Manifest supplies omitted live inputs.
use c6_core::notification_lifecycle::{
    self as life, ActivationBinding, DefinitionIcon, DestructionBinding, Gates, LiveNotification,
    ObjectId, Pool, PoolId, ReleaseBinding, State, UndeliveredProjection,
};
use c6_core::notifications::{Limits, PlayerNotifications};
use std::{env, fs, time::Instant};
fn flag(s: &str) -> Result<bool, Box<dyn std::error::Error>> {
    match s {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err("noncanonical manifest bool".into()),
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 5 {
        return Err(
            "usage: notification_state_phase WIRE LIVE_MANIFEST WIRE_OUT ITERATIONS".into(),
        );
    }
    let wire = PlayerNotifications::decode(&fs::read(&args[1])?, Limits::default())?;
    let manifest = fs::read_to_string(&args[2])?;
    let mut gates = None;
    let mut live = vec![];
    let mut pools = vec![];
    let mut mode = None;
    for line in manifest.lines().filter(|l| !l.is_empty()) {
        let p: Vec<_> = line.split_whitespace().collect();
        match p[0] {
            "mode" if p.len() == 2 && p[1] == "undelivered_projection" => {
                if mode.replace(()).is_some() {
                    return Err("duplicate mode".into());
                }
            }
            "gates" if p.len() == 10 => {
                if gates.is_some() {
                    return Err("duplicate gates".into());
                }
                gates = Some(Gates {
                    manager_initialized: flag(p[1])?,
                    game_uninitializing: flag(p[2])?,
                    human_configuration: flag(p[3])?,
                    owner_turn_active: flag(p[4])?,
                    network_multiplayer: flag(p[5])?,
                    play_by_cloud: flag(p[6])?,
                    manager_game_local_player: if flag(p[7])? {
                        Some(p[8].parse()?)
                    } else {
                        None
                    },
                });
                if p[9] != "v1" {
                    return Err("manifest gates version".into());
                }
            }
            "record" if p.len() == 11 => {
                let index = live.len();
                let record = wire.records.get(index).ok_or("too many live records")?;
                if (record.id.owner, record.id.component_type, record.id.id)
                    != (p[8].parse()?, p[9].parse()?, p[10].parse()?)
                {
                    return Err("live record ordinal/identity mismatch".into());
                }
                live.push(LiveNotification {
                    object: ObjectId(p[1].parse()?),
                    lock_count: p[2].parse()?,
                    pool_binding: if p[3] == "-" {
                        None
                    } else {
                        Some(PoolId(p[3].parse()?))
                    },
                    definition_icon: match p[4] {
                        "?" => DefinitionIcon::Unobserved,
                        "-" => DefinitionIcon::MissingDefinition,
                        value => DefinitionIcon::ShowInSinglePlayer(flag(value)?),
                    },
                    activation: match p[5] {
                        "base" => ActivationBinding::Base,
                        "unsupported" => ActivationBinding::Unsupported,
                        _ => return Err("activation profile".into()),
                    },
                    release: match p[6] {
                        "base" => ReleaseBinding::Base,
                        "unsupported" => ReleaseBinding::Unsupported,
                        _ => return Err("release profile".into()),
                    },
                    destruction: match p[7] {
                        "base" => DestructionBinding::BaseResources,
                        "unsupported" => DestructionBinding::Unsupported,
                        _ => return Err("destruction profile".into()),
                    },
                });
            }
            "pool" if p.len() >= 2 => pools.push(Pool {
                id: PoolId(p[1].parse()?),
                retained_objects: p[2..]
                    .iter()
                    .map(|v| v.parse().map(ObjectId))
                    .collect::<Result<_, _>>()?,
                records: vec![],
            }),
            _ => return Err("unknown/invalid manifest line".into()),
        }
    }
    mode.ok_or("projection mode must be explicit")?;
    let gates = gates.ok_or("missing gates")?;
    let state = State {
        notifications: wire,
        live,
        pools,
    };
    let projection = UndeliveredProjection::default();
    let result = life::activation_and_cleanup(
        &state,
        gates,
        &projection,
        state.notifications.records.len() + 1,
    )?;
    fs::write(
        &args[3],
        result.state.notifications.encode(Limits::default())?,
    )?;
    // Replay public production selection/base-activation leaves separately so
    // captured native branch-time inputs/results can be compared. These probes
    // are diagnostics and remain outside both measured phase timings.
    let mut replay = state.clone();
    let mut replay_delivery = UndeliveredProjection::default();
    for scan in 0..=state.notifications.records.len() {
        for (record, live) in replay.notifications.records.iter().zip(&replay.live) {
            let mut probe = replay.clone();
            probe.notifications.records = vec![record.clone()];
            probe.live = vec![live.clone()];
            probe.live[0].lock_count = 1;
            let auto =
                life::find_auto_activate(&probe, i32::MIN, gates.network_multiplayer)?.is_some();
            // A locked singleton avoids HasBeenActivated's candidate suppression.
            // At MAX threshold, observed lower-severity fixtures isolate override.
            if record.severity == i32::MAX {
                return Err("diagnostic override probe requires below-MAX severity".into());
            }
            let override_value = if auto {
                Some(
                    life::find_auto_activate(&probe, i32::MAX, gates.network_multiplayer)?
                        .is_some(),
                )
            } else {
                None
            };
            let override_json = override_value.map_or("null".to_owned(), |v| v.to_string());
            let pool = live
                .pool_binding
                .map_or("null".to_owned(), |p| p.0.to_string());
            let definition = match live.definition_icon {
                DefinitionIcon::Unobserved => -2,
                DefinitionIcon::MissingDefinition => -1,
                DefinitionIcon::ShowInSinglePlayer(v) => i32::from(v),
            };
            println!("{{\"kind\":\"eligibility\",\"scan\":{scan},\"object\":{},\"owner\":{},\"component_type\":{},\"id\":{},\"type_hash\":{},\"severity\":{},\"dismissed\":{},\"native_auto_activate\":{},\"native_is_expired\":{},\"activated\":{},\"lock_count\":{},\"pool\":{pool},\"definition\":{definition},\"is_auto_activate\":{auto},\"effective_override\":{override_json}}}",live.object.0,record.id.owner,record.id.component_type,record.id.id,record.type_hash,record.severity,record.is_dismissed,record.native_19d,record.native_19c,record.has_been_activated,live.lock_count);
        }
        let selected = life::find_auto_activate(
            &replay,
            if gates.network_multiplayer { 3 } else { 0 },
            gates.network_multiplayer,
        )?;
        let object = selected.map_or("null".to_owned(), |index| {
            replay.live[index].object.0.to_string()
        });
        let locked = selected.is_some_and(|index| replay.live[index].lock_count > 0);
        println!(
            "{{\"kind\":\"selection\",\"scan\":{scan},\"object\":{object},\"locked\":{locked}}}"
        );
        let Some(index) = selected else {
            break;
        };
        if locked {
            break;
        }
        life::activate_base_in_place(&mut replay, index, false, &mut replay_delivery)?;
    }
    life::cleanup_in_place(&mut replay)?;
    assert_eq!(
        replay, result.state,
        "public leaf replay and composed phase disagree"
    );
    for r in &result.delivery.reports {
        println!("{{\"kind\":\"undelivered_activation\",\"event\":{},\"identity_key\":{},\"owner\":{},\"component_type\":{},\"id\":{},\"bool_key\":{},\"argument\":{}}}",r.event_hash,r.identity_key,r.identity.owner,r.identity.component_type,r.identity.id,r.bool_key,r.activation_argument);
    }
    for effect in &result.ownership_effects {
        match effect {
            life::OwnershipEffect::ReturnedToPool { object, pool } => println!(
                "{{\"kind\":\"returned_to_pool\",\"object\":{},\"pool\":{}}}",
                object.0, pool.0
            ),
            life::OwnershipEffect::DestroyedBaseResources { object } => println!(
                "{{\"kind\":\"destroyed_base_resources\",\"object\":{}}}",
                object.0
            ),
        }
    }
    for pool in &result.state.pools {
        let handles: Vec<_> = pool
            .retained_objects
            .iter()
            .copied()
            .chain(pool.records.iter().map(|r| r.live.object))
            .map(|id| id.0)
            .collect();
        println!(
            "{{\"kind\":\"pool\",\"id\":{},\"objects\":{:?}}}",
            pool.id.0, handles
        );
    }
    let iterations: usize = args[4].parse()?;
    if iterations > 10000 {
        return Err("iteration resource bound".into());
    }
    let time = Instant::now();
    for _ in 0..iterations {
        std::hint::black_box(life::activation_and_cleanup(
            std::hint::black_box(&state),
            gates,
            &projection,
            state.notifications.records.len() + 1,
        )?);
    }
    let component_copy_ns = time.elapsed().as_nanos();
    // Clone setup is deliberately outside local-mutation timing.
    let mut local_states = vec![state.clone(); iterations];
    let time = Instant::now();
    for local in &mut local_states {
        let mut delivery = UndeliveredProjection::default();
        std::hint::black_box(life::activation_and_cleanup_in_place(
            std::hint::black_box(local),
            gates,
            &mut delivery,
            state.notifications.records.len() + 1,
        )?);
    }
    let local_mutation_ns = time.elapsed().as_nanos();
    println!("{{\"kind\":\"summary\",\"before_records\":{},\"after_records\":{},\"activations\":{},\"before_next_id\":{},\"after_next_id\":{},\"iterations\":{iterations},\"component_copy_ns\":{component_copy_ns},\"local_mutation_ns\":{local_mutation_ns},\"reporting_delivery_bound\":false}}",state.notifications.records.len(),result.state.notifications.records.len(),result.activations,state.notifications.next_id,result.state.notifications.next_id);
    Ok(())
}
