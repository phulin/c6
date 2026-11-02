//! Version 1 loadable C API. See include/c6.h for pointer and ownership contracts.
use c6_core::rng::RandomState;
use c6_core::{Action, Engine, Outcome, Snapshot};
use std::ptr;

pub const OK: i32 = 0;
pub const INVALID: i32 = 1;
pub const UNSUPPORTED: i32 = 2;
pub const BUFFER_TOO_SMALL: i32 = 3;

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
    if handle.is_null() || action.is_null() || result.is_null() {
        return INVALID;
    }
    let a = &*action;
    let action = match a.kind {
        1 if a.argument <= u16::MAX as u32 => Action::RandomDraw {
            bound: a.argument as u16,
        },
        1 => return INVALID,
        2 => Action::RandomReset { seed: a.argument },
        3 => Action::RandomInitialize { seed: a.argument },
        4 => Action::RandomLock,
        5 => Action::RandomUnlock,
        _ => return UNSUPPORTED,
    };
    match (*handle).0.apply(&action) {
        Ok(o) => {
            *result = match o {
                Outcome::Random(n) => n as u32,
                Outcome::Applied => 0,
            };
            OK
        }
        Err(_) => UNSUPPORTED,
    }
}
/// # Safety
/// Handle must be live; out must be writable and not overlap handle storage.
#[no_mangle]
pub unsafe extern "C" fn c6_random_state_v1(
    handle: *const C6Handle,
    out: *mut C6RandomState,
) -> i32 {
    if handle.is_null() || out.is_null() {
        return INVALID;
    }
    *out = (*handle).0.snapshot().random.into();
    OK
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
