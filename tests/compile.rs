extern crate nvptx;

use nvptx::*;

const GPU_CODE: &'static str = r#"
#![feature(abi_ptx)]
#![no_std]
extern crate accel_core;
#[no_mangle]
pub unsafe extern "ptx-kernel" fn add(a: *const f64, b: *const f64, c: *mut f64, n: usize) {
    let i = accel_core::index();
    if (i as usize) < n {
        *c.offset(i) = *a.offset(i) + *b.offset(i);
    }
}
"#;

#[test]
fn compile_tmp() {
    let dri = Driver::new().unwrap();
    manifest::Generator::new(dri.path())
        .add_crate_with_version("accel-core", "0.2.0-alpha")
        .generate()
        .unwrap();
    let ptx = dri.compile_str(GPU_CODE).unwrap();
    println!("PTX = {:?}", ptx);
}

#[test]
fn compile_path() {
    let dri = Driver::with_path("~/tmp/rust2ptx").unwrap();
    manifest::Generator::new(dri.path())
        .add_crate_with_version("accel-core", "0.2.0-alpha")
        .generate()
        .unwrap();
    let ptx = dri.compile_str(GPU_CODE).unwrap();
    println!("PTX = {:?}", ptx);
}
