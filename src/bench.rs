//! The bench: the root directory holding every project repository, the
//! private vault and third-party reference repositories.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

pub struct Bench {
    pub root: PathBuf,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Registry {
    #[serde(default, rename = "project")]
    pub projects: Vec<RegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    /// Path relative to the bench root.
    pub path: String,
    #[serde(default)]
    pub remote: String,
}

const REGISTRY_HEADER: &str =
    "# Projects on this bench. `jig sync` clones any that are missing locally.\n\n";

impl Bench {
    /// The bench named by `--bench`, else `JIG_BENCH`, else `~/bench`.
    pub fn locate(explicit: Option<&Path>) -> Result<Bench> {
        let root = match explicit {
            Some(path) => path.to_path_buf(),
            None => match std::env::var_os("JIG_BENCH") {
                Some(path) => PathBuf::from(path),
                None => {
                    let home = std::env::var_os("HOME").context("HOME is not set")?;
                    PathBuf::from(home).join("bench")
                }
            },
        };
        Ok(Bench { root })
    }

    pub fn projects_dir(&self) -> PathBuf {
        self.root.join("projects")
    }

    pub fn vault_dir(&self) -> PathBuf {
        self.root.join("vault")
    }

    pub fn refs_dir(&self) -> PathBuf {
        self.root.join("refs")
    }

    pub fn registry_path(&self) -> PathBuf {
        self.vault_dir().join("registry.toml")
    }

    pub fn vault_project_dir(&self, name: &str) -> PathBuf {
        self.vault_dir().join("projects").join(name)
    }

    pub fn workspace_path(&self, name: &str) -> PathBuf {
        self.vault_dir()
            .join("workspaces")
            .join(format!("{name}.code-workspace"))
    }

    pub fn config_path(&self) -> PathBuf {
        self.vault_dir().join("jig.toml")
    }

    /// Terms from the vault's `jig.toml` that must never appear in a project
    /// repository, such as the names of private repositories.
    pub fn private_terms(&self) -> Result<Vec<String>> {
        #[derive(Deserialize, Default)]
        struct Config {
            #[serde(default)]
            private_terms: Vec<String>,
        }
        let path = self.config_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let config: Config =
            toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        Ok(config.private_terms)
    }

    pub fn require_vault(&self) -> Result<()> {
        if !self.vault_dir().is_dir() {
            bail!(
                "no vault at {}. Clone your vault there, or run `jig init` to create an empty one",
                self.vault_dir().display()
            );
        }
        Ok(())
    }

    pub fn registry(&self) -> Result<Registry> {
        let path = self.registry_path();
        if !path.exists() {
            return Ok(Registry::default());
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn registry_text(registry: &Registry) -> Result<String> {
        Ok(format!("{REGISTRY_HEADER}{}", toml::to_string(registry)?))
    }

    /// The registry with `entry` added, or `None` if a project with that name is already registered.
    pub fn with_entry(&self, entry: RegistryEntry) -> Result<Option<Registry>> {
        let mut registry = self.registry()?;
        if registry.projects.iter().any(|p| p.name == entry.name) {
            return Ok(None);
        }
        registry.projects.push(entry);
        registry.projects.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Some(registry))
    }

    /// A path inside the bench, relative to its root, with forward slashes.
    pub fn relative(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }
}
