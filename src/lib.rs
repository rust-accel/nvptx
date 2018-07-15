//! Compile Rust into PTX string using LLVM

#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate serde_derive;
extern crate cargo;
extern crate dirs;
extern crate glob;
extern crate serde;
extern crate tempdir;
extern crate toml;

pub mod compile;
pub mod config;
