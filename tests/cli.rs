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

const SOFTWARE: [&str; 6] = ["--kind", "software", "--code", "DM", "--title", "Demo tool"];

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

/// Confirms every entry criterion in a gate review record.
fn tick_all(record: &Path) {
    fs::write(record, read(record).replace("- [ ]", "- [x]")).unwrap();
}

/// Runs `jig check --json` and returns each finding as (file, rule, severity).
fn check_findings(bench: &Bench, root: &Path) -> Vec<(String, String, String)> {
    let output = bench
        .jig(root)
        .args(["check", "--json"])
        .output()
        .unwrap()
        .stdout;
    let findings: serde_json::Value = serde_json::from_slice(&output).unwrap();
    findings
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            let field = |name: &str| f[name].as_str().unwrap().to_string();
            (field("file"), field("rule"), field("severity"))
        })
        .collect()
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn in_phase(root: &Path, phase: &str) -> bool {
    read(&root.join("project.toml")).contains(&format!("phase = \"{phase}\""))
}

/// A software project in P0 whose first three documents are written. The
/// documents named in `release` are released, and the Concept Review record
/// is open.
fn project_at_concept_review(bench: &Bench, release: &[&str]) -> (PathBuf, PathBuf) {
    let root = bench.project("demo", &SOFTWARE);
    for doc in ["docs/plan.md", "docs/concept.md", "docs/risks.md"] {
        strip_guidance(&root.join(doc));
    }
    for id in release {
        bench
            .jig(&root)
            .args(["doc", "release", id, "--note", "Baseline"])
            .assert()
            .success();
    }
    bench.jig(&root).args(["gate", "open"]).assert().success();
    let record = root.join("docs/reviews/GR-CR.md");
    (root, record)
}

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
    strip_guidance(&record);
    let unconfirmed = read(&record);
    bench
        .jig(&root)
        .args(["gate", "close", "CR", "--outcome", "go"])
        .assert()
        .code(2);
    assert!(in_phase(&root, "P0"), "a refused go leaves the phase alone");
    assert_eq!(
        read(&record),
        unconfirmed,
        "a refused go leaves the record alone"
    );
    tick_all(&record);
    bench
        .jig(&root)
        .args(["gate", "close", "CR", "--outcome", "go"])
        .assert()
        .success();

    assert!(in_phase(&root, "P1"));
    assert!(
        root.join("docs/requirements.md").exists(),
        "the SRS is drafted on entering Definition"
    );
    let record = read(&record);
    assert!(record.contains("status: released"));
    assert!(record.contains("**Outcome:** Go ("));
}

