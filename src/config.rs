use manifest::Crate;
use std::collections::HashMap;
use toml;

#[derive(Serialize)]
pub struct CargoTOML {
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
                let name = c.name();
                let version = c.version();
                let path = c.path_str();
                (name, CrateInfo { version, path })
            })
            .collect();
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
