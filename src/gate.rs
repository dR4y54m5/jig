//! Gate reviews: readiness against the profile, the gate review record, and
//! the decision that moves a project to its next phase.

use std::sync::LazyLock;

use anyhow::{Result, anyhow, bail};
use regex::Regex;
use serde::Serialize;

use crate::check::{Finding, Severity};
use crate::docs::{self, Doc, DocRequest};
use crate::markdown;
use crate::process::{DocState, Naming, Phase, Process};
use crate::project::Project;
use crate::render;
use crate::trace::{Coverage, Matrix};

static CHECKBOX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*[-*]\s+\[( |x|X)\]\s+(.*)$").unwrap());

#[derive(Debug, Serialize)]
pub struct CheckItem {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct CriterionState {
    pub text: String,
    pub checked: bool,
}

#[derive(Debug, Serialize)]
pub struct Readiness {
    pub gate: String,
    pub gate_title: String,
    pub phase: String,
    pub question: String,
    pub record: Option<String>,
    pub checks: Vec<CheckItem>,
    pub criteria: Vec<CriterionState>,
}

impl Readiness {
    pub fn checks_pass(&self) -> bool {
        self.checks.iter().all(|c| c.ok)
    }

    pub fn ready(&self) -> bool {
        self.checks_pass() && self.criteria.iter().all(|c| c.checked)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Go,
    GoWithActions,
    Iterate,
    Kill,
}

impl Outcome {
    pub fn parse(s: &str) -> Result<Outcome> {
        Ok(match s.to_lowercase().as_str() {
            "go" => Outcome::Go,
            "go-with-actions" => Outcome::GoWithActions,
            "iterate" => Outcome::Iterate,
            "kill" => Outcome::Kill,
            other => bail!("unknown outcome `{other}`; use go, go-with-actions, iterate or kill"),
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            Outcome::Go => "Go",
            Outcome::GoWithActions => "Go with actions",
            Outcome::Iterate => "Iterate",
            Outcome::Kill => "Kill",
        }
    }

    fn advances(self) -> bool {
        matches!(self, Outcome::Go | Outcome::GoWithActions)
    }
}

/// The phase a gate closes: `gate` if given, else the project's current phase.
pub fn phase_for<'a>(project: &'a Project, gate: Option<&str>) -> Result<&'a Phase> {
    let profile = project.profile();
    let phase = match gate {
        Some(gate) => profile.phase_by_gate(gate).ok_or_else(|| {
            anyhow!(
                "gate `{gate}` does not exist for kind {}",
                project.meta.kind
            )
        })?,
        None => project.phase(),
    };
    if phase.gate.is_none() {
        bail!(
            "phase {} of a {} project has no gate",
            phase.label(),
            project.meta.kind
        );
    }
    Ok(phase)
}

pub fn record<'a>(docs: &'a [Doc], gate: &str) -> Option<&'a Doc> {
    docs.iter()
        .find(|d| d.is_kind("gr") && d.get("gate").is_some_and(|g| g.eq_ignore_ascii_case(gate)))
}

