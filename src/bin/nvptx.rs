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
    #[structopt(
        name = "build",
        raw(setting = "structopt::clap::AppSettings::ColoredHelp")
    )]
    Build {
        /// Load generated PTX to stdout
        #[structopt(short = "l", long = "load")]
        load: bool,
        /// Release build
        #[structopt(long = "release")]
        release: bool,
        /// alternative toolchain (default=accel-nvptx)
        #[structopt(long = "toolchain")]
        toolchain: Option<String>,
    },

    /// Load PTX to stdout
    #[structopt(
        name = "load",
        raw(setting = "structopt::clap::AppSettings::ColoredHelp")
    )]
    Load {},

    /// Download and Install nvptx-enabled rustc
    #[structopt(
        name = "install",
        raw(setting = "structopt::clap::AppSettings::ColoredHelp")
    )]
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
        Opt::Build {
            load,
            release,
            toolchain,
        } => {
            let manifest_path = get_manifest_path();
            let mut driver = Driver::with_path(manifest_path)?;
            if let Some(toolchain) = toolchain {
                driver.alternative_toolchain(&toolchain);
            }
            if release {
                driver.release_build();
            }
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
