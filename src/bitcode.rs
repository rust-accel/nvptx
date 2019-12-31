use llvm_sys::bit_reader::*;
use llvm_sys::core::*;
use llvm_sys::prelude::*;

use failure::err_msg;
use std::ffi::*;
use std::os::raw::c_char;
use std::path::*;
use std::ptr::null_mut;

use crate::error::*;

struct MemoryBuffer(LLVMMemoryBufferRef);

impl Drop for MemoryBuffer {
    fn drop(&mut self) {
        unsafe { LLVMDisposeMemoryBuffer(self.0) }
    }
}

impl MemoryBuffer {
    fn new(filename: &str) -> ResultAny<Self> {
        let input = CString::new(filename)?;
        let mut membuf: LLVMMemoryBufferRef = null_mut();
        let mut msg: *mut c_char = null_mut();
        let result = unsafe {
            LLVMCreateMemoryBufferWithContentsOfFile(
                input.into_raw(),
                &mut membuf as *mut LLVMMemoryBufferRef,
                &mut msg as *mut *mut c_char,
            )
        };
        if result != 0 {
            let msg = unsafe { CString::from_raw(msg) };
            return Err(err_msg(format!("Canont read input: {:?}", msg)));
        }
        Ok(MemoryBuffer(membuf))
    }
}

#[derive(Debug)]
struct Module(LLVMModuleRef);

#[derive(Debug)]
struct Function(LLVMValueRef);

impl Module {
    fn parse_bitcode(buf: &MemoryBuffer) -> ResultAny<Self> {
        let mut md: LLVMModuleRef = null_mut();
        let res = unsafe { LLVMParseBitcode2(buf.0, &mut md as *mut _) };
        if res != 0 {
            return Err(err_msg("Cannot parse LLVM Bitcode"));
        }
        Ok(Module(md))
    }

    fn read_bitcode(filename: &str) -> ResultAny<Self> {
        let membuf = MemoryBuffer::new(filename)?;
        Self::parse_bitcode(&membuf)
    }

    fn functions(&self) -> Vec<Function> {
        let mut funcs = Vec::new();
        let mut f = unsafe { LLVMGetFirstFunction(self.0) };
        while f != null_mut() {
            funcs.push(Function(f));
            f = unsafe { LLVMGetNextFunction(f) };
        }
        funcs
    }
}

impl Function {
    fn name(&self) -> String {
        let name = unsafe { CString::from_raw(LLVMGetValueName(self.0) as *mut _) };
        name.into_string().expect("Fail to parse function name")
    }

    // See the LLVM call convention list
    //
    // - PTX_Kernel = 71
    // - PTX_Device = 72
    //
    // http://llvm.org/doxygen/CallingConv_8h_source.html
    fn call_conv(&self) -> u32 {
        unsafe { LLVMGetFunctionCallConv(self.0) }
    }

    fn is_ptx_kernel(&self) -> bool {
        self.call_conv() == 71
    }

    fn is_ptx_device_func(&self) -> bool {
        self.call_conv() == 72
    }
}

pub fn get_ptx_functions<P: AsRef<Path>>(filename: P) -> ResultAny<Vec<String>> {
    let path = filename.as_ref().to_str().unwrap();
    let md = Module::read_bitcode(path)?;
    let ptx: Vec<_> = md
        .functions()
        .iter()
        .filter(|f| f.is_ptx_kernel() || f.is_ptx_device_func())
        .map(|f| f.name())
        .collect();
    if ptx.len() == 0 {
        return Err(err_msg("No PTX found"));
    }
    Ok(ptx)
}
