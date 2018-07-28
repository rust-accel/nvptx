extern crate nvptx;

#[macro_use]
extern crate structopt;
extern crate dirs;
extern crate failure;
extern crate tempdir;

use nvptx::error::{Logging, Step};
use nvptx::{install, Driver};

use std::env;
use std::path::*;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
enum Opt {
    /// Compile crate into PTX
    #[structopt(name = "build")]
    Build {
        /// Load generated PTX to stdout
        #[structopt(short = "l", long = "load")]
        load: bool,
    },

    /// Load PTX to stdout
    #[structopt(name = "load")]
    Load {},

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

fn main() -> nvptx::error::Result<()> {
    let opt = Opt::from_args();

    match opt {
        Opt::Build { load } => {
            let manifest_path = get_manifest_path();
            let driver = Driver::with_path(manifest_path)?;
            driver.compile()?;
            if load {
                println!("{}", driver.load_ptx()?);
            }
        }
        Opt::Load {} => {
            let manifest_path = get_manifest_path();
            let driver = Driver::with_path(manifest_path)?;
            println!("{}", driver.load_ptx()?);
        }
        Opt::Install { path } => {
            install(&path.unwrap_or(dirs::data_dir().unwrap().join("accel-nvptx")))
                .log_unwrap(Step::Install)?;
        }
    }
    Ok(())
}
