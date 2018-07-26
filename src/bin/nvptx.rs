extern crate nvptx;

#[macro_use]
extern crate structopt;
extern crate dirs;
extern crate failure;
extern crate tempdir;

use nvptx::error::{Logging, Step};
use nvptx::Driver;

use failure::err_msg;
use std::path::*;
use std::str::from_utf8;
use std::{env, fs, process};
use structopt::StructOpt;
use tempdir::TempDir;

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

/// Download nvptx-enable rustc from AWS S3
///
/// This archive has been generated from rust-accel/rust fork
/// https://github.com/rust-accel/rust
fn install(path: &Path) -> Result<(), failure::Error> {
    fs::create_dir_all(path)?;
    let tmp_dir = TempDir::new("nvptx_install")?;
    let rustc = "rustc";
    let rust_std = "rust-std";
    let rust_doc = "rust-docs";
    let x86 = "x86_64-unknown-linux-gnu";
    let nvptx = "nvptx64-nvidia-cuda";
    let version = "1.28.0-dev";
    for cmp in &[rustc, rust_std, rust_doc] {
        for target in &[x86, nvptx] {
            if (cmp == &rustc) && (target == &nvptx) {
                // rustc does not work on nvptx
                continue;
            }
            let name = format!("{}-{}-{}", cmp, version, target);
            let arc = format!("{}.tar.xz", name);
            let url = format!("https://s3-ap-northeast-1.amazonaws.com/rust-accel/{}", arc);

            // Download using curl
            eprintln!("download: {}", url);
            let ec = process::Command::new("curl")
                .args(&["-o", &arc, &url])
                .current_dir(tmp_dir.path())
                .status()?;
            if !ec.success() {
                return Err(err_msg("Fail to download"));
            }
            // TODO checksum

            // Expand using tar
            eprintln!("expand: {}", name);
            let ec = process::Command::new("tar")
                .args(&["xf", &arc])
                .current_dir(tmp_dir.path())
                .status()?;
            if !ec.success() {
                return Err(err_msg("Fail to expand archive"));
            }

            // install.sh
            let ec = process::Command::new("./install.sh")
                .arg(format!("--prefix={}", path.display()))
                .current_dir(tmp_dir.path().join(name))
                .status()?;
            if !ec.success() {
                return Err(err_msg("Fail to install"));
            }
        }
    }
    eprintln!("Create accel-nvptx toolchain");
    let ec = process::Command::new("rustup")
        .args(&["toolchain", "link", "accel-nvptx"])
        .arg(path)
        .status()?;
    if !ec.success() {
        return Err(err_msg("rustup failed"));
    }

    // Expand rlib into LLVM BC, and link them
    let nvptx_dir = path.join("lib/rustlib/nvptx64-nvidia-cuda/lib");
    eprintln!("Convert rlibs in {}", nvptx_dir.display());
    for entry in fs::read_dir(&nvptx_dir)? {
        let path = entry?.path();
        let name = path.file_stem().unwrap();
        if path.extension().unwrap() != "rlib" {
            eprintln!("Not rlib: {}", path.display());
            continue;
        }

        // `ar xv some.rlib` expand rlib and show its compnent
        let output = process::Command::new("ar")
            .arg("xv")
            .arg(&path)
            .current_dir(&nvptx_dir)
            .output()?;
        let components: Vec<_> = from_utf8(&output.stdout)?
            .lines()
            .map(|line| line.trim_left_matches("x - "))
            .collect();
        let bcs: Vec<_> = components
            .iter()
            .filter(|line| line.ends_with(".rcgu.o"))
            .collect();
        let ec = process::Command::new("llvm-link")
            .args(&bcs)
            .arg("-o")
            .arg(format!("{}.bc", name.to_str().unwrap()))
            .current_dir(&nvptx_dir)
            .status()?;
        if !ec.success() {
            return Err(err_msg("Re-archive failed"));
        }
        // Remove expanded objects
        for c in &components {
            fs::remove_file(nvptx_dir.join(c))?;
        }
    }

    Ok(())
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
