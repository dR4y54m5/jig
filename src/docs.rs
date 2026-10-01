//! Engineering documents: loading, IDs, creation from templates, and the
//! release and revision lifecycle.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use walkdir::WalkDir;

use crate::frontmatter::{self, FrontMatter, ParseError};
use crate::markdown;
use crate::process::{Kind, Naming, Process};
use crate::project::Project;
use crate::templates;

pub const DOCS_DIR: &str = "docs";
pub const REQUIRED_KEYS: [&str; 6] = ["id", "title", "kind", "revision", "status", "date"];
pub const OPTIONAL_KEYS: [&str; 4] = ["author", "gate", "supersedes", "superseded-by"];
pub const STATUSES: [&str; 4] = ["draft", "in-review", "released", "superseded"];
/// Revision letters per ASME Y14.35: A to Y, omitting I, O, Q, S, X and Z.
pub const REVISION_LETTERS: &str = "ABCDEFGHJKLMNPRTUVWY";

#[derive(Debug)]
pub struct Doc {
    pub path: PathBuf,
    /// Path relative to the project root, with forward slashes.
    pub rel: String,
    pub text: String,
    pub front: Option<FrontMatter>,
    pub front_error: Option<ParseError>,
    body_start: usize,
    pub body_line: usize,
}

impl Doc {
    pub fn load(root: &Path, path: &Path) -> Result<Doc> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Ok(Doc::parse(root, path, text))
    }

    /// A document from text that is not, or not yet, on disk at `path`.
    pub fn parse(root: &Path, path: &Path, text: String) -> Doc {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let (front, front_error, body_start, body_line) = match frontmatter::split(&text) {
            Ok(split) => {
                let start = text.len() - split.body.len();
                (split.front, None, start, split.body_line)
            }
            Err(err) => (None, Some(err), 0, 1),
        };
        Doc {
            path: path.to_path_buf(),
            rel,
            text,
            front,
            front_error,
            body_start,
            body_line,
        }
    }

    /// The front matter as written in the file, ending with its closing line.
    pub fn front_text(&self) -> &str {
        &self.text[..self.body_start]
    }

    pub fn body(&self) -> &str {
        &self.text[self.body_start..]
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.front.as_ref()?.get(key)
    }

    pub fn id(&self) -> Option<&str> {
        self.get("id")
    }

    pub fn kind(&self) -> Option<&str> {
        self.get("kind")
    }

    pub fn title(&self) -> &str {
        self.get("title").unwrap_or(&self.rel)
    }

    pub fn status(&self) -> &str {
        self.get("status").unwrap_or("draft")
    }

    pub fn revision(&self) -> &str {
        self.get("revision").unwrap_or("")
    }

    pub fn date(&self) -> &str {
        self.get("date").unwrap_or("")
    }

    pub fn is_released(&self) -> bool {
        self.status() == "released"
    }

    pub fn is_kind(&self, key: &str) -> bool {
        self.kind() == Some(key)
    }
}

/// Every Markdown document under `docs/`, sorted by path.
pub fn load_all(root: &Path) -> Result<Vec<Doc>> {
    let dir = root.join(DOCS_DIR);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut docs = Vec::new();
    for entry in WalkDir::new(&dir).sort_by_file_name() {
        let entry = entry?;
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|e| e == "md") {
            docs.push(Doc::load(root, entry.path())?);
        }
    }
    Ok(docs)
}

/// Every Markdown file in a repository, skipping hidden directories and build output.
pub fn markdown_files(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            e.depth() == 0 || !(name.starts_with('.') || name == "target" || name == "node_modules")
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.path().extension().is_some_and(|x| x == "md"))
        .map(|e| e.into_path())
        .collect()
}

pub fn valid_revision(rev: &str) -> bool {
    matches!(rev.len(), 1 | 2) && rev.chars().all(|c| REVISION_LETTERS.contains(c))
}

/// The revision after `rev`: A → B, Y → AA, AY → BA.
pub fn next_revision(rev: &str) -> Option<String> {
    let letters: Vec<char> = REVISION_LETTERS.chars().collect();
    let index = |c: char| letters.iter().position(|&l| l == c);
    let chars: Vec<char> = rev.chars().collect();
    match chars.as_slice() {
        [c] => {
            let i = index(*c)?;
            Some(match letters.get(i + 1) {
                Some(next) => next.to_string(),
                None => format!("{0}{0}", letters[0]),
            })
        }
        [a, b] => {
            let (i, j) = (index(*a)?, index(*b)?);
            if let Some(next) = letters.get(j + 1) {
                Some(format!("{a}{next}"))
            } else {
                letters
                    .get(i + 1)
                    .map(|next| format!("{next}{}", letters[0]))
            }
        }
        _ => None,
    }
}

