//! End-to-end tests that drive the `jig` binary against a temporary bench.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;

struct Bench {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Bench {
    fn new() -> Bench {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let bench = Bench { _dir: dir, root };
        bench.jig(&bench.root).arg("init").assert().success();
        bench
    }

    fn jig(&self, cwd: &Path) -> Command {
        let mut cmd = Command::cargo_bin("jig").unwrap();
        cmd.env("JIG_BENCH", &self.root).current_dir(cwd);
        cmd
    }

    fn project(&self, name: &str, args: &[&str]) -> PathBuf {
        let mut cmd = self.jig(&self.root);
        cmd.args(["new", name]).args(args).assert().success();
        self.root.join("projects").join(name)
    }
}

fn strip_guidance(path: &Path) {
    let text = fs::read_to_string(path).unwrap();
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some(start) = rest.find("<!-- guide:") {
        out.push_str(&rest[..start]);
        let end = rest[start..].find("-->").unwrap() + start + 3;
        rest = &rest[end..];
    }
    out.push_str(rest);
    fs::write(path, out).unwrap();
}

const PRODUCT: [&str; 8] = [
    "--kind",
    "product",
    "--tier",
    "desk",
    "--code",
    "DM",
    "--title",
    "Demo device",
];

#[test]
fn a_new_product_walks_through_its_first_gate() {
    let bench = Bench::new();
    let root = bench.project("demo", &PRODUCT);
    for file in [
        "project.toml",
        "README.md",
        "docs/plan.md",
        "docs/concept.md",
        "docs/risks.md",
        "hardware/README.md",
    ] {
        assert!(root.join(file).exists(), "{file} was not created");
    }
    assert!(bench.root.join("vault/projects/demo/roadmap.md").exists());
    assert!(
        bench
            .root
            .join("vault/workspaces/demo.code-workspace")
            .exists()
    );
    assert!(root.join(".git/hooks/pre-commit").exists());

    bench.jig(&root).arg("check").assert().success();
    bench.jig(&root).args(["gate", "check"]).assert().code(1);

    for doc in ["docs/plan.md", "docs/concept.md", "docs/risks.md"] {
        strip_guidance(&root.join(doc));
    }
    bench
        .jig(&root)
        .args(["doc", "release", "PLN", "--note", "Baseline"])
        .assert()
        .success();
    bench
        .jig(&root)
        .args(["doc", "release", "DM-CON", "--note", "Baseline"])
        .assert()
        .success();
    bench.jig(&root).args(["gate", "open"]).assert().success();

    let record = root.join("docs/reviews/GR-CR.md");
    bench
        .jig(&root)
        .args(["gate", "close", "CR", "--outcome", "go"])
        .assert()
        .code(2);
    strip_guidance(&record);
    fs::write(
        &record,
        fs::read_to_string(&record)
            .unwrap()
            .replace("- [ ]", "- [x]"),
    )
    .unwrap();
    bench
        .jig(&root)
        .args(["gate", "close", "CR", "--outcome", "go"])
        .assert()
        .success();

    let config = fs::read_to_string(root.join("project.toml")).unwrap();
    assert!(config.contains("phase = \"P1\""));
    assert!(
        root.join("docs/requirements.md").exists(),
        "the SRS is drafted on entering Definition"
    );
    let record = fs::read_to_string(&record).unwrap();
    assert!(record.contains("status: released"));
    assert!(record.contains("**Outcome:** Go"));
}

#[test]
fn check_rejects_teaching_material_and_private_references() {
    let bench = Bench::new();
    let root = bench.project("demo", &PRODUCT);
    fs::write(
        bench.root.join("vault/jig.toml"),
        "private_terms = [\"secret-notes\"]\n",
    )
    .unwrap();
    let risks = root.join("docs/risks.md");
    let mut text = fs::read_to_string(&risks).unwrap();
    text.push_str("\nNow let's measure it. See secret-notes and [x](../../../vault/x.md).\n");
    fs::write(&risks, text).unwrap();

    let output = bench
        .jig(&root)
        .args(["check", "--json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let findings: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let rules: Vec<&str> = findings
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule"].as_str().unwrap())
        .collect();
    assert!(rules.contains(&"separation.teaching"));
    assert_eq!(
        rules
            .iter()
            .filter(|r| **r == "separation.private-reference")
            .count(),
        2
    );
}

#[test]
fn quoted_phrases_in_code_spans_are_not_teaching() {
    let bench = Bench::new();
    let root = bench.project("demo", &PRODUCT);
    let plan = root.join("docs/plan.md");
    let mut text = fs::read_to_string(&plan).unwrap();
    text.push_str("\nThe lint rejects phrases such as `let's` in project documents.\n");
    fs::write(&plan, text).unwrap();
    bench
        .jig(&root)
        .args(["check", "--strict"])
        .assert()
        .code(1); // template guidance warnings remain
    let output = bench
        .jig(&root)
        .args(["check", "--json"])
        .output()
        .unwrap()
        .stdout;
    assert!(!String::from_utf8_lossy(&output).contains("separation.teaching"));
}

#[test]
fn release_is_refused_while_template_guidance_remains() {
    let bench = Bench::new();
    let root = bench.project("demo", &PRODUCT);
    bench
        .jig(&root)
        .args(["doc", "release", "CON", "--note", "Too early"])
        .assert()
        .code(2);
    assert!(
        fs::read_to_string(root.join("docs/concept.md"))
            .unwrap()
            .contains("status: draft")
    );
}

#[test]
fn numbered_documents_get_sequential_ids() {
    let bench = Bench::new();
    let root = bench.project(
        "demo",
        &["--kind", "software", "--code", "DM", "--title", "Demo tool"],
    );
    bench
        .jig(&root)
        .args(["doc", "new", "adr", "--title", "Use Rust"])
        .assert()
        .success();
    bench
        .jig(&root)
        .args(["doc", "new", "ADR", "--title", "Use TOML for settings"])
        .assert()
        .success();
    assert!(root.join("docs/decisions/ADR-001-use-rust.md").exists());
    assert!(
        root.join("docs/decisions/ADR-002-use-toml-for-settings.md")
            .exists()
    );
    bench
        .jig(&root)
        .args(["doc", "new", "adr"])
        .assert()
        .code(2);
}

#[test]
fn adopting_an_existing_repository_keeps_its_files() {
    let bench = Bench::new();
    let repo = bench.root.join("elsewhere/tool");
    fs::create_dir_all(&repo).unwrap();
    fs::write(repo.join("README.md"), "# Existing\n").unwrap();
    bench
        .jig(&bench.root)
        .arg("adopt")
        .arg(&repo)
        .args([
            "--kind", "software", "--code", "TL", "--title", "Tool", "--phase", "P1",
        ])
        .assert()
        .success();
    assert_eq!(
        fs::read_to_string(repo.join("README.md")).unwrap(),
        "# Existing\n"
    );
    assert!(
        repo.join("docs/concept.md").exists(),
        "P0 documents are drafted when starting in P1"
    );
    assert!(repo.join("docs/requirements.md").exists());
    let config = fs::read_to_string(repo.join("project.toml")).unwrap();
    assert!(config.contains("phase = \"P1\""));
}

#[test]
fn dry_run_creates_nothing() {
    let bench = Bench::new();
    bench
        .jig(&bench.root)
        .args(["new", "demo"])
        .args(PRODUCT)
        .arg("--dry-run")
        .assert()
        .success();
    assert!(!bench.root.join("projects/demo").exists());
}

#[test]
fn check_requires_document_control() {
    let bench = Bench::new();
    let root = bench.project("demo", &PRODUCT);
    let risks = root.join("docs/risks.md");
    let text = fs::read_to_string(&risks)
        .unwrap()
        .replace("status: draft\n", "")
        .replace("revision: A\n", "revision: I\n");
    fs::write(&risks, text).unwrap();
    fs::write(root.join("docs/notes.md"), "# Loose notes\n").unwrap();

    let output = bench
        .jig(&root)
        .args(["check", "--json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let findings: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let rules: Vec<&str> = findings
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule"].as_str().unwrap())
        .collect();
    assert!(
        rules.contains(&"front-matter.required"),
        "missing status is reported"
    );
    assert!(
        rules.contains(&"revision.invalid"),
        "revision I is not a Y14.35 letter"
    );
    assert!(
        rules.contains(&"front-matter.missing"),
        "a document without front matter is reported"
    );
}

#[test]
fn check_enforces_requirement_rules() {
    let bench = Bench::new();
    let root = bench.project(
        "demo",
        &[
            "--kind",
            "software",
            "--code",
            "DM",
            "--title",
            "Demo tool",
            "--phase",
            "P1",
        ],
    );
    let srs = root.join("docs/requirements.md");
    let text = fs::read_to_string(&srs).unwrap().replace(
        "The system shall respond.\n\n- **Verification:** Test\n",
        "The system needs to respond quickly.\n\n",
    );
    fs::write(&srs, text).unwrap();
    let vvp = root.join("docs/vv-plan.md");
    fs::write(
        &vvp,
        fs::read_to_string(&vvp).unwrap().replace(
            "- **Verifies:** REQ-001",
            "- **Verifies:** REQ-001, REQ-042",
        ),
    )
    .unwrap();

    let output = bench
        .jig(&root)
        .args(["check", "--json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let findings: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let rules: Vec<&str> = findings
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule"].as_str().unwrap())
        .collect();
    for rule in [
        "requirement.no-shall",
        "requirement.verification",
        "requirement.vague",
        "test.unknown-requirement",
    ] {
        assert!(rules.contains(&rule), "{rule} is reported");
    }
}

fn tools_available() -> bool {
    ["pandoc", "typst"].iter().all(|tool| {
        std::process::Command::new(tool)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    })
}

#[test]
fn documents_and_gate_packages_render_to_pdf() {
    if !tools_available() {
        eprintln!("skipped: pandoc and typst are not installed");
        return;
    }
    let bench = Bench::new();
    let root = bench.project("demo", &PRODUCT);
    bench
        .jig(&root)
        .args(["doc", "pdf", "CON"])
        .assert()
        .success();
    bench
        .jig(&root)
        .args(["doc", "pack", "CR"])
        .assert()
        .success();
    for pdf in ["build/pdf/DM-CON.pdf", "build/pdf/DM-CR-pack.pdf"] {
        let bytes = fs::read(root.join(pdf)).unwrap();
        assert!(bytes.starts_with(b"%PDF"), "{pdf} is a PDF");
    }
}

#[test]
fn sync_clones_missing_projects_from_their_remotes() {
    let bench = Bench::new();
    let source = bench.root.join("remote/demo");
    fs::create_dir_all(&source).unwrap();
    let git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(&source)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.com")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.com")
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    };
    git(&["init", "-q"]);
    fs::write(source.join("README.md"), "# Demo\n").unwrap();
    git(&["add", "."]);
    git(&["-c", "commit.gpgsign=false", "commit", "-q", "-m", "init"]);
    let registry = format!(
        "[[project]]\nname = \"demo\"\npath = \"projects/demo\"\nremote = \"{}\"\n",
        source.display()
    );
    fs::write(bench.root.join("vault/registry.toml"), registry).unwrap();

    bench.jig(&bench.root).arg("sync").assert().success();
    assert!(bench.root.join("projects/demo/README.md").exists());
}

fn parse_json(bytes: &[u8], what: &str) -> serde_json::Value {
    serde_json::from_slice(bytes).unwrap_or_else(|e| {
        panic!(
            "`{what}` did not print JSON ({e}): {}",
            String::from_utf8_lossy(bytes)
        )
    })
}

#[test]
fn every_command_prints_json() {
    let bench = Bench::new();
    let run = |cwd: &Path, args: &[&str]| {
        let output = bench.jig(cwd).args(args).arg("--json").output().unwrap();
        assert!(
            output.status.code().is_some_and(|c| c < 2),
            "`{}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        parse_json(&output.stdout, &args.join(" "))
    };
    run(&bench.root, &["init"]);
    run(
        &bench.root,
        &[
            "new",
            "demo",
            "--kind",
            "product",
            "--tier",
            "desk",
            "--code",
            "DM",
            "--title",
            "Demo device",
        ],
    );
    let root = bench.root.join("projects/demo");
    for doc in ["docs/plan.md", "docs/concept.md", "docs/risks.md"] {
        strip_guidance(&root.join(doc));
    }
    for args in [
        &["status"][..],
        &["status", "--all"],
        &["check"],
        &["doc", "list"],
        &["trace"],
        &["explain"],
        &["explain", "SRR"],
        &["gate", "check"],
        &["doc", "new", "adr", "--title", "Use a shift register"],
        &["doc", "release", "PLN", "--note", "Baseline"],
        &["doc", "revise", "PLN"],
        &["gate", "open"],
        &["vault", "path"],
        &["vault", "new", "note", "Bench notes"],
        &["setup"],
    ] {
        run(&root, args);
    }
    run(&bench.root, &["sync"]);
    if tools_available() {
        run(&root, &["doc", "pdf", "CON"]);
        run(&root, &["doc", "pack", "CR"]);
    }
}

#[test]
fn setup_installs_agent_files_without_overwriting() {
    let bench = Bench::new();
    let root = bench.project("demo", &PRODUCT);
    let claude = root.join("CLAUDE.md");
    let settings = root.join(".claude/settings.local.json");
    assert!(
        fs::read_to_string(&claude)
            .unwrap()
            .contains("instructions for agents")
    );
    assert!(
        fs::read_to_string(root.join(".gitignore"))
            .unwrap()
            .contains(".claude/settings.local.json")
    );

    fs::write(&claude, "# Hand-edited\n").unwrap();
    fs::write(
        &settings,
        r#"{ "permissions": { "allow": ["Bash(ls)"] }, "model": "x" }"#,
    )
    .unwrap();
    bench.jig(&root).arg("setup").assert().success();
    bench.jig(&root).arg("setup").assert().success();

    assert_eq!(fs::read_to_string(&claude).unwrap(), "# Hand-edited\n");
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
    let dirs = value["permissions"]["additionalDirectories"]
        .as_array()
        .unwrap();
    assert_eq!(dirs.len(), 1, "the vault folder is added once");
    assert!(dirs[0].as_str().unwrap().ends_with("vault/projects/demo"));
    assert_eq!(
        value["permissions"]["allow"][0], "Bash(ls)",
        "existing permissions are kept"
    );
    assert_eq!(value["model"], "x", "other settings are kept");
}

#[test]
fn init_installs_the_bench_guide_from_the_vault() {
    let bench = Bench::new();
    let source = bench.root.join("vault/bench/CLAUDE.md");
    fs::write(&source, "# Bench guide\n").unwrap();
    bench.jig(&bench.root).arg("init").assert().success();
    let installed = fs::read_to_string(bench.root.join("CLAUDE.md")).unwrap();
    assert!(installed.contains("vault/bench/CLAUDE.md") && installed.ends_with("# Bench guide\n"));

    fs::write(&source, "# Bench guide, revised\n").unwrap();
    bench.jig(&bench.root).arg("init").assert().success();
    assert!(
        fs::read_to_string(bench.root.join("CLAUDE.md"))
            .unwrap()
            .ends_with("# Bench guide, revised\n")
    );
}
