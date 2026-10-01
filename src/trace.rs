//! Requirements traceability: requirements from the SRS, test cases from the
//! V&V plan and results from test reports, joined into a matrix.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::docs::Doc;
use crate::markdown;

pub const METHODS: [&str; 4] = ["Inspection", "Analysis", "Demonstration", "Test"];
pub const RESULTS: [&str; 4] = ["Pass", "Fail", "Blocked", "Not run"];

static ITEM_HEADING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Z]+-\d{3,})\b\s*[-—:]?\s*(.*)$").unwrap());
static ATTRIBUTE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*[-*]\s+\*\*([^*:]+):\*\*\s*(.*)$").unwrap());
static REQ_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bREQ-\d{3,}\b").unwrap());

/// A requirement, test case or test result: a heading `### ID Title`, a
/// statement paragraph and `- **Attribute:** value` lines.
#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub line: usize,
    pub statement: String,
    pub attributes: Vec<(String, String)>,
}

impl Item {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
            .filter(|v| !v.is_empty())
    }
}

/// Items whose heading ID starts with `prefix-`, e.g. `REQ`.
pub fn items(body: &str, first_line: usize, prefix: &str) -> Vec<Item> {
    let lines = markdown::lines(body, first_line);
    let mut out: Vec<Item> = Vec::new();
    let mut current: Option<Item> = None;
    let mut in_statement = false;
    for line in &lines {
        if line.code {
            continue;
        }
        let heading = markdown::headings(line.text, line.no).into_iter().next();
        if let Some(h) = heading {
            if let Some(item) = current.take() {
                out.push(item);
            }
            if let Some(caps) = ITEM_HEADING.captures(&h.text)
                && caps[1].starts_with(&format!("{prefix}-"))
            {
                current = Some(Item {
                    id: caps[1].to_string(),
                    title: caps[2].trim().to_string(),
                    line: h.line,
                    statement: String::new(),
                    attributes: Vec::new(),
                });
                in_statement = true;
            }
            continue;
        }
        let Some(item) = current.as_mut() else {
            continue;
        };
        if let Some(caps) = ATTRIBUTE.captures(line.text) {
            item.attributes
                .push((caps[1].trim().to_string(), caps[2].trim().to_string()));
            in_statement = false;
        } else if line.text.trim().is_empty() {
            if !item.statement.is_empty() {
                in_statement = false;
            }
        } else if in_statement {
            if !item.statement.is_empty() {
                item.statement.push(' ');
            }
            item.statement.push_str(line.text.trim());
        }
    }
    if let Some(item) = current {
        out.push(item);
    }
    out
}

pub fn requirement_ids(text: &str) -> Vec<String> {
    REQ_ID
        .find_iter(text)
        .map(|m| m.as_str().to_string())
        .collect()
}

/// The attributes of a test case that define what it verifies and how.
const TEST_DEFINITION: [&str; 4] = ["Method", "Level", "Procedure", "Pass criteria"];

