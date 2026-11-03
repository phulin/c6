//! Offline direct raw Core prefix validation, with no Python offsets or engine.
use c6_core::core_stream::CorePrefix;
use c6_core::map::Limits;
use c6_core::unit::variants::{
    ArrayVariant, KeyedHeader, TypedEntry, TypedVariant, VariantMap, VariantPayload,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|x| format!("{x:02x}")).collect()
}
fn array(v: &ArrayVariant) -> Value {
    let payload = match &v.payload {
        VariantPayload::Inline32(x) => json!(hex(&x.to_le_bytes())),
        VariantPayload::Inline64(x) => json!(hex(&x.to_le_bytes())),
        VariantPayload::Char8(x) => json!(hex(x)),
        VariantPayload::VariantMap(x) => untyped(x),
        VariantPayload::EmptyVariantVector => Value::Null,
        VariantPayload::TypedVariantMap(x) => keyed(x.header, &x.entries),
        VariantPayload::TypedVariantVectorMap(x) => keyed(x.header, &x.entries),
        VariantPayload::TypedVariantVector(x) => json!(x.iter().map(typed).collect::<Vec<_>>()),
    };
    json!({"flags":v.count_flags,"element_size":v.element_size,"payload":payload})
}
fn typed(v: &TypedVariant) -> Value {
    json!({"kind":v.type_bits,"modifiers":v.modifier_bits,"value":array(&v.value)})
}
fn keyed(header: KeyedHeader, values: &[TypedEntry]) -> Value {
    let header = match header {
        KeyedHeader::Count => values.len() as u32,
        KeyedHeader::Prefix(h) => h,
    };
    json!({"header":header,"entries":values.iter().map(|v|json!({"key":v.key,"name":v.name.as_ref().map(|s|hex(s)),"value":typed(&v.value)})).collect::<Vec<_>>()})
}
fn untyped(v: &VariantMap) -> Value {
    json!({"scalars":v.scalars,"arrays":v.arrays.iter().map(|(k,v)|json!([k,array(v)])).collect::<Vec<_>>()})
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: c6-core-prefix-candidate RAW_CORE MAP_OUTPUT".into());
    }
    let mut input = Vec::new();
    const MAX_INPUT: u64 = 256 * 1024 * 1024;
    std::fs::File::open(&args[1])?
        .take(MAX_INPUT + 1)
        .read_to_end(&mut input)?;
    if input.len() as u64 > MAX_INPUT {
        return Err("raw input resource bound".into());
    }
    // The parser receives the entire raw Core. Its byte budget covers consumed
    // fields only, while the untouched tail remains a borrow of this buffer.
    let value = CorePrefix::decode(&input, Limits::default())?;
    if value.map_end != value.consumed
        || value.tail != &input[value.consumed..]
        || !std::ptr::eq(value.tail.as_ptr(), input[value.consumed..].as_ptr())
    {
        return Err("prefix boundary/borrow differs".into());
    }
    let encoded = value.map.encode(Limits::default())?;
    if encoded != input[value.map_start..value.map_end] {
        return Err("Map bytes differ".into());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[2])?;
    file.write_all(&encoded)?;
    let g = &value.game_processor;
    let p = &value.player_processor;
    let phase = &value.phase_processor;
    let processor = json!({"last_initial_ai":g.last_initial_ai.as_slice(),"turn_order":g.turn_order.as_slice(),
        "last_ai":g.last_ai_processed_turn,"flags":[g.turn_complete,g.advance_turn,g.new_game],
        "players":{"flags":[p.activation_event.as_slice(),p.first_activation.as_slice(),p.activation_needed.as_slice(),
            p.automation_processed.as_slice(),p.player_complete.as_slice(),p.game_complete.as_slice()],
            "counts":[p.updates_this_phase.as_slice(),p.updates_this_turn.as_slice(),p.ai_updates_this_turn.as_slice()],
            "active_count":p.active_count,"human_first":p.human_first_activated},
        "phase":{"current":phase.current_phase,"time_limit":phase.time_limit,
            "disabled":phase.turn_segments.iter().map(|s|json!([s.hash,s.disabled])).collect::<Vec<_>>(),
            "last_segment":phase.last_valid_segment}});
    println!(
        "{}",
        json!({"map_start":value.map_start,"map_end":value.map_end,"consumed":value.consumed,
        "tail_bytes":value.tail.len(),"tail_sha256":hex(&Sha256::digest(value.tail)),"tail_borrowed":true,
        "map_sha256":hex(&Sha256::digest(&encoded)),"map_bytes":encoded.len(),"map_width":value.map.header.width,
        "map_height":value.map.header.height,"plots":value.map.plots.len(),
        "metadata":keyed(value.metadata.header,&value.metadata.entries),"processor":processor,
        "game_scalars":[value.current_turn,value.max_turns,value.local_player],"game_properties":untyped(&value.game_properties),
        "metadata_compatibility_checked":false,"complete_core_import":false,"gameplay_parity":false,"original_game_execution":false})
    );
    Ok(())
}
