#!/usr/bin/env python3
"""Identity-gated isolated original Linux mod-framework boundary; no standalone context."""
import argparse
import hashlib
import tempfile
import re
import fcntl
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]

def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets',type=Path,default=ROOT.parent/'c6-assets-linux')
    parser.add_argument('--source-view',type=Path)
    parser.add_argument('--launch-host',type=Path)
    parser.add_argument('--saved-configuration',type=Path)
    parser.add_argument('--configure',action='store_true')
    parser.add_argument('--content-paths',action='store_true')
    parser.add_argument('--art-manager',action='store_true')
    parser.add_argument('--update-constructor-callback',action='store_true')
    parser.add_argument('--ui-constructor',action='store_true')
    parser.add_argument('--rules-snapshot-dir',type=Path)
    parser.add_argument('--network',action='store_true')
    parser.add_argument('--attach-saved-session',action='store_true')
    parser.add_argument('--game-load-stream',type=Path)
    parser.add_argument('--game-save-output',type=Path)
    parser.add_argument('--game-save-repeat-output',type=Path)
    parser.add_argument('--timeout',type=int,default=30)
    parser.add_argument('--localization',action='store_true')
    parser.add_argument('--cleanup',action='store_true')
    parser.add_argument('--client-only',action='store_true')
    parser.add_argument('--client-constructor',action='store_true')
    parser.add_argument('--expected-configuration')
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();assets=args.assets.resolve();view=args.source_view.resolve() if args.source_view else None
    if args.update_constructor_callback and not all((args.saved_configuration,args.configure,args.localization,args.content_paths,args.art_manager)):
        parser.error('--update-constructor-callback requires saved configuration, configure, localization, content paths and art manager')
    if args.ui_constructor and not args.localization:parser.error('--ui-constructor requires original localization')
    if args.attach_saved_session and not all((args.network,args.saved_configuration,args.cleanup)):
        parser.error('--attach-saved-session requires network, saved configuration and cleanup')
    if not 1<=args.timeout<=600:parser.error('--timeout must be between 1 and 600 seconds')
    game_input=args.game_load_stream.resolve() if args.game_load_stream else None
    game_outputs={}
    if game_input:
        if not all((args.attach_saved_session,args.update_constructor_callback,args.ui_constructor,args.game_save_output,args.game_save_repeat_output)):
            parser.error('--game-load-stream requires attached session, native update, UI constructor and both save output paths')
        if (not game_input.is_file() or game_input.stat().st_size!=76556938 or
            digest(game_input)!='a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa'):
            raise SystemExit('lower Core stream identity mismatch')
        for label,path in (('save',args.game_save_output),('repeat',args.game_save_repeat_output)):
            path=path.resolve()
            if path.exists() or not path.parent.is_dir() or not path.is_relative_to((ROOT.parent/'c6-ghidra').resolve()):
                parser.error('game save outputs must be new files in an existing private c6-ghidra directory')
            game_outputs[label]=path
        if game_outputs['save']==game_outputs['repeat']:parser.error('game save outputs must be distinct')
    elif args.game_save_output or args.game_save_repeat_output:
        parser.error('game save output paths require --game-load-stream')
    snapshot=args.rules_snapshot_dir.resolve() if args.rules_snapshot_dir else None
    if snapshot and (not args.update_constructor_callback or not snapshot.is_dir() or any(snapshot.iterdir()) or
                     not snapshot.is_relative_to((ROOT.parent/'c6-ghidra').resolve())):
        parser.error('--rules-snapshot-dir requires update callback and an existing empty private c6-ghidra directory')
    if not view and not args.launch_host:parser.error('--source-view or --launch-host required')
    gate=json.loads((ROOT/'tools/linux/runtime-gate.json').read_text())
    gate.update({'libGameCore_Base.so':'8ded6da8035e5e3d6940b5d1dde9d653b6f61947647704523dbc278bcc45bef6',
                 'libGameCore_XP1.so':'431f6e33f0fb5ac081016d14868cff60b3c3c2417acee82e75e23f24c466c80b'})
    for name,expected in gate.items():
        if digest(assets/name)!=expected:raise SystemExit(f'asset identity mismatch: {name}')
    content=json.loads((ROOT/'tools/linux/runtime-content-gate.json').read_text())['files']
    if view:
        profile=json.loads((view/'source-view.json').read_text())
        mapping=[{'source':name,'staged':str(Path(*('Schema' if part=='schema' else part for part in Path(name).parts))),
                  'sha256':expected} for name,expected in sorted(content.items())]
        if profile['mapping']!=mapping:raise SystemExit('source-view mapping mismatch')
        for row in mapping:
            source=assets/row['source'];staged=view/row['staged']
            if digest(source)!=row['sha256'] or not staged.is_symlink() or staged.resolve()!=source.resolve():
                raise SystemExit('source view identity mismatch')
    host=assets/'Civ6'
    filesystem_sources=[]
    filesystem_identity=None
    if args.launch_host:
        host=args.launch_host.absolute()
        if host.is_symlink() or host.resolve().is_relative_to(assets) or digest(host)!=gate['Civ6']:
            raise SystemExit('copied launch host identity mismatch')
        from rules_launch_fixture import validate
        launch_profile=validate(assets,host.parent)
        metadata=json.loads((ROOT/'tools/linux/filesystem_gate.json').read_text())
        full_inputs=json.loads((ROOT/'evidence/linux/rules-content-inputs.json').read_text())
        for profile_inputs in (metadata['files'],full_inputs['files']):
            for name,expected in profile_inputs.items():
                if digest(assets/name)!=expected:raise SystemExit('native content input mismatch')
        filesystem_sources=[ROOT/'native/linux/filesystem.c']
        filesystem_identity={'launch_host':str(host),'launch_fixture_sha256':digest(host.parent/'launch-fixture.json'),'validated_launch':launch_profile,
                             'metadata_gate_sha256':digest(ROOT/'tools/linux/filesystem_gate.json'),
                             'full_content_input_gate_sha256':digest(ROOT/'evidence/linux/rules-content-inputs.json')}
    sources=[ROOT/'native/linux/mod_activation.c',ROOT/'native/linux/mod_activation.h',
             ROOT/'native/linux/host_services.c',ROOT/'native/linux/host_services.h',
             ROOT/'native/linux/temp_heap.c',ROOT/'native/linux/temp_heap.h']
    snapshot_sources=[ROOT/'native/linux/rules_snapshot.c'] if snapshot else []
    network_sources=[ROOT/'native/linux/network.c'] if args.network else []
    lifecycle_sources=[ROOT/'native/linux/save_lifecycle.c'] if args.attach_saved_session else []
    game_sources=[ROOT/'native/linux/game_load.c',ROOT/'native/linux/context_stream.c'] if game_input else []
    configuration_sources=[]
    if args.saved_configuration:
        if digest(args.saved_configuration)!='a328d643201cb11ba226eee46c782f85b0df36b2a7a4e1a32372da8abe9cddb8':raise SystemExit('saved configuration identity mismatch')
        configuration_sources=[ROOT/'native/linux/saved_configuration.c']
    localization_sources=[]
    localization_profile=None
    localization_environment={}
    if args.localization:
        from localization_gate import prepare
        localization_profile,localization_environment=prepare(assets,ROOT/'target/linux-mod-localization')
        localization_sources=[ROOT/'native/linux/localization.c']
    # Full descriptor inventory gates original discovery inputs; data actions are
    # not yet invoked by this init/configure-only probe.
    descriptors={str(p.relative_to(assets)):digest(p) for p in sorted((assets/'DLC').rglob('*.modinfo'))}
    work=ROOT/'target/linux/mod-activation';work.mkdir(parents=True,exist_ok=True)
    lock=(work/'probe.lock').open('w');fcntl.flock(lock,fcntl.LOCK_EX)
    userdata=Path(tempfile.mkdtemp(prefix='userdata-',dir=work))
    preload=work/'mod_activation_probe.so'
    identity={'schema':'c6-linux-mod-activation-v1','assets':gate,'base_content_gate_sha256':digest(ROOT/'tools/linux/runtime-content-gate.json'),
              'update_callback_profile':'native LoadGameViewState constructor only; requestNone and constructor-default censorship=false; no saved Request/Begin, not saved-game parity' if args.update_constructor_callback else None,
              'ui_constructor_only':args.ui_constructor,
              'native_rules_snapshot_requested':bool(snapshot),
              'native_network_requested':args.network,'native_saved_session_control_requested':args.attach_saved_session,
              'lower_core_stream':{'sha256':digest(game_input),'bytes':game_input.stat().st_size} if game_input else None,
              'timeout_seconds':args.timeout,
              'source_view_manifest_sha256':digest(view/'source-view.json') if view else None,'dlc_descriptors':descriptors,
              'sources':{str(p.relative_to(ROOT)):digest(p) for p in sources},'configure_requested':args.configure,'content_paths_requested':args.content_paths,'art_manager_requested':args.art_manager,'localization_profile':localization_profile,'cleanup_requested':args.cleanup,'client_constructor_requested':args.client_constructor,'client_only':args.client_only,
              'source_view':str(view) if view else None,'userdata_policy':'new empty private task directory','userdata_initially_empty':True,'cwd':str(work),
              'runner_sha256':digest(Path(__file__)),'original_filesystem_profile':filesystem_identity,
              'saved_configuration_sha256':digest(args.saved_configuration) if args.saved_configuration else None}
    if configuration_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*configuration_sources,ROOT/'native/linux/saved_configuration.h']})
    if snapshot_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*snapshot_sources,ROOT/'native/linux/rules_snapshot.h']})
    if network_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*network_sources,ROOT/'native/linux/network.h']})
    if lifecycle_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*lifecycle_sources,ROOT/'native/linux/save_lifecycle.h']})
    if game_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*game_sources,ROOT/'native/linux/game_load.h',ROOT/'native/linux/context_stream.h']})
    if localization_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*localization_sources,ROOT/'native/linux/localization.h',ROOT/'tools/linux/localization_gate.py']})
    if filesystem_sources:
        identity['sources'].update({str(p.relative_to(ROOT)):digest(p) for p in [*filesystem_sources,ROOT/'native/linux/filesystem.h']})
    subprocess.run(['cc','-shared','-fPIC','-Wall','-Wextra','-Werror','-DC6_MOD_ACTIVATION_PROBE_MAIN',
                    str(sources[0]),str(sources[2]),str(sources[4]),*map(str,[*filesystem_sources,*configuration_sources,*localization_sources,*snapshot_sources,*network_sources,*lifecycle_sources,*game_sources]),'-ldl','-o',str(preload)],check=True)
    identity['preload_sha256']=digest(preload)
    dependency_env=dict(os.environ,LD_LIBRARY_PATH=f'{ROOT}/target/linux/sysroot/usr/lib/x86_64-linux-gnu:{assets}')
    listing=subprocess.check_output(['ldd',str(host)],env=dependency_env,text=True)
    if 'not found' in listing:raise SystemExit('native runtime dependency missing')
    identity['runtime_dependencies']={}
    for line in listing.splitlines():
        match=re.search(r'(?:=>\s+)?(/\S+)\s+\(',line)
        if match:
            path=Path(match[1]).resolve();identity['runtime_dependencies'][path.name]={'path':str(path),'sha256':digest(path)}
    if any(digest(ROOT/p)!=expected for p,expected in identity['sources'].items()):raise SystemExit('source changed during compile')
    env={k:v for k,v in os.environ.items() if not k.startswith(('LD_','C6_'))}
    env.update({'LD_PRELOAD':str(preload),'LD_LIBRARY_PATH':f'{ROOT}/target/linux/sysroot/usr/lib/x86_64-linux-gnu:{assets}',
                'C6_HOST_ASSETS':str(assets),'C6_RULES_SOURCE_ROOT':str(view) if view else str(host.parent/'steamassets'),'C6_HOST_USERDATA':str(userdata),'C6_HOST_LUA':'1'})
    if args.launch_host:env['C6_MOD_FILESYSTEM']='1'
    if args.saved_configuration:env['C6_SAVED_CONFIGURATION']=str(args.saved_configuration.resolve())
    if args.cleanup:env['C6_MOD_CLEANUP']='1'
    if args.localization:
        env.update(localization_environment);env['C6_LOCALIZATION']='1'
    if args.configure:env['C6_MOD_CONFIGURE']='1'
    if args.content_paths:env['C6_MOD_CONTENT_PATHS']='1'
    if args.art_manager:env['C6_MOD_ART_MANAGER']='1'
    if args.update_constructor_callback:env['C6_MOD_UPDATE_CONSTRUCTOR_CALLBACK']='1'
    if args.ui_constructor:env['C6_MOD_UI_CONSTRUCTOR']='1'
    if snapshot:env['C6_RULES_SNAPSHOT_DIR']=str(snapshot)
    if args.network:env['C6_HOST_NETWORK']='1'
    if args.attach_saved_session:env['C6_MOD_ATTACH_SAVED_SESSION']='1'
    if game_input:
        env['C6_GAME_LOAD_STREAM']=str(game_input)
        env['C6_GAME_SAVE_OUTPUT']=str(game_outputs['save'])
        env['C6_GAME_SAVE_REPEAT_OUTPUT']=str(game_outputs['repeat'])
    if args.client_only:env['C6_MOD_CLIENT_ONLY']='1'
    if args.client_constructor:env['C6_MOD_CLIENT_CONSTRUCTOR']='1'
    configuration_identity=hashlib.sha256(json.dumps(identity,sort_keys=True,separators=(',',':')).encode()).hexdigest()
    if args.expected_configuration and args.expected_configuration!=configuration_identity:
        raise SystemExit('configuration identity mismatch; native execution refused')
    try:
        run=subprocess.run([str(host)],cwd=work,env=env,text=True,capture_output=True,timeout=args.timeout)
        code,out,err=run.returncode,run.stdout,run.stderr
    except subprocess.TimeoutExpired as exc:
        decode=lambda x:x.decode(errors='replace') if isinstance(x,bytes) else x or ''
        code,out,err=124,decode(exc.stdout),decode(exc.stderr)
    report={'configuration':identity,'configuration_sha256':configuration_identity,
            'execution_paths':{'userdata':str(userdata),'native_rules_snapshot_directory':str(snapshot) if snapshot else None,
                               'lower_core_stream':str(game_input) if game_input else None,'game_outputs':{k:str(v) for k,v in game_outputs.items()}},
            'returncode':code,'stdout':out,'stderr':err,'framework_boundary_returned':code==0 and 'boundary_return result 0' in err,
            'framework_cleanup_proven':code==0 and 'cleanup_return 0' in err,'client_cleanup_proven':code==0 and 'client_shutdown_only_return unpublished 1' in err,'xp2_activation_proven':False,'initialized_game':False,'gameplay_parity':False,
            'limits':'Original host content-transition diagnostic only; selected module path and actual active-vtable module are distinct observations. Constructor-state callback is not saved Request/Begin semantics. No initialized-game or gameplay parity proof. Teardown is measured only when requested and its marker is present.'}
    report['native_activation_selection']={}
    report['native_backup_events']=[]
    for line in err.splitlines():
        if line.startswith('C6_LINUX_MOD selection '):
            selection=json.loads(line.removeprefix('C6_LINUX_MOD selection '))
            report['native_activation_selection'].setdefault(selection['phase'],{})[selection['list']]=selection
        if line.startswith('C6_LINUX_RULES '):
            report['native_backup_events'].append(json.loads(line.removeprefix('C6_LINUX_RULES ')))
    report['rules_snapshots']={}
    report['game_saves']={}
    for label,path in game_outputs.items():
        if path.is_file():report['game_saves'][label]={'filename':path.name,'sha256':digest(path),'bytes':path.stat().st_size}
    if snapshot:
        for label in ('configuration','gameplay'):
            path=snapshot/f'{label}.sqlite'
            if path.is_file():
                with path.open('rb') as stream:header=stream.read(16).hex()
                report['rules_snapshots'][label]={'filename':path.name,'sha256':digest(path),'bytes':path.stat().st_size,
                    'header_hex':header,'source':'original App-owned connection getter; native Database::Connection::BackupTo'}
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    tail=[line for line in err.splitlines() if not line.startswith('C6_LINUX_MOD selection ')][-12:]
    print(json.dumps({'output':str(args.output),'returncode':code,'tail':tail},indent=2))

if __name__=='__main__':main()