pub fn valid_date(date: &str) -> bool {
    date.parse::<jiff::civil::Date>().is_ok()
}

/// Whether `id` follows the ID pattern for `kind` in a project with `code`.
pub fn id_matches(code: &str, kind: &Kind, id: &str) -> bool {
    let prefix = format!("{code}-{}", kind.code);
    let Some(rest) = id.strip_prefix(&prefix) else {
        return false;
    };
    match kind.naming {
        Naming::Single => rest.is_empty(),
        Naming::Numbered => {
            rest.len() == 4
                && rest.starts_with('-')
                && rest[1..].chars().all(|c| c.is_ascii_digit())
        }
        Naming::Gate => {
            rest.len() > 1
                && rest.starts_with('-')
                && rest[1..]
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        }
        Naming::Date => rest.strip_prefix('-').is_some_and(valid_date),
    }
}

/// Whether a document of `kind` is stored where the process expects it.
pub fn path_matches(kind: &Kind, rel: &str) -> bool {
    match kind.naming {
        Naming::Single => rel == kind.path,
        _ => rel
            .strip_prefix(&kind.path)
            .and_then(|r| r.strip_prefix('/'))
            .is_some_and(|name| !name.contains('/')),
    }
}

/// Resolves `.` and `..` without touching the file system. `None` if the path
/// climbs above its root or is absolute.
pub fn normalize(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            std::path::Component::Normal(part) => out.push(part),
            _ => return None,
        }
    }
    Some(out)
}

pub fn slugify(title: &str) -> String {
    let mut slug = String::new();
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let mut slug = slug.trim_end_matches('-').to_string();
    if slug.len() > 50 {
        // Cut at the last word boundary that fits, so no word is split.
        let cut = slug[..=50].rfind('-').unwrap_or(50);
        slug.truncate(cut);
    }
    slug
}

pub fn today() -> String {
    jiff::Zoned::now().date().to_string()
}

/// The author recorded on documents: git's `user.name`, else `$USER`.
pub fn author(root: &Path) -> String {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["config", "user.name"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("USER").ok())
        .unwrap_or_else(|| "unknown".to_string())
}

/// A document ready to be written.
#[derive(Debug)]
pub struct Draft {
    pub path: PathBuf,
    pub rel: String,
    pub id: String,
    pub text: String,
}

pub struct DocRequest<'a> {
    pub kind: &'a Kind,
    pub title: Option<String>,
    /// The gate a gate review record belongs to.
    pub gate: Option<String>,
    pub date: String,
    pub author: String,
    /// Extra template values, e.g. a gate review's criteria.
    pub vars: Vec<(String, String)>,
}

