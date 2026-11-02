# Differential verification

`c6-differential` compares a candidate against an independently executed original DLL, stopping at the first differing step and JSON field. It is an extensible verification harness, not an implementation of Civilization VI gameplay. Current executable coverage is isolated XP2 RNG instances and two coordinate primitives. Player actions, game initialization, saves, and end-turn remain unsupported.

## Reproduce

Use the exact XP2 DLL identified by `tools/oracle/config.json`. The native host currently requires Linux x86-64; see [oracle.md](oracle.md) for prerequisites and the explicit isolated execution limitations.

```sh
make -C native/oracle
cargo build -p c6-differential --bins
python3 fixtures/generate.py
target/debug/c6-differential fixtures/corpus/xp2-rng.json fixtures/run-native-oracle target/debug/c6-rng-candidate /tmp/c6-traces
python3 fixtures/check_fail_closed.py
```

The optional last argument saves every request, complete per-step response, and transport error as numbered oracle/candidate JSON files. Trace files contain all logical state fields and RNG draws supplied by each adapter. Exit zero means equality only for the declared operations, inputs, pinned binaries, and isolated configuration. It does not establish full game parity. Unit tests that use fake runners test the harness only and are labeled `harness_test_only`.

The checked-in corpus pins a particular candidate executable SHA. Rebuilding, changing toolchains, changing either adapter, or changing native configuration can require regenerating it. Identity mismatches fail; they never silently update the expected identity. `generate.py` intentionally performs that explicit repinning step and records both identities returned by the actual adapters. The corpus contains deterministic inputs, not invented reference results. Successful comparison always requires the original DLL to execute again.

## Corpus and evidence

The version-one corpus has `schema_version`, `oracle`, `candidate`, `evidence`, and `cases`. Engine identities contain `kind`, `binary_sha256`, and `configuration_sha256`, with 64 lowercase hexadecimal digits for each hash. Oracle kind must be `original_dll`; candidate kind must be `rust`. Both binary and configuration must match each response exactly. Native configuration also pins the host and adapter source; the candidate hashes its own executable and embedded configuration.

Evidence includes a nonempty `level`, `references`, and `coverage`. The current corpus uses `isolated_original_dll`, references pinned disassembly/configuration, and lists each required operation. Each response must advertise all required coverage. Evidence labels propagate into the report; no label is upgraded because two implementations agree.

Every case has a unique `id`, a structured `initial_state`, and a nonempty ordered list of operation objects in `steps`. JSON objects are compared independently of key order; arrays preserve order. Missing fields, extra fields, changed values, and different trace lengths diverge. Integer state is represented exactly. Future opaque bytes should use lowercase hexadecimal, and floating state should use explicit IEEE bit patterns if bit identity matters; JSON decimal floats are not a substitute for raw state.

The current generator creates 10 cases and 7,141 steps. It exercises boundary seeds and counters, zero and truncated bounds, reset/initialize distinction, lock overflow and unlock saturation, retained synchronous flags, long draw sequences, independent interleaved RNG instances, coordinate sentinel boundaries, signed arithmetic extremes, negative/zero widths, odd/negative rows, and both horizontal wrapping settings. These are arithmetic and instance-isolation checks. Stream IDs do not claim discovery of the real game's stream ownership or scheduling.

## Process protocol

Each isolated child receives one JSON request followed by newline, then stdin closes:

```json
{"protocol_version":1,"expected_engine":{"kind":"original_dll","binary_sha256":"...","configuration_sha256":"..."},"case":{"id":"example","initial_state":{},"steps":[{"op":"plot_valid","x":0,"y":0}]}}
```

It emits one JSON response with `protocol_version`, `engine`, `case_id`, `status`, `coverage`, and `steps`. Status is `ok`, `unsupported`, `unavailable`, or `error`; an optional `message` explains refusal. Only `ok` passes. Each observation has contiguous zero-based `step_index`, `snapshot`, and `rng_trace`. Every RNG event contains `stream`, `ordinal`, `operation`, `arguments`, and `result`; traces compare in order alongside the snapshots. Standard output is reserved for the JSON response; diagnostics belong on stderr.

`ProcessRunner` supplies a deadline and bounds each output stream. The CLI uses 30 seconds and 16 MiB per case. Missing executables, nonzero exit, invalid or multiple JSON responses, stale identity, incomplete coverage, unsupported operations, missing steps, and misordered indices all fail closed. It is an execution isolation and reproducibility tool, not a security sandbox for untrusted programs.

## Current operation schema

RNG initial state is `{"rng":{"seed":u32,"calls":u32,"resets":u32,"lock_count":u32,"synchronous":bool}}`. Independent instances optionally appear under `rng_streams`, keyed by nonzero decimal u32 IDs. ID zero is the legacy `rng` instance. RNG operations require an explicitly supplied target instance and accept optional `stream_id` (default zero):

