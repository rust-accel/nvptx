use failure::err_msg;
use std::path::*;
use std::str::from_utf8;
use std::{fs, process};
use tempdir::TempDir;

use super::{TARGET_NAME, TOOLCHAIN_NAME};
use crate::driver::rlib2bc;
use crate::error::ResultAny;

/// Download nvptx-enable rustc from AWS S3
///
/// This archive has been generated from rust-accel/rust fork
/// https://github.com/rust-accel/rust
pub fn install(path: &Path) -> ResultAny<()> {
    fs::create_dir_all(path)?;
    let tmp_dir = TempDir::new("nvptx_install")?;
    let rustc = "rustc";
    let rust_std = "rust-std";
    let rust_doc = "rust-docs";
    let x86 = "x86_64-unknown-linux-gnu";
    let version = "1.28.0-dev";
    for cmp in &[rustc, rust_std, rust_doc] {
        for target in &[x86, TARGET_NAME] {
            if (cmp == &rustc) && (target == &TARGET_NAME) {
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
    eprintln!("Create {} toolchain", TOOLCHAIN_NAME);
    let ec = process::Command::new("rustup")
        .args(&["toolchain", "link", TOOLCHAIN_NAME])
        .arg(path)
        .status()?;
    if !ec.success() {
        return Err(err_msg("rustup failed"));
    }

    // Expand rlib into LLVM BC, and link them
    let nvptx_dir = get_nvptx_lib_path()?;
    eprintln!("Convert rlibs in {}", nvptx_dir.display());
    for entry in fs::read_dir(&nvptx_dir)? {
        let path = entry?.path();
        if path.extension().unwrap() == "rlib" {
            eprintln!(" - {}", path.display());
            rlib2bc(&path)?;
        }
    }
    Ok(())
}

fn get_toolchain_path() -> ResultAny<PathBuf> {
    let output = process::Command::new("rustup")
        .args(&["run", TOOLCHAIN_NAME, "rustc", "--print", "sysroot"])
        .output()?;
    Ok(PathBuf::from(from_utf8(&output.stdout)?.trim()))
}

fn get_nvptx_lib_path() -> ResultAny<PathBuf> {
    Ok(get_toolchain_path()?
        .join("lib/rustlib")
        .join(TARGET_NAME)
        .join("lib"))
}

fn get_all_compiler_rt() -> ResultAny<Vec<PathBuf>> {
    let nvptx_dir = get_nvptx_lib_path()?;
    Ok(fs::read_dir(&nvptx_dir)?
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            if path.extension()? != "bc" {
                return None;
            }
            Some(path)
        })
        .collect())
}

/// Installed runtime libraries
const RUNTIME_LIBS: [&str; 13] = [
    "alloc",
    "alloc_system",
    "compiler_builtins",
    "core",
    "getopts",
    "libc",
    "panic_abort",
    "panic_unwind",
    "std",
    "std_unicode",
    "term",
    "test",
    "unwind",
];

pub fn get_compiler_rt(runtimes: &[String]) -> ResultAny<Vec<PathBuf>> {
    let all = get_all_compiler_rt()?;
    Ok(runtimes
        .iter()
        .filter_map(|rt| {
            if !RUNTIME_LIBS.contains(&rt.as_str()) {
                eprintln!("Runtime not supported: {}", rt);
                return None;
            }
            for path in &all {
                if path.file_stem()?.to_str()?.contains(rt) {
                    return Some(path.clone());
                }
            }
            unreachable!("Corresponding BC does not found");
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_compiler_rt() {
        let rt = get_all_compiler_rt().unwrap();
        println!("Compiler runtimes = {:?}", rt);
        assert_eq!(rt.len(), 13);
    }

    #[test]
    fn get_core_path() {
        let rt = get_compiler_rt(&["core".to_string()]).unwrap();
        println!("libcore = {:?}", rt[0]);
        assert_eq!(rt.len(), 1);
    }
}
