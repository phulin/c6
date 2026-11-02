#!/usr/bin/env python3
"""Verify original Linux request/retained-stream/Begin boundary contracts."""
import argparse
import json
from pathlib import Path
import struct
from save_lifecycle_audit import Elf, HOST_SHA


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--assets', type=Path, default=Path(__file__).resolve().parents[3] / 'c6-assets-linux')
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    h = Elf(args.assets / 'Civ6', HOST_SHA)
    calls = [
        (0x32481a9, 0x3248250, 'Request_device4_FillLastStream'),
        (0x32481b8, 0x2fbff10, 'Request_query_actual_current_state'),
        (0x32481cf, 0x3248540, 'Request_automatic_CheckRequest_unless_gameview_active'),
        (0x3248265, 0x324ab90, 'FillLastStream_clear_previous_owner'),
        (0x32482a3, 0x32520f0, 'FillLastStream_original_GameCoreMemoryIStream_constructor'),
        (0x32482d3, 0x3c3d638, 'FillLastStream_allocate_complete_destination_buffer'),
        (0x32482fc, 0x2f66ca0, 'FillLastStream_construct_whole_size_read_request'),
        (0x324866c, 0x3248af0, 'CheckRequest_DoLoadRequest'),
        (0x3248d29, 0x31d1400, 'DoLoadRequest_without_gameview_ControllerUninitialize'),
        (0x3248d2e, 0x31dd810, 'DoLoadRequest_without_gameview_ShutdownGame'),
        (0x3248d6f, 0x3252080, 'DoLoadRequest_publish_original_load_event'),
        (0x324aad0, 0x324a4b0, 'LoadCurrentRequest_OpenFromSource'),
        (0x324ab5d, 0x3247990, 'DoLoadGameRequestCallback_Reset'),
        (0x2fd16a5, 0x32490a0, 'Begin_query_request_in_progress'),
        (0x2fd16d9, 0x2fd1730, 'Begin_subscribe_to_original_events'),
        (0x2fd16e1, 0x2fd0fe0, 'Begin_LoadUI'),
        (0x2fd16f7, 0x42b9600, 'Begin_SetTickInterval'),
        (0x2fd1703, 0x2fdce10, 'Begin_AmbientAudio'),
        (0x36b12b3, 0x36b20a0, 'NativeLuaRegister_SINGLE_PLAYER'),
        (0x3aaac03, 0x3acf4d0, 'NativeUIManager_constructor_creates_TutorialManager'),
        (0x2fdce22, 0x4352160, 'AmbientAudio_original_SendEvent'),
        (0x325be71, 0x325b050, 'LocalRead_get_original_save_directory'),
        (0x325beb8, 0x39ac7e3, 'LocalRead_strip_to_file_basename'),
        (0x325bed2, 0x325c170, 'LocalRead_add_original_save_extension'),
        (0x325bf6a, 0x3c53ef2, 'LocalRead_original_FileISynchStream_Open'),
    ]
    for address, target, _ in calls:
        h.call(address, target)
    checks = [
        (0x3248936, 'be02000000', 'RequestLoad_RequestType_LOAD2'),
        (0x3247e89, '837b0804', 'Request_supplied_stream_requires_device4'),
        (0x32482e0, 'ff5028', 'FillLastStream_input_seek_zero'),
        (0x3248304, '488b6840', 'FillLastStream_input_MemoryBuffer_Read_slot40'),
        (0x3248325, 'ff5028', 'FillLastStream_retained_seek_zero'),
        (0x32490a9, '803900', 'GetRequestInProgress_checks_progress_flag'),
        (0x3248654, 'c6450001', 'CheckRequest_marks_in_progress'),
        (0x3248b1e, '83780804', 'DoLoadRequest_device4_skips_LeaveGame'),
        (0x324aba3, 'ff5008', 'ClearLastStream_original_deleting_destructor'),
        (0x2fd16aa, '83f8020f9483b1040000', 'Begin_saved_censor_flag_from_active_LOAD'),
        (0x36b12ab, 'ba01000000', 'NativeLua_SINGLE_PLAYER_value1'),
        (0x42b9600, '893d0a931701c3', 'SetTickInterval_original_global_store_only'),
        (0x4352161, '803dc0773f0301751e', 'AudioSendEvent_native_uninitialized_guard'),
        (0x333d5c0, '8a87b1040000c3', 'IsLoadingFromSave_reads_Begin_saved_profile_flag'),
    ]
    for address, value, label in checks:
        if h.read(address, len(bytes.fromhex(value))) != bytes.fromhex(value):
            raise ValueError(f'original request byte contract mismatch: {label}')
    jump = h.read(0x3248940, 5)
    if jump[0] != 0xe9 or 0x3248945 + struct.unpack_from('<i', jump, 1)[0] != 0x3247c10:
        raise ValueError('RequestLoad forwarding contract mismatch')
    if h.read(0x4607dcd, 14) != b'SINGLE_PLAYER\0':
        raise ValueError('native single-player save type name mismatch')
    directory_jumps = [0x4608108 + x for x in struct.unpack('<6i', h.read(0x4608108, 24))]
    if directory_jumps[2] != 0x325b13b:
        raise ValueError('native benchmark directory dispatch mismatch')
    if h.read(0x4608134, 27 * 4).decode('utf-32-le').rstrip('\0') != 'Base/Assets/Benchmark/Saves':
        raise ValueError('native benchmark directory path mismatch')
    result = dict(schema=1, host_sha256=HOST_SHA,
                  method='SHA-gated original Linux host static request audit', checks_passed=True,
                  direct_calls=[dict(call=hex(a), target=hex(t), contract=n) for a,t,n in calls],
                  byte_contracts=[n for _,_,n in checks],
                  supplied_stream_device=4, single_player_save_type=1,
                  local_storage_device=1, benchmark_directory=2,
                  supplied_stream_copied_into_original_owned_stream=True,
                  deferred_request_is_not_in_progress=True,
                  Begin_requires_actual_in_progress_LOAD_before_LoadCurrentRequest=True,
                  Begin_UI_audio_tick_prerequisites_required=True,
                  complete_save_load=False, action_or_turn_parity=False)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'checks_passed': True, 'direct_calls_checked': len(calls)}))


if __name__ == '__main__':
    main()
