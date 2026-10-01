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
/// An absolute path under a home directory, on macOS, Linux or Windows. The
/// character before it must end a word, so the path part of a URL is not one.
static HOME_PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?:^|[\s"'`(=\[<>,;:]|file://)((?:/(?:Users|home)/|[A-Za-z]:\\Users\\)[A-Za-z0-9._-]+)"#,
    )
    .unwrap()
});
/// A URI scheme at the start of a link target, such as `https:` or `mailto:`.
static SCHEME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Za-z][A-Za-z0-9+.-]*):").unwrap());

/// Files larger than this are not searched for private references.
const MAX_SCANNED_BYTES: usize = 4 * 1024 * 1024;

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
    let files = docs::project_files(&project.root, |rel| project.check.excludes(rel));
    for path in files
        .iter()
        .filter(|path| path.extension().is_some_and(|x| x == "md"))
    {
        let doc = Doc::load(&project.root, path)?;
        let in_docs = doc.rel.starts_with(&format!("{}/", docs::DOCS_DIR));
        checker.separation(&doc, in_docs);
        checker.links(&doc);
        checker.diagrams(&doc);
        if in_docs {
            checker.document_control(&doc);
            checker.guide_comments(&doc);
        }
    }
    checker.private_references(&files);
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
    }

    /// Private references in every text file, of any type and including code
    /// blocks and comments: the vault's path, the private terms, and absolute
    /// paths under a home directory.
    fn private_references(&mut self, files: &[std::path::PathBuf]) {
        let private: Vec<(String, String)> = self
            .private_paths
            .iter()
            .map(|term| (term.clone(), term.to_lowercase()))
            .collect();
        for path in files {
            let Ok(bytes) = std::fs::read(path) else {
                continue;
            };
            if bytes.len() > MAX_SCANNED_BYTES || bytes.contains(&0) {
                continue;
            }
            let Ok(text) = String::from_utf8(bytes) else {
                continue;
            };
            let file = path
                .strip_prefix(&self.project.root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            for (i, line) in text.lines().enumerate() {
                let lower = line.to_lowercase();
                let mut messages: Vec<String> = private
                    .iter()
                    .filter(|(_, needle)| lower.contains(needle))
                    .map(|(term, _)| format!("mentions `{term}`, which is private"))
                    .collect();
                if messages.is_empty()
                    && let Some(caps) = HOME_PATH.captures(line)
                {
                    messages.push(format!(
                        "absolute home path `{}`; a path on one machine does not belong in a repository",
                        &caps[1]
                    ));
                }
                for message in messages {
                    self.findings.push(Finding {
                        severity: Severity::Error,
                        rule: "separation.private-reference",
                        file: file.clone(),
                        line: i + 1,
                        message,
                    });
                }
            }
        }
    }

    fn links(&mut self, doc: &Doc) {
        // A file with `{{placeholders}}` is a template: its relative links
        // point at files that exist only in the projects made from it, so a
        // missing target is not reported. Links that leave the repository are.
        let template = doc.text.contains("{{");
        let dir = Path::new(&doc.rel)
            .parent()
            .unwrap_or(Path::new(""))
            .to_path_buf();
        for line in markdown::lines(doc.body(), doc.body_line) {
            if line.code {
                continue;
            }
            for link in markdown::link_targets(line.text) {
                let target = link.split('#').next().unwrap_or("");
                if target.is_empty() {
                    continue;
                }
                let absolute = match SCHEME.captures(target) {
                    // `file:` names a local path, and a one-letter scheme is a Windows drive.
                    Some(caps) => caps[1].eq_ignore_ascii_case("file") || caps[1].len() == 1,
                    None => target.starts_with('/') || target.starts_with('~'),
                };
                if absolute {
                    self.add(
                        Severity::Error,
                        "separation.private-reference",
                        doc,
                        line.no,
                        format!("link to an absolute path `{target}`"),
                    );
                    continue;
                }
                if SCHEME.is_match(target) || target.contains("{{") {
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
                        if !template && !self.project.root.join(&resolved).exists() {
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
                        Severity::Error,
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

/// Why a document cannot be released: the errors `jig check` reports in it,
/// and any template guidance left in its text.
pub fn release_blockers(doc: &Doc, findings: &[Finding]) -> Vec<String> {
    let mut blockers: Vec<String> = findings
        .iter()
        .filter(|f| {
            f.file == doc.rel && f.severity == Severity::Error && f.rule != "template.guide"
        })
        .map(|f| format!("{}:{}: {} [{}]", f.file, f.line, f.message, f.rule))
        .collect();
    let guides = markdown::guide_comments(doc.body(), doc.body_line);
    if !guides.is_empty() {
        let lines: Vec<String> = guides.iter().map(usize::to_string).collect();
        blockers.push(format!(
            "{}: template guidance remains at line{} {}",
            doc.rel,
            if lines.len() == 1 { "" } else { "s" },
            lines.join(", ")
        ));
    }
    blockers
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
    fn home_paths_are_detected_but_urls_are_not() {
        // Built from parts, so that this file holds no home path of its own.
        let mac = ["", "Users", "someone", "notes"].join("/");
        let linux = ["", "home", "someone", ".config"].join("/");
        assert!(HOME_PATH.is_match(&format!("see {mac}")));
        assert!(HOME_PATH.is_match(&format!("PATH=/usr/bin:{linux}/bin")));
        assert!(HOME_PATH.is_match(&format!("file://{mac}")));
        assert!(!HOME_PATH.is_match(&format!("https://example.com{linux}")));
        assert!(!HOME_PATH.is_match("the /Users/ directory on macOS"));
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
