//! jig: scaffolds, checks and renders engineering projects on a bench.

mod bench;
mod check;
mod docs;
mod explain;
mod frontmatter;
mod gate;
mod markdown;
mod process;
mod project;
mod render;
mod scaffold;
mod templates;
mod trace;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow, bail};
use clap::{Args, Parser, Subcommand};
use serde::Serialize;

use bench::Bench;
use check::{Finding, Severity};
use docs::{Doc, DocRequest};
use process::{Naming, Process};
use project::Project;
use trace::{Coverage, Matrix};

#[derive(Parser)]
#[command(
    name = "jig",
    version,
    about = "Scaffolds, checks and renders engineering projects on a bench"
)]
struct Cli {
    /// Bench root [default: $JIG_BENCH, else ~/bench]
    #[arg(long, global = true, value_name = "DIR")]
    bench: Option<PathBuf>,
    /// Print machine-readable JSON
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create the bench directories and an empty vault
    Init {
        #[arg(long)]
        dry_run: bool,
    },
    /// Create a project repository on the bench
    New {
        /// Project name: lowercase letters, digits and hyphens
        name: String,
        #[command(flatten)]
        project: ProjectArgs,
    },
    /// Bring an existing repository under the process
    Adopt {
        /// Path to the repository
        path: PathBuf,
        /// Project name [default: the directory name]
        #[arg(long)]
        name: Option<String>,
        #[command(flatten)]
        project: ProjectArgs,
    },
    /// Show the current project's status, or the whole bench
    Status {
        /// Show every registered project, even inside one
        #[arg(long)]
        all: bool,
    },
    /// Check document control, the separation rule and traceability
    Check {
        /// Print nothing unless there are errors
        #[arg(long)]
        quiet: bool,
        /// Fail on warnings too
        #[arg(long)]
        strict: bool,
    },
    /// Create, release, revise and render engineering documents
    Doc {
        #[command(subcommand)]
        command: DocCommand,
    },
    /// Check readiness for, open and close gate reviews
    Gate {
        #[command(subcommand)]
        command: GateCommand,
    },
    /// Print the requirements traceability matrix
    Trace,
    /// Explain the process: kinds, phases, gates, the separation rule
    Explain {
        topic: Option<String>,
        /// Project kind for `phases` and gates [default: the current project's]
        #[arg(long)]
        kind: Option<String>,
        /// Tier for `phases` and gates
        #[arg(long)]
        tier: Option<String>,
    },
    /// Create teaching documents in the vault for the current project
    Vault {
        #[command(subcommand)]
        command: VaultCommand,
    },
    /// Clone registered projects that are missing from this machine
    Sync {
        #[arg(long)]
        dry_run: bool,
    },
    /// Install the pre-commit hook, the agent instructions and the local agent settings
    Setup,
}

#[derive(Args, Clone)]
struct ProjectArgs {
    /// Short uppercase code used in document IDs, e.g. AR2
    #[arg(long)]
    code: String,
    /// product, software, re or exercise
    #[arg(long)]
    kind: String,
    /// Product tier: desk, batch or retail
    #[arg(long)]
    tier: Option<String>,
    /// One-line title, e.g. "Air mouse remote for Fire TV"
    #[arg(long)]
    title: String,
    /// public or private
    #[arg(long, default_value = "public")]
    visibility: String,
    /// Phase to start in [default: the first]; earlier phases' documents are drafted too
    #[arg(long)]
    phase: Option<String>,
    /// Git remote URL recorded in the registry
    #[arg(long, default_value = "")]
    remote: String,
    /// A path `jig check` skips, such as vendored third-party code; repeat for more
    #[arg(long, value_name = "PATH")]
    exclude: Vec<String>,
    /// Show what would be created without creating it
    #[arg(long)]
    dry_run: bool,
}

