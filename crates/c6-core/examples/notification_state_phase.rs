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
