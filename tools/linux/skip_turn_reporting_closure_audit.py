#!/usr/bin/env python3
"""Check bounded Linux host reporting dirty fanout; never run original code."""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]
METHODS = [['eastl::VectorBase<GameCore::Player::ComponentID, eastl::allocator>::DoAllocate(unsigned int)',
  '0x32b5bf0',
  33,
  '645661c2006cc4e01df6d88753c4f58210d6ac241aa1264098d0a925dc67494c'],
 ['eastl::VectorBase<GameCore::Player::ComponentID, eastl::allocator>::GetNewCapacity(unsigned int)',
  '0x32b5be0',
  14,
  '74b4efe75a71f994cc00a3e8f12fe4ede9f2846102c2370b886358446eb38445'],
 ['eastl::VectorBase<GameCore::Player::ComponentID, '
  'eastl::allocator>::DoFree(GameCore::Player::ComponentID*, unsigned int)',
  '0x32b5ca0',
  29,
  'c544a1ab95eaefacc3ca7ce9ab0d6fe34f3171eb3a149b11ef1f9c9f29e9c771'],
 ['GameCore::Player::ComponentID* eastl::uninitialized_move_ptr<GameCore::Player::ComponentID*, '
  'GameCore::Player::ComponentID*, GameCore::Player::ComponentID*>(GameCore::Player::ComponentID*, '
  'GameCore::Player::ComponentID*, GameCore::Player::ComponentID*)',
  '0x32b5c20',
  104,
  '383a8cba7a87ee62516fc83daee158bdea0a2f76cc73f9f06d08ff72ad65a5c0'],
 ['eastl::rbtree<unsigned int, eastl::pair<unsigned int const, '
  'GameCore::Utility::DirtyComponentsManager::ComponentsArray>, eastl::less<unsigned int>, eastl::allocator, '
  'eastl::use_first<eastl::pair<unsigned int const, '
  'GameCore::Utility::DirtyComponentsManager::ComponentsArray> >, true, true>::end()',
  '0x32b6530',
  18,
  '5afc92c1900a083a912413b2ad3101653f192506e80d13232cebd4a8e77bcc55'],
 ['eastl::rbtree<unsigned int, eastl::pair<unsigned int const, '
  'GameCore::Utility::DirtyComponentsManager::ComponentsArray>, eastl::less<unsigned int>, eastl::allocator, '
  'eastl::use_first<eastl::pair<unsigned int const, '
  'GameCore::Utility::DirtyComponentsManager::ComponentsArray> >, true, true>::find(unsigned int const&)',
  '0x32b6450',
  197,
  'a7e485163b693373202bdfb44d5a69d41b5e58eb40f5f8b76cde944e44342a0f'],
 ['eastl::vector<GameCore::Player::ComponentID, '
  'eastl::allocator>::DoInsertValue(GameCore::Player::ComponentID*, GameCore::Player::ComponentID const&)',
  '0x32b5ac0',
  264,
  'b42711d6119fb92b51a97bd0a710628f7914569eaf64ba8612d85f0e6522c00f'],
 ['eastl::vector<GameCore::Player::ComponentID, '
  'eastl::allocator>::DoDestroyValues(GameCore::Player::ComponentID*, GameCore::Player::ComponentID*)',
  '0x32b5c90',
  1,
  'ae3f4619b0413d70d3004b9131c3752153074e45725be13b9a148978895e359e'],
 ['eastl::vector<GameCore::Player::ComponentID, eastl::allocator>::end()',
  '0x32b5a30',
  5,
  '8e3eaf4f4b235f364076d47e3b5b95d9e60a71cf19dd62030d0fa20e80858d2e'],
 ['eastl::vector<GameCore::Player::ComponentID, eastl::allocator>::begin()',
  '0x32b5a20',
  4,
  'afae45c906d9e5d93a0612a2360b14d31fd758c9acf5c8f8787b5095927afc23'],
 ['eastl::vector<GameCore::Player::ComponentID, eastl::allocator>::push_back(GameCore::Player::ComponentID '
  'const&)',
  '0x32b5a40',
  40,
  '51ddb2cfadaf9eaa6839c687459a06e0c1414c64bde307982b87ef8ffc85231a'],
 ['eastl::vector<GameCore::Utility::DirtyComponents*, eastl::allocator>::end()',
  '0x32b63f0',
  5,
  '8e3eaf4f4b235f364076d47e3b5b95d9e60a71cf19dd62030d0fa20e80858d2e'],
 ['eastl::vector<GameCore::Utility::DirtyComponents*, eastl::allocator>::begin()',
  '0x32b63e0',
  4,
  'afae45c906d9e5d93a0612a2360b14d31fd758c9acf5c8f8787b5095927afc23'],
 ['GameCore::EngineUtility::SendEvent(unsigned int, Data::TypedVariantMap const&)',
  '0x32b4260',
  2492,
  '24826057267fc9c5a6028051e2eed6dbffea87bb34c1de4600f00ae968c370d9'],
 ['GameCore::Player::ComponentID::ComponentID(GameCore::Player::ComponentID const&)',
  '0x2feaef0',
  21,
  '574ca5ba9db429a6a5a90e019d12cd702ad1b683b676123b9edeb6e55212c0d6'],
 ['GameCore::Utility::DirtyComponents::Add(GameCore::Player::ComponentID const&)',
  '0x32b5950',
  86,
  '1f4235247163eba5152a3010fbb5f27a940bf74284dd888757f30869529dd3c2'],
 ['GameCore::Utility::DirtyComponentsManager::ComponentsArray::Add(GameCore::Utility::DirtyComponents*)',
  '0x32b5ec0',
  41,
  '3505f300feccea7e45edea492126c9cbb8d819bc4a42bc2ed5f4e5f72842bd60'],
 ['GameCore::Utility::DirtyComponentsManager::ComponentsArray::SetDirty(GameCore::Player::ComponentID '
  'const&)',
  '0x32b5f90',
  66,
  'ae43abdb887ee6fd52a14001c01dca33c7c46ac9a7998e10134913cc240408b3'],
 ['GameCore::Utility::DirtyComponentsManager::GetEventCompoments(unsigned int)',
  '0x32b6200',
  100,
  '9c914c37466f2772d71d3e074e95aec9fb3256dbe2a7c8b8972558811ad06328'],
 ['eastl::rbtree_iterator<eastl::pair<unsigned int const, '
  'GameCore::Utility::DirtyComponentsManager::ComponentsArray>, eastl::pair<unsigned int const, '
  'GameCore::Utility::DirtyComponentsManager::ComponentsArray>*, eastl::pair<unsigned int const, '
  'GameCore::Utility::DirtyComponentsManager::ComponentsArray>&>::operator*() const',
  '0x32b6580',
  8,
  '111da43af27d403431aff85821d27c6dfb0439f92612e03048c4c92b35072f05'],
 ['GameCore::Events::ParameterUsageDefinition::ChangesObject() const',
  '0x32b5620',
  6,
  'd9678be4ad44a05b93261ee381e5f56a64f924f4c8aa45ce90022721be5e4aa0'],
 ['GameCore::Player::ComponentID::operator==(GameCore::Player::ComponentID const&) const',
  '0x2fb7350',
  34,
  '565a5536704f1c2eda96270c96177cd8dea251e96a1c60e185c70c2ab5c49f80'],
 ['GameCore::Utility::DirtyComponentsManager::ComponentsArray::Contains(GameCore::Utility::DirtyComponents*) '
  'const',
  '0x32b5ef0',
  66,
  'f220f44bc3da71e1e8cfb38517ba6583b5b90760cf62e30031e9357c5014c275']]


