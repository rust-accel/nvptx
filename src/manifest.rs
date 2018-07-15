use std::path::*;

use super::save_str;
use config::CargoTOML;
use error::*;

#[derive(Debug, Clone)]
pub struct Crate {
    name: String,
    version: Option<String>,
    path: Option<PathBuf>,
}

impl Crate {
    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn version(&self) -> String {
        self.version.clone().unwrap_or("*".to_string())
    }

    pub fn path_str(&self) -> Option<String> {
        match &self.path {
            Some(path) => {
                let s = path.to_str()?;
                Some(s.to_owned())
            }
            None => None,
        }
    }
}

pub struct ManifestGenerator {
    path: PathBuf,
    crates: Vec<Crate>,
}

impl ManifestGenerator {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        ManifestGenerator {
            path: path.as_ref().to_owned(),
            crates: Vec::new(),
        }
    }

    pub fn add_crate(mut self, name: &str) -> Self {
        self.crates.push(Crate {
            name: name.to_string(),
            version: None,
            path: None,
        });
        self
    }

    pub fn add_crate_with_version(mut self, name: &str, version: &str) -> Self {
        self.crates.push(Crate {
            name: name.to_string(),
            version: Some(version.to_string()),
            path: None,
        });
        self
    }

    pub fn add_crate_with_path<P: AsRef<Path>>(mut self, name: &str, path: P) -> Self {
        self.crates.push(Crate {
            name: name.to_string(),
            version: None,
            path: Some(path.as_ref().to_owned()),
        });
        self
    }

    /// Generate Cargo.toml
    pub fn generate(self) -> Result<()> {
        let setting = CargoTOML::from_crates(&self.crates);
        save_str(&self.path, &setting.as_toml(), "Cargo.toml")
            .log(Step::Ready, "Failed to write Cargo.toml")?;
        Ok(())
    }
}
