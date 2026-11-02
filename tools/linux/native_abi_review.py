#!/usr/bin/env python3
"""Bounded exact-Linux-host ABI anchors plus synthetic ConfigureContent decoding.

No game original code is executed. Original binary bytes are inspected read-only;
the separately compiled shim exercises the current adapter with fabricated return
register values and cannot count as native state/action/turn parity.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

from inspect_elf import sections

ROOT = Path(__file__).resolve().parents[2]
HOST = '106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4'
ANCHORS = [
    ('configure_result_constructor', 0x3386170, 16, '4b560d05278930e1f22e44e96684101b32e528c7834af6c2a7c5d48011012cc4'),
    ('update_result_constructor', 0x3386200, 16, '0f136fab4865e541e7f567155150831eb25e8c9ebbba34b30ad695923e6dbdf5'),
    ('configure_return', 0x3380908, 4, '95bd7f27c6ed3290c3c1094ec0241cf748e00c6f15a83379f6982a58c42e5212'),
    ('update_return', 0x3381f30, 5, '0042a7a8600dc4b6c2d753f0c1ea33856b7fb10c43b8adefdd74075fd4071aa3'),
    ('configure_success_caller', 0x33818af, 12, 'aabdde9769d1edddf9fec8b333c7a3a09409decdd1ed65a21d02647eb83cde0b'),
    ('setup_network', 0x2f91d60, 79, '0c8499b4bc7653df3dc34fec15385cfb14fbb5a2a20e546a451b34770a5a155e'),
    ('init_instance_publish_return', 0x3385417, 17, 'c8652f0313b6deef058c45e6aac9b531141629f74ac6026ed9649d92bfbbafe1'),
    ('get_context_sret', 0x31dd790, 56, 'bdcc140d12e4f72da54f28f8229153a701cef8541884314129adf97c2715e323'),
    ('network_get_session', 0x3044300, 5, '8e3eaf4f4b235f364076d47e3b5b95d9e60a71cf19dd62030d0fa20e80858d2e'),
    ('client_allocate_and_construct', 0x31a1770, 73, 'e5b7da3cda7bd8d201047a59272c7a012623a66eaee5e45f911cbb179316924b'),
    ('host_game_success_comparison', 0x33be07b, 7, '04d064e0207fcc46dcd04c14eeb65e02b649804fe075df7ed3a9349a8f165d40'),
    ('host_game_integer_return', 0x33be0e6, 2, '1eafaf1e83618c9f051be8d0fdbd78cbbc1b668268d564392da9d6c25fc7b8f9'),
]


def digest(path):
    with path.open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def anchors(host):
    if digest(host) != HOST:
        raise ValueError('unsupported Linux host SHA-256; no ABI checks accepted')
    section = sections(host)['.text']
    result = []
    with host.open('rb') as source:
        for name, address, length, expected in ANCHORS:
            source.seek(address - int(section['address'], 16) + section['offset'])
            actual = hashlib.sha256(source.read(length)).hexdigest()
            if actual != expected:
                raise ValueError('exact native ABI anchor differs: ' + name)
            result.append({'name': name, 'address': hex(address), 'bytes': length,
                           'sha256': actual})
    return result


def adapter_decode():
    # This shim supplies only GetInstance and ConfigureContent. Optional native
    # inventory/path observers are absent. No original native function is called.
    shim = r'''
#define _GNU_SOURCE
#define dlsym c6_review_dlsym
#include "native/linux/mod_activation.c"
#undef dlsym
#include <string.h>
static uint64_t fixture_return;
static char fixture_framework;
static void *fixture_get(void) { return &fixture_framework; }
static uint64_t fixture_configure(void *self) {
    if(self != &fixture_framework) abort();
    return fixture_return;
}
void *c6_review_dlsym(void *handle, const char *name) {
    (void)handle;
    if(!strcmp(name,"_ZN19AppModdingFramework11GetInstanceEv"))return fixture_get;
    if(!strcmp(name,"_ZN19AppModdingFramework16ConfigureContentEv"))return fixture_configure;
    return NULL;
}
int main(void) {
    const uint64_t fixtures[] = {1, UINT64_C(1)<<32, (UINT64_C(1)<<32)|8,
        (UINT64_C(1)<<32)|16, (UINT64_C(1)<<32)|200, 0, 0xc1};
    const int expected[] = {0,116,116,116,116,116,0};
    for(unsigned i=0;i<sizeof fixtures/sizeof fixtures[0];++i) {
        fixture_return=fixtures[i];
        if(c6_linux_mod_framework_configure()!=expected[i])return 1;
    }
    return 0;
}
'''
    with tempfile.TemporaryDirectory(prefix='c6-native-abi-review-') as temp:
        source = Path(temp) / 'review.c'
        binary = Path(temp) / 'review'
        source.write_text(shim)
        subprocess.run(['cc', '-Wall', '-Wextra', '-Werror', '-I', str(ROOT),
                        str(source), '-ldl', '-o', str(binary)], check=True)
        run = subprocess.run([str(binary)], capture_output=True, text=True, check=True)
        for flags in (8, 16):
            if f'configure_content_return flags {flags} code 1' not in run.stderr:
                raise ValueError('ConfigureContent diagnostics lost a defined result flag')
        return {'synthetic_return_vectors_passed': 7,
                'adapter_source_sha256': digest(ROOT / 'native/linux/mod_activation.c'),
                'diagnostics': run.stderr.splitlines(),
                'original_native_execution': False}



def network_guards():
    shim = r'''
#define _GNU_SOURCE
#define dlsym c6_review_dlsym
#include "native/linux/network.c"
#undef dlsym
#include <stdlib.h>
static char old_manager, replacement_manager, fixture_app[0x6800], wrong_app[0x6800];
static void *published_app=fixture_app;
static void *fixture_get_manager(void) { return &replacement_manager; }
static void *fixture_get_session(void *self) { (void)self; abort(); }
int c6_linux_register_host_events(void) { abort(); }
void *c6_review_dlsym(void *handle,const char *name) {
    (void)handle;
    if(!strcmp(name,"g_pkApplication"))return &published_app;
    if(!strcmp(name,"_ZN7Network7Manager11GetInstanceEv"))return fixture_get_manager;
    if(!strcmp(name,"_ZN7Network7Manager10GetSessionEv"))return fixture_get_session;
    return NULL;
}
int main(void) {
    if(c6_linux_network_initialize(wrong_app)!=150)return 1;
    for(unsigned i=0;i<sizeof wrong_app;++i)if(wrong_app[i])return 2;
    owned=true;owned_manager=&old_manager;
    if(c6_linux_network_host_offline()!=160)return 3;
    if(c6_linux_network_leave()!=162)return 4;
    return 0;
}
'''
    with tempfile.TemporaryDirectory(prefix='c6-network-abi-review-') as temp:
        source = Path(temp) / 'review.c'
        binary = Path(temp) / 'review'
        source.write_text(shim)
        subprocess.run(['cc', '-Wall', '-Wextra', '-Werror', '-I', str(ROOT),
                        str(source), '-ldl', '-o', str(binary)], check=True)
        subprocess.run([str(binary)], capture_output=True, text=True, check=True)
        return {'synthetic_receiver_guards_passed': 3,
                'adapter_source_sha256': digest(ROOT / 'native/linux/network.c'),
                'original_native_execution': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--host', type=Path, default=ROOT.parent / 'c6-assets-linux/Civ6')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = {'format': 'c6.linux-native-abi-review-check.v1', 'host_sha256': HOST,
              'native_static_anchors': anchors(args.host), 'synthetic_adapter_check': adapter_decode(),
              'synthetic_network_guards': network_guards()}
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'static_anchors_passed': len(result['native_static_anchors']),
                      'synthetic_vectors_passed': result['synthetic_adapter_check']['synthetic_return_vectors_passed'],
                      'synthetic_receiver_guards_passed': result['synthetic_network_guards']['synthetic_receiver_guards_passed']}))


if __name__ == '__main__':
    main()