#[derive(Subcommand)]
enum DocCommand {
    /// List the project's documents
    List,
    /// Create a document from its kind's template
    New {
        /// Document kind, e.g. adr or SPK (`jig explain kinds`)
        kind: String,
        /// Title; required for numbered kinds such as ADR
        #[arg(long)]
        title: Option<String>,
    },
    /// Release a draft and add a revision-history row
    Release {
        /// Document ID, kind code or path, e.g. EM4-SRS, SRS or docs/requirements.md
        id: String,
        /// What this revision changes, for the revision history
        #[arg(long)]
        note: String,
    },
    /// Open the next revision of a released document as a draft
    Revise { id: String },
    /// Render one document to PDF
    Pdf {
        id: String,
        /// Output file [default: build/pdf/<ID>.pdf]; a `.png` path with `{p}` renders page images
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Render a gate review package to PDF
    Pack {
        /// Gate [default: the current phase's gate]
        gate: Option<String>,
        /// Output file [default: build/pdf/<CODE>-<GATE>-pack.pdf]; a `.png` path with `{p}` renders page images
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum GateCommand {
    /// Show readiness for a gate [default: the current phase's gate]
    Check { gate: Option<String> },
    /// Create the gate review record, or refresh its generated sections
    Open { gate: Option<String> },
    /// Record the gate decision; a go moves the project to its next phase
    Close {
        gate: String,
        /// go, go-with-actions, iterate or kill
        #[arg(long)]
        outcome: String,
        #[arg(long)]
        note: Option<String>,
        /// Record a go even though checks or criteria fail
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum VaultCommand {
    /// Create a teaching document: primer, lab, walkthrough, retro or note
    New { kind: String, title: String },
    /// Print the current project's vault folder
    Path,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let bench = Bench::locate(cli.bench.as_deref())?;
    let json = cli.json;
    let cwd = std::env::current_dir()?;
    match cli.command {
        Command::Init { dry_run } => apply(&bench, scaffold::init_plan(&bench), dry_run, json),
        Command::New { name, project } => {
            let dry_run = project.dry_run;
            let setup = setup(name, None, project);
            let (plan, _) =
                scaffold::project_plan(&bench, &setup, &docs::today(), &docs::author(&bench.root))?;
            apply(&bench, plan, dry_run, json)
        }
        Command::Adopt {
            path,
            name,
            project,
        } => {
            let canonical = path
                .canonicalize()
                .with_context(|| format!("{} does not exist", path.display()))?;
            let name = match name {
                Some(name) => name,
                None => canonical
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .ok_or_else(|| anyhow!("cannot name {}", path.display()))?,
            };
            let dry_run = project.dry_run;
            let author = docs::author(&canonical);
            let setup = setup(name, Some(canonical), project);
            let (plan, project) = scaffold::project_plan(&bench, &setup, &docs::today(), &author)?;
            let (mut value, mut text) = execute(&bench, plan, dry_run)?;

            // What `jig check` makes of the repository, so the engineer knows
            // what to change before the first commit.
            let findings = run_check(&bench, &project)?;
            let (errors, warnings) = check::counts(&findings);
            let failing: Vec<&Finding> = findings
                .iter()
                .filter(|f| f.severity == Severity::Error)
                .collect();
            text.push(String::new());
            text.push(format!(
                "`jig check` on the repository{}: {errors} errors, {warnings} warnings",
                if dry_run { " as it stands" } else { "" }
            ));
            text.extend(
                failing
                    .iter()
                    .map(|f| format!("  {}:{}: [{}] {}", f.file, f.line, f.rule, f.message)),
            );
            if errors > 0 {
                text.push(
                    "The pre-commit hook refuses these errors: fix them, or list third-party paths with --exclude."
                        .to_string(),
                );
            }
            value["check"] = serde_json::json!({
                "errors": errors,
                "warnings": warnings,
                "findings": failing,
            });
            report(json, value, &text.join("\n"))?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Status { all } => match Project::discover(&cwd) {
            Ok(project) if !all => project_status(&bench, &project, json),
            _ => bench_status(&bench, json),
        },
        Command::Check { quiet, strict } => {
            let project = Project::discover(&cwd)?;
            let findings = run_check(&bench, &project)?;
            print_findings(&findings, quiet, json)?;
            let (errors, warnings) = check::counts(&findings);
            Ok(if errors > 0 || (strict && warnings > 0) {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            })
        }
        Command::Doc { command } => doc_command(&bench, &cwd, command, json),
        Command::Gate { command } => gate_command(&bench, &cwd, command, json),
        Command::Trace => {
            let project = Project::discover(&cwd)?;
            let matrix = Matrix::build(&docs::load_all(&project.root)?);
            if json {
                println!("{}", serde_json::to_string_pretty(&matrix)?);
            } else if matrix.rows.is_empty() {
                println!("No requirements found in the system requirements specification.");
            } else {
                print!("{}", matrix.to_markdown());
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Explain { topic, kind, tier } => {
            let current = Project::discover(&cwd).ok();
            let profile = match &kind {
                Some(kind) => Some(
                    Process::get()
                        .profile(kind)
                        .ok_or_else(|| anyhow!("unknown project kind `{kind}`"))?,
                ),
                None => current.as_ref().map(Project::profile),
            };
            let tier = tier.or_else(|| {
                if kind.is_none() {
                    current.as_ref().and_then(|p| p.meta.tier.clone())
                } else {
                    None
                }
            });
            if json {
                let process = Process::get();
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "kinds": process.kinds, "profiles": process.profiles })
                    )?
                );
            } else {
                println!(
                    "{}",
                    explain::explain(topic.as_deref(), profile, tier.as_deref())?
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Vault { command } => vault_command(&bench, &cwd, command, json),
        Command::Sync { dry_run } => sync(&bench, dry_run, json),
        Command::Setup => {
            let project = Project::discover(&cwd)?;
            apply(&bench, scaffold::setup_plan(&bench, &project), false, json)
        }
    }
}

fn setup(name: String, existing: Option<PathBuf>, args: ProjectArgs) -> scaffold::Setup {
    scaffold::Setup {
        name,
        code: args.code,
        title: args.title,
        kind: args.kind,
        tier: args.tier,
        visibility: args.visibility,
        phase: args.phase,
        remote: args.remote,
        exclude: args.exclude,
        existing,
    }
}

/// Applies a plan, or lists it for a dry run. Returns the result as JSON and
/// as lines of text.
fn execute(
    bench: &Bench,
    plan: scaffold::Plan,
    dry_run: bool,
) -> Result<(serde_json::Value, Vec<String>)> {
    if dry_run {
        let lines = plan.describe(bench);
        let mut text = vec!["Would:".to_string()];
        text.extend(lines.iter().map(|line| format!("  {line}")));
        return Ok((
            serde_json::json!({ "dry_run": true, "actions": lines }),
            text,
        ));
    }
    let applied = plan.apply(bench)?;
    let mut text: Vec<String> = applied
        .done
        .iter()
        .map(|line| format!("  {line}"))
        .collect();
    text.extend(
        applied
            .skipped
            .iter()
            .map(|line| format!("  skipped (already there): {line}")),
    );
    text.extend(applied.notes.iter().map(|note| format!("  note: {note}")));
    Ok((serde_json::to_value(&applied)?, text))
}

fn apply(bench: &Bench, plan: scaffold::Plan, dry_run: bool, json: bool) -> Result<ExitCode> {
    let (value, text) = execute(bench, plan, dry_run)?;
    report(json, value, &text.join("\n"))?;
    Ok(ExitCode::SUCCESS)
}

fn run_check(bench: &Bench, project: &Project) -> Result<Vec<Finding>> {
    let vault = bench.vault_dir();
    let vault = vault.is_dir().then_some(vault);
    check::check(project, vault.as_deref(), &bench.private_terms()?)
}

fn print_findings(findings: &[Finding], quiet: bool, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(findings)?);
        return Ok(());
    }
    let (errors, warnings) = check::counts(findings);
    if quiet && errors == 0 {
        return Ok(());
    }
    for f in findings
        .iter()
        .filter(|f| !quiet || f.severity == Severity::Error)
    {
        let severity = match f.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        println!(
            "{}:{}: {severity} [{}] {}",
            f.file, f.line, f.rule, f.message
        );
    }
    println!("{errors} errors, {warnings} warnings");
    Ok(())
}

/// Finds a document by ID (EM4-SRS), kind code for single documents (SRS) or path.
fn find_doc<'a>(project: &Project, docs: &'a [Doc], name: &str) -> Result<&'a Doc> {
    let by_code = Process::get()
        .kind(name)
        .filter(|k| k.naming == Naming::Single)
        .map(|k| format!("{}-{}", project.meta.code, k.code));
    docs.iter()
        .find(|d| {
            d.id()
                .is_some_and(|id| id.eq_ignore_ascii_case(name) || Some(id) == by_code.as_deref())
                || d.rel == name
                || d.path == Path::new(name)
        })
        .ok_or_else(|| anyhow!("no document `{name}`; `jig doc list` shows them"))
}

#[derive(Serialize)]
struct DocSummary<'a> {
    id: &'a str,
    kind: &'a str,
    title: &'a str,
    revision: &'a str,
    status: &'a str,
    date: &'a str,
    path: &'a str,
}

fn doc_command(bench: &Bench, cwd: &Path, command: DocCommand, json: bool) -> Result<ExitCode> {
    let project = Project::discover(cwd)?;
    let all = docs::load_all(&project.root)?;
    let date = docs::today();
    let author = docs::author(&project.root);
    match command {
        DocCommand::List => {
            let rows: Vec<DocSummary> = all
                .iter()
                .map(|d| DocSummary {
                    id: d.id().unwrap_or("-"),
                    kind: d.kind().unwrap_or("-"),
                    title: d.title(),
                    revision: d.revision(),
                    status: d.status(),
                    date: d.date(),
                    path: &d.rel,
                })
                .collect();
            if json {
                println!("{}", serde_json::to_string_pretty(&rows)?);
            } else {
                for r in rows {
                    println!(
                        "{:<22} {:<3} {:<10} {:<10} {}",
                        r.id, r.revision, r.status, r.date, r.title
                    );
                }
            }
        }
        DocCommand::New { kind, title } => {
            let kind = Process::get()
                .kind(&kind)
                .ok_or_else(|| explain::unknown_kind(&kind))?;
            if kind.naming == Naming::Gate {
                bail!("gate review records are created with `jig gate open`");
            }
            let draft = docs::draft(
                &project,
                &all,
                DocRequest {
                    kind,
                    title,
                    gate: None,
                    date,
                    author,
                    vars: Vec::new(),
                },
            )?;
            if let Some(parent) = draft.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&draft.path, &draft.text)?;
            report(
                json,
                serde_json::json!({ "created": draft.id, "path": draft.rel }),
                &format!("created {} at {}", draft.id, draft.rel),
            )?;
        }
        DocCommand::Release { id, note } => {
            let doc = find_doc(&project, &all, &id)?;
            let blockers = check::release_blockers(doc, &run_check(bench, &project)?);
            if !blockers.is_empty() {
                bail!(
                    "{} cannot be released:\n  {}",
                    doc.id().unwrap_or(&doc.rel),
                    blockers.join("\n  ")
                );
            }
            std::fs::write(&doc.path, docs::release(doc, &note, &date, &author)?)?;
            let id = doc.id().unwrap_or(&doc.rel);
            report(
                json,
                serde_json::json!({ "released": id, "revision": doc.revision() }),
                &format!("released {id} Rev {}", doc.revision()),
            )?;
        }
        DocCommand::Revise { id } => {
            let doc = find_doc(&project, &all, &id)?;
            std::fs::write(&doc.path, docs::revise(doc, &date)?)?;
            let reloaded = Doc::load(&project.root, &doc.path)?;
            let id = reloaded.id().unwrap_or(&reloaded.rel);
            report(
                json,
                serde_json::json!({ "revised": id, "revision": reloaded.revision(), "status": reloaded.status() }),
                &format!("{id} is now Rev {} (draft)", reloaded.revision()),
            )?;
        }
        DocCommand::Pdf { id, out } => {
            let doc = find_doc(&project, &all, &id)?;
            let out = out.unwrap_or_else(|| {
                render::output_dir(&project).join(format!("{}.pdf", doc.id().unwrap_or("document")))
            });
            render::pdf(&project, &all, doc, &out)?;
            report(
                json,
                serde_json::json!({ "path": out }),
                &out.display().to_string(),
            )?;
        }
        DocCommand::Pack { gate, out } => {
            let phase = gate::phase_for(&project, gate.as_deref())?;
            let gate = phase.gate.clone().unwrap_or_default();
            let out = out.unwrap_or_else(|| {
                render::output_dir(&project).join(format!("{}-{gate}-pack.pdf", project.meta.code))
            });
            let info = render::pack(&project, &all, &gate, &date, &author, &out)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "path": out, "gate": info.gate, "documents": info.included })
                    )?
                );
            } else {
                println!(
                    "{} ({} documents: {})",
                    out.display(),
                    info.included.len(),
                    info.included.join(", ")
                );
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn gate_command(bench: &Bench, cwd: &Path, command: GateCommand, json: bool) -> Result<ExitCode> {
    let mut project = Project::discover(cwd)?;
    let all = docs::load_all(&project.root)?;
    let findings = run_check(bench, &project)?;
    let date = docs::today();
    let author = docs::author(&project.root);
    match command {
        GateCommand::Check { gate } => {
            let phase = gate::phase_for(&project, gate.as_deref())?;
            let readiness = gate::readiness(&project, &all, &findings, phase);
            if json {
                println!("{}", serde_json::to_string_pretty(&readiness)?);
            } else {
                print_readiness(&readiness);
            }
            Ok(if readiness.ready() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        GateCommand::Open { gate } => {
            let phase = gate::phase_for(&project, gate.as_deref())?;
            let (action, id, text) = match gate::open(
                &project, &all, &findings, phase, &date, &author,
            )? {
                gate::Opened::Created(id) => (
                    "created",
                    id.clone(),
                    format!(
                        "created {id}; confirm each entry criterion in it, then run `jig gate close`"
                    ),
                ),
                gate::Opened::Refreshed(id) => (
                    "refreshed",
                    id.clone(),
                    format!("refreshed the checks and evidence in {id}"),
                ),
                gate::Opened::Revised(id) => (
                    "revised",
                    id.clone(),
                    format!("opened a new revision of {id} for another review"),
                ),
            };
            report(
                json,
                serde_json::json!({ "record": id, "action": action }),
                &text,
            )?;
            Ok(ExitCode::SUCCESS)
        }
        GateCommand::Close {
            gate,
            outcome,
            note,
            force,
        } => {
            let phase = gate::phase_for(&project, Some(&gate))?.clone();
            let outcome = gate::Outcome::parse(&outcome)?;
            let closed = gate::close(
                &mut project,
                &all,
                &findings,
                &phase,
                gate::CloseRequest {
                    outcome,
                    note: note.as_deref(),
                    force,
                    date: &date,
                    author: &author,
                },
            )?;
            let gate_id = phase.gate.clone().unwrap_or_default();
            let mut lines = vec![format!(
                "{gate_id}: {}{} recorded in {}",
                closed.outcome.label(),
                if closed.forced { ", forced," } else { "" },
                closed.record
            )];
            if let Some(next) = &closed.new_phase {
                lines.push(format!("{} is now in {next}", project.meta.name));
            }
            lines.extend(closed.created.iter().map(|id| format!("  drafted {id}")));
            if closed.outcome != gate::Outcome::Iterate {
                lines.push(format!(
                    "After committing, tag the baseline: git tag gate/{}",
                    gate_id.to_lowercase()
                ));
            }
            report(
                json,
                serde_json::json!({
                    "gate": gate_id,
                    "outcome": closed.outcome.label(),
                    "forced": closed.forced,
                    "record": closed.record,
                    "phase": closed.new_phase,
                    "drafted": closed.created,
                }),
                &lines.join("\n"),
            )?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn print_readiness(r: &gate::Readiness) {
    println!("{}: {} (closes {})", r.gate, r.gate_title, r.phase);
    println!("{}", r.question);
    println!("\nChecks:");
    for c in &r.checks {
        println!(
            "  [{}] {} ({})",
            if c.ok { "x" } else { " " },
            c.name,
            c.detail
        );
    }
    let note = if r.record.is_some() {
        ""
    } else {
        " (no review record yet; `jig gate open` creates it)"
    };
    println!("\nEntry criteria{note}:");
    for c in &r.criteria {
        println!(
            "  [{}] {}{}",
            if c.checked { "x" } else { " " },
            c.text,
            if c.missing {
                " (missing from the review record; `jig gate open` restores it)"
            } else {
                ""
            }
        );
    }
    let passed = r.checks.iter().filter(|c| c.ok).count();
    let confirmed = r.criteria.iter().filter(|c| c.checked).count();
    println!(
        "\n{}: {passed} of {} checks pass, {confirmed} of {} criteria confirmed",
        if r.ready() { "Ready" } else { "Not ready" },
        r.checks.len(),
        r.criteria.len()
    );
}

#[derive(Serialize)]
struct ProjectStatus {
    name: String,
    title: String,
    kind: String,
    tier: Option<String>,
    visibility: String,
    status: String,
    phase: String,
    gate: Option<String>,
    documents: usize,
    released: usize,
    errors: usize,
    warnings: usize,
    requirements: usize,
    verified: usize,
    not_covered: usize,
    open_risks: usize,
    gate_ready: Option<bool>,
}

fn status_of(bench: &Bench, project: &Project) -> Result<(ProjectStatus, Option<gate::Readiness>)> {
    let all = docs::load_all(&project.root)?;
    let findings = run_check(bench, project)?;
    let (errors, warnings) = check::counts(&findings);
    let matrix = Matrix::build(&all);
    let phase = project.phase();
    let readiness = phase
        .gate
        .as_ref()
        .map(|_| gate::readiness(project, &all, &findings, phase));
    let open_risks = all
        .iter()
        .filter(|d| d.is_kind("rsk"))
        .flat_map(|d| d.body().lines())
        .filter(|l| l.trim_start().starts_with("| RISK-") && l.to_lowercase().contains("| open |"))
        .count();
    let status = ProjectStatus {
        name: project.meta.name.clone(),
        title: project.display_title(),
        kind: project.profile().title.clone(),
        tier: project.meta.tier.clone(),
        visibility: project.meta.visibility.clone(),
        status: project.meta.status.clone(),
        phase: phase.label(),
        gate: phase.gate.clone(),
        documents: all.len(),
        released: all.iter().filter(|d| d.is_released()).count(),
        errors,
        warnings,
        requirements: matrix.rows.len(),
        verified: matrix.count(Coverage::Verified),
        not_covered: matrix.count(Coverage::NotCovered),
        open_risks,
        gate_ready: readiness.as_ref().map(gate::Readiness::ready),
    };
    Ok((status, readiness))
}

fn project_status(bench: &Bench, project: &Project, json: bool) -> Result<ExitCode> {
    let (s, readiness) = status_of(bench, project)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&s)?);
        return Ok(ExitCode::SUCCESS);
    }
    let tier = s
        .tier
        .as_ref()
        .map(|t| format!(", {t} tier"))
        .unwrap_or_default();
    println!("{}: {}", s.name, s.title);
    println!(
        "Kind:       {}{tier}, {} ({})",
        s.kind, s.visibility, s.status
    );
    println!("Phase:      {}", s.phase);
    println!("Documents:  {} ({} released)", s.documents, s.released);
    println!("Check:      {} errors, {} warnings", s.errors, s.warnings);
    println!(
        "Trace:      {} requirements, {} verified, {} without a test case",
        s.requirements, s.verified, s.not_covered
    );
    println!("Risks:      {} open", s.open_risks);
    if let Some(r) = readiness {
        println!();
        print_readiness(&r);
    }
    Ok(ExitCode::SUCCESS)
}

fn bench_status(bench: &Bench, json: bool) -> Result<ExitCode> {
    let registry = bench.registry()?;
    let mut rows = Vec::new();
    for entry in &registry.projects {
        let root = bench.root.join(&entry.path);
        match Project::load(&root) {
            Ok(project) => rows.push(serde_json::to_value(status_of(bench, &project)?.0)?),
            Err(err) => {
                rows.push(serde_json::json!({ "name": entry.name, "error": format!("{err:#}") }))
            }
        }
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(ExitCode::SUCCESS);
    }
    if rows.is_empty() {
        println!(
            "No projects registered in {}.",
            bench.registry_path().display()
        );
    }
    for row in rows {
        let name = row["name"].as_str().unwrap_or("");
        if let Some(err) = row.get("error") {
            println!("{name:<14} {}", err.as_str().unwrap_or(""));
            continue;
        }
        let kind = format!(
            "{}{}",
            row["kind"].as_str().unwrap_or(""),
            row["tier"]
                .as_str()
                .map(|t| format!(" ({t})"))
                .unwrap_or_default()
        );
        let gate = row["gate"]
            .as_str()
            .map(|g| format!(" → {g}"))
            .unwrap_or_default();
        let ready = if row["gate_ready"].as_bool() == Some(true) {
            " ready"
        } else {
            ""
        };
        let phase = format!("{}{gate}{ready}", row["phase"].as_str().unwrap_or(""));
        println!(
            "{name:<14} {kind:<26} {phase:<34} {}/{} released, {} errors",
            row["released"], row["documents"], row["errors"]
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn vault_command(bench: &Bench, cwd: &Path, command: VaultCommand, json: bool) -> Result<ExitCode> {
    let project = Project::discover(cwd)?;
    let dir = bench.vault_project_dir(&project.meta.name);
    match command {
        VaultCommand::Path => report(
            json,
            serde_json::json!({ "path": dir }),
            &dir.display().to_string(),
        )?,
        VaultCommand::New { kind, title } => {
            let folder = match kind.as_str() {
                "primer" | "lab" | "walkthrough" | "retro" | "note" => format!("{kind}s"),
                other => bail!(
                    "unknown teaching kind `{other}`; use primer, lab, walkthrough, retro or note"
                ),
            };
            let path = dir
                .join(&folder)
                .join(format!("{}.md", docs::slugify(&title)));
            if path.exists() {
                bail!("{} already exists", path.display());
            }
            let template = std::fs::read_to_string(
                bench
                    .vault_dir()
                    .join("templates")
                    .join(format!("{kind}.md")),
            )
            .unwrap_or_else(|_| "# {{title}}\n".to_string());
            let text = templates::fill(
                &template,
                &[
                    ("title", &title),
                    ("project", &project.meta.name),
                    ("date", &docs::today()),
                ],
            );
            std::fs::create_dir_all(path.parent().expect("a file path has a parent"))?;
            std::fs::write(&path, text)?;
            report(
                json,
                serde_json::json!({ "created": path }),
                &path.display().to_string(),
            )?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn sync(bench: &Bench, dry_run: bool, json: bool) -> Result<ExitCode> {
    let mut results = Vec::new();
    for entry in bench.registry()?.projects {
        let path = bench.root.join(&entry.path);
        let outcome = if path.exists() {
            "present"
        } else if entry.remote.is_empty() {
            "missing without a remote"
        } else if dry_run {
            "would clone"
        } else {
            let status = std::process::Command::new("git")
                .arg("clone")
                .arg("--quiet")
                .arg(&entry.remote)
                .arg(&path)
                .status()?;
            if !status.success() {
                bail!("cloning {} failed", entry.remote);
            }
            // A cloned project needs this machine's hook and local agent settings.
            match Project::load(&path) {
                Ok(project) => scaffold::setup_plan(bench, &project).apply(bench)?,
                Err(_) => scaffold::hook_plan(&path).apply(bench)?,
            };
            "cloned"
        };
        results.push((entry.name, entry.path, outcome));
    }
    let value: Vec<_> = results
        .iter()
        .map(|(name, path, outcome)| serde_json::json!({ "name": name, "path": path, "outcome": outcome }))
        .collect();
    let text: Vec<String> = results
        .iter()
        .map(|(name, path, outcome)| format!("{name:<14} {outcome} ({path})"))
        .collect();
    report(json, serde_json::Value::Array(value), &text.join("\n"))?;
    Ok(ExitCode::SUCCESS)
}

/// Prints `value` as JSON when `--json` is given, otherwise `text`.
fn report(json: bool, value: serde_json::Value, text: &str) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else if !text.is_empty() {
        println!("{text}");
    }
    Ok(())
}
