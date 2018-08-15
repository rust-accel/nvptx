use colored::*;
use dirs::home_dir;
use failure::err_msg;
use serde_json::{self, Value};
use std::io::Read;
use std::path::*;
use std::str::from_utf8;
use std::{fs, io, process};
use tempdir::TempDir;

use super::*;
use error::*;

/// Compile Rust string into PTX string
pub struct Driver {
    path: PathBuf,
    release: bool,
    toolchain: String,
}

impl Driver {
    /// Create builder on /tmp
    pub fn new() -> Result<Self> {
        let path = TempDir::new("nvptx-driver")
            .expect("Failed to create temporal directory")
            .into_path();
        Self::with_path(&path)
    }

    /// Create builder at the specified path
    pub fn with_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut path = path.as_ref().to_owned();
        if path.starts_with("~") {
            let home = home_dir().unwrap();
            path = home.join(path.strip_prefix("~").unwrap());
        }
        fs::create_dir_all(path.join("src")).log(Step::Ready, "Cannot create build directory")?;
        Ok(Driver {
            path: path,
            release: false,
            toolchain: TOOLCHAIN_NAME.into(),
        })
    }

    pub fn alternative_toolchain(&mut self, toolchain: &str) {
        self.toolchain = toolchain.into();
    }

    pub fn release_build(&mut self) {
        self.release = true;
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn compile(&self) -> Result<()> {
        self.build()?;
        self.link()?;
        Ok(())
    }

    pub fn compile_str(&self, kernel: &str) -> Result<String> {
        save_str(&self.path, kernel, "src/lib.rs").log(Step::Ready, "Failed to save lib.rs")?;
        self.format();
        self.clean();
        self.compile()?;
        self.load_ptx()
    }

    pub fn build(&self) -> Result<()> {
        let mut cmd = process::Command::new("cargo");
        cmd.arg(format!("+{}", self.toolchain))
            .args(&["build", "--target", TARGET_NAME]);
        if self.release {
            cmd.arg("--release");
        }
        cmd.current_dir(&self.path).check_run(Step::Build)
    }

    fn target_dir(&self) -> io::Result<PathBuf> {
        Ok(fs::canonicalize(self.path.join(self.target_dir_name()))?)
    }

    fn target_dir_name(&self) -> String {
        format!(
            "target/{}/{}",
            TARGET_NAME,
            if self.release { "release" } else { "debug" }
        )
    }

    /// Link rlib into a single PTX file
    pub fn link(&self) -> Result<()> {
        let target_dir = self.target_dir().log_unwrap(Step::Link)?;
        let bc_name = "kernel.bc";
        let opt_bc_name = "kernel.opt.bc";
        let ptx_name = "kernel.ptx";

        // List bitcodes
        let bitcodes: ResultAny<Vec<PathBuf>> = fs::read_dir(target_dir.join("deps"))
            .log(Step::Link, "deps dir not found")?
            .filter_map(|entry| {
                let path = entry.unwrap().path();
                if path.extension()? == "rlib" {
                    Some(rlib2bc(&path))
                } else {
                    None
                }
            }).collect();

        // Link Rust runtime libraries
        eprintln!(
            "{:>12} Rust runtimes ({}/{})",
            "Linking".bright_green(),
            self.target_dir_name(),
            bc_name
        );
        let rt = self
            .get_runtime_setting()
            .log(Step::Link, "Fail to load package.metadata.nvptx.runtime")?;
        process::Command::new(llvm_command("llvm-link").log(Step::Link, "llvm-link not found")?)
            .args(&bitcodes.log(Step::Link, "Fail to convert to LLVM BC")?)
            .args(get_compiler_rt(&rt).log(Step::Link, "Fail to get copiler-rt libs")?)
            .args(&["-o", bc_name])
            .current_dir(&target_dir)
            .check_run(Step::Link)?;

        // Internalize unused symbols
        eprintln!(
            "{:>12} unused bitcodes ({}/{})",
            "Drop".bright_green(),
            self.target_dir_name(),
            opt_bc_name
        );
        let ptx_funcs = bitcode::get_ptx_functions(&target_dir.join(bc_name))
            .log(Step::Link, "Fail to parse LLVM bitcode")?;
        process::Command::new(llvm_command("opt").log(Step::Link, "opt not found")?)
            .arg("-internalize")
            .arg(format!(
                "-internalize-public-api-list={}",
                ptx_funcs.join(",")
            )).arg("-globaldce")
            .args(&[bc_name, "-o", opt_bc_name])
            .current_dir(&target_dir)
            .check_run(Step::Link)?;

        // Generate PTX
        eprintln!(
            "{:>12} PTX code ({}/{})",
            "Generating".bright_green(),
            self.target_dir_name(),
            ptx_name
        );
        process::Command::new(llvm_command("llc").log_unwrap(Step::Link)?)
            .arg(if self.release { "-O3" } else { "-O0" })
            .args(&["-mcpu=sm_50", opt_bc_name, "-o", ptx_name])
            .current_dir(&target_dir)
            .check_run(Step::Link)?;
        Ok(())
    }

    pub fn load_ptx(&self) -> Result<String> {
        let target_dir = self.target_dir().log_unwrap(Step::Load)?;
        let mut f = fs::File::open(target_dir.join("kernel.ptx"))
            .log(Step::Load, "kernel.ptx cannot open")?;
        let mut res = String::new();
        f.read_to_string(&mut res).unwrap();
        Ok(res)
    }

    fn clean(&self) {
        let path = self.path.join("target");
        match fs::remove_dir_all(&path) {
            Ok(_) => {}
            Err(_) => info!("Already clean (dir = {})", path.display()),
        };
    }

    // Format generated code using cargo-fmt for better debugging
    fn format(&self) {
        let result = process::Command::new("cargo")
            .args(&["fmt", "--all"])
            .current_dir(&self.path)
            .check_run(Step::Ready);

        if let Err(e) = result {
            warn!("Format failed: {:?}", e);
        }
    }

    /// Runtime setting is writen in Cargo.toml like
    ///
    /// ```text
    /// [package.metadata.nvptx]
    /// runtime = ["core"]
    /// ```
    fn get_runtime_setting(&self) -> ResultAny<Vec<String>> {
        let output = process::Command::new("cargo")
            .args(&["metadata", "--no-deps", "--format-version=1"])
            .current_dir(&self.path)
            .output()?;
        let json = from_utf8(&output.stdout)?;
        if json.len() == 0 {
            return Ok(Vec::new());
        }
        let meta: Value = serde_json::from_str(json)?;
        Ok(match meta.pointer("/packages/0/metadata/nvptx/runtime") {
            Some(rt) => {
                let rt = rt
                    .as_array()
                    .ok_or(err_msg("nvptx.runtime must be array"))?;
                rt.iter()
                    .map(|name| {
                        name.as_str()
                            .ok_or(err_msg("Component of nvptx.runtime must be string"))
                            .map(|s| s.to_string())
                    }).collect::<ResultAny<Vec<String>>>()?
            }
            None => Vec::new(),
        })
    }
}

