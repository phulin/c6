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
    ('run_app_argument_stores', 0x42b9aa3, 8, '298c72ca11e4b1209260e10c1f011969e6bb18eec147c27ae55304c0d0e30c65'),
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
int c6_linux_bind_host_arguments(void *candidate) { (void)candidate; abort(); }
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


def argument_binding():
    shim = r'''
#define _GNU_SOURCE
#define dlsym c6_review_dlsym
#include "native/linux/host_services.c"
#undef dlsym
#include <stddef.h>
#include <string.h>
static union { max_align_t alignment; unsigned char bytes[0x6800]; } valid, wrong;
static void *published;
static char program[]="actual-fixture-argv0";
static char *good_argv[]={program,NULL};
static char *other_argv[]={program,NULL};
static char *bad_argv[]={program,program,NULL};
static char *empty_argv[]={NULL,NULL};
static int supplied_argc=1, supplied_status, getter_calls, lookups;
static bool getter_available=true;
static char **supplied_argv=good_argv;
static int fixture_arguments(int *argc,char ***argv) {
    ++getter_calls;*argc=supplied_argc;*argv=supplied_argv;return supplied_status;
}
void *c6_review_dlsym(void *handle,const char *name) {
    (void)handle;++lookups;
    if(getter_available&&!strcmp(name,"c6_linux_main_arguments"))return fixture_arguments;
    return NULL;
}
static void reset(void) {
    memset(valid.bytes,0xa5,sizeof valid.bytes);memset(wrong.bytes,0x5a,sizeof wrong.bytes);
    application=valid.bytes;published=application;application_global=&published;
    host_arguments_bound=false;supplied_argc=1;supplied_status=0;
    supplied_argv=good_argv;getter_available=true;getter_calls=lookups=0;
}
static bool refusal(void *candidate,bool early) {
    unsigned char before_valid[sizeof valid.bytes],before_wrong[sizeof wrong.bytes];
    memcpy(before_valid,valid.bytes,sizeof before_valid);
    memcpy(before_wrong,wrong.bytes,sizeof before_wrong);
    int calls=getter_calls, searches=lookups;
    int result=c6_linux_bind_host_arguments(candidate);
    return result==1&&!memcmp(before_valid,valid.bytes,sizeof before_valid)&&
        !memcmp(before_wrong,wrong.bytes,sizeof before_wrong)&&
        (!early||(calls==getter_calls&&searches==lookups));
}
int main(void) {
    reset();application=NULL;
    if(!refusal(wrong.bytes,true))return 1;
    reset();if(!refusal(wrong.bytes,true))return 2;
    reset();application_global=NULL;if(!refusal(valid.bytes,true))return 3;
    reset();published=wrong.bytes;if(!refusal(valid.bytes,true))return 4;
    reset();supplied_status=1;if(!refusal(valid.bytes,false)||host_arguments_bound)return 5;
    reset();supplied_argc=2;if(!refusal(valid.bytes,false)||host_arguments_bound)return 6;
    reset();supplied_argv=NULL;if(!refusal(valid.bytes,false)||host_arguments_bound)return 7;
    reset();supplied_argv=empty_argv;if(!refusal(valid.bytes,false)||host_arguments_bound)return 8;
    reset();supplied_argv=bad_argv;if(!refusal(valid.bytes,false)||host_arguments_bound)return 9;
    reset();getter_available=false;if(!refusal(valid.bytes,false)||getter_calls||host_arguments_bound)return 10;
    reset();
    unsigned char expected[sizeof valid.bytes];memcpy(expected,valid.bytes,sizeof expected);
    memcpy(expected+0x30,&supplied_argc,sizeof supplied_argc);
    memcpy(expected+0x38,&supplied_argv,sizeof supplied_argv);
    if(c6_linux_bind_host_arguments(valid.bytes)||!host_arguments_bound||getter_calls!=1||
       memcmp(expected,valid.bytes,sizeof expected))return 11;
    if(c6_linux_bind_host_arguments(valid.bytes)||getter_calls!=2||
       memcmp(expected,valid.bytes,sizeof expected))return 12;
    *(int *)(valid.bytes+0x30)=2;
    if(!refusal(valid.bytes,false))return 13;
    *(int *)(valid.bytes+0x30)=1;*(char ***)(valid.bytes+0x38)=other_argv;
    if(!refusal(valid.bytes,false))return 14;
    *(char ***)(valid.bytes+0x38)=good_argv;supplied_argv=other_argv;
    if(!refusal(valid.bytes,false))return 15;
    if(!refusal(wrong.bytes,true))return 16;
    return 0;
}
'''
    with tempfile.TemporaryDirectory(prefix='c6-arguments-abi-review-') as temp:
        source = Path(temp) / 'review.c'
        binary = Path(temp) / 'review'
        source.write_text(shim)
        subprocess.run(['cc', '-Wall', '-Wextra', '-Werror', '-I', str(ROOT),
                        str(source), '-ldl', '-o', str(binary)], check=True)
        subprocess.run([str(binary)], capture_output=True, text=True, check=True)
        return {'synthetic_argument_cases_passed': 16,
                'adapter_source_sha256': digest(ROOT / 'native/linux/host_services.c'),
                'supplied_buffers_unchanged_on_refusal': True,
                'original_native_execution': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--host', type=Path, default=ROOT.parent / 'c6-assets-linux/Civ6')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = {'format': 'c6.linux-native-abi-review-check.v1', 'host_sha256': HOST,
              'native_static_anchors': anchors(args.host), 'synthetic_adapter_check': adapter_decode(),
              'synthetic_network_guards': network_guards(),
              'synthetic_argument_binding': argument_binding()}
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'static_anchors_passed': len(result['native_static_anchors']),
                      'synthetic_vectors_passed': result['synthetic_adapter_check']['synthetic_return_vectors_passed'],
                      'synthetic_receiver_guards_passed': result['synthetic_network_guards']['synthetic_receiver_guards_passed'],
                      'synthetic_argument_cases_passed': result['synthetic_argument_binding']['synthetic_argument_cases_passed']}))


if __name__ == '__main__':
    main()
