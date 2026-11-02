#!/usr/bin/env python3
"""Hash-gated native ELF context lifetime diagnostic; no initialized game."""
import argparse
import hashlib
import fcntl
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    h = hashlib.sha256()
    with open(path, 'rb') as source:
        for data in iter(lambda: source.read(1024 * 1024), b''):
            h.update(data)
    return h.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--sysroot', type=Path, default=ROOT / 'target/linux/sysroot')
    parser.add_argument('--expected-configuration')
    parser.add_argument('--initialize', action='store_true',
                        help='construct genuine host services and enter original Context Initialize')
    parser.add_argument('--startup', action='store_true',
                        help='also enter genuine StartupGame after Context Initialize')
    parser.add_argument('--network-host', action='store_true',
                        help='isolated native offline HostGame/session/Leave proof using default configuration')
    parser.add_argument('--attach-saved-session', action='store_true',
                        help='call original AttachToSession with native framework/network and saved config')
    parser.add_argument('--network', action='store_true',
                        help='use original Civ6App SetupNetwork with genuine main arguments')
    parser.add_argument('--framework-initialize', action='store_true',
                        help='let original AppModdingFramework create and own its Base context')
    parser.add_argument('--framework-configure', action='store_true',
                        help='also call original ConfigureContent with its native result contract')
    parser.add_argument('--prepare-host-first', action='store_true',
                        help='prepare genuine host services before any GameCore load or context factory')
    parser.add_argument('--save-lifecycle-check', action='store_true',
                        help='check original serializer missing-stream failure without a gameplay load')
    parser.add_argument('--filesystem', action='store_true',
                        help='initialize original ASL filesystem before GameCore loading')
    parser.add_argument('--host-launch-fixture', type=Path,
                        help='validated exact copied host and original ASL content aliases')
    parser.add_argument('--rules-source-view', type=Path,
                        help='gated original Linux source symlinks with native Schema directory case')
    parser.add_argument('--rules-snapshot-dir', type=Path,
                        help='private new directory for genuine Linux database exports')
    parser.add_argument('--lua', action='store_true',
                        help='create genuine host AppLuaScriptSystem singleton')
    parser.add_argument('--saved-configuration', type=Path,
                        help='restore original host configuration from exact Linux XP2 save')
    parser.add_argument('--saved-configuration-output', type=Path,
                        help='private original GameConfig writeback for restore comparison')
    parser.add_argument('--localization', action='store_true',
                        help='initialize genuine Linux base English localization fixture')
    parser.add_argument('--stream-check', action='store_true',
                        help='check genuine host memory stream boundaries')
    parser.add_argument('--load-stream', type=Path,
                        help='exact private XP2 benchmark GameCore payload; implies --startup')
    parser.add_argument('--save-output', type=Path,
                        help='private native Save output after original Load returns OK')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.network_host:
        if args.saved_configuration or args.framework_initialize or args.framework_configure or args.load_stream:
            parser.error('--network-host is an isolated default-configuration fixture; saved/framework/load modes are separate')
        args.network = args.startup = args.localization = args.lua = True
    if args.attach_saved_session:
        if not (args.network and args.saved_configuration and
                (args.framework_initialize or args.framework_configure)):
            parser.error('--attach-saved-session requires --network, --saved-configuration and --framework-initialize')
        if args.framework_configure:
            parser.error('the isolated attachment boundary cannot precede an unverified content transition')
    if args.network:
        args.prepare_host_first = True
    args.framework_initialize = args.framework_initialize or args.framework_configure
    if args.framework_initialize:
        if args.startup or args.load_stream or args.save_output or args.stream_check or args.save_lifecycle_check:
            parser.error('framework mode requires original framework-owned startup/load orchestration; independent-factory operations are refused')
        args.prepare_host_first = args.filesystem = args.localization = args.lua = True
    args.filesystem = args.filesystem or args.host_launch_fixture is not None
    if args.filesystem and not args.host_launch_fixture:
        parser.error('--filesystem requires --host-launch-fixture')
    if args.filesystem and args.rules_source_view:
        parser.error('original ASL filesystem and diagnostic Schema-only source view are separate profiles')
    args.startup = args.startup or args.load_stream is not None
    args.localization = args.localization or args.load_stream is not None
    args.lua = args.lua or args.load_stream is not None
    args.initialize = (args.initialize or args.startup or args.localization or args.lua
                       or args.saved_configuration is not None or args.rules_snapshot_dir is not None
                       or args.rules_source_view is not None or args.save_lifecycle_check or args.prepare_host_first or args.network)
    args.stream_check = args.stream_check or args.load_stream is not None
    if args.saved_configuration_output and not args.saved_configuration:
        parser.error('--saved-configuration-output requires --saved-configuration')
    if args.saved_configuration_output:
        args.saved_configuration_output = args.saved_configuration_output.resolve()
        if args.saved_configuration_output.exists():
            parser.error('--saved-configuration-output must not already exist')
        if (args.saved_configuration_output.is_relative_to(ROOT) and
                not args.saved_configuration_output.is_relative_to(ROOT / 'target')):
            parser.error('configuration writeback must stay private or in ignored target')
        args.saved_configuration_output.parent.mkdir(parents=True, exist_ok=True)
    if args.rules_snapshot_dir:
        args.rules_snapshot_dir = args.rules_snapshot_dir.resolve()
        if args.rules_snapshot_dir.exists():
            parser.error('--rules-snapshot-dir must be new')
        if args.rules_snapshot_dir.is_relative_to(ROOT) and not args.rules_snapshot_dir.is_relative_to(ROOT / 'target'):
            parser.error('rules snapshots must stay private or in ignored target')
        args.rules_snapshot_dir.parent.mkdir(parents=True, exist_ok=True)
    if args.save_output and not args.load_stream:
        parser.error('--save-output requires --load-stream')
    if args.save_output:
        args.save_output = args.save_output.resolve()
        if args.save_output.exists():
            parser.error('--save-output must not already exist')
        if args.save_output.is_relative_to(ROOT) and not args.save_output.is_relative_to(ROOT / 'target'):
            parser.error('proprietary Save output must stay outside the public repository or in ignored target')
        args.save_output.parent.mkdir(parents=True, exist_ok=True)
    assets = args.assets.resolve()
    if args.rules_snapshot_dir and args.rules_snapshot_dir.is_relative_to(assets):
        parser.error('rules snapshots must not modify original assets')
    if args.saved_configuration_output and args.saved_configuration_output.is_relative_to(assets):
        parser.error('configuration output must not modify original assets')
    if args.save_output and args.save_output.is_relative_to(assets):
        parser.error('Save output must not modify the original asset tree')
    if args.saved_configuration:
        args.saved_configuration = args.saved_configuration.resolve()
        if digest(args.saved_configuration) != 'a328d643201cb11ba226eee46c782f85b0df36b2a7a4e1a32372da8abe9cddb8':
            raise SystemExit('saved configuration source must be exact Linux XP2 benchmark')
    work = ROOT / 'target/linux/runtime'
    work.mkdir(parents=True, exist_ok=True)
    runtime_lock = (work / 'runtime.lock').open('w')
    fcntl.flock(runtime_lock, fcntl.LOCK_EX)
    # Keep this descriptor alive through child execution: concurrent runners
    # must not truncate a preload that another native child has mapped.
    # Only genuine supplied libraries and genuine system libraries are providers.
    # No game assets or original executables are copied or modified.
    gate = json.loads((ROOT / 'tools/linux/runtime-gate.json').read_text())
    for name, expected in gate.items():
        if digest(assets / name) != expected:
            raise SystemExit(f'original asset SHA-256 mismatch: {name}')
    stream_fixture = None
    if args.load_stream:
        args.load_stream = args.load_stream.resolve()
        save_source = assets / 'Base/assets/benchmark/saves/xp2benchmark.civ6save'
        if digest(save_source) != 'a328d643201cb11ba226eee46c782f85b0df36b2a7a4e1a32372da8abe9cddb8':
            raise SystemExit('Linux XP2 benchmark source identity mismatch')
        if (args.load_stream.stat().st_size != 76556938 or digest(args.load_stream) !=
                'a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa'):
            raise SystemExit('XP2 benchmark GameCore payload identity mismatch')
        stream_fixture = {'source_sha256': digest(save_source),
                          'payload_sha256': digest(args.load_stream),
                          'payload_bytes': args.load_stream.stat().st_size,
                          'saved_configuration_restore_requested': bool(args.saved_configuration),
                          'source': str(save_source.relative_to(assets))}
    launch_profile = None
    host = assets / 'Civ6'
    if args.host_launch_fixture:
        from rules_launch_fixture import validate
        args.host_launch_fixture = args.host_launch_fixture.resolve(strict=True)
        launch_profile = validate(assets, args.host_launch_fixture)
        host = args.host_launch_fixture / 'Civ6'
        if args.framework_initialize and launch_profile.get('format') != 'c6.linux-original-asl-launch-fixture.v2':
            parser.error('framework initialization requires the gated v2 sibling-module launch fixture')
    content_gate = None
    if args.initialize:
        content_gate = json.loads((ROOT / 'tools/linux/runtime-content-gate.json').read_text())
        for name, expected in content_gate['files'].items():
            if digest(assets / name) != expected:
                raise SystemExit(f'content source SHA-256 mismatch: {name}')
        actual_sources = {str(p.relative_to(assets)) for directory in
                          ['Base/assets/configuration/data', 'Base/assets/gameplay/data']
                          for p in (assets / directory).rglob('*') if p.is_file()}
        if actual_sources != set(content_gate['files']):
            raise SystemExit('content source inventory mismatch')
    rules_source_profile = None
    if args.rules_source_view:
        args.rules_source_view = args.rules_source_view.resolve(strict=True)
        manifest_path = args.rules_source_view / 'source-view.json'
        rules_source_profile = json.loads(manifest_path.read_text())
        mapping = [{'source': name,
                    'staged': str(Path(*('Schema' if p == 'schema' else p for p in Path(name).parts))),
                    'sha256': expected} for name, expected in sorted(content_gate['files'].items())]
        view_identity = hashlib.sha256(json.dumps(mapping, sort_keys=True,
                                      separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()
        if (rules_source_profile.get('mapping') != mapping or
                rules_source_profile.get('source_view_sha256') != view_identity or
                rules_source_profile.get('original_source_gate_sha256') !=
                digest(ROOT / 'tools/linux/runtime-content-gate.json')):
            raise SystemExit('original Linux rules source-view identity mismatch')
        for item in mapping:
            view = args.rules_source_view / item['staged']
            if not view.is_symlink() or view.resolve(strict=True) != (assets / item['source']).resolve():
                raise SystemExit('original Linux source-view symlink mismatch')
        declared = {args.rules_source_view / item['staged'] for item in mapping} | {manifest_path}
        actual_view = {p for p in args.rules_source_view.rglob('*') if p.is_file() or p.is_symlink()}
        if actual_view != declared:
            raise SystemExit('original Linux source-view inventory mismatch')
    localization_gate = None
    localization_env = {}
    if args.localization:
        from localization_gate import prepare
        localization_gate, localization_env = prepare(assets, ROOT / 'target/linux-localization')
    library_path = f'{args.sysroot.resolve()}/usr/lib/x86_64-linux-gnu:{assets}'
    env = os.environ.copy()
    # Do not inherit arbitrary loader hooks into the diagnostic child.
    for name in list(env):
        if name.startswith('LD_') or name.startswith('C6_'):
            del env[name]
    env['LD_LIBRARY_PATH'] = library_path
    listing = subprocess.run(['ldd', str(host)], cwd=work,
                             env=env, capture_output=True, text=True, check=True).stdout
    if 'not found' in listing:
        raise SystemExit(f'unavailable native runtime dependency:\n{listing}')
    dependencies = {}
    for line in listing.splitlines():
        match = re.search(r'(?:=>\s+)?(/\S+)\s+\(', line)
        if match:
            path = Path(match[1]).resolve()
            dependencies[path.name] = {'path': str(path), 'sha256': digest(path)}
    source = ROOT / 'native/linux/host_probe.c'
    probe = work / 'host_probe.so'
    compiler = subprocess.check_output(['cc', '--version'], text=True).splitlines()[0]
    extra_sources = [ROOT / 'native/linux/context_initialize.c',
                     ROOT / 'native/linux/host_services.c'] if args.initialize else []
    if args.network:
        extra_sources += [ROOT / 'native/linux/network.c']
    if args.framework_initialize:
        extra_sources += [ROOT / 'native/linux/framework_runtime.c', ROOT / 'native/linux/mod_activation.c']
    if args.save_lifecycle_check or args.attach_saved_session:
        extra_sources += [ROOT / 'native/linux/save_lifecycle.c']
    if args.filesystem:
        extra_sources += [ROOT / 'native/linux/filesystem.c']
    if args.rules_snapshot_dir:
        extra_sources += [ROOT / 'native/linux/rules_snapshot.c']
    if args.saved_configuration:
        extra_sources += [ROOT / 'native/linux/saved_configuration.c']
    if args.localization:
        extra_sources += [ROOT / 'native/linux/localization.c']
    if args.stream_check:
        extra_sources += [ROOT / 'native/linux/context_stream.c']
    subprocess.run(['cc', '-shared', '-fPIC', '-Wall', '-Wextra', '-Werror',
                    str(source), *map(str, extra_sources), '-ldl', '-o', str(probe)], check=True)
    configuration = {
        'schema': 'c6-linux-context-lifetime-v1',
        'runner_source_sha256': digest(Path(__file__)),
        'original_assets': gate,
        'runtime_dependencies': dependencies,
        'probe_source_sha256': digest(source),
        'probe_sha256': digest(probe),
        'compiler': compiler,
        'mode': ('host-crt-init-original-framework-owned-base-context' if args.framework_initialize
                 else 'host-crt-init-original-host-services-context-initialize' if args.initialize
                 else 'host-crt-init-intercept-main-whole-gamecore-factory-3-cycles'),
        'rules_database': None, 'enabled_dlc_mods': None,
        'initialized_game': False,
        'host_application_main_called': False,
        'host_application_shutdown_called': False,
    }
    if args.network:
        configuration['native_network_profile'] = {
            'mode': ('original-Civ6App-SetupNetwork-and-saved-AttachToSession' if args.attach_saved_session
                     else 'original-offline-HostGame-default-configuration' if args.network_host
                     else 'original-Civ6App-SetupNetwork-no-session'),
            'arguments': 'genuine-CRT-main-argc1-argv0-host',
            'header_sha256': digest(ROOT / 'native/linux/network.h')}
    if args.framework_initialize:
        configuration['framework_owner_profile'] = {
            'configure_requested': args.framework_configure, 'client_fixture': 'original-unstarted-AppFiraxisLiveClient',
            'header_sources': {str(p.relative_to(ROOT)): digest(p) for p in
                [ROOT / 'native/linux/framework_runtime.h', ROOT / 'native/linux/mod_activation.h']}}
    if args.attach_saved_session:
        configuration['saved_session_attachment_profile'] = {
            'requested': True, 'scope': 'original-host-attachment-with-framework-owned-Base-context',
            'header_sha256': digest(ROOT / 'native/linux/save_lifecycle.h')}
    if args.save_lifecycle_check:
        configuration['save_lifecycle_boundary_profile'] = {
            'requested': True, 'header_sha256': digest(ROOT / 'native/linux/save_lifecycle.h')}
    if launch_profile:
        configuration['original_asl_launch_profile'] = launch_profile
        configuration['filesystem_adapter_sources'] = {str(p.relative_to(ROOT)): digest(p) for p in
            [ROOT / 'native/linux/filesystem.c', ROOT / 'native/linux/filesystem.h',
             ROOT / 'tools/linux/rules_launch_fixture.py']}
    if rules_source_profile:
        configuration['rules_source_view'] = rules_source_profile
    if args.rules_snapshot_dir:
        configuration['rules_snapshot_profile'] = {
            'requested': True, 'header_sha256': digest(ROOT / 'native/linux/rules_snapshot.h')}
    if args.saved_configuration:
        configuration['saved_configuration_profile'] = {
            'source_sha256': digest(args.saved_configuration),
            'restoration_requested': True,
            'native_writeback_requested': bool(args.saved_configuration_output),
            'adapter_header_sha256': digest(ROOT / 'native/linux/saved_configuration.h')}
    if args.localization:
        configuration['localization_profile'] = localization_gate
        configuration['localization_environment'] = localization_env
        configuration['localization_adapter_sources'] = {str(p.relative_to(ROOT)): digest(p) for p in
            [ROOT / 'native/linux/localization.c', ROOT / 'native/linux/localization.h',
             ROOT / 'tools/linux/localization_gate.py']}
    if args.stream_check:
        configuration['native_stream_adapter'] = {
            'source_sha256': digest(ROOT / 'native/linux/context_stream.c'),
            'header_sha256': digest(ROOT / 'native/linux/context_stream.h'),
            'boundary_fixture': 'native input offset0/end/reset; output empty/full/overflow-no-mutation'}
    if stream_fixture:
        configuration['load_stream_fixture'] = stream_fixture
        configuration['native_save_requested'] = bool(args.save_output)
    if args.initialize:
        configuration['content_profile'] = content_gate
        configuration['startup_game_requested'] = args.startup
        configuration['prepare_host_before_gamecore'] = args.prepare_host_first
        configuration['host_lua_requested'] = args.lua
        configuration['host_userdata'] = str(ROOT / 'target/linux-runtime/userdata')
        configuration['host_adapter_sources'] = {
            str(p.relative_to(ROOT)): digest(p) for p in
            [*extra_sources, ROOT / 'native/linux/host_services.h',
             ROOT / 'native/linux/context_initialize.h']}
    identity = hashlib.sha256(json.dumps(configuration, sort_keys=True,
                              separators=(',', ':')).encode()).hexdigest()
    if args.expected_configuration and args.expected_configuration != identity:
        raise SystemExit('configuration identity mismatch; original execution refused')
    env['LD_PRELOAD'] = str(probe)
    env['C6_GAMECORE'] = str(assets / 'libGameCore_XP2.so')
    if args.network_host:
        env['C6_NETWORK_HOST'] = '1'
    if args.attach_saved_session:
        env['C6_ATTACH_SAVED_SESSION'] = '1'
    if args.network:
        env['C6_HOST_NETWORK'] = '1'
    if args.framework_initialize:
        env['C6_FRAMEWORK'] = env['C6_MOD_CLIENT_CONSTRUCTOR'] = '1'
        if args.framework_configure:
            env['C6_FRAMEWORK_CONFIGURE'] = '1'
    if args.prepare_host_first:
        env['C6_PREPARE_HOST_FIRST'] = '1'
    if args.save_lifecycle_check:
        env['C6_SAVE_LIFECYCLE_CHECK'] = '1'
    if args.filesystem:
        env['C6_FILESYSTEM'] = '1'
        env['C6_RULES_SOURCE_ROOT'] = str(args.host_launch_fixture / 'steamassets')
    if args.rules_source_view:
        env['C6_RULES_SOURCE_ROOT'] = str(args.rules_source_view)
    if args.rules_snapshot_dir:
        args.rules_snapshot_dir.mkdir()
        env['C6_RULES_SNAPSHOT_DIR'] = str(args.rules_snapshot_dir)
    if args.saved_configuration_output:
        env['C6_SAVED_CONFIGURATION_OUTPUT'] = str(args.saved_configuration_output)
    if args.saved_configuration:
        env['C6_SAVED_CONFIGURATION'] = str(args.saved_configuration)
    if args.lua:
        env['C6_HOST_LUA'] = '1'
    if args.localization:
        env.update(localization_env)
        env['C6_LOCALIZATION'] = '1'
    if args.stream_check:
        env['C6_STREAM_CHECK'] = '1'
    if args.load_stream:
        env['C6_LOAD_STREAM'] = str(args.load_stream)
    if args.save_output:
        env['C6_SAVE_OUTPUT'] = str(args.save_output)
    if args.initialize:
        env['C6_INITIALIZE'] = '1'
        if args.startup:
            env['C6_STARTUP'] = '1'
        env['C6_HOST_ASSETS'] = str(assets)
        userdata = ROOT / 'target/linux-runtime/userdata'
        userdata.mkdir(parents=True, exist_ok=True)
        env['C6_HOST_USERDATA'] = str(userdata)
    try:
        run = subprocess.run([str(host)], cwd=work, env=env,
                             capture_output=True, text=True, timeout=30)
        code, out, err = run.returncode, run.stdout, run.stderr
    except subprocess.TimeoutExpired as exc:
        def timeout_text(value):
            return value.decode('utf-8', errors='replace') if isinstance(value, bytes) else value or ''
        code, out, err = 124, timeout_text(exc.stdout), timeout_text(exc.stderr)
    expected = ['C6_LINUX host_main_intercepted']
    if args.filesystem:
        expected += ['C6_LINUX filesystem_enter', 'C6_LINUX filesystem_return 0']
    host_expected = ['C6_LINUX host_services_enter', 'C6_LINUX host_services_return']
    if args.localization:
        host_expected += ['C6_LINUX localization_enter', 'C6_LINUX localization_return 0']
    if args.prepare_host_first:
        expected += host_expected
    expected += ['C6_LINUX whole_gamecore_loaded']
    load_match = re.search(r'^C6_LINUX_STREAM load_return result (-?\d+) offset (\d+) size (\d+)$', err, re.M)
    fault_match = re.search(r'^C6_LINUX_STREAM fault signal (\d+) offset (\d+) instruction (\d+) gamecore_rva (\d+)$', err, re.M)
    save_match = re.search(r'^C6_LINUX_STREAM save_return result (-?\d+) size (\d+)$', err, re.M)
    if args.stream_check:
        expected += ['C6_LINUX_STREAM boundary_check pass']
    for i in range(3):
        expected += [f'C6_LINUX create_enter {i}', f'C6_LINUX create_return {i} nonnull']
        if i == 0 and args.initialize:
            if not args.prepare_host_first:
                expected += host_expected
            expected += ['C6_LINUX set_engine_enter', 'C6_LINUX set_engine_return',
                         'C6_LINUX is_initialized_before 0', 'C6_LINUX initialize_enter',
                         'C6_LINUX initialize_return', 'C6_LINUX is_initialized_after 1']
        if i == 0 and args.saved_configuration:
            expected += ['C6_LINUX saved_configuration_enter', 'C6_LINUX saved_configuration_return 0']
        if i == 0 and args.rules_snapshot_dir:
            expected += ['C6_LINUX rules_snapshot_enter', 'C6_LINUX rules_snapshot_return 0']
        if i == 0 and args.save_lifecycle_check:
            expected += ['C6_LINUX save_lifecycle_check_enter', 'C6_LINUX save_lifecycle_check_return 0']
        if i == 0 and args.startup:
            expected += ['C6_LINUX startup_enter', 'C6_LINUX startup_return']
        if i == 0 and args.network_host:
            expected += ['C6_LINUX network_host_enter', 'C6_LINUX network_host_return 0']
        if i == 0 and args.load_stream:
            expected += ['C6_LINUX_STREAM load_enter size 76556938']
            if load_match and int(load_match[1]) == 0 and 0 < int(load_match[2]) <= 76556938:
                expected += [load_match[0]]
            else:
                expected += ['C6_LINUX_STREAM expected_native_Load_OK']
        if i == 0 and args.save_output:
            expected += ['C6_LINUX_STREAM save_enter capacity 134217728']
            expected += [save_match[0] if save_match and int(save_match[1]) == 0
                         else 'C6_LINUX_STREAM expected_native_Save_OK']
        if i == 0 and args.network_host:
            expected += ['C6_LINUX network_leave_enter', 'C6_LINUX network_leave_return 0']
        expected += [f'C6_LINUX destroy_return {i}']
    if args.initialize:
        expected += ['C6_LINUX host_services_destroy_enter',
                     'C6_LINUX host_services_destroy_return 0']
    expected += ['C6_LINUX dlclose_return 0']
    if args.filesystem:
        expected += ['C6_LINUX filesystem_shutdown_return 0']
    context_module_match = re.search(r'^C6_LINUX_FRAMEWORK borrowed_context_path (.+)$', err, re.M)
    context_module_identity = None
    if context_module_match:
        observed_path = Path(context_module_match[1]).resolve()
        expected_path = (assets / 'libGameCore_Base.so').resolve()
        expected_sha = '8ded6da8035e5e3d6940b5d1dde9d653b6f61947647704523dbc278bcc45bef6'
        context_module_identity = {'path': context_module_match[1],
                                   'canonical_path': str(observed_path),
                                   'expected_sha256': expected_sha,
                                   'sha256': digest(observed_path) if observed_path.is_file() else None}
        context_module_identity['matches_gated_Base'] = (observed_path == expected_path and
                                                        context_module_identity['sha256'] == expected_sha)
    if args.framework_initialize:
        expected = ['C6_LINUX host_main_intercepted', 'C6_LINUX filesystem_enter',
                    'C6_LINUX filesystem_return 0', *host_expected,
                    'C6_LINUX_FRAMEWORK asset_working_directory_return',
                    'C6_LINUX_FRAMEWORK initialize_enter', 'C6_LINUX_FRAMEWORK initialize_return 0',
                    context_module_match[0] if context_module_identity and context_module_identity['matches_gated_Base']
                    else 'C6_LINUX_FRAMEWORK expected_gated_Base_module_path',
                    'C6_LINUX_FRAMEWORK borrowed_context_vtable _ZTVN8GameCore7Exports7ContextE',
                    'C6_LINUX_FRAMEWORK borrowed_context_module libGameCore_Base.so',
                    'C6_LINUX_FRAMEWORK app_database_connections configuration 1 gameplay 1']
        if args.saved_configuration:
            expected += ['C6_LINUX saved_configuration_enter', 'C6_LINUX saved_configuration_return 0']
        if args.attach_saved_session:
            expected += ['C6_LINUX_FRAMEWORK attach_session_enter', 'C6_LINUX_FRAMEWORK attach_session_return 0']
        if args.framework_configure:
            expected += ['C6_LINUX_FRAMEWORK configure_enter', 'C6_LINUX_FRAMEWORK configure_return 0']
        if args.rules_snapshot_dir:
            expected += ['C6_LINUX rules_snapshot_enter', 'C6_LINUX rules_snapshot_return 0']
        if args.attach_saved_session:
            expected += ['C6_LINUX_FRAMEWORK leave_session_enter', 'C6_LINUX_FRAMEWORK leave_session_return 0']
        expected += ['C6_LINUX_FRAMEWORK shutdown_enter', 'C6_LINUX_FRAMEWORK shutdown_return 0',
                     'C6_LINUX_FRAMEWORK client_shutdown_return 0',
                     'C6_LINUX host_services_destroy_enter', 'C6_LINUX host_services_destroy_return 0',
                     'C6_LINUX filesystem_shutdown_return 0']
    observed = [line for line in err.splitlines()
                if not ((args.initialize and line.startswith('C6_LINUX_HOST ')) or
                        (args.localization and line.startswith('C6_LINUX_LOCALIZATION ')) or
                        (args.saved_configuration and line.startswith('C6_LINUX_CONFIGURATION ')) or
                        (args.rules_snapshot_dir and line.startswith('C6_LINUX_RULES ')) or
                        (args.filesystem and line.startswith('C6_LINUX_FILESYSTEM ')) or
                        ((args.save_lifecycle_check or args.attach_saved_session) and line.startswith('C6_LINUX_SAVE_LIFECYCLE ')) or
                        (args.framework_initialize and line.startswith('C6_LINUX_MOD ')) or
                        (args.network and line.startswith('C6_LINUX_NETWORK ')))]
    passed = code == 0 and observed == expected
    result = {'configuration_sha256': identity, 'configuration': configuration,
              'exit_code': code, 'stdout': out, 'stderr': err,
              'whole_gamecore_loaded': ('C6_LINUX whole_gamecore_loaded' in err or
                                        'C6_LINUX_FRAMEWORK initialize_return 0' in err.splitlines()),
              'native_offline_session_hosted': 'C6_LINUX network_host_return 0' in err.splitlines(),
              'native_offline_session_left': 'C6_LINUX network_leave_return 0' in err.splitlines(),
              'native_saved_session_attached': 'C6_LINUX_FRAMEWORK attach_session_return 0' in err.splitlines(),
              'native_saved_session_left': 'C6_LINUX_FRAMEWORK leave_session_return 0' in err.splitlines(),
              'framework_context_module_identity': context_module_identity,
              'framework_app_database_connections_present': 'C6_LINUX_FRAMEWORK app_database_connections configuration 1 gameplay 1' in err.splitlines(),
              'framework_owned_context': args.framework_initialize,
              'framework_initialize_returned_ok': 'C6_LINUX_FRAMEWORK initialize_return 0' in err.splitlines(),
              'framework_owner_shutdown_returned_ok': 'C6_LINUX_FRAMEWORK shutdown_return 0' in err.splitlines(),
              'active_context_module': ('libGameCore_Base.so' if
                    'C6_LINUX_FRAMEWORK borrowed_context_module libGameCore_Base.so' in err.splitlines() else None),
              'context_lifetime_cycles_passed': 3 if passed and not args.framework_initialize else 0,
              'diagnostic_passed': passed, 'initialized_game': False,
              'original_network_setup_returned_ok': 'C6_LINUX_NETWORK setup_return 1 providers_ready 1 no_session 1' in err.splitlines(),
              'native_save_lifecycle_boundary_passed': 'C6_LINUX save_lifecycle_check_return 0' in err.splitlines(),
              'original_filesystem_initialized': 'C6_LINUX filesystem_return 0' in err.splitlines(),
              'rules_snapshots': ([{'filename': p.name, 'sha256': digest(p),
                                    'bytes': p.stat().st_size}
                                   for p in sorted(args.rules_snapshot_dir.glob('*.sqlite'))]
                                  if args.rules_snapshot_dir else []),
              'rules_snapshot_returned': 'C6_LINUX rules_snapshot_return 0' in err.splitlines(),
              'saved_configuration_writeback_sha256': (digest(args.saved_configuration_output) if
                    args.saved_configuration_output and args.saved_configuration_output.is_file() else None),
              'saved_configuration_restored': 'C6_LINUX saved_configuration_return 0' in err.splitlines(),
              'localization_initialized': 'C6_LINUX localization_return 0' in err.splitlines(),
              'startup_game_returned': 'C6_LINUX startup_return' in err.splitlines(),
              'context_initialize_returned': 'C6_LINUX initialize_return' in err.splitlines(),
              'context_initialized_flag': 'C6_LINUX is_initialized_after 1' in err.splitlines(),
              'native_stream_boundary_check_passed': 'C6_LINUX_STREAM boundary_check pass' in err.splitlines(),
              'native_load_attempted': 'C6_LINUX_STREAM load_enter size 76556938' in err.splitlines(),
              'native_stream_fault_signal': int(fault_match[1]) if fault_match else None,
              'native_stream_fault_offset': int(fault_match[2]) if fault_match else None,
              'native_stream_fault_gamecore_rva': int(fault_match[4]) if fault_match else None,
              'native_stream_fault_instruction': int(fault_match[3]) if fault_match else None,
              'native_load_returned': load_match is not None,
              'native_load_result': int(load_match[1]) if load_match else None,
              'native_load_offset': int(load_match[2]) if load_match else None,
              'native_save_returned': save_match is not None,
              'native_save_result': int(save_match[1]) if save_match else None,
              'native_save_bytes': int(save_match[2]) if save_match else None,
              'native_save_output_sha256': (digest(args.save_output) if args.save_output and
                                            args.save_output.is_file() else None),
              'save_loaded': False, 'action_or_turn_parity': False,
              'limits': ['host App main and shutdown intentionally bypassed',
                         'no initialized gameplay session, map, players or rules fixture',
                         'separate Linux lineage; Windows parity is not implied']}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'diagnostic_passed': passed, 'configuration_sha256': identity,
                      'exit_code': code, 'output': str(args.output)}))
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main())
