extern crate nvptx;

#[macro_use]
extern crate structopt;
extern crate dirs;

use nvptx::error::Result;
use nvptx::Driver;

use std::env;
use std::path::*;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
enum Opt {
    /// Compile crate into PTX
    #[structopt(name = "build")]
    Build {
        /// Release build
        #[structopt(long = "release")]
        release: bool,
    },

    /// Download and Install nvptx-enabled rustc
    #[structopt(name = "install")]
    Install {
        /// Install path
        #[structopt(short = "p", long = "path", parse(from_os_str))]
        path: Option<PathBuf>,
    },
}

/// Search Cargo.toml from current directory
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

/// Download nvptx-enable rustc from AWS S3
///
/// This archive has been generated from rust-accel/rust fork
/// https://github.com/rust-accel/rust
fn download(_path: &Path) -> Result<()> {
    // TODO impl
    Ok(())
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    match opt {
        Opt::Build { release } => {
            let manifest_path = get_manifest_path();
            let ptx = Driver::with_path(manifest_path)?.compile(release)?;
            println!("{}", ptx);
        }
        Opt::Install { path } => {
            download(&path.unwrap_or(dirs::data_dir().unwrap().join("accel-nvptx")))?;
        }
    }
    Ok(())
}