/// Builds a new document from its kind's template.
pub fn draft(project: &Project, existing: &[Doc], req: DocRequest) -> Result<Draft> {
    let kind = req.kind;
    let code = &project.meta.code;
    let profile = project.profile();
    let (id, rel, title) = match kind.naming {
        Naming::Single => {
            let title = req.title.clone().unwrap_or_else(|| kind.title.clone());
            (format!("{code}-{}", kind.code), kind.path.clone(), title)
        }
        Naming::Numbered => {
            let title = req
                .title
                .clone()
                .ok_or_else(|| anyhow!("a {} needs a title", kind.title.to_lowercase()))?;
            let number = existing
                .iter()
                .filter(|d| d.is_kind(&kind.key))
                .filter_map(|d| d.id()?.rsplit('-').next()?.parse::<u32>().ok())
                .max()
                .unwrap_or(0)
                + 1;
            let slug = slugify(&title);
            let rel = format!("{}/{}-{number:03}-{slug}.md", kind.path, kind.code);
            (format!("{code}-{}-{number:03}", kind.code), rel, title)
        }
        Naming::Gate => {
            let gate = req
                .gate
                .clone()
                .ok_or_else(|| anyhow!("a gate review record needs a gate"))?
                .to_uppercase();
            let phase = profile.phase_by_gate(&gate).ok_or_else(|| {
                anyhow!("gate {gate} does not exist for kind {}", project.meta.kind)
            })?;
            let title = req.title.clone().unwrap_or_else(|| {
                format!("{} ({gate})", phase.gate_title.clone().unwrap_or_default())
            });
            (
                format!("{code}-{}-{gate}", kind.code),
                format!("{}/{}-{gate}.md", kind.path, kind.code),
                title,
            )
        }
        Naming::Date => {
            let title = req
                .title
                .clone()
                .unwrap_or_else(|| format!("Engineering notebook {}", req.date));
            (
                format!("{code}-{}-{}", kind.code, req.date),
                format!("{}/{}.md", kind.path, req.date),
                title,
            )
        }
    };

    if existing
        .iter()
        .any(|d| d.id() == Some(id.as_str()) || d.rel == rel)
    {
        bail!("{id} already exists ({rel})");
    }

    let gate = match kind.naming {
        Naming::Gate => req.gate.clone().map(|g| g.to_uppercase()),
        _ => profile
            .baseline_gate(&kind.key, project.tier())
            .map(str::to_string)
            .or_else(|| project.phase().gate.clone()),
    }
    .unwrap_or_default();

    let mut front = FrontMatter::default();
    front.set("id", id.clone());
    front.set("title", title.clone());
    front.set("kind", kind.key.clone());
    front.set("revision", "A");
    front.set("status", "draft");
    front.set("date", req.date.clone());
    front.set("author", req.author.clone());
    front.set("gate", gate.clone());

    let template = templates::doc_template(&kind.key)
        .ok_or_else(|| anyhow!("no template for {}", kind.key))?;
    let lifecycle = lifecycle_table(project);
    let mut vars: Vec<(&str, &str)> = vec![
        ("title", &title),
        ("name", &project.meta.name),
        ("code", code),
        ("project_title", &project.meta.title),
        ("profile_title", &profile.title),
        ("tier", project.tier().unwrap_or("n/a")),
        ("visibility", &project.meta.visibility),
        ("date", &req.date),
        ("author", &req.author),
        ("gate", &gate),
        ("lifecycle", &lifecycle),
    ];
    vars.extend(req.vars.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    let body = templates::fill(template, &vars);

    Ok(Draft {
        path: project.root.join(&rel),
        rel,
        id,
        text: format!("{}\n{body}", front.to_yaml()),
    })
}

/// The project's phases and gates as a Markdown table.
pub fn lifecycle_table(project: &Project) -> String {
    let profile = project.profile();
    let mut out = String::from("| Phase | Gate | The gate answers |\n|---|---|---|\n");
    for phase in profile.phases_for(project.tier()) {
        let gate = match (&phase.gate, &phase.gate_title) {
            (Some(gate), Some(title)) => format!("{gate}: {title}"),
            (Some(gate), None) => gate.clone(),
            _ => "None".to_string(),
        };
        out.push_str(&format!(
            "| {} | {gate} | {} |\n",
            phase.label(),
            phase.question.as_deref().unwrap_or("")
        ));
    }
    out
}

/// A released document: status `released`, today's date, and a revision history row.
pub fn release(doc: &Doc, note: &str, date: &str, author: &str) -> Result<String> {
    let mut front = doc
        .front
        .clone()
        .ok_or_else(|| anyhow!("{} has no front matter", doc.rel))?;
    match doc.status() {
        "draft" | "in-review" => {}
        "released" => bail!(
            "{} is already released at revision {}; run `jig doc revise` first",
            doc.id().unwrap_or(&doc.rel),
            doc.revision()
        ),
        other => bail!(
            "{} has status {other} and cannot be released",
            doc.id().unwrap_or(&doc.rel)
        ),
    }
    let revision = doc.revision().to_string();
    front.set("status", "released");
    front.set("date", date);
    let row = format!(
        "| {revision} | {date} | {} | {author} |",
        note.replace('|', "\\|")
    );
    Ok(format!(
        "{}{}",
        front.to_yaml(),
        append_history_row(doc.body(), &row)
    ))
}

/// Opens the next revision of a released document as a draft.
pub fn revise(doc: &Doc, date: &str) -> Result<String> {
    let mut front = doc
        .front
        .clone()
        .ok_or_else(|| anyhow!("{} has no front matter", doc.rel))?;
    if !doc.is_released() {
        bail!(
            "{} is {}, not released; edit the draft directly",
            doc.id().unwrap_or(&doc.rel),
            doc.status()
        );
    }
    let next = next_revision(doc.revision())
        .ok_or_else(|| anyhow!("{} has an invalid revision `{}`", doc.rel, doc.revision()))?;
    front.set("revision", next);
    front.set("status", "draft");
    front.set("date", date);
    Ok(format!("{}{}", front.to_yaml(), doc.body()))
}

fn append_history_row(body: &str, row: &str) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let heading = markdown::headings(body, 1)
        .into_iter()
        .rfind(|h| markdown::section_name(&h.text) == "revision history");
    let Some(heading) = heading else {
        let mut out = body.trim_end().to_string();
        out.push_str(
            "\n\n## Revision history\n\n| Rev | Date | Description | Author |\n|---|---|---|---|\n",
        );
        out.push_str(row);
        out.push('\n');
        return out;
    };
    let start = heading.line; // 1-based heading line == index of the next line
    let table_start = lines[start..]
        .iter()
        .position(|l| l.trim_start().starts_with('|'))
        .map(|i| start + i);
    let insert_at = match table_start {
        Some(first) => {
            let len = lines[first..]
                .iter()
                .take_while(|l| l.trim_start().starts_with('|'))
                .count();
            first + len
        }
        None => {
            let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
            out.insert(start, String::new());
            out.insert(start + 1, "| Rev | Date | Description | Author |".into());
            out.insert(start + 2, "|---|---|---|---|".into());
            out.insert(start + 3, row.to_string());
            return out.join("\n") + "\n";
        }
    };
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    out.insert(insert_at, row.to_string());
    out.join("\n") + "\n"
}

