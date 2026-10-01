//! A project repository and its `project.toml`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};

use crate::process::{Phase, Process, Profile};

pub const PROJECT_FILE: &str = "project.toml";

const HEADER: &str = "# Managed by jig. Change `phase` with `jig gate close`, not by hand.\n# The paths under `[check] exclude` are skipped by `jig check`; edit that list by hand.\n\n";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    pub project: ProjectMeta,
    #[serde(default, skip_serializing_if = "CheckConfig::is_empty")]
    pub check: CheckConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub name: String,
    pub code: String,
    pub title: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    pub visibility: String,
    pub phase: String,
    #[serde(default = "active")]
    pub status: String,
}

fn active() -> String {
    "active".to_string()
}

/// Settings for `jig check` that the engineer edits by hand.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckConfig {
    /// Paths relative to the repository root that `jig check` skips, such as
    /// vendored third-party code.
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl CheckConfig {
    fn is_empty(&self) -> bool {
        self.exclude.is_empty()
    }

    /// Whether `rel`, a path relative to the repository root, is excluded
    /// itself or lies under an excluded directory.
    pub fn excludes(&self, rel: &str) -> bool {
        self.exclude.iter().any(|entry| {
            let entry = entry.trim_start_matches("./").trim_end_matches('/');
            !entry.is_empty()
                && (rel == entry
                    || rel
                        .strip_prefix(entry)
                        .is_some_and(|rest| rest.starts_with('/')))
        })
    }
}

#[derive(Debug, Clone)]
pub struct Project {
    pub root: PathBuf,
    pub meta: ProjectMeta,
    pub check: CheckConfig,
}

impl Project {
    /// The project containing `start`: the nearest ancestor with a `project.toml`.
    pub fn discover(start: &Path) -> Result<Project> {
        let start = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
        for dir in start.ancestors() {
            if dir.join(PROJECT_FILE).is_file() {
                return Project::load(dir);
            }
        }
        bail!(
            "not inside a jig project (no {PROJECT_FILE} in {} or its parents)",
            start.display()
        )
    }

    pub fn load(root: &Path) -> Result<Project> {
        let path = root.join(PROJECT_FILE);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let file: ProjectFile =
            toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        let project = Project {
            root: root.to_path_buf(),
            meta: file.project,
            check: file.check,
        };
        project.validate()?;
        Ok(project)
    }

    fn validate(&self) -> Result<()> {
        let profile = Process::get().profile(&self.meta.kind).ok_or_else(|| {
            anyhow!(
                "{PROJECT_FILE}: unknown kind `{}` (expected one of {})",
                self.meta.kind,
                Process::get().profile_names().join(", ")
            )
        })?;
        if profile.phase(&self.meta.phase).is_none() {
            bail!(
                "{PROJECT_FILE}: phase `{}` does not exist for kind `{}`",
                self.meta.phase,
                self.meta.kind
            );
        }
        if !profile.tiers.is_empty() {
            match &self.meta.tier {
                Some(tier) if profile.tiers.contains(tier) => {}
                Some(tier) => bail!(
                    "{PROJECT_FILE}: unknown tier `{tier}` (expected one of {})",
                    profile.tiers.join(", ")
                ),
                None => bail!(
                    "{PROJECT_FILE}: kind `{}` needs a tier ({})",
                    self.meta.kind,
                    profile.tiers.join(", ")
                ),
            }
        }
        Ok(())
    }

    /// The text of `project.toml`.
    pub fn to_toml(&self) -> Result<String> {
        let file = ProjectFile {
            project: self.meta.clone(),
            check: self.check.clone(),
        };
        Ok(format!("{HEADER}{}", toml::to_string(&file)?))
    }

    pub fn save(&self) -> Result<()> {
        let path = self.root.join(PROJECT_FILE);
        std::fs::write(&path, self.to_toml()?)
            .with_context(|| format!("writing {}", path.display()))
    }

    pub fn profile(&self) -> &'static Profile {
        Process::get()
            .profile(&self.meta.kind)
            .expect("validated on load")
    }

    pub fn tier(&self) -> Option<&str> {
        self.meta.tier.as_deref()
    }

    pub fn phase(&self) -> &'static Phase {
        self.profile()
            .phase(&self.meta.phase)
            .expect("validated on load")
    }

    /// The title used on rendered documents, e.g. "EM4 · Enigma M4 rotor machine".
    pub fn display_title(&self) -> String {
        format!("{} · {}", self.meta.code, self.meta.title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(exclude: &[&str]) -> Project {
        Project {
            root: PathBuf::from("/tmp/demo"),
            meta: ProjectMeta {
                name: "demo".into(),
                code: "DM".into(),
                title: "Demo".into(),
                kind: "software".into(),
                tier: None,
                visibility: "public".into(),
                phase: "P0".into(),
                status: "active".into(),
            },
            check: CheckConfig {
                exclude: exclude.iter().map(|e| e.to_string()).collect(),
            },
        }
    }

    #[test]
    fn exclusions_survive_a_rewrite_of_the_file() {
        let text = project(&["vendor", "third_party/lib"]).to_toml().unwrap();
        let file: ProjectFile = toml::from_str(&text).unwrap();
        assert_eq!(file.check.exclude, ["vendor", "third_party/lib"]);
        assert!(
            !project(&[])
                .to_toml()
                .unwrap()
                .lines()
                .any(|line| line == "[check]"),
            "an empty list is not written"
        );
    }

    #[test]
    fn exclusions_cover_a_path_and_everything_under_it() {
        let check = project(&["vendor/", "./notes.md"]).check;
        assert!(check.excludes("vendor/lib/README.md"));
        assert!(check.excludes("notes.md"));
        assert!(!check.excludes("vendored/README.md"));
        assert!(!check.excludes("docs/notes.md"));
    }
}