/// Text with runs of white space collapsed, so that rewrapping a paragraph
/// does not change a stamp.
fn squash(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A stamp of what a test case verifies: the test case as the plan defines
/// it, and the statement and verification method of each requirement it
/// names. A result records the stamp of the day it was obtained; when the
/// test case or a requirement changes, the stamps differ and the result is
/// stale. Titles, priorities and rationales do not count.
pub fn basis(test: &Item, requirements: &[Item]) -> String {
    let mut verified = test
        .attr("Verifies")
        .map(requirement_ids)
        .unwrap_or_default();
    verified.sort();
    verified.dedup();
    let mut text = format!("{}\n", test.id);
    for name in TEST_DEFINITION {
        text.push_str(&squash(test.attr(name).unwrap_or("")));
        text.push('\n');
    }
    for id in verified {
        text.push_str(&id);
        text.push('\n');
        if let Some(req) = requirements.iter().find(|r| r.id == id) {
            text.push_str(&squash(&req.statement));
            text.push('\n');
            text.push_str(req.attr("Verification").unwrap_or(""));
        }
        text.push('\n');
    }
    // FNV-1a, 64 bits: stable across versions and platforms, which a stamp
    // written into a document has to be.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")[..10].to_string()
}

/// The results section of a new test report: every test case of the plan,
/// not run, with today's basis stamp.
pub fn result_stubs(docs: &[Doc]) -> String {
    let requirements = requirements(docs);
    test_cases(docs)
        .iter()
        .map(|test| {
            format!(
                "### {} {}\n\n- **Result:** Not run\n- **Evidence:**\n- **Basis:** {}\n",
                test.id,
                test.title,
                basis(test, &requirements)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn requirements(docs: &[Doc]) -> Vec<Item> {
    docs.iter()
        .filter(|d| d.is_kind("srs"))
        .flat_map(|d| items(d.body(), d.body_line, "REQ"))
        .collect()
}

fn test_cases(docs: &[Doc]) -> Vec<Item> {
    docs.iter()
        .filter(|d| d.is_kind("vvp"))
        .flat_map(|d| items(d.body(), d.body_line, "TC"))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Coverage {
    /// No test case verifies the requirement.
    NotCovered,
    /// Test cases exist, without results for all of them.
    Planned,
    /// At least one test case failed or was blocked.
    Failed,
    /// A test case or a requirement changed after a result was recorded.
    Stale,
    /// Every test case passed.
    Verified,
}

impl Coverage {
    pub fn label(self) -> &'static str {
        match self {
            Coverage::NotCovered => "Not covered",
            Coverage::Planned => "Planned",
            Coverage::Failed => "Failed",
            Coverage::Stale => "Stale",
            Coverage::Verified => "Verified",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TestOutcome {
    pub test: String,
    pub result: String,
    pub report: String,
    pub released: bool,
    /// The result's basis stamp no longer matches the test case and its requirements.
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub requirement: String,
    pub title: String,
    pub method: String,
    pub tests: Vec<String>,
    pub outcomes: Vec<TestOutcome>,
    pub coverage: Coverage,
    /// Verified using only released test reports.
    pub verified_released: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Matrix {
    pub rows: Vec<Row>,
}

impl Matrix {
    pub fn build(docs: &[Doc]) -> Matrix {
        let requirements = requirements(docs);
        let tests = test_cases(docs);
        let current: BTreeMap<&str, String> = tests
            .iter()
            .map(|test| (test.id.as_str(), basis(test, &requirements)))
            .collect();

        // The latest report wins for each test case: reports sort by date, then ID.
        // "Not run" is the absence of a result, so it replaces nothing.
        let mut reports: Vec<&Doc> = docs.iter().filter(|d| d.is_kind("tr")).collect();
        reports.sort_by(|a, b| (a.date(), a.id()).cmp(&(b.date(), b.id())));
        let mut latest: BTreeMap<String, TestOutcome> = BTreeMap::new();
        for report in reports {
            for result in items(report.body(), report.body_line, "TC") {
                let Some(value) = result.attr("Result") else {
                    continue;
                };
                if value.eq_ignore_ascii_case("not run") {
                    continue;
                }
                // A result without a stamp is taken as it stands.
                let stale = match (result.attr("Basis"), current.get(result.id.as_str())) {
                    (Some(stamp), Some(now)) => stamp != now,
                    _ => false,
                };
                latest.insert(
                    result.id.clone(),
                    TestOutcome {
                        test: result.id.clone(),
                        result: value.to_string(),
                        report: report.id().unwrap_or(&report.rel).to_string(),
                        released: report.is_released(),
                        stale,
                    },
                );
            }
        }

        let rows = requirements
            .iter()
            .map(|req| {
                let covering: Vec<&Item> = tests
                    .iter()
                    .filter(|t| {
                        t.attr("Verifies")
                            .is_some_and(|v| requirement_ids(v).contains(&req.id))
                    })
                    .collect();
                let outcomes: Vec<TestOutcome> = covering
                    .iter()
                    .filter_map(|t| latest.get(&t.id).cloned())
                    .collect();
                let passed = |o: &TestOutcome| o.result.eq_ignore_ascii_case("pass");
                let coverage = if covering.is_empty() {
                    Coverage::NotCovered
                } else if outcomes.iter().any(|o| {
                    !o.stale
                        && (o.result.eq_ignore_ascii_case("fail")
                            || o.result.eq_ignore_ascii_case("blocked"))
                }) {
                    Coverage::Failed
                } else if outcomes.iter().any(|o| o.stale) {
                    Coverage::Stale
                } else if outcomes.len() == covering.len() && outcomes.iter().all(passed) {
                    Coverage::Verified
                } else {
                    Coverage::Planned
                };
                let verified_released =
                    coverage == Coverage::Verified && outcomes.iter().all(|o| o.released);
                Row {
                    requirement: req.id.clone(),
                    title: req.title.clone(),
                    method: req.attr("Verification").unwrap_or("").to_string(),
                    tests: covering.iter().map(|t| t.id.clone()).collect(),
                    outcomes,
                    coverage,
                    verified_released,
                }
            })
            .collect();
        Matrix { rows }
    }

    pub fn count(&self, coverage: Coverage) -> usize {
        self.rows.iter().filter(|r| r.coverage == coverage).count()
    }

    pub fn to_markdown(&self) -> String {
        let mut out = String::from(
            "| Requirement | Title | Method | Test cases | Results | Status |\n|---|---|---|---|---|---|\n",
        );
        for row in &self.rows {
            let tests = if row.tests.is_empty() {
                "None".to_string()
            } else {
                row.tests.join(", ")
            };
            let results = if row.outcomes.is_empty() {
                "None".to_string()
            } else {
                row.outcomes
                    .iter()
                    .map(|o| {
                        format!(
                            "{} {}{} ({})",
                            o.test,
                            o.result,
                            if o.stale { ", stale" } else { "" },
                            o.report
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            };
            out.push_str(&format!(
                "| {} | {} | {} | {tests} | {results} | {} |\n",
                row.requirement,
                row.title.replace('|', "\\|"),
                row.method,
                row.coverage.label()
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const SRS: &str = "---\nid: X-SRS\nkind: srs\n---\n# SRS\n\n## 3. Functional\n\n### REQ-001 Encipher\n\nWhen a key is pressed, the machine shall\nshow the letter.\n\n- **Verification:** Test\n- **Priority:** Must\n\n### REQ-002 Persist\n\nThe machine shall keep settings.\n\n- **Verification:** Demonstration\n\n### REQ-003 Uncovered\n\nThe machine shall do more.\n\n- **Verification:** Test\n";
    const VVP: &str = "---\nid: X-VVP\nkind: vvp\n---\n# VVP\n\n### TC-001 Keys\n\n- **Verifies:** REQ-001\n\n### TC-002 Settings\n\n- **Verifies:** REQ-002, REQ-001\n";
    const TR_OLD: &str = "---\nid: X-TR-001\nkind: tr\nstatus: released\ndate: 2026-01-01\n---\n# TR\n\n### TC-001 Keys\n\n- **Result:** Fail\n";
    const TR_NEW: &str = "---\nid: X-TR-002\nkind: tr\nstatus: draft\ndate: 2026-02-01\n---\n# TR\n\n### TC-001 Keys\n\n- **Result:** Pass\n\n### TC-002 Settings\n\n- **Result:** Pass\n";

    fn doc(name: &str, text: &str) -> Doc {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        std::fs::write(&path, text).unwrap();
        Doc::load(Path::new(dir.path()), &path).unwrap()
    }

    #[test]
    fn items_capture_statement_and_attributes() {
        let d = doc("srs.md", SRS);
        let reqs = items(d.body(), d.body_line, "REQ");
        assert_eq!(reqs.len(), 3);
        assert_eq!(reqs[0].id, "REQ-001");
        assert_eq!(reqs[0].title, "Encipher");
        assert_eq!(
            reqs[0].statement,
            "When a key is pressed, the machine shall show the letter."
        );
        assert_eq!(reqs[0].attr("verification"), Some("Test"));
        assert_eq!(reqs[0].line, 9);
    }

    #[test]
    fn matrix_joins_requirements_tests_and_latest_results() {
        let docs = vec![
            doc("srs.md", SRS),
            doc("vvp.md", VVP),
            doc("tr1.md", TR_OLD),
            doc("tr2.md", TR_NEW),
        ];
        let m = Matrix::build(&docs);
        assert_eq!(m.rows[0].tests, ["TC-001", "TC-002"]);
        assert_eq!(m.rows[0].coverage, Coverage::Verified);
        assert!(
            !m.rows[0].verified_released,
            "the passing report is still a draft"
        );
        assert_eq!(m.rows[1].coverage, Coverage::Verified);
        assert_eq!(m.rows[2].coverage, Coverage::NotCovered);
        assert!(
            m.to_markdown()
                .contains("| REQ-003 | Uncovered | Test | None | None | Not covered |")
        );
    }

    fn report(id: &str, date: &str, results: &str) -> String {
        format!("---\nid: {id}\nkind: tr\nstatus: released\ndate: {date}\n---\n# TR\n\n{results}")
    }

    #[test]
    fn a_result_goes_stale_when_what_it_verified_changes() {
        let srs = doc("srs.md", SRS);
        let vvp = doc("vvp.md", VVP);
        let stamp = basis(
            &items(vvp.body(), vvp.body_line, "TC")[1],
            &items(srs.body(), srs.body_line, "REQ"),
        );
        assert_eq!(stamp.len(), 10);
        let run = report(
            "X-TR-001",
            "2026-01-01",
            &format!("### TC-002 Settings\n\n- **Result:** Pass\n- **Basis:** {stamp}\n"),
        );
        let persist = |srs: &str, vvp: &str| {
            let docs = vec![doc("srs.md", srs), doc("vvp.md", vvp), doc("tr.md", &run)];
            Matrix::build(&docs).rows.remove(1)
        };

        let fresh = persist(SRS, VVP);
        assert_eq!(fresh.coverage, Coverage::Verified);
        assert!(fresh.verified_released);

        // Rewrapping the statement or touching another requirement changes nothing.
        let rewrapped = SRS
            .replace(
                "The machine shall keep settings.",
                "The machine shall\nkeep   settings.",
            )
            .replace("The machine shall do more.", "The machine shall do less.");
        assert_eq!(persist(&rewrapped, VVP).coverage, Coverage::Verified);

        let reworded = SRS.replace("keep settings.", "keep settings for a year.");
        let stale = persist(&reworded, VVP);
        assert_eq!(stale.coverage, Coverage::Stale);
        assert!(!stale.verified_released);
        assert!(stale.outcomes[0].stale);

        let retargeted = VVP.replace(
            "- **Verifies:** REQ-002, REQ-001",
            "- **Verifies:** REQ-002",
        );
        assert_eq!(persist(SRS, &retargeted).coverage, Coverage::Stale);
    }

    #[test]
    fn a_result_not_run_replaces_nothing() {
        let later = report(
            "X-TR-002",
            "2026-03-01",
            "### TC-001 Keys\n\n- **Result:** Not run\n",
        );
        let docs = vec![
            doc("srs.md", SRS),
            doc("vvp.md", VVP),
            doc("tr1.md", TR_OLD),
            doc("tr2.md", &later),
        ];
        let m = Matrix::build(&docs);
        assert_eq!(m.rows[0].outcomes[0].result, "Fail");
        assert_eq!(m.rows[0].coverage, Coverage::Failed);
    }

    #[test]
    fn new_reports_list_every_test_case_with_its_stamp() {
        let docs = vec![doc("srs.md", SRS), doc("vvp.md", VVP)];
        let stubs = result_stubs(&docs);
        assert!(stubs.starts_with(
            "### TC-001 Keys\n\n- **Result:** Not run\n- **Evidence:**\n- **Basis:** "
        ));
        assert_eq!(stubs.matches("- **Basis:** ").count(), 2);
    }

    #[test]
    fn missing_results_leave_coverage_planned() {
        let docs = vec![doc("srs.md", SRS), doc("vvp.md", VVP)];
        let m = Matrix::build(&docs);
        assert_eq!(m.rows[0].coverage, Coverage::Planned);
    }
}