- `{"op":"rng_get","bound":u32,"stream_id":u32,"stream":"trace-label"}` truncates the bound to the original unsigned-short argument. `stream` defaults to `sync` and is a trace label; numeric `stream_id` selects the instance. The trace records original and effective bounds, numeric stream ID, output, and the post-call wrapping counter.
- `{"op":"rng_reset","seed":u32}` and `{"op":"rng_initialize","seed":u32}` preserve their distinct native counter semantics.
- `{"op":"rng_lock"}` and `{"op":"rng_unlock"}` change the selected instance's lock counter.

Every RNG snapshot includes all five logical fields of the default instance and, when configured, every additional instance under `rng_streams`. These do not include native vtable pointers, padding, or logger state. The null logger deliberately excludes logging behavior and game services. A true synchronous flag tests field/arithmetic preservation, not real synchronized-game logging.

Geometry operations accept signed i32 arguments:

- `{"op":"plot_valid","x":i32,"y":i32}` returns `{"result":bool}`. This is the native coordinate sentinel check, not map bounds validation.
- `{"op":"plot_distance","width":i32,"wrap":bool,"x1":i32,"y1":i32,"x2":i32,"y2":i32}` returns `{"result":i32}` using the pinned native map function.

Geometry traces are empty. They run against an isolated minimal native object layout and do not establish real map topology or pathfinding parity.

## Extend and use as a library

Implement the `Runner` trait, accepting `Request` and returning `Response`, then call `compare(&corpus, &mut original, &mut candidate)`. `RecordingRunner` preserves raw responses, including responses subsequently rejected by validation. `RunError` distinguishes invalid corpus, engine/transport failure, and structured `Divergence` with case ID, step index, JSON pointer, and both values. This API can back evaluation tooling without parsing console output.

To add an operation, first establish pinned native evidence and a callable original function or initialized game context. Add an explicit schema and full snapshot coverage to both adapters, assign coverage labels, and extend deterministic inputs. Keep unsupported game operations refused until the required context and state observation exist. Do not replace the original DLL with a Rust reference implementation and call the result game parity.

## Original Random wire-record serialization

The dedicated serialization suite verifies the original `Random::Write` and normal `Random::Read` stream payload separately from action protocol v1 and the simulator's `C6SN` snapshot envelope:

```sh
make -C native/oracle
cargo build -p c6-differential --bins
python3 fixtures/serialization.py generate
python3 fixtures/serialization.py replay --trace-dir /tmp/c6-serialization-traces
python3 -m unittest discover -s fixtures -p 'test_serialization_harness.py'
```

`generate` explicitly pins the dedicated original adapter identity and the actual `c6-serialization-candidate` executable. `replay` never repins. It compares 801 cases: 528 supported native executions and 273 explicit unsupported-input refusals. Cases include every raw synchronous byte from 0 through 255, arbitrary ignored version words, exact byte output, prefix consumption of 18 bytes, untouched trailing bytes, and preservation of the destination lock counter. All 18 short prefix lengths and every nonzero legacy flag are refused before native execution. These refusals establish the adapter's supported boundary; they do not reproduce native short-read errors or legacy-vector behavior.

The small candidate uses `c6_core::rng::native_serialization::SerializedRandomRecord` for raw byte fidelity. Canonical synchronous bytes zero/one additionally exercise `read_into` against the actual logical `RandomState`. Bytes above one remain raw wire records: the logical boolean application API rejects them atomically rather than silently coercing them. They are not valid imported simulator boolean states.

The dedicated native JSONL contract is documented by `tools/oracle/serialization.py`. Requests contain `id`, `expected_engine`, `op` (`write` or `read`), `state` with raw `synchronous_byte`, and `bytes_hex` for reads. Responses identify the exact engine and case, returning either `bytes_hex` or the complete resulting record state plus `consumed` and `trailing_hex`. The corpus records source evidence and expected supported/refused classification. The replay checks identities, case IDs, response counts, status, and every semantic output field; differing refusal wording is excluded. A mismatch reports the first case and JSON pointer. Optional traces save every request and response for both sides.

The dedicated process guard services stdin and both output streams concurrently, enforces 16 MiB per stream while reading (4 KiB for identity), applies a 30-second deadline, and terminates the child process group on failure. Its tests deliberately exercise excessive stdout, excessive stderr, blocked pipes, inherited pipe handles, missing executables, nonzero exit, and structured byte/state/cursor mismatches. Those tests verify transport and reporting only; the separate 801-case replay establishes the stated original DLL comparison.

Because rebuilding all binaries can legitimately change the prior action candidate's executable SHA, rerun `fixtures/generate.py` explicitly when necessary, then replay that corpus as well. The native action oracle configuration remains independent of serialization.