pub fn readiness(
    project: &Project,
    docs: &[Doc],
    findings: &[Finding],
    phase: &Phase,
) -> Readiness {
    let profile = project.profile();
    let tier = project.tier();
    let gate = phase.gate.clone().unwrap_or_default();
    let mut checks = Vec::new();

    for req in profile.requirements(phase, tier) {
        let Some(kind) = Process::get().kind(&req.kind) else {
            continue;
        };
        let of_kind: Vec<&Doc> = docs.iter().filter(|d| d.is_kind(&kind.key)).collect();
        let satisfied = |d: &&Doc| match req.state {
            DocState::Exists => true,
            DocState::Released => d.is_released(),
        };
        let ok = of_kind.iter().any(satisfied);
        let state = match req.state {
            DocState::Exists => "exists",
            DocState::Released => "released",
        };
        let quantity = if kind.naming == Naming::Single {
            ""
        } else {
            "at least one "
        };
        let detail = if of_kind.is_empty() {
            "missing".to_string()
        } else {
            of_kind
                .iter()
                .map(|d| {
                    format!(
                        "{} Rev {} {}",
                        d.id().unwrap_or(&d.rel),
                        d.revision(),
                        d.status()
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        checks.push(CheckItem {
            name: format!("{quantity}{} {state}", kind.title.to_lowercase()),
            ok,
            detail,
        });
    }

    let matrix = Matrix::build(docs);
    for check in &phase.checks {
        let item = match check.as_str() {
            "requirements-well-formed" => {
                let bad = findings
                    .iter()
                    .filter(|f| {
                        f.severity == Severity::Error
                            && (f.rule.starts_with("requirement.") || f.rule.starts_with("test."))
                    })
                    .count();
                let has_reqs = !matrix.rows.is_empty();
                CheckItem {
                    name: "requirements are well formed".into(),
                    ok: bad == 0 && has_reqs,
                    detail: if has_reqs {
                        format!("{} requirements, {bad} errors", matrix.rows.len())
                    } else {
                        "no requirements found".into()
                    },
                }
            }
            "rtm-planned" => {
                let covered = matrix
                    .rows
                    .iter()
                    .filter(|r| r.coverage != Coverage::NotCovered)
                    .count();
                CheckItem {
                    name: "every requirement has a test case".into(),
                    ok: !matrix.rows.is_empty() && covered == matrix.rows.len(),
                    detail: format!("{covered} of {} covered", matrix.rows.len()),
                }
            }
            "rtm-verified" => {
                let verified = matrix.rows.iter().filter(|r| r.verified_released).count();
                CheckItem {
                    name: "every requirement is verified in a released test report".into(),
                    ok: !matrix.rows.is_empty() && verified == matrix.rows.len(),
                    detail: format!("{verified} of {} verified", matrix.rows.len()),
                }
            }
            other => CheckItem {
                name: other.to_string(),
                ok: false,
                detail: "unknown check".into(),
            },
        };
        checks.push(item);
    }

    let errors = findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    checks.push(CheckItem {
        name: "`jig check` reports no errors".into(),
        ok: errors == 0,
        detail: format!("{errors} errors"),
    });

    let record_doc = record(docs, &gate);
    let criteria = match record_doc {
        Some(doc) => criteria_in(doc),
        None => profile
            .criteria(phase, tier)
            .into_iter()
            .map(|t| CriterionState {
                text: t.to_string(),
                checked: false,
            })
            .collect(),
    };

    Readiness {
        gate,
        gate_title: phase.gate_title.clone().unwrap_or_default(),
        phase: phase.label(),
        question: phase.question.clone().unwrap_or_default(),
        record: record_doc.and_then(|d| d.id().map(str::to_string)),
        checks,
        criteria,
    }
}

/// The checklist under a record's "Entry criteria" heading.
fn criteria_in(doc: &Doc) -> Vec<CriterionState> {
    let body = doc.body();
    let headings = markdown::headings(body, 1);
    let Some(start) = headings
        .iter()
        .position(|h| markdown::section_name(&h.text) == "entry criteria")
    else {
        return Vec::new();
    };
    let from = headings[start].line;
    let level = headings[start].level;
    let to = headings[start + 1..]
        .iter()
        .find(|h| h.level <= level)
        .map(|h| h.line - 1)
        .unwrap_or(usize::MAX);
    body.lines()
        .enumerate()
        .filter(|(i, _)| *i + 1 > from && *i < to)
        .filter_map(|(_, line)| CHECKBOX.captures(line))
        .map(|c| CriterionState {
            text: c[2].trim().to_string(),
            checked: &c[1] != " ",
        })
        .collect()
}

fn checks_table(readiness: &Readiness) -> String {
    let mut out = String::from("| Check | Result | Detail |\n|---|---|---|\n");
    for c in &readiness.checks {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            c.name,
            if c.ok { "Pass" } else { "Fail" },
            c.detail
        ));
    }
    out
}

fn evidence_table(project: &Project, docs: &[Doc], phase: &Phase) -> String {
    let mut out = String::from("| Document | Title | Revision | Status |\n|---|---|---|---|\n");
    for doc in render::pack_contents(project, docs, phase)
        .into_iter()
        .filter(|d| !d.is_kind("gr"))
    {
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            doc.id().unwrap_or(&doc.rel),
            doc.title(),
            doc.revision(),
            doc.status()
        ));
    }
    out
}

