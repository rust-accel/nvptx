use std::collections::HashMap;
use std::path::*;
use toml;

use super::save_str;
use error::*;

#[derive(Debug, Clone)]
pub struct Crate {
    pub name: String,
    pub version: Option<String>,
    pub path: Option<PathBuf>,
}

impl Crate {
    pub fn latest(name: &str) -> Self {
        Self {
            name: name.into(),
            version: None,
            path: None,
        }
    }

    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.into(),
            version: Some(version.into()),
            path: None,
        }
    }

    pub fn with_path<P: AsRef<Path>>(name: &str, path: P) -> Self {
        Self {
            name: name.into(),
            version: None,
            path: Some(path.as_ref().into()),
        }
    }
}

/// Generate Cargo.toml
pub fn generate<P: AsRef<Path>>(path: P, crates: &[Crate]) -> Result<()> {
    let setting = CargoTOML::from_crates(&crates);
    save_str(&path, &setting.as_toml(), "Cargo.toml")
        .log(Step::Ready, "Failed to write Cargo.toml")?;
    Ok(())
}

#[derive(Serialize)]
struct CargoTOML {
    package: Package,
    profile: Profile,
    dependencies: Dependencies,
}

impl CargoTOML {
    pub fn from_crates(crates: &[Crate]) -> Self {
        let dependencies = crates
            .iter()
            .cloned()
            .map(|c| {
                let name = c.name;
                let version = c.version.unwrap_or("*".to_string());
                let path = c.path.map(|p| p.to_str().unwrap().into());
                (name, CrateInfo { version, path })
            }).collect();
        CargoTOML {
            package: Package::default(),
            profile: Profile::default(),
            dependencies,
        }
    }

    pub fn as_toml(&self) -> String {
        toml::to_string(&self).unwrap()
    }
}

#[derive(Serialize)]
struct Package {
    name: String,
    version: String,
}

impl Default for Package {
    fn default() -> Self {
        Package {
            name: "ptx-builder".to_string(),
            version: "0.1.0".to_string(),
        }
    }
}

#[derive(Serialize)]
struct Profile {
    dev: DevProfile,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            dev: DevProfile::default(),
        }
    }
}

#[derive(Serialize)]
struct DevProfile {
    debug: bool,
}

impl Default for DevProfile {
    fn default() -> Self {
        DevProfile { debug: false }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct CrateInfo {
    pub path: Option<String>,
    pub version: String,
}

type Dependencies = HashMap<String, CrateInfo>;
