//! `jig check`: document control, the separation rule, requirement quality,
//! traceability references, diagrams and links.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::LazyLock;

use anyhow::Result;
use regex::Regex;
use serde::Serialize;

use crate::docs::{self, Doc, OPTIONAL_KEYS, REQUIRED_KEYS, STATUSES};
use crate::markdown;
use crate::project::Project;
use crate::render;
use crate::trace::{self, METHODS};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub severity: Severity,
    pub rule: &'static str,
    pub file: String,
    pub line: usize,
    pub message: String,
}

/// Phrases that only appear in teaching material. Always an error in a project repository.
pub const TEACHING_PHRASES: [&str; 14] = [
    r"you(?:'|’)ll learn",
    r"you will learn",
    r"we(?:'|’)ll learn",
    r"let(?:'|’)s",
    r"let us",
    r"notice (?:that|how)",
    r"your turn",
    r"exercises? for the reader",
    r"predict,? then",
    r"decode box(?:es)?",
    r"as a beginner",
    r"learning (?:goal|objective)s?",
    r"quiz",
    r"homework",
];

/// Words that usually signal teaching material but have legitimate uses. A warning.
pub const TEACHING_WORDS: [&str; 4] = [
    r"tutorial",
    r"walkthrough",
    r"try (?:it|this)",
    r"step-by-step guide",
];

/// Terms that make a requirement ambiguous or unverifiable (after the INCOSE Guide to Writing Requirements).
pub const VAGUE_TERMS: [&str; 24] = [
    "appropriate",
    "adequate",
    "as applicable",
    "as appropriate",
    "easy",
    "easily",
    "user-friendly",
    "fast",
    "quickly",
    "sufficient",
    "robust",
    "flexible",
    "approximately",
    "etc",
    "and/or",
    "if possible",
    "as far as possible",
    "minimize",
    "maximize",
    "optimal",
    "best",
    "typical",
    "several",
    "many",
];

fn alternation(words: &[&str]) -> Regex {
    Regex::new(&format!(r"(?i)\b(?:{})\b", words.join("|"))).unwrap()
}

static TEACHING: LazyLock<Regex> = LazyLock::new(|| alternation(&TEACHING_PHRASES));
static TEACHING_SOFT: LazyLock<Regex> = LazyLock::new(|| alternation(&TEACHING_WORDS));
static SECOND_PERSON: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\byou(?:r|rs|rself)?\b").unwrap());
static VAGUE: LazyLock<Regex> = LazyLock::new(|| {
    let escaped: Vec<String> = VAGUE_TERMS.iter().map(|t| regex::escape(t)).collect();
    Regex::new(&format!(r"(?i)\b(?:{})\b", escaped.join("|"))).unwrap()
});
static SHALL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bshall\b").unwrap());

pub struct Checker<'a> {
    project: &'a Project,
    /// Absolute paths whose mention in a project repository leaks private material.
    private_paths: Vec<String>,
    findings: Vec<Finding>,
}

/// Checks a project. `private_terms` are strings from the vault's `jig.toml`
/// that must never appear in a project repository.
pub fn check(
    project: &Project,
    vault_dir: Option<&Path>,
    private_terms: &[String],
) -> Result<Vec<Finding>> {
    let mut private_paths: Vec<String> = private_terms
        .iter()
        .filter(|t| !t.trim().is_empty())
        .cloned()
        .collect();
    if let Some(vault) = vault_dir {
        private_paths.push(vault.to_string_lossy().into_owned());
        if let Some(rel) = std::env::var_os("HOME")
            .and_then(|home| vault.strip_prefix(home).ok().map(Path::to_path_buf))
        {
            private_paths.push(format!("~/{}", rel.to_string_lossy()));
        }
    }
    let mut checker = Checker {
        project,
        private_paths,
        findings: Vec::new(),
    };
    let docs = docs::load_all(&project.root)?;
    for path in docs::markdown_files(&project.root) {
        let doc = Doc::load(&project.root, &path)?;
        let in_docs = doc.rel.starts_with(&format!("{}/", docs::DOCS_DIR));
        checker.separation(&doc, in_docs);
        checker.links(&doc);
        checker.diagrams(&doc);
        if in_docs {
            checker.document_control(&doc);
            checker.guide_comments(&doc);
        }
    }
    checker.unique_ids(&docs);
    checker.requirements(&docs);
    checker.test_cases(&docs);
    let mut findings = checker.findings;
    findings.sort_by(|a, b| (&a.file, a.line, b.severity).cmp(&(&b.file, b.line, a.severity)));
    Ok(findings)
}

