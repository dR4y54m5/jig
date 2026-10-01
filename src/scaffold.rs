//! Creating projects and the bench. Every command first builds a plan of
//! actions, so `--dry-run` shows exactly what would happen, and applying a
//! plan never overwrites an existing file unless the action says so.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;

use crate::bench::{Bench, RegistryEntry};
use crate::docs::{self, DocRequest};
use crate::process::{Naming, Process};
use crate::project::{CheckConfig, PROJECT_FILE, Project, ProjectMeta};
use crate::templates;

const HOOK_MARKER: &str = "Installed by jig";
/// The pre-commit hook. It fails closed: a commit that cannot be checked is
/// refused, because a check that silently does not run protects nothing.
const HOOK: &str = r#"#!/bin/sh
# Installed by jig. Blocks a commit when `jig check` reports errors, or when
# jig cannot be found to run the check.
if command -v jig >/dev/null 2>&1; then
  exec jig check --quiet
fi
if [ -x "$HOME/.cargo/bin/jig" ]; then
  exec "$HOME/.cargo/bin/jig" check --quiet
fi
echo "pre-commit: jig was not found, so this commit cannot be checked. Install jig or put it on the PATH." >&2
exit 1
"#;
/// The agent instructions installed at the root of a project repository.
const INSTRUCTIONS_FILE: &str = "CLAUDE.md";
const INSTRUCTIONS_MARKER: &str = "<!-- Installed by `jig setup`";
const INSTRUCTIONS_HEADER: &str = "<!-- Installed by `jig setup` from this project's vault folder, and never committed. Edit the copy there, then run `jig setup` again. -->\n\n";
/// Files jig installs for agents. Git is told to leave them out through the
/// repository's local exclude file, so no tracked file names them.
const AGENT_FILES: [&str; 2] = ["/CLAUDE.md", "/.claude/settings.local.json"];
const VAULT_DIRS: [&str; 8] = [
    "bench",
    "projects",
    "knowledge",
    "ideas",
    "inventory",
    "templates",
    "workspaces",
    "archive",
];
const TEACHING_DIRS: [&str; 4] = ["primers", "labs", "walkthroughs", "retros"];

#[derive(Debug, Serialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
pub enum Action {
    CreateDir {
        path: PathBuf,
    },
    WriteFile {
        path: PathBuf,
        #[serde(skip)]
        contents: String,
        overwrite: bool,
        /// Written read-only, for an installed copy whose source is elsewhere.
        read_only: bool,
    },
    AppendLine {
        path: PathBuf,
        line: String,
    },
    GitInit {
        path: PathBuf,
    },
    Register {
        name: String,
        path: String,
    },
    InstallHook {
        repo: PathBuf,
    },
    /// Lists `lines` in the repository's local exclude file, which git reads
    /// like `.gitignore` and never commits.
    LocalExclude {
        repo: PathBuf,
        lines: Vec<String>,
    },
    /// Installs the project's agent instructions: a read-only copy, at the
    /// repository root, of the master kept in the project's vault folder.
    AgentInstructions {
        repo: PathBuf,
        master: PathBuf,
        /// The text of a master that does not exist yet.
        #[serde(skip)]
        template: String,
        /// Whether a file jig did not install is replaced. Adoption leaves it.
        replace: bool,
    },
    /// Grants agents working in `repo` access to `dir` through the repository's
    /// untracked `.claude/settings.local.json`.
    AgentSettings {
        repo: PathBuf,
        dir: PathBuf,
    },
}

