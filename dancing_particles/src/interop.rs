//! Defines functions that are exported to the JavaScript frontend, allowing access to the engine during runtime.
//!
//! Copied from https://github.com/Ameobea/noise-asmjs/blob/master/engine/src/interop.rs

use std::os::raw::c_char;
use std::ffi::CString;

extern {
    /// Direct line to `console.log` from JS since the simulated `stdout` is dead after `main()` completes
    pub fn js_debug(msg: *const c_char);
    /// Direct line to `console.error` from JS since the simulated `stdout` is dead after `main()` completes
    pub fn js_error(msg: *const c_char);
}

/// Wrapper around the JS debug function that accepts a Rust `&str`.
pub fn debug(msg: &str) {
    let c_str = CString::new(msg).unwrap();
    unsafe { js_debug(c_str.as_ptr()) };
}

/// Wrapper around the JS error function that accepts a Rust `&str`.
pub fn error(msg: &str) {
    let c_str = CString::new(msg).unwrap();
    unsafe { js_error(c_str.as_ptr()) };
}

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum GenType {
    Fbm,
    Worley,
    OpenSimplex,
    Billow,
    HybridMulti,
    SuperSimplex,
    Value,
    RidgedMulti,
    BasicMulti,
}