impl Checker<'_> {
    fn add(
        &mut self,
        severity: Severity,
        rule: &'static str,
        doc: &Doc,
        line: usize,
        message: impl Into<String>,
    ) {
        self.findings.push(Finding {
            severity,
            rule,
            file: doc.rel.clone(),
            line,
            message: message.into(),
        });
    }

    fn document_control(&mut self, doc: &Doc) {
        if let Some(err) = &doc.front_error {
            self.add(
                Severity::Error,
                "front-matter.invalid",
                doc,
                err.line,
                err.message.clone(),
            );
            return;
        }
        let Some(front) = &doc.front else {
            self.add(
                Severity::Error,
                "front-matter.missing",
                doc,
                1,
                "documents under docs/ need document-control front matter",
            );
            return;
        };
        for key in REQUIRED_KEYS {
            if front.get(key).is_none_or(str::is_empty) {
                self.add(
                    Severity::Error,
                    "front-matter.required",
                    doc,
                    1,
                    format!("missing `{key}`"),
                );
            }
        }
        for key in front.keys() {
            if !REQUIRED_KEYS.contains(&key) && !OPTIONAL_KEYS.contains(&key) {
                self.add(
                    Severity::Warning,
                    "front-matter.unknown-key",
                    doc,
                    1,
                    format!("unknown key `{key}`"),
                );
            }
        }
        let status = doc.status();
        if !STATUSES.contains(&status) {
            self.add(
                Severity::Error,
                "status.invalid",
                doc,
                1,
                format!("status `{status}` is not one of {}", STATUSES.join(", ")),
            );
        }
        let revision = doc.revision();
        if !revision.is_empty() && !docs::valid_revision(revision) {
            self.add(
                Severity::Error,
                "revision.invalid",
                doc,
                1,
                format!(
                    "revision `{revision}` is not a Y14.35 letter (A-Y without I, O, Q, S, X, Z)"
                ),
            );
        }
        let date = doc.date();
        if !date.is_empty() && !docs::valid_date(date) {
            self.add(
                Severity::Error,
                "date.invalid",
                doc,
                1,
                format!("date `{date}` is not YYYY-MM-DD"),
            );
        }
        let Some(kind_name) = doc.kind() else { return };
        let Some(kind) = docs::kind_of(doc) else {
            self.add(
                Severity::Error,
                "kind.unknown",
                doc,
                1,
                format!("unknown kind `{kind_name}`; `jig explain kinds` lists them"),
            );
            return;
        };
        if let Some(id) = doc.id()
            && !docs::id_matches(&self.project.meta.code, kind, id)
        {
            self.add(
                Severity::Error,
                "id.format",
                doc,
                1,
                format!(
                    "id `{id}` does not follow the {} pattern for kind `{}`",
                    self.project.meta.code, kind.key
                ),
            );
        }
        if !docs::path_matches(kind, &doc.rel) {
            self.add(
                Severity::Warning,
                "path.location",
                doc,
                1,
                format!("a {} belongs at {}", kind.title.to_lowercase(), kind.path),
            );
        }
    }

    fn separation(&mut self, doc: &Doc, in_docs: bool) {
        let kind = docs::kind_of(doc);
        let end_user = kind.is_some_and(|k| k.end_user());
        for (line, text) in markdown::prose(doc.body(), doc.body_line) {
            if let Some(m) = TEACHING.find(&text) {
                self.add(
                    Severity::Error,
                    "separation.teaching",
                    doc,
                    line,
                    format!(
                        "teaching phrase `{}`: teaching material belongs in the vault",
                        m.as_str()
                    ),
                );
            }
            if let Some(m) = TEACHING_SOFT.find(&text)
                && !end_user
            {
                self.add(
                    Severity::Warning,
                    "separation.teaching",
                    doc,
                    line,
                    format!(
                        "`{}` usually signals teaching material, which belongs in the vault",
                        m.as_str()
                    ),
                );
            }
            if in_docs
                && !end_user
                && let Some(m) = SECOND_PERSON.find(&text)
            {
                self.add(
                    Severity::Warning,
                    "separation.second-person",
                    doc,
                    line,
                    format!("`{}`: engineering documents are impersonal", m.as_str()),
                );
            }
        }
        // Private references are checked everywhere, including code blocks and comments.
        for (i, line) in doc.text.lines().enumerate() {
            for private in &self.private_paths {
                if line.to_lowercase().contains(&private.to_lowercase()) {
                    self.findings.push(Finding {
                        severity: Severity::Error,
                        rule: "separation.private-reference",
                        file: doc.rel.clone(),
                        line: i + 1,
                        message: format!("mentions `{private}`, which is private"),
                    });
                }
            }
        }
    }

    fn links(&mut self, doc: &Doc) {
        // Templates link to files that exist only in the projects made from them.
        if doc.text.contains("{{") {
            return;
        }
        let dir = Path::new(&doc.rel)
            .parent()
            .unwrap_or(Path::new(""))
            .to_path_buf();
        for line in markdown::lines(doc.body(), doc.body_line) {
            if line.code {
                continue;
            }
            for link in markdown::links(line.text) {
                let target = link.target.split('#').next().unwrap_or("");
                if target.is_empty() || target.contains("://") || target.starts_with("mailto:") {
                    continue;
                }
                if target.starts_with('/') || target.starts_with('~') || target.starts_with("file:")
                {
                    self.add(
                        Severity::Error,
                        "separation.private-reference",
                        doc,
                        line.no,
                        format!("link to an absolute path `{target}`"),
                    );
                    continue;
                }
                match docs::normalize(&dir.join(target)) {
                    None => self.add(
                        Severity::Error,
                        "separation.private-reference",
                        doc,
                        line.no,
                        format!("link `{target}` leads outside the repository"),
                    ),
                    Some(resolved) => {
                        if !self.project.root.join(&resolved).exists() {
                            self.add(
                                Severity::Warning,
                                "link.broken",
                                doc,
                                line.no,
                                format!("link target `{target}` does not exist"),
                            );
                        }
                    }
                }
            }
        }
    }

    fn diagrams(&mut self, doc: &Doc) {
        for fence in markdown::fences(doc.body(), doc.body_line) {
            let result = match fence.lang.as_str() {
                "mermaid" => {
                    if fence.content.trim_start().starts_with("block-beta") {
                        self.add(Severity::Warning, "diagram.unsupported", doc, fence.line, "block-beta labels with spaces render incorrectly; use a flowchart with subgraphs");
                    }
                    render::mermaid_svg(&fence.content).map(|_| ())
                }
                "wavedrom" => render::wavedrom_svg(&fence.content).map(|_| ()),
                _ => continue,
            };
            if let Err(err) = result {
                self.add(
                    Severity::Error,
                    "diagram.render",
                    doc,
                    fence.line,
                    format!("{} diagram does not render: {err}", fence.lang),
                );
            }
        }
    }

    fn guide_comments(&mut self, doc: &Doc) {
        let severity = if doc.is_released() {
            Severity::Error
        } else {
            Severity::Warning
        };
        for line in markdown::guide_comments(doc.body(), doc.body_line) {
            self.add(
                severity,
                "template.guide",
                doc,
                line,
                "template guidance is still present; replace it with content",
            );
        }
    }

    fn unique_ids(&mut self, docs: &[Doc]) {
        let mut seen: BTreeMap<&str, &Doc> = BTreeMap::new();
        for doc in docs {
            let Some(id) = doc.id() else { continue };
            if let Some(first) = seen.get(id) {
                let message = format!("id `{id}` is also used by {}", first.rel);
                self.add(Severity::Error, "id.duplicate", doc, 1, message);
            } else {
                seen.insert(id, doc);
            }
        }
    }

    fn requirements(&mut self, docs: &[Doc]) {
        let mut seen = BTreeSet::new();
        for doc in docs.iter().filter(|d| d.is_kind("srs")) {
            for req in trace::items(doc.body(), doc.body_line, "REQ") {
                if !seen.insert(req.id.clone()) {
                    self.add(
                        Severity::Error,
                        "requirement.duplicate",
                        doc,
                        req.line,
                        format!("{} is defined more than once", req.id),
                    );
                }
                // Quoted terms in code spans are names, not requirement language.
                let statement = markdown::without_code_spans(&req.statement);
                let shalls = SHALL.find_iter(&statement).count();
                if shalls == 0 {
                    self.add(
                        Severity::Error,
                        "requirement.no-shall",
                        doc,
                        req.line,
                        format!("{}: the statement needs one `shall`", req.id),
                    );
                } else if shalls > 1 {
                    self.add(
                        Severity::Warning,
                        "requirement.not-singular",
                        doc,
                        req.line,
                        format!(
                            "{}: {shalls} `shall`s; split it into singular requirements",
                            req.id
                        ),
                    );
                }
                match req.attr("Verification") {
                    None => self.add(
                        Severity::Error,
                        "requirement.verification",
                        doc,
                        req.line,
                        format!(
                            "{}: missing **Verification:** ({})",
                            req.id,
                            METHODS.join(", ")
                        ),
                    ),
                    Some(method) if !METHODS.contains(&method) => self.add(
                        Severity::Error,
                        "requirement.verification",
                        doc,
                        req.line,
                        format!(
                            "{}: verification `{method}` is not one of {}",
                            req.id,
                            METHODS.join(", ")
                        ),
                    ),
                    Some(_) => {}
                }
                if let Some(m) = VAGUE.find(&statement) {
                    self.add(
                        Severity::Warning,
                        "requirement.vague",
                        doc,
                        req.line,
                        format!(
                            "{}: `{}` is vague; state a measurable condition",
                            req.id,
                            m.as_str()
                        ),
                    );
                }
            }
        }
    }

    fn test_cases(&mut self, docs: &[Doc]) {
        let known: BTreeSet<String> = docs
            .iter()
            .filter(|d| d.is_kind("srs"))
            .flat_map(|d| trace::items(d.body(), d.body_line, "REQ"))
            .map(|r| r.id)
            .collect();
        for doc in docs.iter().filter(|d| d.is_kind("vvp")) {
            for tc in trace::items(doc.body(), doc.body_line, "TC") {
                let verifies = tc
                    .attr("Verifies")
                    .map(trace::requirement_ids)
                    .unwrap_or_default();
                if verifies.is_empty() {
                    self.add(
                        Severity::Error,
                        "test.verifies",
                        doc,
                        tc.line,
                        format!("{}: **Verifies:** names no requirement", tc.id),
                    );
                }
                for id in verifies.iter().filter(|id| !known.contains(*id)) {
                    self.add(
                        Severity::Error,
                        "test.unknown-requirement",
                        doc,
                        tc.line,
                        format!("{}: verifies {id}, which the SRS does not define", tc.id),
                    );
                }
                if let Some(method) = tc.attr("Method")
                    && !METHODS.contains(&method)
                {
                    self.add(
                        Severity::Error,
                        "test.method",
                        doc,
                        tc.line,
                        format!(
                            "{}: method `{method}` is not one of {}",
                            tc.id,
                            METHODS.join(", ")
                        ),
                    );
                }
            }
        }
        for doc in docs.iter().filter(|d| d.is_kind("tr")) {
            for result in trace::items(doc.body(), doc.body_line, "TC") {
                if let Some(value) = result.attr("Result")
                    && !trace::RESULTS.contains(&value)
                {
                    self.add(
                        Severity::Error,
                        "test.result",
                        doc,
                        result.line,
                        format!(
                            "{}: result `{value}` is not one of {}",
                            result.id,
                            trace::RESULTS.join(", ")
                        ),
                    );
                }
            }
        }
    }
}

pub fn counts(findings: &[Finding]) -> (usize, usize) {
    let errors = findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    (errors, findings.len() - errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn teaching_phrases_match_whole_words_only() {
        assert!(TEACHING.is_match("Now let's wire the display."));
        assert!(TEACHING.is_match("Notice that the bus is idle."));
        assert!(!TEACHING.is_match("The outlet shall supply 5 V."));
        assert!(!TEACHING.is_match("Squizzed"));
        assert!(TEACHING_SOFT.is_match("Try it: cargo run"));
    }

    #[test]
    fn vague_terms_are_detected() {
        assert!(VAGUE.is_match("The display shall update quickly."));
        assert!(VAGUE.is_match("The unit shall log errors, warnings etc."));
        assert!(!VAGUE.is_match("The display shall update within 50 ms."));
    }

    #[test]
    fn normalize_refuses_to_leave_the_root() {
        assert_eq!(
            docs::normalize(Path::new("docs/../README.md")),
            Some(PathBuf::from("README.md"))
        );
        assert_eq!(docs::normalize(Path::new("docs/../../vault/x.md")), None);
    }
}