/// Expand rlib into a linked LLVM/BC binary (*.bc)
pub fn rlib2bc(path: &Path) -> ResultAny<PathBuf> {
    let parent = path.parent().unwrap_or(Path::new(""));
    let name = path.file_stem().unwrap();
    let dir = TempDir::new("rlib2bc")?;
    let target = parent.join(format!("{}.bc", name.to_str().unwrap()));

    // `ar xv some.rlib` expand rlib and show its compnent
    let output = process::Command::new("ar")
        .arg("xv")
        .arg(&path)
        .current_dir(&dir)
        .output()?;
    // trim ar output
    let components: Vec<_> = from_utf8(&output.stdout)?
        .lines()
        .map(|line| line.trim_left_matches("x - "))
        .collect();
    // filter LLVM BC files
    let bcs: Vec<_> = components
        .iter()
        .filter(|line| line.ends_with(".rcgu.o"))
        .collect(); // FIXME filtering using suffix will cause compiler dependency
    let ec = process::Command::new(llvm_command("llvm-link")?)
        .args(&bcs)
        .arg("-o")
        .arg(&target)
        .current_dir(&dir)
        .status()?;
    if !ec.success() {
        return Err(err_msg("Re-archive failed"));
    }
    Ok(target)
}

/// Check if the command exists using "--help" flag
fn check_exists(name: &str) -> bool {
    process::Command::new(name)
        .arg("--help")
        .stdout(process::Stdio::null())
        .stderr(process::Stdio::null())
        .check_run(Step::Ready)
        .is_ok()
}

/// Resolve LLVM command name with postfix
fn llvm_command(name: &str) -> ResultAny<String> {
    let name6 = format!("{}-6.0", name);
    let name7 = format!("{}-7.0", name);
    if check_exists(&name6) {
        Ok(name6)
    } else if check_exists(&name7) {
        Ok(name7)
    } else if check_exists(&name) {
        Ok(name.into())
    } else {
        Err(err_msg(
            "LLVM Command {} or postfixed by *-6.0 or *-7.0 are not found.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use manifest;

    #[test]
    fn get_runtime_here() {
        let driver = Driver::with_path(".").unwrap();
        let rt = driver.get_runtime_setting().unwrap();
        assert_eq!(rt, vec!["core".to_string()]);
    }

    #[test]
    fn get_runtime_tmp() {
        let dri = Driver::new().unwrap();
        let core = manifest::Crate::new("accel-core", "0.2.0-alpha");
        manifest::generate(dri.path(), &[core]).unwrap();
        let rt = dri
            .get_runtime_setting()
            .expect("Failed to get runtime setting");
        assert_eq!(rt, Vec::<String>::new());
    }
}
