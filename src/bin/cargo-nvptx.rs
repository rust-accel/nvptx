extern crate nvptx;
#[macro_use]
extern crate structopt;

use nvptx::error::Result;
use nvptx::Driver;

use std::env;
use std::path::*;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(bin_name = "cargo")]
enum Opt {
    #[structopt(name = "nvbuild")]
    Build {
        /// Release build
        #[structopt(long = "release")]
        release: bool,
    },
}

fn get_manifest_path() -> PathBuf {
    let mut dir = env::current_dir().unwrap();
    loop {
        let manif = dir.join("Cargo.toml");
        if manif.exists() {
            return dir;
        }
        dir = match dir.parent() {
            Some(dir) => dir.to_owned(),
            None => panic!("Cargo.toml cannot found"),
        };
    }
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    println!("{:?}", opt);
    let manifest_path = get_manifest_path();
    let ptx = Driver::with_path(manifest_path)?.compile()?;
    println!("{}", ptx);
    Ok(())
}
