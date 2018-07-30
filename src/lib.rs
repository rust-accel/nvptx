//! Compile Rust into PTX string using LLVM

#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate serde_derive;
extern crate dirs;
extern crate glob;
extern crate serde;
extern crate serde_json;
extern crate tempdir;
extern crate toml;

mod config;
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

pub(crate) fn save_str<P: AsRef<Path>>(path: P, contents: &str, filename: &str) -> io::Result<()> {
    let mut f = fs::File::create(path.as_ref().join(filename))?;
    f.write(contents.as_bytes())?;
    Ok(())
}