impl Action {
    /// Whether applying the action would change nothing, where that can be
    /// told without applying it.
    fn satisfied(&self) -> bool {
        match self {
            Action::CreateDir { path } => path.is_dir(),
            Action::WriteFile {
                path, overwrite, ..
            } => path.exists() && !overwrite,
            Action::AppendLine { path, line } => std::fs::read_to_string(path)
                .is_ok_and(|text| text.lines().any(|l| l.trim() == line)),
            Action::GitInit { path } => path.join(".git").exists(),
            _ => false,
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct Plan {
    pub actions: Vec<Action>,
    #[serde(skip)]
    registry: Option<crate::bench::Registry>,
}

#[derive(Debug, Serialize)]
pub struct Applied {
    pub done: Vec<String>,
    pub skipped: Vec<String>,
    /// What the engineer has to do by hand, such as adding the check to a
    /// hook that jig does not manage.
    pub notes: Vec<String>,
}

impl Plan {
    fn dir(&mut self, path: PathBuf) {
        self.actions.push(Action::CreateDir { path });
    }

    fn file(&mut self, path: PathBuf, contents: String) {
        self.actions.push(Action::WriteFile {
            path,
            contents,
            overwrite: false,
            read_only: false,
        });
    }

    pub fn describe(&self, bench: &Bench) -> Vec<String> {
        self.actions
            .iter()
            .map(|a| match a {
                Action::CreateDir { path } => format!("create directory {}", bench.relative(path)),
                Action::WriteFile {
                    path, overwrite, ..
                } => {
                    format!(
                        "{} {}",
                        if *overwrite { "write" } else { "create" },
                        bench.relative(path)
                    )
                }
                Action::AppendLine { path, line } => {
                    format!("add `{line}` to {}", bench.relative(path))
                }
                Action::GitInit { path } => format!("git init {}", bench.relative(path)),
                Action::Register { name, .. } => format!("register {name} in vault/registry.toml"),
                Action::InstallHook { repo } => {
                    format!("install the pre-commit hook in {}", bench.relative(repo))
                }
                Action::LocalExclude { repo, lines } => format!(
                    "keep {} out of git in {}",
                    lines.join(" and "),
                    bench.relative(repo)
                ),
                Action::AgentInstructions { repo, master, .. } => format!(
                    "install the agent instructions in {} from {}",
                    bench.relative(repo),
                    bench.relative(master)
                ),
                Action::AgentSettings { repo, dir } => format!(
                    "grant agents in {} access to {}",
                    bench.relative(repo),
                    bench.relative(dir)
                ),
            })
            .collect()
    }

    /// The plan as a dry run shows it: each step, and whether it would change
    /// anything.
    pub fn preview(&self, bench: &Bench) -> Vec<String> {
        self.actions
            .iter()
            .zip(self.describe(bench))
            .map(|(action, text)| {
                if action.satisfied() {
                    format!("nothing to do: {text} (already there)")
                } else {
                    text
                }
            })
            .collect()
    }

    pub fn apply(self, bench: &Bench) -> Result<Applied> {
        let mut applied = Applied {
            done: Vec::new(),
            skipped: Vec::new(),
            notes: Vec::new(),
        };
        for (action, text) in self.actions.iter().zip(self.describe(bench)) {
            let done = match action {
                Action::CreateDir { path } => {
                    if path.is_dir() {
                        false
                    } else {
                        std::fs::create_dir_all(path)
                            .with_context(|| format!("creating {}", path.display()))?;
                        true
                    }
                }
                Action::WriteFile {
                    path,
                    contents,
                    overwrite,
                    read_only,
                } => {
                    let current = std::fs::read_to_string(path).ok();
                    if path.exists() && (!overwrite || current.as_deref() == Some(contents)) {
                        false
                    } else if *read_only {
                        write_read_only(path, contents)?;
                        true
                    } else {
                        if let Some(parent) = path.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        std::fs::write(path, contents)
                            .with_context(|| format!("writing {}", path.display()))?;
                        true
                    }
                }
                Action::AppendLine { path, line } => {
                    let existing = std::fs::read_to_string(path).unwrap_or_default();
                    if existing.lines().any(|l| l.trim() == line) {
                        false
                    } else {
                        let separator = if existing.is_empty() || existing.ends_with('\n') {
                            ""
                        } else {
                            "\n"
                        };
                        std::fs::write(path, format!("{existing}{separator}{line}\n"))?;
                        true
                    }
                }
                Action::GitInit { path } => {
                    if path.join(".git").exists() {
                        false
                    } else {
                        let status = Command::new("git")
                            .args(["init", "-q", "--initial-branch", "main"])
                            .arg(path)
                            .status()
                            .context("running git init")?;
                        if !status.success() {
                            bail!("git init failed in {}", path.display());
                        }
                        true
                    }
                }
                Action::Register { .. } => match &self.registry {
                    Some(registry) => {
                        std::fs::write(bench.registry_path(), Bench::registry_text(registry)?)?;
                        true
                    }
                    None => false,
                },
                Action::InstallHook { repo } => {
                    let (done, note) = install_hook(repo)?;
                    applied.notes.extend(note.map(|reason| {
                        format!(
                            "the pre-commit hook was not installed in {}: {reason}. Add `jig check --quiet` to the pre-commit hook in use.",
                            bench.relative(repo)
                        )
                    }));
                    done
                }
                Action::AgentSettings { repo, dir } => merge_agent_settings(repo, dir)?,
                Action::LocalExclude { repo, lines } => exclude_locally(repo, lines)?,
                Action::AgentInstructions {
                    repo,
                    master,
                    template,
                    replace,
                } => install_instructions(repo, master, template, *replace)?,
            };
            if done {
                applied.done.push(text)
            } else {
                applied.skipped.push(text)
            }
        }
        Ok(applied)
    }
}

/// Installs the pre-commit hook. Returns whether it was written, and the
/// reason when the repository's hooks are not jig's to manage.
fn install_hook(repo: &Path) -> Result<(bool, Option<String>)> {
    let hooks = repo.join(".git").join("hooks");
    if !repo.join(".git").is_dir() {
        return Ok((false, None));
    }
    let hooks_path = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["config", "--get", "core.hooksPath"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|path| !path.is_empty());
    if let Some(path) = hooks_path {
        return Ok((
            false,
            Some(format!(
                "its hooks are managed through core.hooksPath ({path})"
            )),
        ));
    }
    std::fs::create_dir_all(&hooks)?;
    let path = hooks.join("pre-commit");
    match std::fs::read_to_string(&path) {
        Ok(existing) if existing == HOOK => return Ok((false, None)),
        Ok(existing) if !existing.contains(HOOK_MARKER) => {
            return Ok((
                false,
                Some("a pre-commit hook that jig did not install is already there".to_string()),
            ));
        }
        _ => {}
    }
    std::fs::write(&path, HOOK)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok((true, None))
}

/// Writes a file that is an installed copy: read-only, so that an edit fails
/// loudly and is made to the source instead.
fn write_read_only(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if path.exists() {
        // The directory, not the file, decides whether a file can be removed.
        std::fs::remove_file(path).with_context(|| format!("replacing {}", path.display()))?;
    }
    std::fs::write(path, contents).with_context(|| format!("writing {}", path.display()))?;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

/// Adds `lines` to the repository's local exclude file. Nothing is done
/// outside a git repository.
fn exclude_locally(repo: &Path, lines: &[String]) -> Result<bool> {
    if !repo.join(".git").exists() {
        return Ok(false);
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "--git-path", "info/exclude"])
        .output()
        .context("running git rev-parse")?;
    if !output.status.success() {
        return Ok(false);
    }
    let path = repo.join(String::from_utf8_lossy(&output.stdout).trim());
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let missing: Vec<&String> = lines
        .iter()
        .filter(|line| !existing.lines().any(|l| l.trim() == line.as_str()))
        .collect();
    if missing.is_empty() {
        return Ok(false);
    }
    let mut text = existing;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    for line in missing {
        text.push_str(line);
        text.push('\n');
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    Ok(true)
}

/// Installs the agent instructions at the repository root from the master in
/// the project's vault folder. A master that does not exist is created from
/// the repository's own file, or from `template` when there is none.
fn install_instructions(repo: &Path, master: &Path, template: &str, replace: bool) -> Result<bool> {
    let installed = repo.join(INSTRUCTIONS_FILE);
    let existing = std::fs::read_to_string(&installed).ok();
    let mut changed = false;
    if !master.exists() {
        let contents = match &existing {
            Some(text) => text.strip_prefix(INSTRUCTIONS_HEADER).unwrap_or(text),
            None => template,
        };
        if let Some(parent) = master.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(master, contents)
            .with_context(|| format!("writing {}", master.display()))?;
        changed = true;
    }
    let wanted = format!(
        "{INSTRUCTIONS_HEADER}{}",
        std::fs::read_to_string(master).with_context(|| format!("reading {}", master.display()))?
    );
    let foreign = existing
        .as_deref()
        .is_some_and(|text| !text.starts_with(INSTRUCTIONS_MARKER));
    if existing.as_deref() != Some(wanted.as_str()) && (replace || !foreign) {
        write_read_only(&installed, &wanted)?;
        changed = true;
    }
    Ok(changed)
}

/// Adds `dir` to `permissions.additionalDirectories` in the repository's
/// `.claude/settings.local.json`, keeping every other setting.
fn merge_agent_settings(repo: &Path, dir: &Path) -> Result<bool> {
    let path = repo.join(".claude").join("settings.local.json");
    let mut value: serde_json::Value = match std::fs::read_to_string(&path) {
        Ok(text) => {
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?
        }
        Err(_) => serde_json::json!({}),
    };
    let not_object = || anyhow!("{} does not hold the expected JSON objects", path.display());
    let dir = dir.to_string_lossy().into_owned();
    let dirs = value
        .as_object_mut()
        .ok_or_else(not_object)?
        .entry("permissions")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(not_object)?
        .entry("additionalDirectories")
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()
        .ok_or_else(not_object)?;
    if dirs.iter().any(|d| d.as_str() == Some(dir.as_str())) {
        return Ok(false);
    }
    dirs.push(serde_json::Value::String(dir));
    std::fs::create_dir_all(path.parent().expect("settings path has a parent"))?;
    std::fs::write(&path, serde_json::to_string_pretty(&value)? + "\n")?;
    Ok(true)
}

/// What agents and the commit hook need in a project repository: the agent
/// instructions installed from the vault, the local settings that grant
/// access to the project's vault folder, and the pre-commit hook. None of
/// them is tracked by git, and no tracked file names them.
fn agent_actions(plan: &mut Plan, bench: &Bench, root: &Path, meta: &ProjectMeta, replace: bool) {
    let template = templates::template(kind_template("CLAUDE", &meta.kind))
        .expect("CLAUDE.md template is embedded");
    if bench.vault_dir().is_dir() {
        plan.actions.push(Action::AgentInstructions {
            repo: root.to_path_buf(),
            master: bench.vault_project_dir(&meta.name).join(INSTRUCTIONS_FILE),
            template: templates::fill(
                template,
                &[
                    ("name", &meta.name),
                    ("code", &meta.code),
                    ("title", &meta.title),
                    ("layout", layout(&meta.kind)),
                ],
            ),
            replace,
        });
    }
    plan.actions.push(Action::AgentSettings {
        repo: root.to_path_buf(),
        dir: bench.vault_project_dir(&meta.name),
    });
    plan.actions.push(Action::LocalExclude {
        repo: root.to_path_buf(),
        lines: AGENT_FILES.iter().map(|line| line.to_string()).collect(),
    });
    plan.actions.push(Action::InstallHook {
        repo: root.to_path_buf(),
    });
}

/// Installs what an existing project needs on this machine: the hook, the
/// local agent settings, and the agent instructions if they are missing.
pub fn setup_plan(bench: &Bench, project: &Project) -> Plan {
    let mut plan = Plan::default();
    agent_actions(&mut plan, bench, &project.root, &project.meta, true);
    plan
}

/// Installs only the pre-commit hook, for a repository without `project.toml`.
pub fn hook_plan(repo: &Path) -> Plan {
    Plan {
        actions: vec![Action::InstallHook {
            repo: repo.to_path_buf(),
        }],
        registry: None,
    }
}

/// Creates the bench directories and an empty vault.
pub fn init_plan(bench: &Bench) -> Plan {
    let mut plan = Plan::default();
    plan.dir(bench.projects_dir());
    plan.dir(bench.refs_dir());
    for dir in VAULT_DIRS {
        plan.dir(bench.vault_dir().join(dir));
    }
    plan.file(
        bench.registry_path(),
        Bench::registry_text(&Default::default()).unwrap_or_default(),
    );
    plan.file(
        bench.vault_dir().join("README.md"),
        VAULT_README.to_string(),
    );
    plan.file(bench.config_path(), VAULT_CONFIG.to_string());
    plan.actions.push(Action::GitInit {
        path: bench.vault_dir(),
    });
    // The bench-wide agent guide is versioned in the vault and installed at the
    // bench root, where every session under the bench loads it.
    if let Ok(guide) = std::fs::read_to_string(bench.vault_dir().join("bench").join("CLAUDE.md")) {
        plan.actions.push(Action::WriteFile {
            path: bench.root.join("CLAUDE.md"),
            contents: format!("{BENCH_GUIDE_HEADER}{guide}"),
            overwrite: true,
            read_only: true,
        });
    }
    plan
}

const BENCH_GUIDE_HEADER: &str = "<!-- Installed by `jig init` from vault/bench/CLAUDE.md. Edit that file, then run `jig init` again. -->\n\n";

const VAULT_CONFIG: &str = "# Settings for jig on this bench.\n\n# Strings that must never appear in a project repository, such as the names\n# of private repositories. `jig check` reports any mention as an error.\nprivate_terms = []\n";

const VAULT_README: &str = "# Vault\n\nPrivate companion to the project repositories on this bench. Everything written to build skills lives here; engineering documents live in each project's repository.\n\n| Directory | Contents |\n|---|---|\n| `projects/<name>/` | Roadmap, concept primers, labs, walkthroughs and retrospectives for one project, and the project's agent instructions |\n| `knowledge/` | Notes and lessons that span projects |\n| `ideas/` | One file per idea not yet started as a project |\n| `inventory/` | Parts, tools and suppliers |\n| `bench/` | Guides for agents and the bench's state; `bench/CLAUDE.md` is installed at the bench root by `jig init` |\n| `templates/` | Templates for teaching documents |\n| `workspaces/` | VS Code workspaces pairing each project with its vault folder |\n| `archive/` | Material kept for reference only |\n\n`registry.toml` lists every project, so `jig sync` can clone them onto a new machine.\n";

pub struct Setup {
    pub name: String,
    pub code: String,
    pub title: String,
    pub kind: String,
    pub tier: Option<String>,
    pub visibility: String,
    pub phase: Option<String>,
    pub remote: String,
    /// Paths `jig check` skips in this project, relative to its root.
    pub exclude: Vec<String>,
    /// An existing repository to adopt instead of creating a new one.
    pub existing: Option<PathBuf>,
}

fn validate(setup: &Setup) -> Result<()> {
    let name_ok = setup
        .name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && setup
            .name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !name_ok {
        bail!(
            "project name `{}` must be lowercase letters, digits and hyphens",
            setup.name
        );
    }
    let code_ok = (2..=6).contains(&setup.code.len())
        && setup
            .code
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase())
        && setup
            .code
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    if !code_ok {
        bail!(
            "project code `{}` must be 2-6 uppercase letters or digits, starting with a letter",
            setup.code
        );
    }
    if !["public", "private"].contains(&setup.visibility.as_str()) {
        bail!("visibility must be public or private");
    }
    Ok(())
}

/// The project template for a kind. An exercise has no documents and no
/// gates, so its README and agent instructions say less.
fn kind_template(name: &str, kind: &str) -> &'static str {
    match (name, kind) {
        ("README", "exercise") => "project/README-exercise.md",
        ("README", _) => "project/README.md",
        (_, "exercise") => "project/CLAUDE-exercise.md",
        _ => "project/CLAUDE.md",
    }
}

fn layout(kind: &str) -> &'static str {
    match kind {
        "product" => {
            "| Path | Contents |\n|---|---|\n| `docs/` | Engineering documents |\n| `firmware/` | Device firmware |\n| `hardware/` | Electronics design (KiCad) |\n| `mechanical/` | Enclosure and mechanical parts (build123d) |"
        }
        "re" => {
            "| Path | Contents |\n|---|---|\n| `docs/` | Engineering documents and findings |\n| `captures/` | Measurements, captures and photos |"
        }
        _ => "| Path | Contents |\n|---|---|\n| `docs/` | Engineering documents |",
    }
}

/// The plan that creates or adopts a project, and the project as it will be.
pub fn project_plan(
    bench: &Bench,
    setup: &Setup,
    date: &str,
    author: &str,
) -> Result<(Plan, Project)> {
    validate(setup)?;
    bench.require_vault()?;
    let profile = Process::get().profile(&setup.kind).ok_or_else(|| {
        anyhow!(
            "unknown kind `{}`; expected one of {}",
            setup.kind,
            Process::get().profile_names().join(", ")
        )
    })?;
    match (&setup.tier, profile.tiers.is_empty()) {
        (Some(tier), false) if !profile.tiers.contains(tier) => {
            bail!("tier must be one of {}", profile.tiers.join(", "))
        }
        (None, false) => bail!(
            "a {} project needs --tier ({})",
            setup.kind,
            profile.tiers.join(", ")
        ),
        (Some(_), true) => bail!("{} projects have no tiers", setup.kind),
        _ => {}
    }
    let start = setup
        .phase
        .clone()
        .unwrap_or_else(|| profile.phases[0].id.clone());
    let phases = profile.phases_for(setup.tier.as_deref());
    let upto = phases
        .iter()
        .position(|p| p.id.eq_ignore_ascii_case(&start))
        .ok_or_else(|| anyhow!("phase `{start}` does not exist for this kind and tier"))?;

    let root = match &setup.existing {
        Some(path) => {
            let root = path
                .canonicalize()
                .with_context(|| format!("{} does not exist", path.display()))?;
            if root.join(PROJECT_FILE).exists() {
                bail!("{} is already a jig project", root.display());
            }
            root
        }
        None => {
            let root = bench.projects_dir().join(&setup.name);
            if root.exists() {
                bail!(
                    "{} already exists; use `jig adopt` for an existing repository",
                    root.display()
                );
            }
            root
        }
    };

    let meta = ProjectMeta {
        name: setup.name.clone(),
        code: setup.code.clone(),
        title: setup.title.clone(),
        kind: setup.kind.clone(),
        tier: setup.tier.clone(),
        visibility: setup.visibility.clone(),
        phase: phases[upto].id.clone(),
        status: "active".into(),
    };
    let project = Project {
        root: root.clone(),
        meta: meta.clone(),
        check: CheckConfig {
            exclude: setup.exclude.clone(),
        },
    };
    let mut plan = Plan::default();

    if setup.existing.is_none() {
        plan.dir(root.clone());
        plan.actions.push(Action::GitInit { path: root.clone() });
    }
    plan.file(root.join(PROJECT_FILE), project.to_toml()?);
    let readme = templates::template(kind_template("README", &setup.kind))
        .expect("README template is embedded");
    plan.file(
        root.join("README.md"),
        templates::fill(
            readme,
            &[
                ("name", &setup.name),
                ("title", &setup.title),
                ("layout", layout(&setup.kind)),
            ],
        ),
    );
    plan.actions.push(Action::AppendLine {
        path: root.join(".gitignore"),
        line: "/build/".into(),
    });
    if setup.existing.is_none() {
        let dirs: &[&str] = match setup.kind.as_str() {
            "product" => &["firmware", "hardware", "mechanical"],
            "re" => &["captures"],
            _ => &[],
        };
        for dir in dirs {
            let text = templates::template(&format!("project/{dir}.md"))
                .expect("directory README is embedded");
            plan.file(root.join(dir).join("README.md"), text.to_string());
        }
    }

    // Draft every single-instance document the phases up to the starting one require.
    let existing = docs::load_all(&root).unwrap_or_default();
    let mut drafted: Vec<String> = Vec::new();
    for phase in &phases[..=upto] {
        for req in profile.requirements(phase, setup.tier.as_deref()) {
            let kind = Process::get()
                .kind(&req.kind)
                .expect("profiles reference known kinds");
            if kind.naming != Naming::Single
                || drafted.contains(&kind.key)
                || root.join(&kind.path).exists()
            {
                continue;
            }
            let draft = docs::draft(
                &project,
                &existing,
                DocRequest {
                    kind,
                    title: None,
                    gate: None,
                    date: date.into(),
                    author: author.into(),
                    vars: Vec::new(),
                },
            )?;
            plan.file(draft.path, draft.text);
            drafted.push(kind.key.clone());
        }
    }

    let vault = bench.vault_project_dir(&setup.name);
    for dir in TEACHING_DIRS {
        plan.file(vault.join(dir).join(".gitkeep"), String::new());
    }
    plan.file(
        vault.join("roadmap.md"),
        roadmap(
            bench,
            setup,
            &phases.iter().map(|p| p.label()).collect::<Vec<_>>(),
        ),
    );
    plan.actions.push(Action::WriteFile {
        path: bench.workspace_path(&setup.name),
        contents: workspace(bench, &setup.name, &root)?,
        overwrite: true,
        read_only: false,
    });

    let entry = RegistryEntry {
        name: setup.name.clone(),
        path: bench.relative(&root),
        remote: setup.remote.clone(),
    };
    if let Some(registry) = bench.with_entry(entry.clone())? {
        plan.registry = Some(registry);
        plan.actions.push(Action::Register {
            name: entry.name,
            path: entry.path,
        });
    }
    // Adoption leaves the repository's own agent instructions as they are.
    agent_actions(&mut plan, bench, &root, &meta, setup.existing.is_none());
    Ok((plan, project))
}

fn roadmap(bench: &Bench, setup: &Setup, phases: &[String]) -> String {
    let template = std::fs::read_to_string(bench.vault_dir().join("templates").join("roadmap.md"))
        .unwrap_or_else(|_| "# {{title}}: roadmap\n\n{{phases}}\n".to_string());
    let phases: String = phases.iter().map(|p| format!("- [ ] {p}\n")).collect();
    templates::fill(
        &template,
        &[
            ("title", &setup.title),
            ("name", &setup.name),
            ("phases", phases.trim_end()),
        ],
    )
}

/// A VS Code workspace pairing the project repository with its vault folder.
fn workspace(bench: &Bench, name: &str, root: &Path) -> Result<String> {
    let from = bench
        .workspace_path(name)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let project_path = relative_path(&from, root);
    let vault_path = relative_path(&from, &bench.vault_project_dir(name));
    let value = serde_json::json!({
        "folders": [
            { "name": name, "path": project_path },
            { "name": format!("{name} (vault)"), "path": vault_path },
        ],
        "settings": {
            "workbench.editorAssociations": { "*.md": "vscode.markdown.preview.editor" },
            "workbench.editor.enablePreview": false,
        },
    });
    Ok(serde_json::to_string_pretty(&value)? + "\n")
}

/// `to` expressed relative to the directory `from`.
fn relative_path(from: &Path, to: &Path) -> String {
    let from: Vec<_> = from.components().collect();
    let to_components: Vec<_> = to.components().collect();
    let common = from
        .iter()
        .zip(&to_components)
        .take_while(|(a, b)| a == b)
        .count();
    if common == 0 {
        return to.to_string_lossy().into_owned();
    }
    let mut parts: Vec<String> = vec!["..".to_string(); from.len() - common];
    parts.extend(
        to_components[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_climb_to_the_common_ancestor() {
        assert_eq!(
            relative_path(
                Path::new("/b/vault/workspaces"),
                Path::new("/b/projects/em4")
            ),
            "../../projects/em4"
        );
        assert_eq!(
            relative_path(
                Path::new("/b/vault/workspaces"),
                Path::new("/b/vault/projects/em4")
            ),
            "../projects/em4"
        );
    }

    #[test]
    fn names_and_codes_are_validated() {
        let setup = |name: &str, code: &str| Setup {
            name: name.into(),
            code: code.into(),
            title: "t".into(),
            kind: "software".into(),
            tier: None,
            visibility: "public".into(),
            phase: None,
            remote: String::new(),
            exclude: Vec::new(),
            existing: None,
        };
        assert!(validate(&setup("air-remote-2", "AR2")).is_ok());
        assert!(validate(&setup("Air", "AR2")).is_err());
        assert!(validate(&setup("air", "ar2")).is_err());
        assert!(validate(&setup("air", "A")).is_err());
    }
}
