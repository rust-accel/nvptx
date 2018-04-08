use glob::glob;
use std::io::{Read, Write};
use std::path::*;
use std::{fs, io, process};
use tempdir::TempDir;

use config::{Crate, Depends};

#[derive(Debug, Clone, Copy)]
pub enum Step {
    Ready,
    Format,
    Link,
    Build,
    Load,
}

#[derive(Debug, From)]
pub enum CompileError {
    ExternalComandError((Step, i32)),
    IOError((Step, io::Error)),
}
pub type Result<T> = ::std::result::Result<T, CompileError>;

trait Logging {
    type T;
    fn log(self, step: Step) -> Result<Self::T>;
}

impl<T> Logging for io::Result<T> {
    type T = T;
    fn log(self, step: Step) -> Result<Self::T> {
        self.map_err(|e| (step, e).into())
    }
}

/// Compile Rust string into PTX string
pub struct Builder {
    path: PathBuf,
    depends: Depends,
}

impl Builder {
    pub fn new(depends: Depends) -> Self {
        let path = TempDir::new("ptx-builder")
            .expect("Failed to create temporal directory")
            .into_path();
        Self::with_path(&path, depends)
    }

    pub fn with_path<P: AsRef<Path>>(path: P, depends: Depends) -> Self {
        let path = path.as_ref();
        fs::create_dir_all(path.join("src")).unwrap();
        Builder {
            path: path.to_owned(),
            depends: depends,
        }
    }

    pub fn crates(&self) -> &[Crate] {
        &self.depends
    }

    pub fn compile(&mut self, kernel: &str) -> Result<String> {
        self.generate_config()?;
        self.save(kernel, "src/lib.rs").log(Step::Ready)?;
        self.format()?;
        self.clean();
        self.build()?;
        self.link()?;
        self.load_ptx()
    }

    /// save string as a file on the Builder directory
    fn save(&self, contents: &str, filename: &str) -> io::Result<()> {
        let mut f = fs::File::create(self.path.join(filename))?;
        f.write(contents.as_bytes())?;
        Ok(())
    }

    fn link(&self) -> Result<()> {
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
            .map(|x| x.unwrap().to_str().unwrap().to_owned())
            .collect();
        process::Command::new("llvm-link")
            .args(&bcs)
            .args(&["-o", "kernel.bc"])
            .current_dir(&self.path)
            .check_run(Step::Link)?;
        // compile bytecode to PTX
        process::Command::new("llc")
            .args(&["-mcpu=sm_20", "kernel.bc", "-o", "kernel.ptx"])
            .current_dir(&self.path)
            .check_run(Step::Link)?;
        Ok(())
    }

    fn load_ptx(&self) -> Result<String> {
        let mut f = fs::File::open(self.path.join("kernel.ptx")).log(Step::Load)?;
        let mut res = String::new();
        f.read_to_string(&mut res).unwrap();
        Ok(res)
    }

    fn generate_config(&self) -> Result<()> {
        self.save(&self.depends.to_string(), "Cargo.toml").log(Step::Ready)?;
        self.save(include_str!("nvptx64-nvidia-cuda.json"), "nvptx64-nvidia-cuda.json")
            .log(Step::Ready)?;
        Ok(())
    }

    fn clean(&self) {
        let path = self.path.join("target");
        match fs::remove_dir_all(&path) {
            Ok(_) => {}
            Err(_) => eprintln!("Already clean (dir = {})", path.display()),
        };
    }

    fn format(&self) -> Result<()> {
        process::Command::new("cargo")
            .args(&["fmt"])
            .current_dir(&self.path)
            .check_run(Step::Format)
    }

    fn build(&self) -> Result<()> {
        process::Command::new("xargo")
            .args(&["+nightly", "rustc", "--release", "--target", "nvptx64-nvidia-cuda"])
            .current_dir(&self.path)
            .check_run(Step::Build)
    }
}

trait CheckRun {
    fn check_run(&mut self, step: Step) -> Result<()>;
}

impl CheckRun for process::Command {
    fn check_run(&mut self, step: Step) -> Result<()> {
        let st = self.status().log(step)?;
        match st.code() {
            Some(c) => {
                if c != 0 {
                    Err(CompileError::ExternalComandError((step, c)).into())
                } else {
                    Ok(())
                }
            }
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::*;

    #[test]
    fn compile() {
        let src = r#"
        #![feature(abi_ptx)]
        #![no_std]
        extern crate accel_core;
        #[no_mangle]
        pub unsafe extern "ptx-kernel" fn add(a: *const f64, b: *const f64, c: *mut f64, n: usize) {
            let i = accel_core::index();
            if (i as usize) < n {
                *c.offset(i) = *a.offset(i) + *b.offset(i);
            }
        }
        "#;
        let depends = Depends::from(&[Crate::with_version("accel-core", "0.2.0-alpha")]);
        let mut builder = Builder::new(depends);
        let ptx = builder.compile(src).unwrap();
        println!("PTX = {:?}", ptx);
    }
}
