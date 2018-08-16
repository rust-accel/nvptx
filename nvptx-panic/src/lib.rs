#![feature(abi_ptx, lang_items, core_intrinsics)]
#![no_std]

#[lang = "panic_impl"]
extern "C" fn rust_begin_panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::intrinsics::abort() }
}
