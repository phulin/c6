//! Independent attributed-owner prefix comparison; no original execution.
use c6_core::game_diplomatic_relations::{Error, GameDiplomaticRelations};
use c6_core::unit::Limits;
use std::{env, fs};

fn main() {
    let paths: Vec<_> = env::args().skip(1).collect();
    assert_eq!(paths.len(), 2);
    let owner = fs::read(&paths[0]).unwrap();
    let expected = fs::read(&paths[1]).unwrap();
    let parsed = GameDiplomaticRelations::decode_prefix(&owner, Limits::default()).unwrap();
    assert_eq!(parsed.state.player_count(), 64);
    assert_eq!(parsed.consumed, 121005);
    assert_eq!(parsed.tail.len(), 74616);
    assert_eq!(parsed.state.is_allied(0, 1), Err(Error::MissingAlliances));
    let prefix = parsed.state.read_prefix().unwrap();
    let mut actual = Vec::new();
    for values in [&prefix.at_c8, &prefix.at_f0, &prefix.at_118, &prefix.at_140] {
        for value in values {
            actual.push(u8::from(value.value));
            actual.extend(value.changed_turn.to_le_bytes());
        }
    }
    for values in [
        &prefix.at_168,
        &prefix.saved_war,
        &prefix.at_1b8,
        &prefix.at_1e0,
        &prefix.saved_alliances,
    ] {
        for (&value, &turn) in values.values().iter().zip(values.changed_turns().unwrap()) {
            actual.extend(value.to_le_bytes());
            actual.extend(turn.to_le_bytes());
        }
    }
    assert_eq!(actual, expected);
    println!("{{\"prefix_bytes\":{},\"unclassified_tail_bytes\":{},\"vectors_compared\":9,\"records_compared\":18144,\"alliance_runtime_comparison\":false}}",
        parsed.consumed, parsed.tail.len());
}
