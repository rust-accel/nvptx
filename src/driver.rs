use dirs::home_dir;
use glob::glob;
use std::io::Read;
use std::path::*;
use std::{fs, io, process};
use tempdir::TempDir;

use super::save_str;
use error::*;

/// Compile Rust string into PTX string
pub struct Driver {
    path: PathBuf,
    release: bool,
}

impl Driver {
    /// Create builder on /tmp
    pub fn new() -> Result<Self> {
        let path = TempDir::new("accel-nvptx")
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
            release: true,
        })
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
        process::Command::new("cargo")
            .args(&["+accel-nvptx", "build", "--target", "nvptx64-nvidia-cuda"])
            .arg(if self.release { "--release" } else { "" })
            .current_dir(&self.path)
            .check_run(Step::Build)
    }

    fn target_dir(&self) -> io::Result<PathBuf> {
        Ok(fs::canonicalize(format!(
            "{}/target/nvptx64-nvidia-cuda/{}",
            self.path.display(),
            if self.release { "release" } else { "debug" }
        ))?)
    }

    /// Link rlib into a single PTX file
    pub fn link(&self) -> Result<()> {
        let target_dir = self.target_dir().log_unwrap(Step::Link)?;
        let tmp = TempDir::new("nvptx-link").log(Step::Link, "Cannot create tmp dir")?;
        let mut bitcodes = Vec::new();
        // extract rlibs using ar x
        for path in glob(&format!("{}/deps/*.rlib", target_dir.display())).log_unwrap(Step::Link)? {
            let path = path.unwrap();
            // get object archived in rlib
            let obj_output = String::from_utf8(
                process::Command::new("ar")
                    .arg("t")
                    .arg(&path)
                    .output()
                    .log_unwrap(Step::Link)?
                    .stdout,
            ).log_unwrap(Step::Link)?;
            // get only *.o (drop *.z and metadata)
            let mut objs = obj_output
                .split('\n')
                .filter_map(|f| {
                    let f = f.trim();
                    if f.ends_with(".o") {
                        Some(f.to_owned())
                    } else {
                        None
                    }
                })
                .collect();
            bitcodes.append(&mut objs);
            // expand to temporal directory
            process::Command::new("ar")
                .arg("x")
                .arg(path)
                .current_dir(tmp.path())
                .check_run(Step::Link)?;
        }
        // link them
        process::Command::new(llvm_command("llvm-link")?)
            .args(&bitcodes)
            .arg("-o")
            .arg(target_dir.join("kernel.bc"))
            .current_dir(&tmp.path())
            .check_run(Step::Link)?;
        // compile bytecode to PTX
        process::Command::new(llvm_command("llc")?)
            .args(&["-mcpu=sm_50", "kernel.bc", "-o", "kernel.ptx"])
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
pub(crate) fn llvm_command(name: &str) -> Result<String> {
    let name6 = format!("{}-6.0", name);
    let name7 = format!("{}-7.0", name);
    if check_exists(&name6) {
        Ok(name6)
    } else if check_exists(&name7) {
        Ok(name7)
    } else if check_exists(&name) {
        Ok(name.into())
    } else {
        Err(CompileError::LLVMCommandNotFound {
            command: name.into(),
        }.into())
    }
}
