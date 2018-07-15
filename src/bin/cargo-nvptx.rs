extern crate nvptx;

use nvptx::error::Result;
use nvptx::Driver;

use std::env;
use std::path::*;

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
    let manifest_path = get_manifest_path();
    let ptx = Driver::with_path(manifest_path)?.compile()?;
    println!("{}", ptx);
    Ok(())
}
