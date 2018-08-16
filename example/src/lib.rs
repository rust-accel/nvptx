#![feature(abi_ptx, lang_items, core_intrinsics)]
#![no_std]

extern crate accel_core;

#[no_mangle]
pub unsafe extern "ptx-kernel" fn add(a: *const f64, b: *const f64, c: *mut f64, n: usize) {
    let i = accel_core::index();
    if (i as usize) < n {
        *c.offset(i) = *a.offset(i) + *b.offset(i);
    }
}
