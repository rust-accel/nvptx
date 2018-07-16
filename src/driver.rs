use dirs::home_dir;
use glob::glob;
use std::io::Read;
use std::path::*;
use std::{fs, process};
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

    pub fn compile(&self) -> Result<String> {
        self.build()?;
        self.link()?;
        self.load_ptx()
    }

    pub fn compile_str(&self, kernel: &str) -> Result<String> {
        save_str(&self.path, kernel, "src/lib.rs").log(Step::Ready, "Failed to save lib.rs")?;
        self.format();
        self.clean();
        self.compile()
    }

    pub fn build(&self) -> Result<()> {
        process::Command::new("cargo")
            .args(&["+accel-nvptx", "build", "--target", "nvptx64-nvidia-cuda"])
            .arg(if self.release { "--release" } else { "" })
            .current_dir(&self.path)
            .check_run(Step::Build)
    }

    pub fn link(&self) -> Result<()> {
        // extract rlibs using ar x
        let pat_rlib = format!("{}/target/**/deps/*.rlib", self.path.display());
        for path in glob(&pat_rlib).unwrap() {
            let path = path.unwrap();
            process::Command::new("ar")
                .args(&["x", path.file_name().unwrap().to_str().unwrap()])
                .current_dir(path.parent().unwrap())
                .check_run(Step::Link)?;
        }
        // link them
        let pat_rsbc = format!("{}/target/**/deps/*.o", self.path.display());
        let bcs: Vec<_> = glob(&pat_rsbc)
            .unwrap()
            .map(|x| {
                fs::canonicalize(x.unwrap())
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        process::Command::new(llvm_command("llvm-link")?)
            .args(&bcs)
            .args(&["-o", "kernel.bc"])
            .current_dir(&self.path)
            .check_run(Step::Link)?;
        // compile bytecode to PTX
        process::Command::new(llvm_command("llc")?)
            .args(&["-mcpu=sm_20", "kernel.bc", "-o", "kernel.ptx"])
            .current_dir(&self.path)
            .check_run(Step::Link)?;
        Ok(())
    }

    pub fn load_ptx(&self) -> Result<String> {
        let mut f =
            fs::File::open(self.path.join("kernel.ptx")).log(Step::Load, "kernel.ptx cannot open")?;
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

fn llvm_command(name: &str) -> Result<String> {
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
