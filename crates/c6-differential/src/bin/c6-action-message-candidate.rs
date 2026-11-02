//! Dedicated transport candidate; never calls Engine::apply or claims action parity.
use c6_core::messages::{HandlerId, NullParameterUnitOperationBody, UnitIdentifier};
use serde::Deserialize;
use std::io::{self, BufRead, Write};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    owner: u16,
    object_type: u16,
    id: u32,
    handler: i32,
    secondary: i32,
    flags: u32,
    mode: i32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = io::stdout();
    let mut output = io::BufWriter::new(stdout.lock());
    for line in io::stdin().lock().lines() {
        let request: Request = serde_json::from_str(&line?)?;
        let body = NullParameterUnitOperationBody {
            unit: UnitIdentifier {
                owner: request.owner,
                object_type: request.object_type,
                id: request.id,
            },
            handler: HandlerId(request.handler),
            secondary: request.secondary,
            flags: request.flags,
            mode: request.mode,
        };
        let encoded = body.encode();
        let decoded = NullParameterUnitOperationBody::decode(&encoded)
            .expect("encoder produces complete null-parameter body");
        let hex: String = encoded.iter().map(|b| format!("{b:02x}")).collect();
        writeln!(
            output,
            "{}",
            serde_json::json!({"body_hex":hex,"decoded":{
                "owner":decoded.unit.owner,"object_type":decoded.unit.object_type,
                "id":decoded.unit.id,"handler":decoded.handler.0,
                "secondary":decoded.secondary,"flags":decoded.flags,"mode":decoded.mode
            }})
        )?;
    }
    Ok(())
}