/// Replaces the text between `<!-- jig:begin name -->` and `<!-- jig:end name -->`.
fn replace_block(body: &str, name: &str, content: &str) -> String {
    let begin = format!("<!-- jig:begin {name} -->");
    let end = format!("<!-- jig:end {name} -->");
    match (body.find(&begin), body.find(&end)) {
        (Some(b), Some(e)) if b < e => format!("{}{begin}\n{}{}", &body[..b], content, &body[e..]),
        _ => body.to_string(),
    }
}

fn set_outcome(body: &str, outcome: &str) -> String {
    body.lines()
        .map(|line| {
            if line.starts_with("**Outcome:**") {
                format!("**Outcome:** {outcome}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

pub enum Opened {
    Created(String),
    Refreshed(String),
    Revised(String),
}

/// Creates the gate review record, or refreshes its generated sections.
pub fn open(
    project: &Project,
    docs: &[Doc],
    findings: &[Finding],
    phase: &Phase,
    date: &str,
    author: &str,
) -> Result<Opened> {
    let gate = phase.gate.clone().unwrap_or_default();
    let readiness = readiness(project, docs, findings, phase);
    let checks = checks_table(&readiness);
    let evidence = evidence_table(project, docs, phase);

    if let Some(existing) = record(docs, &gate) {
        let (text, revised) = if existing.is_released() {
            let revised = docs::revise(existing, date)?;
            (set_outcome(&revised, "Pending"), true)
        } else {
            (existing.text.clone(), false)
        };
        let text = replace_block(
            &replace_block(&text, "checks", &checks),
            "evidence",
            &evidence,
        );
        std::fs::write(&existing.path, text)?;
        let id = existing.id().unwrap_or(&existing.rel).to_string();
        return Ok(if revised {
            Opened::Revised(id)
        } else {
            Opened::Refreshed(id)
        });
    }

    let criteria: String = project
        .profile()
        .criteria(phase, project.tier())
        .iter()
        .map(|c| format!("- [ ] {c}\n"))
        .collect();
    let kind = Process::get().kind("gr").expect("gate review kind exists");
    let draft = docs::draft(
        project,
        docs,
        DocRequest {
            kind,
            title: None,
            gate: Some(gate.clone()),
            date: date.to_string(),
            author: author.to_string(),
            vars: vec![
                ("gate_title".into(), readiness.gate_title.clone()),
                ("phase".into(), readiness.phase.clone()),
                ("question".into(), readiness.question.clone()),
                ("criteria".into(), criteria.trim_end().to_string()),
                ("checks".into(), checks.trim_end().to_string()),
                ("evidence".into(), evidence.trim_end().to_string()),
            ],
        },
    )?;
    if let Some(parent) = draft.path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&draft.path, &draft.text)?;
    Ok(Opened::Created(draft.id))
}

pub struct Closed {
    pub record: String,
    pub outcome: Outcome,
    pub new_phase: Option<String>,
    pub created: Vec<String>,
}

pub struct CloseRequest<'a> {
    pub outcome: Outcome,
    pub note: Option<&'a str>,
    pub force: bool,
    pub date: &'a str,
    pub author: &'a str,
}

/// Records the gate decision, releases the record and, on a go, moves the
/// project to its next phase and drafts that phase's documents.
pub fn close(
    project: &mut Project,
    docs: &[Doc],
    findings: &[Finding],
    phase: &Phase,
    req: CloseRequest,
) -> Result<Closed> {
    let gate = phase.gate.clone().unwrap_or_default();
    if phase.id != project.meta.phase {
        bail!(
            "{gate} closes phase {}, but the project is in {}",
            phase.label(),
            project.phase().label()
        );
    }
    let record_doc = record(docs, &gate)
        .ok_or_else(|| anyhow!("no review record for {gate}; run `jig gate open` first"))?;
    let readiness = readiness(project, docs, findings, phase);
    if req.outcome.advances() && !readiness.ready() && !req.force {
        let failing: Vec<String> = readiness
            .checks
            .iter()
            .filter(|c| !c.ok)
            .map(|c| format!("check failed: {} ({})", c.name, c.detail))
            .chain(
                readiness
                    .criteria
                    .iter()
                    .filter(|c| !c.checked)
                    .map(|c| format!("criterion unchecked: {}", c.text)),
            )
            .collect();
        bail!(
            "{gate} is not ready for a go:\n  {}\nUse --force to record the decision anyway.",
            failing.join("\n  ")
        );
    }

    let mut outcome_text = format!("{} ({})", req.outcome.label(), req.date);
    if let Some(note) = req.note.filter(|n| !n.trim().is_empty()) {
        outcome_text.push_str(&format!(". {}", note.trim()));
    }
    let body = replace_block(record_doc.body(), "checks", &checks_table(&readiness));
    let body = replace_block(&body, "evidence", &evidence_table(project, docs, phase));
    let body = set_outcome(&body, &outcome_text);
    let front = record_doc
        .front
        .as_ref()
        .ok_or_else(|| anyhow!("{} has no front matter", record_doc.rel))?;
    std::fs::write(&record_doc.path, format!("{}{body}", front.to_yaml()))?;
    let reloaded = Doc::load(&project.root, &record_doc.path)?;
    let released = docs::release(
        &reloaded,
        &format!("Gate decision: {}", req.outcome.label()),
        req.date,
        req.author,
    )?;
    std::fs::write(&record_doc.path, released)?;

    let mut closed = Closed {
        record: record_doc.id().unwrap_or(&record_doc.rel).to_string(),
        outcome: req.outcome,
        new_phase: None,
        created: Vec::new(),
    };
    match req.outcome {
        Outcome::Go | Outcome::GoWithActions => {
            let profile = project.profile();
            if let Some(next) = profile.next_phase(&phase.id, project.tier()) {
                project.meta.phase = next.id.clone();
                project.save()?;
                closed.new_phase = Some(next.label());
                closed.created = draft_phase_documents(project, next, req.date, req.author)?;
            } else {
                project.meta.status = "closed".into();
                project.save()?;
            }
        }
        Outcome::Kill => {
            project.meta.status = "killed".into();
            project.save()?;
        }
        Outcome::Iterate => {}
    }
    Ok(closed)
}

/// Drafts the single-instance documents a phase requires that do not exist yet.
pub fn draft_phase_documents(
    project: &Project,
    phase: &Phase,
    date: &str,
    author: &str,
) -> Result<Vec<String>> {
    let mut created = Vec::new();
    for req in project.profile().requirements(phase, project.tier()) {
        let Some(kind) = Process::get().kind(&req.kind) else {
            continue;
        };
        if kind.naming != Naming::Single || project.root.join(&kind.path).exists() {
            continue;
        }
        let existing = docs::load_all(&project.root)?;
        let draft = docs::draft(
            project,
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
        if let Some(parent) = draft.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&draft.path, &draft.text)?;
        created.push(draft.id);
    }
    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcomes_parse_and_label() {
        assert_eq!(
            Outcome::parse("go-with-actions").unwrap(),
            Outcome::GoWithActions
        );
        assert!(Outcome::parse("maybe").is_err());
        assert!(Outcome::Go.advances() && !Outcome::Iterate.advances());
    }

    #[test]
    fn generated_blocks_are_replaced_in_place() {
        let body = "a\n<!-- jig:begin checks -->\nold\n<!-- jig:end checks -->\nb\n";
        assert_eq!(
            replace_block(body, "checks", "new\n"),
            "a\n<!-- jig:begin checks -->\nnew\n<!-- jig:end checks -->\nb\n"
        );
    }

    #[test]
    fn outcome_line_is_rewritten() {
        assert_eq!(
            set_outcome("x\n**Outcome:** Pending\ny", "Go (2026-10-01)"),
            "x\n**Outcome:** Go (2026-10-01)\ny\n"
        );
    }
}
