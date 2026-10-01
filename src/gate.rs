//! Gate reviews: readiness against the profile, the gate review record, and
//! the decision that moves a project to its next phase.

use std::sync::LazyLock;

use anyhow::{Result, anyhow, bail};
use regex::Regex;
use serde::Serialize;

use crate::check::{self, Finding, Severity};
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
    /// Required by the profile but absent from the review record.
    pub missing: bool,
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

    /// What stands between the gate and a go, one line per failed check and
    /// unconfirmed criterion.
    pub fn shortfalls(&self) -> Vec<String> {
        self.checks
            .iter()
            .filter(|c| !c.ok)
            .map(|c| format!("check failed: {} ({})", c.name, c.detail))
            .chain(self.criteria.iter().filter(|c| !c.checked).map(|c| {
                if c.missing {
                    format!("criterion missing from the review record: {}", c.text)
                } else {
                    format!("criterion unconfirmed: {}", c.text)
                }
            }))
            .collect()
    }

    /// The shortfalls as a phrase, e.g. "1 failed check and 2 unconfirmed criteria".
    fn shortfall_summary(&self) -> String {
        let failed = self.checks.iter().filter(|c| !c.ok).count();
        let open = self.criteria.iter().filter(|c| !c.checked).count();
        let mut parts = Vec::new();
        if failed > 0 {
            parts.push(format!(
                "{failed} failed check{}",
                if failed == 1 { "" } else { "s" }
            ));
        }
        if open > 0 {
            parts.push(format!(
                "{open} unconfirmed criteri{}",
                if open == 1 { "on" } else { "a" }
            ));
        }
        parts.join(" and ")
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
    let criteria = criteria_state(
        &profile.criteria(phase, tier),
        &record_doc.map(|doc| checklist(doc.body())),
    );

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

/// The body lines under a record's "Entry criteria" heading, as a half-open
/// range of 0-based line indices: from the line after the heading to the next
/// heading of the same or a higher level.
fn criteria_section(body: &str) -> Option<(usize, usize)> {
    let headings = markdown::headings(body, 1);
    let at = headings
        .iter()
        .position(|h| markdown::section_name(&h.text) == "entry criteria")?;
    let level = headings[at].level;
    let end = headings[at + 1..]
        .iter()
        .find(|h| h.level <= level)
        .map(|h| h.line - 1)
        .unwrap_or(usize::MAX);
    // A heading's 1-based line number is the 0-based index of the line after it.
    Some((headings[at].line, end))
}

/// The checklist under a record's "Entry criteria" heading: each line's text
/// and whether it is ticked.
fn checklist(body: &str) -> Vec<(String, bool)> {
    let Some((start, end)) = criteria_section(body) else {
        return Vec::new();
    };
    body.lines()
        .enumerate()
        .filter(|(i, _)| *i >= start && *i < end)
        .filter_map(|(_, line)| CHECKBOX.captures(line))
        .map(|c| (c[2].trim().to_string(), &c[1] != " "))
        .collect()
}

/// Criterion text with runs of white space collapsed, for comparison.
fn squash(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The state of every criterion: those the profile requires, in its order,
/// then any the record adds. A required criterion that the record lacks is
/// unconfirmed, so deleting a line from the record never satisfies a gate.
fn criteria_state(required: &[&str], record: &Option<Vec<(String, bool)>>) -> Vec<CriterionState> {
    let Some(entries) = record else {
        return required
            .iter()
            .map(|text| CriterionState {
                text: text.to_string(),
                checked: false,
                missing: false,
            })
            .collect();
    };
    let mut out: Vec<CriterionState> = required
        .iter()
        .map(|text| {
            let wanted = squash(text);
            let found: Vec<bool> = entries
                .iter()
                .filter(|(entry, _)| squash(entry) == wanted)
                .map(|(_, checked)| *checked)
                .collect();
            CriterionState {
                text: text.to_string(),
                checked: !found.is_empty() && found.iter().all(|checked| *checked),
                missing: found.is_empty(),
            }
        })
        .collect();
    for (entry, checked) in entries {
        let own = squash(entry);
        if !required.iter().any(|text| squash(text) == own) {
            out.push(CriterionState {
                text: entry.clone(),
                checked: *checked,
                missing: false,
            });
        }
    }
    out
}

/// Rebuilds a body from its lines, keeping the final newline.
fn join_lines(lines: &[String]) -> String {
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Unticks every entry criterion, for a review that starts again.
fn clear_confirmations(body: &str) -> String {
    let Some((start, end)) = criteria_section(body) else {
        return body.to_string();
    };
    let lines: Vec<String> = body
        .lines()
        .enumerate()
        .map(|(i, line)| {
            if i >= start && i < end && CHECKBOX.is_match(line) {
                line.replacen("[x]", "[ ]", 1).replacen("[X]", "[ ]", 1)
            } else {
                line.to_string()
            }
        })
        .collect();
    join_lines(&lines)
}

/// Adds every required criterion the checklist lacks, unticked.
fn add_missing_criteria(body: &str, required: &[&str]) -> String {
    let present: Vec<String> = checklist(body)
        .iter()
        .map(|(text, _)| squash(text))
        .collect();
    let missing: Vec<String> = required
        .iter()
        .filter(|text| !present.contains(&squash(text)))
        .map(|text| format!("- [ ] {text}"))
        .collect();
    if missing.is_empty() {
        return body.to_string();
    }
    let mut lines: Vec<String> = body.lines().map(str::to_string).collect();
    let Some((start, end)) = criteria_section(body) else {
        lines.push(String::new());
        lines.push("## Entry criteria".to_string());
        lines.push(String::new());
        lines.extend(missing);
        return join_lines(&lines);
    };
    let end = end.min(lines.len());
    let last_item = (start..end).rev().find(|i| CHECKBOX.is_match(&lines[*i]));
    let mut at = match last_item {
        Some(i) => i + 1,
        // An empty checklist: keep one blank line under the heading.
        None if lines.get(start).is_some_and(|l| l.trim().is_empty()) => start + 1,
        None => start,
    };
    for line in missing {
        lines.insert(at, line);
        at += 1;
    }
    if lines.get(at).is_some_and(|l| !l.trim().is_empty()) {
        lines.insert(at, String::new());
    }
    join_lines(&lines)
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
        // A released record holds a decision. Another review starts in a new
        // revision, with the outcome pending and every criterion unconfirmed.
        let (current, revised) = if existing.is_released() {
            let text = docs::revise(existing, date)?;
            let reopened = Doc::parse(&project.root, &existing.path, text);
            let body = clear_confirmations(&set_outcome(reopened.body(), "Pending"));
            (format!("{}{body}", reopened.front_text()), true)
        } else {
            (existing.text.clone(), false)
        };
        let current = Doc::parse(&project.root, &existing.path, current);
        let required = project.profile().criteria(phase, project.tier());
        let body = add_missing_criteria(current.body(), &required);
        let body = replace_block(
            &replace_block(&body, "checks", &checks),
            "evidence",
            &evidence,
        );
        std::fs::write(&existing.path, format!("{}{body}", current.front_text()))?;
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
    /// A go recorded with `--force` although the gate was not ready.
    pub forced: bool,
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
    let record_id = record_doc.id().unwrap_or(&record_doc.rel).to_string();

    // Every refusal comes before the first write, so a refused decision
    // leaves the record and the project as they were.
    if record_doc.is_released() {
        bail!(
            "{record_id} is released and already holds a decision; run `jig gate open` to start another review"
        );
    }
    let blockers = check::release_blockers(record_doc, findings);
    if !blockers.is_empty() {
        bail!(
            "{record_id} cannot be released, so no decision is recorded:\n  {}",
            blockers.join("\n  ")
        );
    }
    let readiness = readiness(project, docs, findings, phase);
    let forced = req.outcome.advances() && !readiness.ready();
    if forced && !req.force {
        bail!(
            "{gate} is not ready for a go:\n  {}\nUse --force to record the decision anyway.",
            readiness.shortfalls().join("\n  ")
        );
    }

    let mut label = req.outcome.label().to_string();
    let mut outcome_text = label.clone();
    if forced {
        label.push_str(", forced");
        outcome_text = format!("{label} past {}", readiness.shortfall_summary());
    }
    outcome_text.push_str(&format!(" ({})", req.date));
    if let Some(note) = req.note.filter(|n| !n.trim().is_empty()) {
        outcome_text.push_str(&format!(". {}", note.trim()));
    }
    let body = replace_block(record_doc.body(), "checks", &checks_table(&readiness));
    let body = replace_block(&body, "evidence", &evidence_table(project, docs, phase));
    let body = set_outcome(&body, &outcome_text);
    let decided = Doc::parse(
        &project.root,
        &record_doc.path,
        format!("{}{body}", record_doc.front_text()),
    );
    let released = docs::release(
        &decided,
        &format!("Gate decision: {label}"),
        req.date,
        req.author,
    )?;
    std::fs::write(&record_doc.path, released)?;

    let mut closed = Closed {
        record: record_id,
        outcome: req.outcome,
        forced,
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

    const RECORD: &str = "# Review\n\n## 3. Entry criteria\n\n- [x] First criterion.\n- [ ] Second criterion.\n\n## 4. Automated checks\n\n- [x] not a criterion\n";

    #[test]
    fn a_required_criterion_missing_from_the_record_is_unconfirmed() {
        let entries = Some(checklist(RECORD));
        let state = criteria_state(&["First criterion.", "Third criterion."], &entries);
        assert_eq!(state.len(), 3);
        assert!(state[0].checked && !state[0].missing);
        assert!(
            !state[1].checked && state[1].missing,
            "the third criterion is required but absent"
        );
        assert_eq!(state[2].text, "Second criterion.");
        assert!(!state[2].checked && !state[2].missing);
    }

    #[test]
    fn an_empty_checklist_confirms_nothing() {
        let state = criteria_state(&["Only criterion."], &Some(Vec::new()));
        assert!(state.iter().all(|c| !c.checked && c.missing));
    }

    #[test]
    fn missing_criteria_are_added_unticked() {
        let out = add_missing_criteria(RECORD, &["First criterion.", "Third criterion."]);
        assert!(out.contains("- [ ] Second criterion.\n- [ ] Third criterion.\n\n## 4."));
        assert_eq!(add_missing_criteria(&out, &["Third criterion."]), out);
        let emptied = "# Review\n\n## 3. Entry criteria\n\n\n## 4. Automated checks\n";
        assert_eq!(
            add_missing_criteria(emptied, &["Only criterion."]),
            "# Review\n\n## 3. Entry criteria\n\n- [ ] Only criterion.\n\n## 4. Automated checks\n"
        );
    }

    #[test]
    fn confirmations_are_cleared_in_the_checklist_only() {
        let out = clear_confirmations(RECORD);
        assert!(out.contains("- [ ] First criterion."));
        assert!(out.contains("- [x] not a criterion"));
    }

    #[test]
    fn outcome_line_is_rewritten() {
        assert_eq!(
            set_outcome("x\n**Outcome:** Pending\ny", "Go (2026-10-01)"),
            "x\n**Outcome:** Go (2026-10-01)\ny\n"
        );
    }
}