#[test]
fn a_gate_takes_its_criteria_from_the_profile() {
    let bench = Bench::new();
    let (root, record) = project_at_concept_review(&bench, &["PLN", "CON"]);
    strip_guidance(&record);
    // Delete every criterion from the record instead of confirming it.
    let emptied: String = read(&record)
        .lines()
        .filter(|line| !line.starts_with("- [ ]"))
        .map(|line| format!("{line}\n"))
        .collect();
    fs::write(&record, &emptied).unwrap();

    let output = bench
        .jig(&root)
        .args(["gate", "check", "--json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let readiness: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let criteria = readiness["criteria"].as_array().unwrap();
    assert_eq!(
        criteria.len(),
        4,
        "the profile's criteria are still required"
    );
    assert!(
        criteria
            .iter()
            .all(|c| c["missing"] == true && c["checked"] == false)
    );

    bench
        .jig(&root)
        .args(["gate", "close", "CR", "--outcome", "go"])
        .assert()
        .code(2);
    assert!(in_phase(&root, "P0"));
    assert_eq!(read(&record), emptied, "a refused go changes nothing");

    // Refreshing the record puts the missing criteria back, unconfirmed.
    bench.jig(&root).args(["gate", "open"]).assert().success();
    assert_eq!(read(&record).matches("- [ ] ").count(), 4);
}

#[test]
fn a_review_record_is_finished_and_decided_once() {
    let bench = Bench::new();
    let (root, record) = project_at_concept_review(&bench, &["PLN", "CON"]);
    tick_all(&record);

    // The summary is still template guidance, so no decision can be recorded.
    let unfinished = read(&record);
    for outcome in ["go", "iterate", "kill"] {
        bench
            .jig(&root)
            .args(["gate", "close", "CR", "--outcome", outcome, "--force"])
            .assert()
            .code(2);
    }
    assert_eq!(read(&record), unfinished);
    assert!(in_phase(&root, "P0"));

    strip_guidance(&record);
    bench
        .jig(&root)
        .args(["gate", "close", "CR", "--outcome", "iterate"])
        .assert()
        .success();
    let decided = read(&record);
    assert!(decided.contains("status: released"));
    assert!(decided.contains("**Outcome:** Iterate"));

    // A second decision without reopening the review is refused and changes nothing.
    bench
        .jig(&root)
        .args(["gate", "close", "CR", "--outcome", "go"])
        .assert()
        .code(2);
    assert_eq!(read(&record), decided);
    assert!(in_phase(&root, "P0"));

    // Reopening starts a new revision with every confirmation cleared.
    bench.jig(&root).args(["gate", "open"]).assert().success();
    let reopened = read(&record);
    assert!(reopened.contains("revision: B") && reopened.contains("status: draft"));
    assert!(reopened.contains("**Outcome:** Pending"));
    assert!(!reopened.contains("- [x]"));
    assert_eq!(reopened.matches("- [ ] ").count(), 4);
}

#[test]
fn a_go_needs_every_check_unless_it_is_forced() {
    let bench = Bench::new();
    // The concept brief stays a draft, so one automated check fails.
    let (root, record) = project_at_concept_review(&bench, &["PLN"]);
    strip_guidance(&record);
    tick_all(&record);

    let refused = bench
        .jig(&root)
        .args(["gate", "close", "CR", "--outcome", "go"])
        .assert()
        .code(2)
        .get_output()
        .stderr
        .clone();
    assert!(String::from_utf8_lossy(&refused).contains("check failed"));
    assert!(in_phase(&root, "P0"));
    assert!(read(&record).contains("status: draft"));

    bench
        .jig(&root)
        .args([
            "gate",
            "close",
            "CR",
            "--outcome",
            "go",
            "--force",
            "--note",
            "Accepted for the demonstration",
        ])
        .assert()
        .success();
    assert!(in_phase(&root, "P1"));
    let record = read(&record);
    assert!(record.contains("**Outcome:** Go, forced past 1 failed check ("));
    assert!(record.contains("Gate decision: Go, forced"));
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
fn check_finds_private_references_in_every_text_file() {
    let bench = Bench::new();
    let root = bench.project("demo", &SOFTWARE);
    fs::write(
        bench.root.join("vault/jig.toml"),
        "private_terms = [\"secret-notes\"]\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    // Built from parts, so that this file holds no home path of its own.
    let home_path = ["", "Users", "someone", "plans"].join("/");
    fs::write(
        root.join("src/main.rs"),
        format!("// see secret-notes\nconst PLANS: &str = \"{home_path}\";\n"),
    )
    .unwrap();
    fs::write(
        root.join("settings.toml"),
        format!(
            "notes = \"{}\"\n",
            bench.root.join("vault/projects/demo").display()
        ),
    )
    .unwrap();
    fs::write(root.join("logo.bin"), [0u8, 159, 146, 150]).unwrap();

    let private: Vec<String> = check_findings(&bench, &root)
        .into_iter()
        .filter(|(_, rule, severity)| rule == "separation.private-reference" && severity == "error")
        .map(|(file, _, _)| file)
        .collect();
    assert_eq!(
        private,
        ["settings.toml", "src/main.rs", "src/main.rs"],
        "the vault path, a private term and a home path, none of them in Markdown"
    );
}

#[test]
fn check_follows_every_kind_of_link() {
    let bench = Bench::new();
    let root = bench.project("demo", &SOFTWARE);
    let readme = root.join("README.md");
    let mut text = read(&readme);
    // A template placeholder elsewhere in the file must not switch the link checks off.
    text.push_str("\nThe title is filled in from the `{{title}}` placeholder: {{title}}.\n\n");
    text.push_str(
        "See [the notes](../../vault/notes.md), [the guide][g] and [the plan](/opt/plans/plan.md).\n",
    );
    text.push_str("<a href=\"../../vault/page.html\">A page</a>\n\n[g]: ../../vault/guide.md\n");
    fs::write(&readme, text).unwrap();

    let outside = check_findings(&bench, &root)
        .into_iter()
        .filter(|(file, rule, severity)| {
            file == "README.md" && rule == "separation.private-reference" && severity == "error"
        })
        .count();
    assert_eq!(
        outside, 4,
        "an inline link, an absolute path, an HTML attribute and a reference definition"
    );
}

#[test]
fn check_skips_ignored_and_excluded_files() {
    let bench = Bench::new();
    let root = bench.project("demo", &SOFTWARE);
    let ignore = root.join(".gitignore");
    fs::write(&ignore, read(&ignore) + "deps/\n").unwrap();
    for dir in ["deps/lib", "vendor/lib"] {
        fs::create_dir_all(root.join(dir)).unwrap();
        fs::write(root.join(dir).join("README.md"), "Now let's begin.\n").unwrap();
    }
    let teaching: Vec<String> = check_findings(&bench, &root)
        .into_iter()
        .filter(|(_, rule, _)| rule == "separation.teaching")
        .map(|(file, _, _)| file)
        .collect();
    assert_eq!(
        teaching,
        ["vendor/lib/README.md"],
        "a file that git ignores is not checked"
    );

    let config = root.join("project.toml");
    fs::write(
        &config,
        read(&config) + "\n[check]\nexclude = [\"vendor\"]\n",
    )
    .unwrap();
    bench.jig(&root).arg("check").assert().success();
}

#[test]
fn adoption_reports_what_the_existing_files_fail() {
    let bench = Bench::new();
    let repo = bench.root.join("projects/old");
    fs::create_dir_all(repo.join("docs")).unwrap();
    fs::write(repo.join("README.md"), "# Old\n\nNow let's begin.\n").unwrap();
    fs::write(repo.join("docs/setup.md"), "# Setup\n").unwrap();
    let adopt = |extra: &[&str]| {
        let output = bench
            .jig(&bench.root)
            .arg("adopt")
            .arg(&repo)
            .args([
                "--kind", "software", "--code", "OLD", "--title", "Old tool", "--json",
            ])
            .args(extra)
            .output()
            .unwrap();
        assert!(output.status.success());
        parse_json(&output.stdout, "adopt")
    };

    let planned = adopt(&["--dry-run"]);
    assert_eq!(planned["check"]["errors"], 2);
    assert!(!repo.join("project.toml").exists());

    let applied = adopt(&[]);
    assert_eq!(applied["check"]["errors"], 2);
    let rules: Vec<&str> = applied["check"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule"].as_str().unwrap())
        .collect();
    assert!(rules.contains(&"separation.teaching") && rules.contains(&"front-matter.missing"));
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
    git(&repo, &["init", "-q"]);
    fs::write(repo.join("README.md"), "# Existing\n").unwrap();
    fs::write(repo.join(".gitignore"), "node_modules\n").unwrap();
    fs::write(repo.join("CLAUDE.md"), "# Existing rules\n").unwrap();
    bench
        .jig(&bench.root)
        .arg("adopt")
        .arg(&repo)
        .args([
            "--kind", "software", "--code", "TL", "--title", "Tool", "--phase", "P1",
        ])
        .assert()
        .success();
    assert_eq!(read(&repo.join("README.md")), "# Existing\n");
    assert_eq!(
        read(&repo.join(".gitignore")),
        "node_modules\n/build/\n",
        "only the build directory is added"
    );
    assert_eq!(
        read(&repo.join("CLAUDE.md")),
        "# Existing rules\n",
        "the repository's own agent instructions are left alone"
    );
    assert_eq!(
        read(&bench.root.join("vault/projects/tool/CLAUDE.md")),
        "# Existing rules\n",
        "and become the copy in the project's vault folder"
    );
    assert!(
        repo.join("docs/concept.md").exists(),
        "P0 documents are drafted when starting in P1"
    );
    assert!(repo.join("docs/requirements.md").exists());
    assert!(in_phase(&repo, "P1"));

    // `jig setup` then installs the vault copy in its place.
    bench.jig(&repo).arg("setup").assert().success();
    let installed = read(&repo.join("CLAUDE.md"));
    assert!(installed.starts_with("<!-- Installed by `jig setup`"));
    assert!(installed.ends_with("# Existing rules\n"));
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
    ) + "\n### REQ-002 Start and stop\n\nThe system shall start and shall stop.\n\n- **Verification:** Test\n";
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
    assert!(
        check_findings(&bench, &root).contains(&(
            "docs/requirements.md".to_string(),
            "requirement.not-singular".to_string(),
            "error".to_string()
        )),
        "a statement with two `shall`s is an error"
    );
}

#[test]
fn a_test_report_is_stamped_and_its_results_go_stale() {
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
            "P3",
        ],
    );
    let srs = root.join("docs/requirements.md");
    fs::write(
        &srs,
        read(&srs)
            + "\n### REQ-101 Start\n\nThe tool shall start within 1 s.\n\n- **Verification:** Test\n",
    )
    .unwrap();
    let vvp = root.join("docs/vv-plan.md");
    fs::write(
        &vvp,
        read(&vvp)
            + "\n### TC-101 Start time\n\n- **Verifies:** REQ-101\n- **Method:** Test\n- **Level:** System\n- **Procedure:** `demo --time`\n- **Pass criteria:** Under 1 s.\n",
    )
    .unwrap();

    // A new report lists the test case, not run, with its basis stamp.
    let created = bench
        .jig(&root)
        .args(["doc", "new", "tr", "--title", "First run", "--json"])
        .output()
        .unwrap();
    let created = parse_json(&created.stdout, "doc new tr");
    let report = root.join(created["path"].as_str().unwrap());
    let text = read(&report);
    let stub = "### TC-101 Start time\n\n- **Result:** Not run\n- **Evidence:**\n- **Basis:** ";
    assert!(text.contains(stub), "{text}");

    let coverage = |bench: &Bench| {
        let output = bench.jig(&root).args(["trace", "--json"]).output().unwrap();
        let matrix = parse_json(&output.stdout, "trace");
        let row = matrix["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["requirement"] == "REQ-101")
            .unwrap()
            .clone();
        row["coverage"].as_str().unwrap().to_string()
    };
    assert_eq!(coverage(&bench), "planned", "a case not run is no result");

    fs::write(
        &report,
        text.replace(
            "### TC-101 Start time\n\n- **Result:** Not run",
            "### TC-101 Start time\n\n- **Result:** Pass",
        ),
    )
    .unwrap();
    assert_eq!(coverage(&bench), "verified");

    // The requirement changes after the run: the result no longer proves it.
    fs::write(&srs, read(&srs).replace("within 1 s", "within 2 s")).unwrap();
    assert_eq!(coverage(&bench), "stale");
    assert!(
        check_findings(&bench, &root)
            .iter()
            .any(|(file, rule, severity)| file.starts_with("docs/reports/")
                && rule == "test.stale"
                && severity == "warning")
    );
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
fn agent_instructions_are_installed_from_the_vault() {
    let bench = Bench::new();
    let root = bench.project("demo", &PRODUCT);
    let installed = root.join("CLAUDE.md");
    let master = bench.root.join("vault/projects/demo/CLAUDE.md");
    let settings = root.join(".claude/settings.local.json");

    // A new project gets its instructions in the vault and a read-only copy.
    assert!(read(&master).contains("instructions for agents"));
    let copy = read(&installed);
    assert!(copy.starts_with("<!-- Installed by `jig setup`") && copy.ends_with(&read(&master)));
    assert!(fs::metadata(&installed).unwrap().permissions().readonly());

    // Neither agent file is named in a tracked file, and git sees neither.
    assert!(
        !read(&root.join(".gitignore"))
            .to_lowercase()
            .contains("claude")
    );
    let seen = git(&root, &["status", "--porcelain", "--untracked-files=all"]);
    assert!(!seen.to_lowercase().contains("claude"), "{seen}");

    // The vault copy is the one to edit; `jig setup` installs it again.
    fs::write(&master, "# Demo rules\n").unwrap();
    fs::write(
        &settings,
        r#"{ "permissions": { "allow": ["Bash(ls)"] }, "model": "x" }"#,
    )
    .unwrap();
    bench.jig(&root).arg("setup").assert().success();
    bench.jig(&root).arg("setup").assert().success();
    assert!(read(&installed).ends_with("# Demo rules\n"));
    let value: serde_json::Value = serde_json::from_str(&read(&settings)).unwrap();
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

    // A copy lost on this machine comes back from the vault.
    fs::remove_file(&installed).unwrap();
    bench.jig(&root).arg("setup").assert().success();
    assert!(read(&installed).ends_with("# Demo rules\n"));
}

#[test]
fn check_rejects_tracked_agent_files() {
    let bench = Bench::new();
    let root = bench.project("demo", &SOFTWARE);
    let tracked = |bench: &Bench| -> Vec<String> {
        check_findings(bench, &root)
            .into_iter()
            .filter(|(_, rule, severity)| rule == "separation.agent-file" && severity == "error")
            .map(|(file, _, _)| file)
            .collect()
    };
    assert!(tracked(&bench).is_empty());
    git(
        &root,
        &["add", "--force", "CLAUDE.md", ".claude/settings.local.json"],
    );
    assert_eq!(
        tracked(&bench),
        [".claude/settings.local.json", "CLAUDE.md"]
    );
}

#[test]
fn the_hook_blocks_a_commit_that_fails_or_cannot_be_checked() {
    let bench = Bench::new();
    let root = bench.project("demo", &SOFTWARE);
    let hook = root.join(".git/hooks/pre-commit");
    let run = |path: &str| {
        std::process::Command::new("/bin/sh")
            .arg(&hook)
            .current_dir(&root)
            .env("PATH", path)
            .env("HOME", bench.root.join("nobody"))
            .env("JIG_BENCH", &bench.root)
            .output()
            .unwrap()
    };
    let bin = Path::new(env!("CARGO_BIN_EXE_jig")).parent().unwrap();
    let with_jig = format!("{}:/usr/bin:/bin", bin.display());

    assert_eq!(
        run(&with_jig).status.code(),
        Some(0),
        "a clean repository passes"
    );
    let unchecked = run("/usr/bin:/bin");
    assert_eq!(
        unchecked.status.code(),
        Some(1),
        "without jig the commit is refused"
    );
    assert!(String::from_utf8_lossy(&unchecked.stderr).contains("jig was not found"));

    let readme = root.join("README.md");
    fs::write(&readme, read(&readme) + "\nNow let's begin.\n").unwrap();
    assert_eq!(
        run(&with_jig).status.code(),
        Some(1),
        "an error refuses the commit"
    );
}

#[test]
fn setup_reports_hooks_that_are_managed_elsewhere() {
    let bench = Bench::new();
    let repo = bench.root.join("projects/old");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "core.hooksPath", ".husky"]);
    let output = bench
        .jig(&bench.root)
        .arg("adopt")
        .arg(&repo)
        .args([
            "--kind", "exercise", "--code", "OLD", "--title", "Old kata", "--json",
        ])
        .output()
        .unwrap();
    let value = parse_json(&output.stdout, "adopt");
    let notes = value["notes"].as_array().unwrap();
    assert!(
        notes
            .iter()
            .any(|note| note.as_str().unwrap().contains("core.hooksPath"))
    );
    assert!(!repo.join(".git/hooks/pre-commit").exists());
}

#[test]
fn new_repositories_start_on_main() {
    let dir = tempfile::tempdir().unwrap();
    // A machine whose git still defaults to another branch name.
    let jig = |args: &[&str]| {
        Command::cargo_bin("jig")
            .unwrap()
            .env("JIG_BENCH", dir.path())
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "init.defaultBranch")
            .env("GIT_CONFIG_VALUE_0", "trunk")
            .current_dir(dir.path())
            .args(args)
            .assert()
            .success();
    };
    jig(&["init"]);
    jig(&[
        "new",
        "demo",
        "--kind",
        "software",
        "--code",
        "DM",
        "--title",
        "Demo tool",
    ]);
    for repo in ["vault", "projects/demo"] {
        assert_eq!(
            git(&dir.path().join(repo), &["symbolic-ref", "--short", "HEAD"]),
            "main",
            "{repo}"
        );
    }
}

#[test]
fn init_installs_the_bench_guide_from_the_vault() {
    let bench = Bench::new();
    let source = bench.root.join("vault/bench/CLAUDE.md");
    fs::write(&source, "# Bench guide\n").unwrap();
    bench.jig(&bench.root).arg("init").assert().success();
    let installed = fs::read_to_string(bench.root.join("CLAUDE.md")).unwrap();
    assert!(installed.contains("vault/bench/CLAUDE.md") && installed.ends_with("# Bench guide\n"));
    assert!(
        fs::metadata(bench.root.join("CLAUDE.md"))
            .unwrap()
            .permissions()
            .readonly(),
        "the installed guide is read-only, so edits go to the vault copy"
    );

    fs::write(&source, "# Bench guide, revised\n").unwrap();
    bench.jig(&bench.root).arg("init").assert().success();
    assert!(
        fs::read_to_string(bench.root.join("CLAUDE.md"))
            .unwrap()
            .ends_with("# Bench guide, revised\n")
    );
}
