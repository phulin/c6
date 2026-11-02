//! Version 1 loadable C API. See include/c6.h for pointer and ownership contracts.
use c6_core::rng::RandomState;
use c6_core::{Engine, Error, Outcome, RandomOperation, Snapshot};
use std::ptr;

pub const OK: i32 = 0;
pub const INVALID: i32 = 1;
pub const UNSUPPORTED: i32 = 2;
pub const BUFFER_TOO_SMALL: i32 = 3;
pub const MISSING_STREAM: i32 = 4;
pub const DUPLICATE_STREAM: i32 = 5;
fn status(error: Error) -> i32 {
    match error {
        Error::MissingStream(_) => MISSING_STREAM,
        Error::DuplicateStream(_) => DUPLICATE_STREAM,
        Error::InvalidStreamId => INVALID,
        _ => UNSUPPORTED,
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct C6RandomState {
    pub seed: u32,
    pub calls: u32,
    pub resets: u32,
    pub lock_count: u32,
    pub synchronous: u32,
}
impl From<RandomState> for C6RandomState {
    fn from(s: RandomState) -> Self {
        Self {
            seed: s.seed,
            calls: s.calls,
            resets: s.resets,
            lock_count: s.lock_count,
            synchronous: s.synchronous as u32,
        }
    }
}
#[repr(C)]
pub struct C6Action {
    pub kind: u32,
    pub argument: u32,
}
pub struct C6Handle(Engine);

#[no_mangle]
pub extern "C" fn c6_abi_version() -> u32 {
    1
}

#[no_mangle]
pub extern "C" fn c6_plot_valid_v1(x: i32, y: i32) -> u32 {
    c6_core::geometry::plot_valid(x, y) as u32
}

/// # Safety
/// result must be a writable aligned i32 pointer. wrap must be 0 or 1.
#[no_mangle]
pub unsafe extern "C" fn c6_plot_distance_v1(
    width: i32,
    wrap: u32,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    result: *mut i32,
) -> i32 {
    if result.is_null() || wrap > 1 {
        return INVALID;
    }
    *result = c6_core::geometry::plot_distance(width, wrap != 0, x1, y1, x2, y2);
    OK
}
/// # Safety
/// `out` must point to writable pointer storage.
#[no_mangle]
pub unsafe extern "C" fn c6_create_v1(seed: u32, out: *mut *mut C6Handle) -> i32 {
    if out.is_null() {
        return INVALID;
    }
    *out = Box::into_raw(Box::new(C6Handle(
        Engine::new(Snapshot::new(RandomState::new(seed))).unwrap(),
    )));
    OK
}
/// # Safety
/// `handle` must be null or a live handle from this library, freed exactly once.
#[no_mangle]
pub unsafe extern "C" fn c6_destroy_v1(handle: *mut C6Handle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}
/// # Safety
/// All pointers must be valid, aligned and nonaliasing for their declared sizes.
#[no_mangle]
pub unsafe extern "C" fn c6_apply_v1(
    handle: *mut C6Handle,
    action: *const C6Action,
    result: *mut u32,
) -> i32 {
    c6_random_stream_apply_v1(handle, 0, action, result)
}
/// # Safety
/// All pointers must be valid, aligned and nonaliasing for their declared sizes.
#[no_mangle]
pub unsafe extern "C" fn c6_random_stream_apply_v1(
    handle: *mut C6Handle,
    id: u32,
    action: *const C6Action,
    result: *mut u32,
) -> i32 {
    if handle.is_null() || action.is_null() || result.is_null() {
        return INVALID;
    }
    let a = &*action;
    let operation = match a.kind {
        1 if a.argument <= u16::MAX as u32 => RandomOperation::Draw {
            bound: a.argument as u16,
        },
        1 => return INVALID,
        2 => RandomOperation::Reset { seed: a.argument },
        3 => RandomOperation::Initialize { seed: a.argument },
        4 => RandomOperation::Lock,
        5 => RandomOperation::Unlock,
        _ => return UNSUPPORTED,
    };
    match (*handle).0.apply_random(id, operation) {
        Ok(outcome) => {
            *result = match outcome {
                Outcome::Random(n) => n as u32,
                Outcome::Applied => 0,
            };
            OK
        }
        Err(error) => status(error),
    }
}
/// Register a caller-identified independent GameCore::Random instance.
/// # Safety
/// handle must be live; state must be readable, aligned and not overlap handle.
#[no_mangle]
pub unsafe extern "C" fn c6_random_stream_create_v1(
    handle: *mut C6Handle,
    id: u32,
    state: *const C6RandomState,
) -> i32 {
    if handle.is_null() || state.is_null() {
        return INVALID;
    }
    let state = &*state;
    if state.synchronous > 1 {
        return INVALID;
    }
    let state = RandomState {
        seed: state.seed,
        calls: state.calls,
        resets: state.resets,
        lock_count: state.lock_count,
        synchronous: state.synchronous != 0,
    };
    match (*handle).0.insert_random_stream(id, state) {
        Ok(()) => OK,
        Err(error) => status(error),
    }
}
/// # Safety
/// handle must be live; out writable, aligned and not overlap handle.
#[no_mangle]
pub unsafe extern "C" fn c6_random_stream_state_v1(
    handle: *const C6Handle,
    id: u32,
    out: *mut C6RandomState,
) -> i32 {
    if handle.is_null() || out.is_null() {
        return INVALID;
    }
    match (*handle).0.random_state(id) {
        Ok(state) => {
            *out = (*state).into();
            OK
        }
        Err(error) => status(error),
    }
}
/// # Safety
/// Handle must be live; out must be writable and not overlap handle storage.
#[no_mangle]
pub unsafe extern "C" fn c6_random_state_v1(
    handle: *const C6Handle,
    out: *mut C6RandomState,
) -> i32 {
    c6_random_stream_state_v1(handle, 0, out)
}
/// # Safety
/// Handle must be live. written must be writable. A nonnull buffer must have capacity
/// writable bytes and not alias the handle or written. Null buffer queries size.
#[no_mangle]
pub unsafe extern "C" fn c6_export_v1(
    handle: *const C6Handle,
    buffer: *mut u8,
    capacity: usize,
    written: *mut usize,
) -> i32 {
    if handle.is_null() || written.is_null() {
        return INVALID;
    }
    let bytes = (*handle).0.snapshot().encode();
    *written = bytes.len();
    if buffer.is_null() || capacity < bytes.len() {
        return BUFFER_TOO_SMALL;
    }
    ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, bytes.len());
    OK
}
/// # Safety
/// Bytes must be readable for length bytes, out writable. Neither may alias.
#[no_mangle]
pub unsafe extern "C" fn c6_import_v1(
    bytes: *const u8,
    length: usize,
    out: *mut *mut C6Handle,
) -> i32 {
    if bytes.is_null() || out.is_null() || length > isize::MAX as usize {
        return INVALID;
    }
    let Ok(snapshot) = Snapshot::decode(std::slice::from_raw_parts(bytes, length)) else {
        return INVALID;
    };
    let Ok(engine) = Engine::new(snapshot) else {
        return INVALID;
    };
    *out = Box::into_raw(Box::new(C6Handle(engine)));
    OK
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ffi_streams_preserve_independence_and_roundtrip() {
        unsafe {
            let mut handle = ptr::null_mut();
            assert_eq!(c6_create_v1(1, &mut handle), OK);
            let state = C6RandomState {
                seed: 17,
                calls: u32::MAX,
                resets: 9,
                lock_count: 2,
                synchronous: 1,
            };
            assert_eq!(c6_random_stream_create_v1(handle, 7, &state), OK);
            assert_eq!(
                c6_random_stream_create_v1(handle, 7, &state),
                DUPLICATE_STREAM
            );
            assert_eq!(c6_random_stream_create_v1(handle, 0, &state), INVALID);
            let mut output = 123;
            let draw = C6Action {
                kind: 1,
                argument: 65535,
            };
            assert_eq!(
                c6_random_stream_apply_v1(handle, 8, &draw, &mut output),
                MISSING_STREAM
            );
            assert_eq!(output, 123);
            let mut observed = C6RandomState::default();
            assert_eq!(c6_random_stream_state_v1(handle, 7, &mut observed), OK);
            assert_eq!(observed, state);
            assert_eq!(c6_random_stream_apply_v1(handle, 7, &draw, &mut output), OK);
            assert_eq!(c6_random_state_v1(handle, &mut observed), OK);
            assert_eq!(observed.seed, 1);
            assert_eq!(observed.calls, 0);
            let mut size = 0;
            assert_eq!(
                c6_export_v1(handle, ptr::null_mut(), 0, &mut size),
                BUFFER_TOO_SMALL
            );
            let mut bytes = vec![0; size];
            assert_eq!(
                c6_export_v1(handle, bytes.as_mut_ptr(), size, &mut size),
                OK
            );
            let mut branch = ptr::null_mut();
            assert_eq!(c6_import_v1(bytes.as_ptr(), size, &mut branch), OK);
            assert_eq!(c6_random_stream_state_v1(branch, 7, &mut observed), OK);
            assert_eq!(observed.calls, 0);
            assert_eq!(observed.synchronous, 1);
            let mut branch_output = 0;
            assert_eq!(c6_random_stream_apply_v1(handle, 7, &draw, &mut output), OK);
            assert_eq!(
                c6_random_stream_apply_v1(branch, 7, &draw, &mut branch_output),
                OK
            );
            assert_eq!(output, branch_output);
            c6_destroy_v1(handle);
            c6_destroy_v1(branch);
        }
    }
    #[test]
    fn ffi_branch_export_import_and_failure_atomicity() {
        unsafe {
            let mut a = ptr::null_mut();
            assert_eq!(c6_create_v1(1, &mut a), OK);
            let mut result = 123;
            assert_eq!(
                c6_apply_v1(
                    a,
                    &C6Action {
                        kind: 1,
                        argument: 65536
                    },
                    &mut result
                ),
                INVALID
            );
            assert_eq!(result, 123);
            assert_eq!(
                c6_apply_v1(
                    a,
                    &C6Action {
                        kind: 1,
                        argument: 65535
                    },
                    &mut result
                ),
                OK
            );
            assert_eq!(result, 16837);
            let mut size = 0;
            assert_eq!(
                c6_export_v1(a, ptr::null_mut(), 0, &mut size),
                BUFFER_TOO_SMALL
            );
            let mut bytes = vec![0; size];
            assert_eq!(c6_export_v1(a, bytes.as_mut_ptr(), size, &mut size), OK);
            let mut b = ptr::null_mut();
            assert_eq!(c6_import_v1(bytes.as_ptr(), size, &mut b), OK);
            let mut sa = C6RandomState::default();
            let mut sb = sa;
            assert_eq!(c6_random_state_v1(a, &mut sa), OK);
            assert_eq!(c6_random_state_v1(b, &mut sb), OK);
            assert_eq!(sa, sb);
            assert_eq!(
                c6_apply_v1(
                    b,
                    &C6Action {
                        kind: 100,
                        argument: 0
                    },
                    &mut result
                ),
                UNSUPPORTED
            );
            c6_destroy_v1(a);
            c6_destroy_v1(b);
        }
    }
}
