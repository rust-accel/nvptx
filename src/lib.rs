//! Compile Rust into PTX string using LLVM

mod bitcode;
mod driver;
pub mod error;
pub mod manifest;
mod toolchain;

pub use driver::Driver;
pub use toolchain::{get_compiler_rt, install};

use std::io::Write;
use std::path::Path;
use std::{fs, io};

const TOOLCHAIN_NAME: &'static str = "accel-nvptx";
const TARGET_NAME: &'static str = "nvptx64-nvidia-cuda";

pub(crate) fn save_str<P: AsRef<Path>>(path: P, contents: &str, filename: &str) -> io::Result<()> {
    let mut f = fs::File::create(path.as_ref().join(filename))?;
    f.write(contents.as_bytes())?;
    Ok(())
}
