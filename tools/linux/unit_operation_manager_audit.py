#!/usr/bin/env python3
"""Pinned Linux full Manager CanStart control-flow audit; no original execution."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA
from unit_operation_legality_audit import METHODS as LEGALITY
ROOT = Path(__file__).resolve().parents[2]
METHODS = [('Manager.CanStart', 47849712, 579, '573164c5e12ad82eeb6761d2492634ae376ae6de63526a3b94cf0608d8550927'), ('Manager.CanCurrentOperationBeInterrupted', 47852128, 116, '6b2b1cbb8b18460cf4fb4f4f7a8000cd6f42ae2108cab549666eb1a387772ebe'), ('Unit.IsOperationDisabled', 46307296, 73, 'de0f4942cf7c5e7e3e4fb4cd9ea1973ef50dc1b903add1ff673c871556839751'), ('Phase.IsAllowTacticalCommands', 44388144, 37, '794ae6ccbdb27984a0baa907c011cab9858884f434482532c7d853791350169b'), ('Phase.CurrentDefinition', 44388912, 35, '234514ebec562e9eaf779528d82ee52914ca963c8b8559bc4ef6469203e72e5e'), ('Phase.ActiveSegment', 44391328, 5, 'b811adb2d88317c4120dc7c33dd574a1678d7f98965dd45cb6cf6745cafe2e73'), ('Segment.AllowTactical', 44391360, 8, '06059c117d2525723825777083e470ee9220181f3044fd23f4b764be0ac6b038'), ('Player.ProcessingAutomation', 45466560, 7, '750b0f58d460bdbe162c9ce1512bdd6a141a3527ce526bc67ef708695777aafb'), ('Player.IsTurnActive', 33689648, 7, 'f44ea052578d46f191ac4e546446f30a3022dc5085b44ee6b9378719eabb8c5b'), ('Player.IsHuman', 45290800, 32, '7b956c7055aea66551b2309c8d001e35a098fc6e19355bd959b5a46241245c95'), ('Game.IsPaused', 42055456, 7, 'bf1c7941b3b62b57c2112fc90a57e28fbe0b8e7ab2b783e84a7c6731d061630f'), ('Unit.GetPlot', 46254112, 34, '70482659dd93faebc86071ffed12f087fac73975901d7d2ea272edf981fad99a'), ('Queue.Size', 40906496, 5, 'ac6738f18aec9e8ecc5485d5a8d50fb556201d33e235a6e801311cc7a366a268'), ('Queue.GetAt', 47845456, 38, '2b7cb3be8910ab9acbe185c49e7173f71569af10e9bb11a3a3114b30fa358060'), ('Unit.GetQueue', 34473744, 8, 'eaee35c24a14cd015937e182b1d93f582e7715eea23c0cddbb987495a6431719'), ('Definition.FindIndex', 46318112, 27, '0987f96defd19d195435710a642c1e5fcee85058aee80d49076836b0190e8f2d'), ('Disabled.IsValidIndex', 46318160, 23, 'cfae545fe0031fd04ae072a8b013c6d5ba4739a1269f753a10ecc2b12655707f'), ('Disabled.AtConst', 46319424, 55, 'e08593a7c1125bb9289d36e04c49770b7015123437fc7e131ea4c5661bfa2258'), ('Script.CallTestAll', 47086000, 116, '47ad40fd9efa7ddcbe88b6bc4de3eff8c174262c6cc031ce76d78e63094c431e'), ('Script.ArgsCtor', 47441744, 35, 'd6f72c2fe605bd69ac48758eeaf43eedc6f85fafe066e76184f3456c3d17f9f9'), ('Script.ArgsDtor', 47441840, 36, '924ad43ce8225ccfead12de01459448e302fcc12f42fa61aee6706163e07c27d'), ('Script.System', 35067488, 11, '0d5bf15629235063deb60052f71390c42fd80e98a458e0775078bcd7bb77e07e'), ('Script.ArgsGet', 35067504, 4, 'afae45c906d9e5d93a0612a2360b14d31fd758c9acf5c8f8787b5095927afc23'), ('Script.ArgsArrow', 35512960, 4, 'afae45c906d9e5d93a0612a2360b14d31fd758c9acf5c8f8787b5095927afc23'), ('BaseHandler.CanBeInterrupted', 47797856, 3, 'e18aacce29affcdddaa5f641ac1ec12552e0fe9d066e6408941b1792818b8619')]
CALLS = [(47849747, 31870880), (47849768, 31726528), (47849775, 31682320), (47849806, 31726528), (47849813, 33050512), (47849826, 31451760), (47849836, 31659936), (47849845, 32955104), (47849856, 33228816), (47849870, 31451760), (47849881, 32012288), (47849893, 31529456), (47849909, 31734304), (47849917, 32648896), (47849967, 32018096), (47849980, 32904368), (47850005, 32092480), (47850013, 33070464), (47850031, 31726528), (47850049, 33070464), (47850067, 31731584), (47850085, 33070464), (47850116, 32746416), (47850139, 32775680), (47850161, 32855824), (47850192, 32108304), (47850278, 32855824), (47850286, 32992576), (47852143, 31734304), (47852151, 32648896), (47852163, 31734304), (47852173, 31738928), (47852186, 31734304), (47852196, 31738928), (46307304, 32242656), (46307314, 32235840), (46307333, 31520592), (46307347, 33205680), (44388145, 32663424), (44388158, 32693360), (44388931, 32373280), (45290804, 31685296), (45290818, 32976880), (46254116, 32437584), (47845464, 31911152), (47845478, 32847312), (46318113, 31879312), (46318163, 32227360), (46318171, 31909040), (46319439, 31909040), (47086023, 32706720), (47086043, 32706720), (47086085, 32706720), (47441755, 32904368), (47441844, 32904368), (47441871, 33346032)]

def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    elf = Elf(a.assets / 'libGameCore_XP2.so', GAME_SHA)
    Elf(a.assets / 'Civ6', HOST_SHA)
    rows = []
    for name, address, size, fingerprint in METHODS + LEGALITY:
        if sha(elf.read(address, size)) != fingerprint:
            raise ValueError('method fingerprint mismatch: ' + name)
        rows.append(dict(name=name, address=hex(address), size=size, sha256=fingerprint))
    for address, target in CALLS:
        elf.call(address, target)
    bindings = [(0x3aff880+0xd8,0x2da2a60),(0x3aff880+0x88,0x2da1650),
                (0x3b00db0+0x18,0x2dac950),(0x3b00db0+0x20,0x2d95660)]
    for address, target in bindings:
        if elf.relocated_pointer(address) != target:
            raise ValueError('virtual binding mismatch')
    if elf.read(0x2f0bac6,18) != b'CanStartOperation\0':
        raise ValueError('script event identity mismatch')
    for address, code in [(0x2da227a,'c644240700'),(0x2da22b6,'4584e6'),
                          (0x2da22c1,'7548'),(0x2da2305,'41ff5218'),
                          (0x2da2aad,'741e'),(0x2c29818,'8038000f95c0')]:
        if elf.read(address,len(code)//2).hex() != code:
            raise ValueError('control-flow boundary mismatch: '+hex(address))
    source = ['crates/c6-core/src/unit_operation_legality.rs',
              'tools/linux/unit_operation_manager_audit.py']
    report = dict(schema='c6-linux-unit-operation-manager-v1',checks_passed=True,
        gamecore_sha256=GAME_SHA,host_sha256=HOST_SHA,original_execution_performed=False,
        methods=rows,checked_direct_call_edges=[dict(address=hex(x),target=hex(y))for x,y in CALLS],
        checked_virtual_bindings=[dict(address=hex(x),target=hex(y))for x,y in bindings],
        source_sha256={x:sha((ROOT/x).read_bytes())for x in source},
        ordering=['busy','nonvisible tactical','actual owner Player','nonvisible turn or automation',
                  'Game lookup','nonvisible human pause','nonempty queue interruption',
                  'disabled operation','script lookup/args/ordered integer pushes/CallTestAll/destruction/veto',
                  'supplied plot or actual current plot','actual registered requested handler','handler CanStart'],
        script_veto='CallTestAll.called && !out_value; out_value initially false; destruction before branch',
        interruption='empty/missing front/same actual runtime ID/missing current handler permit; otherwise actual current handler CanBeInterrupted',
        unknown_dependencies=['current phase and active segment','retained actual Player/Game receivers and mutable live state',
            'actual registry runtime IDs, handler bindings and virtual overrides',
            'live disabled IndexList definition mapping/validity/value',
            'script allocation, int pushes, engine utility locking, callbacks and current-system destruction',
            'current plot lookup and complete handler live providers/results'],
        synthetic_manager_test_functions=5,native_manager_boundary_fixtures_available=False,
        manager_orchestration_allocates=False,provider_allocations_and_effects_retained=True,
        full_action_supported=False,
        complete_state_action_rng_turns_supported=False,historical_raw_save_load_failure_bytes=89198)
    a.output.parent.mkdir(parents=True,exist_ok=True)
    a.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,method_contracts=len(rows),call_edges=len(CALLS),original_execution=False)))


if __name__ == '__main__':
    main()