def audit(assets):
    host = Elf(assets / 'Civ6', HOST_SHA)
    Elf(assets / 'libGameCore_XP2.so', GAME_SHA)
    methods = []
    for name, address, size, digest in METHODS:
        if hashlib.sha256(host.read(int(address, 16), size)).hexdigest() != digest:
            raise ValueError('full original method differs: ' + name)
        methods.append(dict(name=name, address=address, bytes=size, sha256=digest))
    for site, target in [
        (0x32b427a, 0x32b6200),
        (0x32b4b97, 0x32b5f90),
        (0x32b5fb6, 0x32b5950),
        (0x32b5987, 0x2fb7350),
        (0x32b4c03, 0x3214c50),
    ]:
        host.call(site, target)
    sources = {}
    for name in ['reporting-provider-passive-1', 'reporting-provider-focused-2',
                 'reporting-effects-passive-4', 'reporting-yield-query-closure',
                 'reporting-trade-effects']:
        path = ROOT / 'evidence/linux' / (name + '.json')
        sources[name] = dict(sha256=hashlib.sha256(path.read_bytes()).hexdigest())
    passive = json.loads((ROOT / 'evidence/linux/reporting-provider-passive-1.json').read_text())['passive_replay']
    if passive['native_exit'] != 0 or passive['gdb_exit'] != 0:
        raise ValueError('provider parent did not complete')
    prepublish = passive['prepublish']
    if not prepublish['all_observed_pointers_null'] or prepublish['observed_calls'] != 17:
        raise ValueError('captured prepublish facts differ')
    callbacks = passive['cache_callbacks']
    if len(callbacks) != 8 or sum(c['continuation_rva'] == '0x27ff430' for c in callbacks) != 7:
        raise ValueError('captured cache inventory differs')
    translators = passive['translators']
    if len(translators) != 10:
        raise ValueError('captured translator inventory differs')
    return dict(
        schema='c6-linux-skip-turn-reporting-closure-v1', checks_passed=True,
        native_execution=False, full_host_admission=False, gameplay_parity=False,
        gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA, methods=methods,
        evidence_inputs=sources,
        observed_inventory=dict(cache_unit_snapshots=7, cache_yield_monitor=1,
            translators=[dict(event_hash=t['event_hash'], target=t['target_elf_virtual_address']) for t in translators],
            prepublish=prepublish, dirty_registrations_observed=False),
        new_closed_dependency=dict(
            lookup='GetEventCompoments uses existing event-map find and returns ComponentsArray address or null; no insertion',
            fanout='SetDirty loops registered receiver pointers in vector order and directly calls concrete DirtyComponents.Add; it does not dispatch a callback',
            identity='Eight-byte ComponentID: u16 owner at0, u16 type at2, u32 id at4; equality compares all three fields',
            update='Add scans existing IDs in order, returns on equal identity, otherwise appends a fieldwise copy',
            guard='SendEvent calls SetDirty only for a recognized identity parameter with nonnull registration and ParameterUsageDefinition bit0 set',
            writes='Receiver ID vector and optional host allocator storage; no arbitrary receiver delegate/virtual call in SetDirty or Add',
            allocation_limit='Full vector may allocate/free through EASTL allocator. Allocation state/failure and allocator replacement are not modeled or proved inert',
            does_not_prove='Actual registration presence, current receiver vector contents, numerical plot/trade yields, unrelated callbacks, complete gameplay state or RNG'),
        rust_boundary=dict(module='crates/c6-core/src/reporting_dirty_components.rs',
            accepted_native_parity=False, live_skip_turn_admission=False,
            supported='One already-decoded identity parameter fanout with explicit registration and receiver list; preserves ordered identity dedup',
            rejects='Unknown registration or missing receiver before mutation',
            excludes='Native pointer layout, allocation/capacity state, event map decoding, parameter usage lookup, translators, Core dirty-index history and queued consumers'),
        minimal_future_capture=dict(status='Prepared data need only; no replay authorized and no observer/launcher added',
            profile='Separate future profile approved by sole native execution owner; preserve existing frozen handler profile unchanged',
            sites=[
                dict(address='0x32b427f', capture='At GetEventCompoments return, retain event R15D and RAX including null; if nonnull capture registration vector raw32 and ordered receiver pointers'),
                dict(address='0x32b4b97', capture='Before SetDirty: RDI actual registration vector, RSI raw8 ComponentID, and each receiver raw32 plus all used ComponentID bytes'),
                dict(address='0x32b4c08', capture='Before SendEvent epilogue: capture same receiver headers and used bytes after fanout and translator; correlate original thread/stack and balanced caller interval'),
            ],
            comparison='Preserve raw pointers/bytes, all eleven Send intervals, original command and five Save identities, worker/cleanup and original deadline; no inferior calls/writes'),
        remaining_consumers=[
            'Seven Unit snapshot Fill transitive rules/readiness and live cache entries; existing getter refinements retain their scope',
            'One YieldMonitor plot/trade query and cache/report continuation; passive endpoint equality is not numerical implementation',
            'Dirty registrations and receiver vector before/after values; now a concrete typed fanout rather than an unknown callback surface',
            'Later queued host event consumers remain separate from synchronous prepublish; seventeen actual prepublish pointers are null',
        ],
        save_load_raw_failure_bytes=89198, unknown_save_bytes_masked=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ROOT.parent / 'c6-assets-linux')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = audit(args.assets)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps(dict(checks_passed=True, full_method_gates=len(METHODS), native_execution=False)))


if __name__ == '__main__':
    main()
