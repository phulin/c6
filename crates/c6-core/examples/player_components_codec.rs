//! Offline component wire roundtrip; never runs original game code.
use c6_core::player_components::{Bonuses, Congress, GoodyHut};
use c6_core::unit::Limits;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: player_components_codec goody_hut|bonuses|congress INPUT OUTPUT".into(),
        );
    }
    let input = std::fs::read(&args[2])?;
    let limits = Limits::default();
    let (output, items) = match args[1].as_str() {
        "goody_hut" => {
            let value = GoodyHut::decode(&input, limits)?;
            (value.encode(limits)?, value.player_goodies.len())
        }
        "bonuses" => {
            let value = Bonuses::decode(&input, limits)?;
            (value.encode(limits)?, value.flat_bonuses.len())
        }
        "congress" => {
            let value = Congress::decode(&input, limits)?;
            (value.encode(limits)?, value.resolution_effect_refunds.len())
        }
        _ => return Err("unsupported component".into()),
    };
    if input != output {
        return Err("component roundtrip changed wire bytes".into());
    }
    let mut target = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[3])?;
    std::io::Write::write_all(&mut target, &output)?;
    println!("{{\"kind\":\"{}\",\"bytes\":{},\"items\":{},\"wire_roundtrip_equal\":true,\"complete_player\":false}}", args[1], output.len(), items);
    Ok(())
}
