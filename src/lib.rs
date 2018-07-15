//! Compile Rust into PTX string using LLVM

#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate dirs;
extern crate glob;
extern crate tempdir;
extern crate toml;
extern crate failure;
extern crate cargo;

pub mod config;
pub mod compile;
