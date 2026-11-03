#!/usr/bin/env python3
"""Gate the Linux Plot5/6 Read contract; never execute original game code."""
import argparse
import json
from pathlib import Path
import subprocess
from definition_ids_audit import ROOT, TARGET, elf, sha

METHODS = [
    (0x2a472c0, 0x6e, 'collection_read', '9cf942f7ab8e710a2a4c5066470c6146f61776ed8d9d33dbfeffdb12d4afa8c2'),
    (0x2bc8820, 0x435, 'plot_read', 'd8c6bd76add824f31a23a544617d6b211f486a0854537d69ff4ff08eb78f0276'),
    (0x2bc8090, 0x5b, 'city_read', '4735021680e42fe397e22a42fcde4e37114f3a69045747faae2e7a3e29378f77'),
    (0x2bc8c60, 6, 'version', '23334c82ef0133c65cc0394d318a2637276496cb9b99f6dd580d0ee4f6c9e7b5'),
    (0x2bc8c70, 0x35, 'plot_constructor', 'e87704dc4307eb2e0205e0d27d308d93f5fb2bd434339b9567fc72d193666ba2'),
    (0x2bc8cb0, 0x61, 'plot_initialize', '44bf4136d46dce05132a0ac246856a5594783e2c391f3a05d133245f844bfc26'),
    (0x27bbc20, 0x27, 'city_initialize', '8c63b3a6c242a1a59e3eecf7042bff0a94e03e0ba5d9c508ffe948370bc8799d'),
]
DWARF_SHA = 'a663b5969d2f2365d649d8cdda270ac9c5219b783616b4e1b988b1fe05179835'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    module = args.assets / 'libGameCore_XP2.so'
    read = elf(module, TARGET)
    for address, size, name, digest in METHODS:
        if sha(read(address, size)) != digest:
            raise ValueError(f'{name} body identity differs')
    # Signed >5 skips the explicit clear of native object byte 0x4d, bit 6.
    if read(0x2bc8bf2, 10) != bytes.fromhex('4183fe057f0480634dbf'):
        raise ValueError('versioned volatile clear differs')
    dwarf = subprocess.check_output([
        'gdb', '-nx', '-batch', str(module),
        '-ex', "ptype /o 'GameCore::Map::Plot::Reflection::Variants1'",
        '-ex', "ptype /o 'GameCore::Map::Plot::CityAttributes'"], text=True)
    if sha(dwarf.encode()) != DWARF_SHA:
        raise ValueError('matching target DWARF differs')
    report = dict(
        schema='c6-linux-plot-versions-v1', gamecore_sha256=TARGET,
        methods=[dict(address=hex(a), size=n, name=k, sha256=h) for a,n,k,h in METHODS],
        dwarf_sha256=DWARF_SHA, supported_plot_versions=[5, 6],
        collection_wire='i32 version, u32 count, consecutive unversioned records',
        minimum_record_bytes=55, version_5_6_wire_size_delta=0,
        version_5_read_effect=dict(address='0x2bc8bf2', field='FeatureVolatile', value=False,
                                   wire_bit=16, native_offset=77, native_bit=6),
        spare_bits_policy='reject bits 17..23 for both supported versions',
        version_identity_api='Map::decode_prefix_with_version_and_usage',
        encoding_version=6, version_5_rewriting=False,
        constructor_findings=dict(plot_coordinates='Initialize takes x,y; constructor uses -9999,-9999',
                                  city_valid_domains=0, city_plot_yield_effect=0,
                                  valid_found_location='Initialize preserves native byte 0x4d bit4; not decoded'),
        native_heap_and_definition_reconstruction=False,
        original_code_executed=False, native_differential_fixture_added=False,
        full_raw_load=False, complete_gameplay_state=False, actions=False, turns=False,
        reference_revision='5af389445118bed2bdebf7a8a42b219c8fba5175', reference_role='discovery only',
        attribution='cru121 reference findings, CC BY 4.0',
        audit_source_sha256=sha(Path(__file__).read_bytes()))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
    print(json.dumps(dict(checks_passed=True, output=str(args.output))))


if __name__ == '__main__':
    main()
