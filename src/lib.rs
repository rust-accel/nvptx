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
extern crate tempdir;
extern crate toml;

mod config;
mod driver;
pub mod error;
mod manifest;

pub use driver::Driver;
pub use manifest::ManifestGenerator;

use std::io::Write;
use std::path::Path;
use std::{fs, io};

pub(crate) fn save_str<P: AsRef<Path>>(path: P, contents: &str, filename: &str) -> io::Result<()> {
    let mut f = fs::File::create(path.as_ref().join(filename))?;
    f.write(contents.as_bytes())?;
    Ok(())
}
