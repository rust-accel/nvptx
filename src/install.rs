use failure::{self, err_msg};
use std::path::*;
use std::{fs, process};
use tempdir::TempDir;

use driver::rlib2bc;

/// Download nvptx-enable rustc from AWS S3
///
/// This archive has been generated from rust-accel/rust fork
/// https://github.com/rust-accel/rust
pub fn install(path: &Path) -> Result<(), failure::Error> {
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
        rlib2bc(&path);
    }

    Ok(())
}