/// The kind of a document, if it names a known one.
pub fn kind_of(doc: &Doc) -> Option<&'static Kind> {
    Process::get()
        .kind(doc.kind()?)
        .filter(|k| Some(k.key.as_str()) == doc.kind())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revision_letters_skip_confusable_letters() {
        assert!(valid_revision("A") && valid_revision("AB"));
        assert!(
            !valid_revision("I")
                && !valid_revision("O")
                && !valid_revision("a")
                && !valid_revision("ABC")
        );
        assert_eq!(next_revision("A").unwrap(), "B");
        assert_eq!(next_revision("H").unwrap(), "J");
        assert_eq!(next_revision("Y").unwrap(), "AA");
        assert_eq!(next_revision("AY").unwrap(), "BA");
        assert_eq!(next_revision("YY"), None);
    }

    #[test]
    fn ids_follow_the_naming_of_their_kind() {
        let process = Process::get();
        let srs = process.kind("srs").unwrap();
        let adr = process.kind("adr").unwrap();
        let gr = process.kind("gr").unwrap();
        let log = process.kind("log").unwrap();
        assert!(id_matches("EM4", srs, "EM4-SRS"));
        assert!(!id_matches("EM4", srs, "EM4-SRS-001"));
        assert!(id_matches("EM4", adr, "EM4-ADR-004"));
        assert!(!id_matches("EM4", adr, "EM4-ADR-4"));
        assert!(id_matches("EM4", gr, "EM4-GR-SRR"));
        assert!(id_matches("EM4", log, "EM4-LOG-2026-10-01"));
        assert!(!id_matches("EM4", log, "EM4-LOG-2026-13-01"));
    }

    #[test]
    fn paths_follow_the_naming_of_their_kind() {
        let process = Process::get();
        assert!(path_matches(
            process.kind("srs").unwrap(),
            "docs/requirements.md"
        ));
        assert!(path_matches(
            process.kind("adr").unwrap(),
            "docs/decisions/ADR-001-x.md"
        ));
        assert!(!path_matches(
            process.kind("adr").unwrap(),
            "docs/decisions/old/ADR-001-x.md"
        ));
    }

    #[test]
    fn slugs_are_short_lowercase_words() {
        assert_eq!(slugify("Share one no_std core!"), "share-one-no-std-core");
        assert!(slugify(&"word ".repeat(30)).len() <= 50);
        assert_eq!(
            slugify("Drive an SSD1306 OLED over I2C for the rotor windows"),
            "drive-an-ssd1306-oled-over-i2c-for-the-rotor"
        );
    }

    #[test]
    fn release_appends_to_the_revision_history_table() {
        let body = "# T\n\n## 3. Revision history\n\n| Rev | Date | Description | Author |\n|---|---|---|---|\n\n## Appendix\n";
        let out = append_history_row(body, "| A | 2026-10-01 | First | me |");
        assert!(out.contains("|---|---|---|---|\n| A | 2026-10-01 | First | me |\n\n## Appendix"));
    }

    #[test]
    fn release_adds_a_history_section_when_missing() {
        let out = append_history_row("# T\n\nText.\n", "| A | d | n | a |");
        assert!(out.ends_with("## Revision history\n\n| Rev | Date | Description | Author |\n|---|---|---|---|\n| A | d | n | a |\n"));
    }
}
