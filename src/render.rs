//! PDF output. Each document becomes a Typst fragment (diagrams rendered in
//! process, links rewritten, then Pandoc with `bench.lua`), and Typst lays
//! fragments out with `bench.typ`: one document per PDF, or a gate package.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};

use crate::docs::{self, Doc};
use crate::markdown;
use crate::process::Phase;
use crate::project::Project;
use crate::templates;
use crate::trace::{Coverage, Matrix};

pub fn mermaid_svg(source: &str) -> Result<String> {
    mermaid_rs_renderer::render(source).map_err(|e| anyhow!("{e}"))
}

pub fn wavedrom_svg(source: &str) -> Result<String> {
    let mut out = Vec::new();
    wavedrom::render_json5(source, &mut out).map_err(|e| anyhow!("{e:?}"))?;
    Ok(String::from_utf8(out)?)
}

fn require_tool(name: &str, install: &str) -> Result<()> {
    match Command::new(name).arg("--version").output() {
        Ok(output) if output.status.success() => Ok(()),
        _ => bail!("`{name}` is needed for PDF output; install it with `{install}`"),
    }
}

pub fn require_tools() -> Result<()> {
    require_tool("pandoc", "brew install pandoc")?;
    require_tool("typst", "brew install typst")
}

/// The default output directory for rendered PDFs, ignored by git.
pub fn output_dir(project: &Project) -> PathBuf {
    project.root.join("build").join("pdf")
}

fn typst_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn typst_dict(pairs: &[(&str, &str)]) -> String {
    let fields: Vec<String> = pairs
        .iter()
        .map(|(k, v)| format!("{k}: {}", typst_str(v)))
        .collect();
    format!("({})", fields.join(", "))
}

/// Metadata for a document's title block, as a Typst dictionary.
struct DocMeta {
    id: String,
    title: String,
    revision: String,
    status: String,
    kind_title: String,
    date: String,
    author: String,
    gate: String,
}

impl DocMeta {
    fn of(doc: &Doc) -> DocMeta {
        DocMeta {
            id: doc.id().unwrap_or(&doc.rel).to_string(),
            title: doc.title().to_string(),
            revision: doc.revision().to_string(),
            status: doc.status().to_string(),
            kind_title: docs::kind_of(doc)
                .map(|k| k.title.clone())
                .unwrap_or_default(),
            date: doc.date().to_string(),
            author: doc.get("author").unwrap_or("").to_string(),
            gate: doc.get("gate").unwrap_or("").to_string(),
        }
    }

    fn typst(&self, project: &Project) -> String {
        typst_dict(&[
            ("id", &self.id),
            ("title", &self.title),
            ("revision", &self.revision),
            ("status", &self.status),
            ("kind-title", &self.kind_title),
            ("project", &project.meta.name),
            ("project-title", &project.display_title()),
            ("date", &self.date),
            ("author", &self.author),
            ("gate", &self.gate),
        ])
    }
}

/// A temporary directory holding the Typst sources of one PDF.
struct Build {
    dir: tempfile::TempDir,
}

