#!/usr/bin/env python3
"""Static pinned-target closure of bounded SkipTurn Continue/Activate branches.

No original code executes. Native live bindings, subscribers and whole-action
state/RNG parity remain separate admission requirements.
"""
import argparse
import hashlib
import json
from pathlib import Path
from save_lifecycle_audit import Elf, GAME_SHA, HOST_SHA

ROOT = Path(__file__).resolve().parents[2]
METHODS = [
    (0x2DA1D70, 0x373, "Manager.Continue"),
    (0x2DA2790, 0x222, "Manager.Activate"),
    (0x2DA3200, 0x1D, "ProcessingData.constructor"),
    (0x2D95670, 7, "BaseHandler.CheckComplete"),
    (0x2D96890, 3, "BaseHandler.Update"),
    (0x2D956A0, 3, "BaseHandler.TimerComplete"),
    (0x293D100, 0x2E, "Formation.CanMove"),
    (0x2C438F0, 0x6D, "Placement.CompleteMovement.Formation"),
    (0x2C42AB0, 0x5C, "Placement.CompleteMovement.Unit"),
]
SLOTS = [
    (0x3AFF870, 0x140, 0x2DA1D70),
    (0x3B00DA0, 0x28, 0x2D96890),
    (0x3B00DA0, 0x30, 0x2D95670),
    (0x3B00DA0, 0x50, 0x2D956A0),
]

def require(value, reason):
    if not value:
        raise ValueError(reason)


def audit(module, host):
    elf = Elf(module, GAME_SHA)
    Elf(host, HOST_SHA)
    methods = [dict(name=name, rva=hex(rva), bytes=size,
                    sha256=hashlib.sha256(elf.read(rva, size)).hexdigest())
               for rva, size, name in METHODS]
    for table, slot, target in SLOTS:
        require(elf.relocated_pointer(table + 16 + slot) == target,
                f"original virtual slot differs {table:x}+{slot:x}")
    require(elf.read(0x2D95670, 7).hex() == 'c641100131c0c3',
            'CheckComplete no longer sets delete and returns false')
    for leaf in [0x2D96890, 0x2D956A0]:
        require(elf.read(leaf, 3).hex() == '31c0c3', 'inherited false leaf differs')
    for address, target in [
        (0x2DA1DB4, 0x1E930A0), (0x2DA1DF8, 0x1FB0440),
        (0x2DA2076, 0x1FB0440), (0x2DA1FA3, 0x1F16830),
        (0x2DA1FAF, 0x1E9ACB0), (0x2DA20D7, 0x1F63180),
        (0x2DA2017, 0x1ED1AA0), (0x2DA2067, 0x1EEA180),
        (0x2C43947, 0x1F63180), (0x2DA2948, 0x1FAFF50),
    ]:
        elf.call(address, target)
    require(elf.read(0x293D107, 7).hex() == '4881c3f8000000',
            'Formation moves wrapper address differs')
    require(elf.read(0x2DA2969, 4).hex() == '39c57d36',
            'false-activation signed push-turn branch differs')
    require(elf.read(0x2C42AC1, 2).hex() == '7441',
            'Unit completion false-predicate fast path differs')
    return dict(
        schema='c6-linux-loaded-skip-turn-continuation-source-v1', checks_passed=True,
        gamecore_sha256=GAME_SHA, host_sha256=HOST_SHA, methods=methods,
        original_virtual_slots=[dict(vtable=hex(t), offset=hex(s), target=hex(v))
                                for t, s, v in SLOTS],
        source_scope=dict(singleton_formation=True, complete_movement_pending=False,
                          actual_unit_and_formation_moves=True,
                          inherited_check_complete_always_deletes=True,
                          inherited_update_and_timer_complete_false=True,
                          no_moved_or_blocked_report_on_inherited_skip_turn=True,
                          evaluating_true_then_false=True,
                          clear_path_at_continue_entry=True,
                          activation_false_requires_unit_has_moved=True,
                          activation_false_retains_only_if_moved_and_push_turn_ge_current=True),
        native_execution=False, live_virtual_bindings_observed=False,
        whole_action_acceptance=False, complete_state_actions_rng_turns=False,
        private_raw_evidence='../c6-ghidra/evidence/linux-loaded-skip-turn-continuation/')


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--module', type=Path, default=ROOT.parent/'c6-assets-linux/libGameCore_XP2.so')
    p.add_argument('--host', type=Path, default=ROOT.parent/'c6-assets-linux/Civ6')
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    result = audit(a.module, a.host)
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({k: result[k] for k in ['schema', 'checks_passed', 'native_execution', 'whole_action_acceptance']}))

if __name__ == '__main__':
    main()
