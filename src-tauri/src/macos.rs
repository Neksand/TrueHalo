//! Minimal Objective-C runtime helpers for the macOS-specific window tweaks.
//!
//! Every selector we call takes at most one pointer-sized argument, so a single
//! `objc_msgSend` signature is enough and avoids conflicting extern
//! declarations elsewhere in the crate.

use std::ffi::{c_char, c_void};

extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;

    #[link_name = "objc_msgSend"]
    fn msg_send(receiver: *mut c_void, sel: *mut c_void, arg: *mut c_void) -> *mut c_void;
}

/// Send `selector` to `receiver`, passing a single pointer-sized argument.
///
/// Pass `std::ptr::null_mut()` for selectors that take no arguments; the extra
/// register is ignored by the Objective-C runtime.
///
/// # Safety
///
/// `receiver` must be a valid Objective-C object pointer or a valid class
/// pointer, and the selector must exist on it with a compatible signature.
pub unsafe fn send(receiver: *mut c_void, selector: &str, arg: *mut c_void) -> *mut c_void {
    let sel = with_c_string(selector, |name| sel_registerName(name));
    msg_send(receiver, sel, arg)
}

/// Look up an Objective-C class by name.
///
/// # Safety
///
/// The returned pointer is only valid while the Objective-C runtime is loaded;
/// using it after the runtime is torn down is undefined behaviour.
pub unsafe fn class(name: &str) -> *mut c_void {
    with_c_string(name, |n| objc_getClass(n))
}

fn with_c_string<T>(value: &str, f: impl FnOnce(*const c_char) -> T) -> T {
    let mut bytes = value.as_bytes().to_vec();
    if bytes.last() != Some(&0) {
        bytes.push(0);
    }
    f(bytes.as_ptr() as *const c_char)
}
