#!/usr/bin/env python3
"""Read-only original Linux actor/legality ABI audit; does not execute gameplay."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import Elf, HOST_SHA, GAME_SHA

METHODS = [['Civ6',
  '_ZN8GameCore10Interfaces10ICvUnknowndlEPv',
  '0x2f72f20',
  22,
  '422501e1b66233c39f606430f3670003902b33cb8c494a2b5d4e0f28d4a0e008'],
 ['Civ6',
  '_ZN4Data19TypedVariantManager10GetDefaultEv',
  '0x395ec10',
  8,
  'b9e92991972d70e4e0c2e14696b233b5445045d4f6ed869f27160636ae5f1ac4'],
 ['Civ6',
  '_ZN4Data19TypedVariantManager32CreateKeyedTypedVariantContainerENS_21VariantContainerTypesE',
  '0x395eb00',
  204,
  '662d4be466c1b9b6c0749b0e6e4395756818c3bbbb3a251525fffcf3a6a04c3b'],
 ['Civ6',
  '_ZN4Data15TypedVariantMapD0Ev',
  '0x395f2e0',
  18,
  '8fc58c9ac96a1ae7ebbc9691d499931d00b0a7c4c51715cc50acfc145dffe62c'],
 ['libGameCore_XP2.so',
  '_ZN8GameCore7Exports7Context7GetGameEv',
  '0x2c7cae0',
  92,
  '0bee95a94a869e776ee221963d7e80aa885739a6d73aff21785121122d1a2c9f'],
 ['libGameCore_XP2.so',
  '_ZN8GameCore10Interfaces10ICvUnknowndlEPv',
  '0x290bba0',
  22,
  'ad7d214f06fbbe64670982468ad970bc59593dbb904cc8cf29764bd75e59f480'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports7Context14GetLocalPlayerEv',
  '0x2c7ca80',
  46,
  '0d969ff2852c1f80a836c7eb5bb91445c1af01c4302ebb98bfb6f3b4db0a5fff'],
 ['libGameCore_XP2.so',
  '_ZN8GameCore7Exports7Context9GetPlayerENS_11PlayerTypesE',
  '0x2c7cc80',
  116,
  '59a756c03636b0d30f3ee7b8b6b0f5e075801e81e4e7fdf1de0ef2e7a79fc56a'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports6Player7IsHumanEv',
  '0x2c8c700',
  26,
  'dc88f8c265bcb872fbe8b76a8f7f50286f44ec6d15b550d4505e46cdd7daad40'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports6Player12IsTurnActiveEv',
  '0x2c8c600',
  26,
  '0d0486722135f9a89135eeb7e8fb897587bbbde05acd40354df05b31ff956cbd'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports6Player8GetUnitsEv',
  '0x2c8cce0',
  103,
  'd02da68c75b44121fafe55182cd1049895e4bf2d8f181aa3f5f58da9b4572cf3'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports11PlayerUnits7GetSizeEv',
  '0x2c912a0',
  45,
  'e7867d9e6ff026649f3715536f007cc9a7aef49f69149d03ddc0ec7a816c885e'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports11PlayerUnits5BeginEv',
  '0x2c912d0',
  55,
  '3aea184b4e840d1dad475722dfc431c5d15e8ecf04023f1f0cd6ad51f3281372'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports11PlayerUnits3EndEv',
  '0x2c91310',
  55,
  '65a1b8fe29eee4f9c26d2a14a68c4f27db85f19d42a2314df268d343ac6faf5c'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports11PlayerUnits4NextEPv',
  '0x2c91350',
  78,
  '5d98d27af89e67091956fc58eec580bcf59024fe8af63d9628231e7f926e6103'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports11PlayerUnits3GetEPv',
  '0x2c913a0',
  137,
  '99b4cb644356a4d82abd5b15509f6277380995fcac2cc8e898e0a3cbd71aa458'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports4Unit5GetIDEv',
  '0x2c93840',
  26,
  'cf8ae22bd62201de77a61dffdb8a14484d4877fe6c4603bbcb0b8a1e23176942'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports4Unit8GetOwnerEv',
  '0x2c938b0',
  26,
  '2f5e43983b04e3661d95b2720cdc9303a86f83721a2f2d7f6b2347d15f9163cb'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports4Unit14GetComponentIDEv',
  '0x2c93860',
  43,
  'e7e6b9e733e8122e6d61d5d147f819e44dc39de41026472dee6c5415aa440445'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports4Unit6IsDeadEv',
  '0x2c939b0',
  26,
  '50a7884a6bc51fcd95125379f2aa097b2dcbdfe70d5ccb2d464907bcb2d2dd94'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports4Unit14IsDelayedDeathEv',
  '0x2c939d0',
  26,
  'c2b6a00fe8af8f19b9b7f321b88ccd80f1642774c889b8cfd42665ea404963bb'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports4Unit15IsReadyToSelectEv',
  '0x2c939f0',
  26,
  '45e50947264c36b9d876866b2e1ef99fc78867eacab910c85f8458861022d417'],
 ['libGameCore_XP2.so',
  '_ZN8GameCore7Exports7Context17GetUnitOperationsEv',
  '0x2c7ce10',
  55,
  '9198ac229aa2a3795a917e2810b1fb2bdf945eef9c831c74c415477b077440ca'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports14UnitOperations8CanStartERKNS_6Player11ComponentIDENS_4Unit10Operations5Types8TypeHashERKN4Data15TypedVariantMapEbPSB_',
  '0x2c94340',
  14,
  '8d858547f744dac5c4e5edd8dcbf000c32acfe6261eb7fa32a12c09158a2a505'],
 ['libGameCore_XP2.so',
  '_ZN8GameCore7Exports7Context18HasPendingMessagesEv',
  '0x2c7d340',
  5,
  'a13fab7c3feace9b5fef1163add3c7fbbda42a32c4fe5ac88f62a3abf08925f7'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports4Game11NeedsUpdateEv',
  '0x2c86a60',
  5,
  '004efac29e3e520fcc9081b35e77f6061852cfa8247d9fb62a0f44b99ca0138d'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports4Game16NeedsTurnAdvanceEv',
  '0x2c869f0',
  5,
  'dcc5e3123bd6c64652459018d79aefc65327a2ea83e34a7376ff858f789660d9'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports4Game14IsAiProcessingEv',
  '0x2c867e0',
  17,
  'e0ef93e4ea97b56bd9298409a6e521208c467b38727ebdfd62acbbd04db9b0f0'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports4Game16IsTurnProcessingEv',
  '0x2c86800',
  17,
  'ea09cb9a5f48d27d0fef30ffc6ee853e05641c9a0b878a5ccef0cc81811bc127'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports4Game14GetCurrentTurnEv',
  '0x2c868b0',
  26,
  '3fba850f8c4e237bc934b712d2324c2d8b97e526091f50e64745ee77aebe6be6'],
 ['libGameCore_XP2.so',
  '_ZNK8GameCore7Exports4Game15GetCurrentPhaseEv',
  '0x2c86850',
  17,
  '90c17f8c9b20a2ffc0169505de38c74a5ea5b2aedba456d569e61a47d1c74e3e']]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    host = Elf(args.assets/'Civ6', HOST_SHA)
    game = Elf(args.assets/'libGameCore_XP2.so', GAME_SHA)
    results = []
    for module, name, address, size, digest in METHODS:
        elf = host if module == 'Civ6' else game
        if hashlib.sha256(elf.read(int(address,16),size)).hexdigest() != digest:
            raise ValueError('original method bytes changed: '+name)
        results.append(dict(module=module,symbol=name,address=address,size=size,sha256=digest))
    game.call(0x2c93873, 0x1f16bf0)  # hidden UnitID sret forwarding
    game.call(0x2c93881, 0x1e05490)
    host.call(0x395eb3f, 0x395f070)  # original empty map constructor
    host.call(0x395f2e4, 0x395f290)  # original deleting destructor
    if game.read(0x2c93861,7) != bytes.fromhex('4889fb488b4610'):
        raise ValueError('UnitID hidden return receiver ABI differs')
    if game.read(0x2daca50,6) != bytes.fromhex('b80819682cc3'):
        raise ValueError('actual SkipTurn operation hash differs')
    if host.read(0x395eb0c,3) != bytes.fromhex('83fe02') or host.read(0x395eb1f,5) != bytes.fromhex('bf58000000'):
        raise ValueError('original native typed-map factory branch differs')
    report=dict(schema='c6-linux-action-observer-abi-v1',host_sha256=HOST_SHA,gamecore_sha256=GAME_SHA,
                method='SHA-gated original ELF byte identities and selected caller ABI checks',
                methods=results,checks_passed=True,unit_identity_bytes=8,unit_identity_return='hidden sret RDI, receiver RSI',
                typed_map_factory_argument=2,typed_map_original_allocation_bytes=88,
                operation='SKIP_TURN',operation_hash='0x2c681908',
                native_execution=False,action_executed=False,gameplay_parity=False)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(checks_passed=True,methods_checked=len(results),native_execution=False)))


if __name__ == '__main__':
    main()
