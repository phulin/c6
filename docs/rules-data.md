# Immutable rules inputs

`c6-data` loads caller-supplied immutable SQLite inputs for an explicitly selected
registered profile. The crate-owned [profile descriptors](../crates/c6-data/profiles)
retain complete source identities and database/schema/typed-row fingerprints.
They contain no database payload and do not initialize a gameplay session.

| `RulesProfile` | Meaning |
| --- | --- |
| `LinuxXp2Base` | Registered Linux base inputs; no activated XP2 session is implied |
| `LinuxXp2ActivatedDiagnostic` | Registered activated Linux XP2 inputs under a bounded diagnostic configuration; not a complete initialized game |
| `WindowsXp2Historical` | Frozen compatibility descriptor; adds no Linux coverage |

The Linux profiles share a GameCore identity but have distinct complete input
identities. A module hash, filename or SQL row ID cannot select the correct
profile on its own. There is no fallback between profiles.

## Load contract

```rust
use c6_data::{Rules, RulesProfile};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args().nth(1).ok_or("rules directory required")?;
    let profile = RulesProfile::LinuxXp2ActivatedDiagnostic;
    let declared = profile.source_identity()?;
    let rules = Rules::load_profile(profile, directory, &declared)?;
    println!("{}", rules.gameplay().fingerprint().semantic_sha256);
    Ok(())
}
```

Obtaining `source_identity()` returns the registered descriptor; it does not
validate files. `Rules::load_profile` requires exact equality with that full
descriptor before filesystem access. `Rules::load` resolves only an exact
registered identity. Unknown, altered or mismatched inputs fail explicitly.

The loader requires `configuration.sqlite` and `gameplay.sqlite` with matching
file hashes and all stored database fingerprints. Bundled SQLite opens them
read-only, checks integrity and journal state, inspects schema and typed rows
in a read transaction, then verifies the files did not change. Connections close
before returning; tables, values and lookup indices are owned Rust data.

NULL, signed integers, raw floating-point bits, text and blob values remain
explicit. Database row IDs, native-order collection indices and serialized type
hashes are distinct identities. Definition registries require their actual
ordered inputs and retain their lookup/duplicate semantics; SQL iteration order
is not a substitute.

Rules-backed providers also need their declared state/definition/configuration
bindings. A valid rules descriptor or internally consistent binding alone does
not authenticate an original process, supply missing live state or implement
all rules effects. Missing providers and unsupported forms remain errors.
See [architecture](architecture.md), [target policy](targets.md) and
[current implementation limits](status.md).
