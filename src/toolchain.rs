use failure::err_msg;
use serde_json;
use std::path::*;
use std::str::from_utf8;
use std::{fs, process};
use tempdir::TempDir;

use super::TOOLCHAIN_NAME;
use driver::rlib2bc;
use error::ResultAny;

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
    Ok(get_toolchain_path()?.join("lib/rustlib/nvptx64-nvidia-cuda/lib"))
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

/// Runtime setting is writen in Cargo.toml like
///
/// ```
/// [package.metadata.nvptx]
/// runtime = ["core"]
/// ```
fn load_runtime_setting() -> ResultAny<Vec<String>> {
    let output = process::Command::new("cargo")
        .args(&["metadata", "--no-deps", "--format-version=1"])
        .output()?;
    let json = from_utf8(&output.stdout)?;
    let meta: serde_json::Value = serde_json::from_str(json)?;
    let meta = &meta["packages"][0]["metadata"];
    if meta.is_null() {
        return Ok(Vec::new());
    }
    let nvptx = meta["nvptx"].as_object().expect("Invlid nvptx metadata");
    Ok(match nvptx.get("runtime") {
        Some(rt) => {
            let rt = rt.as_array().expect("nvptx.runtime must be array");
            rt.iter()
                .map(|name| {
                    name.as_str()
                        .expect("Component of nvptx.runtime must be string")
                        .to_string()
                })
                .collect()
        }
        None => Vec::new(),
    })
}

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