impl Build {
    fn new() -> Result<Build> {
        let build = Build {
            dir: tempfile::tempdir()?,
        };
        build.write("bench.typ", templates::asset("bench.typ"))?;
        build.write("bench.lua", templates::asset("bench.lua"))?;
        Ok(build)
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn write(&self, name: &str, contents: &str) -> Result<()> {
        std::fs::write(self.path().join(name), contents).with_context(|| format!("writing {name}"))
    }

    /// Converts Markdown to a Typst fragment that imports the bench library.
    fn fragment(&self, index: usize, markdown: &str) -> Result<String> {
        let source = format!("doc-{index}.md");
        let body = format!("doc-{index}.body.typ");
        self.write(&source, markdown)?;
        let output = Command::new("pandoc")
            .current_dir(self.path())
            .args([
                "-f",
                "gfm",
                "-t",
                "typst",
                "--shift-heading-level-by=-1",
                "--columns=100000",
                "--lua-filter=bench.lua",
            ])
            .arg(format!("--metadata=bench-prefix:d{index}-"))
            .args([&source, "-o", &body])
            .output()
            .context("running pandoc")?;
        if !output.status.success() {
            bail!("pandoc failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        let typst = std::fs::read_to_string(self.path().join(&body))?;
        let name = format!("doc-{index}.typ");
        self.write(&name, &format!("#import \"bench.typ\": *\n{typst}"))?;
        Ok(name)
    }

    /// Prepares a project document: renders its diagrams and points links to
    /// other documents at their IDs.
    fn document(&self, index: usize, doc: &Doc, ids: &HashMap<String, String>) -> Result<String> {
        let body = self.diagrams(index, doc)?;
        let dir = Path::new(&doc.rel)
            .parent()
            .unwrap_or(Path::new(""))
            .to_path_buf();
        let body = markdown::rewrite_links(&body, |link| {
            if link.image {
                return None;
            }
            let (path, _) = link.target.split_once('#').unwrap_or((&link.target, ""));
            if !path.ends_with(".md") || path.contains("://") {
                return None;
            }
            let resolved = docs::normalize(&dir.join(path))?;
            let id = ids.get(&resolved.to_string_lossy().replace('\\', "/"))?;
            Some(format!("#doc:{id}"))
        });
        self.fragment(index, &body)
    }

    fn diagrams(&self, index: usize, doc: &Doc) -> Result<String> {
        let body = doc.body();
        let fences: Vec<_> = markdown::fences(body, 1)
            .into_iter()
            .filter(|f| f.lang == "mermaid" || f.lang == "wavedrom")
            .collect();
        let mut out = String::with_capacity(body.len());
        let mut skip_until = 0;
        for (i, line) in body.lines().enumerate() {
            let no = i + 1;
            if no <= skip_until {
                continue;
            }
            if let Some((n, fence)) = fences.iter().enumerate().find(|(_, f)| f.line == no) {
                let svg = match fence.lang.as_str() {
                    "mermaid" => mermaid_svg(&fence.content),
                    _ => wavedrom_svg(&fence.content),
                }
                .with_context(|| {
                    format!(
                        "{}:{}: {} diagram",
                        doc.rel,
                        doc.body_line + no - 1,
                        fence.lang
                    )
                })?;
                let name = format!("diagram-{index}-{}.svg", n + 1);
                self.write(&name, &svg)?;
                out.push_str(&format!("![Diagram {}]({name})\n", n + 1));
                skip_until = fence.end_line.unwrap_or(usize::MAX);
                continue;
            }
            out.push_str(line);
            out.push('\n');
        }
        Ok(out)
    }

    fn compile(&self, out: &Path) -> Result<()> {
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let output = Command::new("typst")
            .arg("compile")
            .arg("--root")
            .arg(self.path())
            .arg(self.path().join("main.typ"))
            .arg(out)
            .output()
            .context("running typst")?;
        if !output.status.success() {
            bail!("typst failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(())
    }
}

/// Maps each document's path to its ID, for links between documents.
fn id_map(docs: &[Doc]) -> HashMap<String, String> {
    docs.iter()
        .filter_map(|d| Some((d.rel.clone(), d.id()?.to_string())))
        .collect()
}

pub fn pdf(project: &Project, all: &[Doc], doc: &Doc, out: &Path) -> Result<()> {
    require_tools()?;
    let build = Build::new()?;
    let fragment = build.document(1, doc, &id_map(all))?;
    let meta = DocMeta::of(doc).typst(project);
    build.write(
        "main.typ",
        &format!(
            "#import \"bench.typ\": *\n#show: bench-doc.with({meta})\n#include \"{fragment}\"\n"
        ),
    )?;
    build.compile(out)
}

/// The order documents appear in a gate package after the gate review record.
const PACK_ORDER: [&str; 20] = [
    "pln", "con", "rsk", "srs", "arc", "icd", "spc", "fmea", "vvp", "tp", "tr", "fnd", "err",
    "eco", "rel", "mfg", "usr", "cmp", "spk", "adr",
];

/// Kinds in a gate package: those required at this gate or an earlier one,
/// plus decision records, spike, test and findings reports, specifications,
/// errata and change orders, which are evidence at any gate once they exist.
pub fn pack_kinds(project: &Project, phase: &Phase) -> BTreeSet<String> {
    let profile = project.profile();
    let tier = project.tier();
    let phases = profile.phases_for(tier);
    let upto = phases
        .iter()
        .position(|p| p.id == phase.id)
        .unwrap_or(phases.len().saturating_sub(1));
    let mut kinds: BTreeSet<String> = phases[..=upto]
        .iter()
        .flat_map(|p| profile.requirements(p, tier))
        .map(|r| r.kind.clone())
        .collect();
    kinds.extend(["adr", "spk", "spc", "tr", "fnd", "err", "eco"].map(String::from));
    kinds
}

/// The documents in a gate package: the gate's review record, then the
/// baseline documents in lifecycle order.
pub fn pack_contents<'a>(project: &Project, docs: &'a [Doc], phase: &Phase) -> Vec<&'a Doc> {
    let gate = phase.gate.as_deref().unwrap_or("");
    let kinds = pack_kinds(project, phase);
    let gate_record = docs
        .iter()
        .find(|d| d.is_kind("gr") && d.get("gate").is_some_and(|g| g.eq_ignore_ascii_case(gate)));
    let mut out: Vec<&Doc> = gate_record.into_iter().collect();
    for kind in PACK_ORDER.iter().filter(|k| kinds.contains(**k)) {
        let mut of_kind: Vec<&Doc> = docs.iter().filter(|d| d.is_kind(kind)).collect();
        of_kind.sort_by(|a, b| a.id().cmp(&b.id()));
        out.extend(of_kind);
    }
    out
}

fn rtm_markdown(matrix: &Matrix, date: &str) -> String {
    let mut out = format!(
        "# Requirements traceability matrix\n\n## 1. Purpose and scope\n\nThis matrix joins every system requirement to the test cases that verify it and their latest results, as recorded on {date}. It is generated from the system requirements specification, the verification and validation plan and the test reports.\n\n## 2. Summary\n\n| Status | Requirements |\n|---|---|\n"
    );
    for coverage in [
        Coverage::Verified,
        Coverage::Stale,
        Coverage::Planned,
        Coverage::Failed,
        Coverage::NotCovered,
    ] {
        out.push_str(&format!(
            "| {} | {} |\n",
            coverage.label(),
            matrix.count(coverage)
        ));
    }
    out.push_str("\n## 3. Matrix\n\n");
    out.push_str(&matrix.to_markdown());
    out
}

pub struct PackInfo {
    pub gate: String,
    pub included: Vec<String>,
}

pub fn pack(
    project: &Project,
    all: &[Doc],
    gate: &str,
    date: &str,
    author: &str,
    out: &Path,
) -> Result<PackInfo> {
    require_tools()?;
    let profile = project.profile();
    let phase = profile
        .phase_by_gate(gate)
        .ok_or_else(|| anyhow!("gate {gate} does not exist for kind {}", project.meta.kind))?;
    let gate = phase.gate.clone().unwrap_or_default();
    let gate_title = phase.gate_title.clone().unwrap_or_default();

    let build = Build::new()?;
    let ids = id_map(all);
    let contents = pack_contents(project, all, phase);
    let mut metas = Vec::new();
    let mut sections = String::new();
    let mut included = Vec::new();
    for (i, doc) in contents.iter().enumerate() {
        let fragment = build.document(i + 1, doc, &ids)?;
        let meta = DocMeta::of(doc).typst(project);
        sections.push_str(&format!("#doc-section({meta})[#include \"{fragment}\"]\n"));
        metas.push(meta);
        included.push(DocMeta::of(doc).id);
    }

    if pack_kinds(project, phase).contains("srs") && all.iter().any(|d| d.is_kind("srs")) {
        let matrix = Matrix::build(all);
        let fragment = build.fragment(contents.len() + 1, &rtm_markdown(&matrix, date))?;
        let meta = DocMeta {
            id: format!("{}-RTM", project.meta.code),
            title: "Requirements traceability matrix".into(),
            revision: String::new(),
            status: "generated".into(),
            kind_title: "Generated report".into(),
            date: date.into(),
            author: author.into(),
            gate: gate.clone(),
        };
        included.push(meta.id.clone());
        let meta = meta.typst(project);
        sections.push_str(&format!("#doc-section({meta})[#include \"{fragment}\"]\n"));
        metas.push(meta);
    }

    // A trailing comma keeps a one-element list an array in Typst.
    let docs_array = if metas.is_empty() {
        String::new()
    } else {
        format!("{},", metas.join(", "))
    };
    let pack = format!(
        "(project: {}, project-title: {}, gate: {}, title: {}, gate-title: {}, question: {}, phase: {}, date: {}, author: {}, docs: ({}))",
        typst_str(&project.meta.name),
        typst_str(&project.display_title()),
        typst_str(&gate),
        typst_str(&gate_title),
        typst_str(&format!("{gate}: {gate_title}")),
        typst_str(phase.question.as_deref().unwrap_or("")),
        typst_str(&phase.label()),
        typst_str(date),
        typst_str(author),
        docs_array,
    );
    build.write(
        "main.typ",
        &format!("#import \"bench.typ\": *\n#show: bench-pack.with({pack})\n{sections}"),
    )?;
    build.compile(out)?;
    Ok(PackInfo { gate, included })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typst_strings_are_escaped() {
        assert_eq!(typst_str(r#"a "b" \c"#), r#""a \"b\" \\c""#);
    }

    #[test]
    fn mermaid_flowcharts_render() {
        assert!(
            mermaid_svg("flowchart LR\n  a --> b\n")
                .unwrap()
                .contains("<svg")
        );
    }

    #[test]
    fn wavedrom_signals_render() {
        let svg = wavedrom_svg("{ signal: [{ name: 'clk', wave: 'p....' }] }").unwrap();
        assert!(svg.contains("<svg"));
    }
}
