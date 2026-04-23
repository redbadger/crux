//! Plain C ABI exports for P/Invoke from the C# / WinUI3 shell.
//!
//! Other shells use tool-specific bindings (`uniffi` for Apple / Android,
//! `wasm-bindgen` for web) defined directly on [`CoreFFI`]. Those tools
//! don't target .NET, so the Windows shell talks to the same core through
//! plain `extern "C"` entry points via `DllImport`.

#![allow(unsafe_code)]

use std::slice;

use crate::ffi::CoreFFI;

/// A Rust-allocated byte buffer handed to the shell.
///
/// The shell must return it via [`crux_counter_free_buf`] to release the
/// underlying `Vec<u8>`. `cap` is the original `Vec` capacity so we can
/// reconstruct the exact allocation for `drop`.
#[repr(C)]
pub struct ByteBuf {
    pub ptr: *mut u8,
    pub len: usize,
    pub cap: usize,
}

impl ByteBuf {
    fn from_vec(mut v: Vec<u8>) -> Self {
        let buf = Self {
            ptr: v.as_mut_ptr(),
            len: v.len(),
            cap: v.capacity(),
        };
        std::mem::forget(v);
        buf
    }
}

/// Create a new [`CoreFFI`] instance. The caller owns the returned pointer
/// and must release it with [`crux_counter_free`].
#[unsafe(no_mangle)]
pub extern "C" fn crux_counter_new() -> *mut CoreFFI {
    Box::into_raw(Box::new(CoreFFI::new()))
}

/// Free a [`CoreFFI`] instance previously returned by [`crux_counter_new`].
///
/// # Safety
/// `core` must be a pointer returned by [`crux_counter_new`] that has not
/// already been freed. Passing a null pointer is a no-op.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crux_counter_free(core: *mut CoreFFI) {
    if core.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(core) });
}

/// Send a bincode-encoded event to the core. The returned buffer contains a
/// bincode-encoded `Vec<Request>` and must be released via
/// [`crux_counter_free_buf`].
///
/// # Safety
/// `core` must be a valid pointer from [`crux_counter_new`]. `data` must be
/// readable for `len` bytes. `out` must be a writable pointer to `ByteBuf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crux_counter_update(
    core: *mut CoreFFI,
    data: *const u8,
    len: usize,
    out: *mut ByteBuf,
) {
    let core = unsafe { &*core };
    let slice = unsafe { slice::from_raw_parts(data, len) };
    let result = core.update(slice);
    unsafe { out.write(ByteBuf::from_vec(result)) };
}

/// Resolve an effect with the given id and shell response.
///
/// # Safety
/// See [`crux_counter_update`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crux_counter_resolve(
    core: *mut CoreFFI,
    id: u32,
    data: *const u8,
    len: usize,
    out: *mut ByteBuf,
) {
    let core = unsafe { &*core };
    let slice = unsafe { slice::from_raw_parts(data, len) };
    let result = core.resolve(id, slice);
    unsafe { out.write(ByteBuf::from_vec(result)) };
}

/// Read the current bincode-encoded `ViewModel`.
///
/// # Safety
/// `core` must be a valid pointer from [`crux_counter_new`]. `out` must be a
/// writable pointer to `ByteBuf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crux_counter_view(core: *mut CoreFFI, out: *mut ByteBuf) {
    let core = unsafe { &*core };
    let result = core.view();
    unsafe { out.write(ByteBuf::from_vec(result)) };
}

/// Release a [`ByteBuf`] produced by the core.
///
/// # Safety
/// `buf` must have been produced by this library and not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crux_counter_free_buf(buf: ByteBuf) {
    if buf.ptr.is_null() {
        return;
    }
    drop(unsafe { Vec::from_raw_parts(buf.ptr, buf.len, buf.cap) });
}
