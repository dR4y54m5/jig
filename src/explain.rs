//! `jig explain`: the process rules, read from the same data the other
//! commands enforce, so there is one source of truth.

use anyhow::{Result, anyhow, bail};

use crate::check::{TEACHING_PHRASES, TEACHING_WORDS, VAGUE_TERMS};
use crate::docs::REVISION_LETTERS;
use crate::process::{DocState, Naming, Process, Profile};
use crate::trace::METHODS;

pub const TOPICS: &str = "Topics:
  kinds            every document kind
  <kind>           one kind, e.g. `srs` or `ADR`
  profiles         the project kinds and their tiers
  phases           the lifecycle of a project kind (--kind, or the current project)
  <gate>           one gate review, e.g. `SRR` (--kind, or the current project)
  separation       what belongs in a project repository and what belongs in the vault
  requirements     how requirements and test cases are written
  ids              document IDs, revisions and statuses";

pub fn explain(
    topic: Option<&str>,
    profile: Option<&Profile>,
    tier: Option<&str>,
) -> Result<String> {
    let process = Process::get();
    let Some(topic) = topic else {
        return Ok(TOPICS.to_string());
    };
    let default_profile =
        || profile.unwrap_or_else(|| process.profile("product").expect("product profile"));
    Ok(match topic.to_lowercase().as_str() {
        "kinds" => kinds(),
        "profiles" => profiles(),
        "phases" => phases(default_profile(), tier),
        "separation" => separation(),
        "requirements" => requirements(),
        "ids" => ids(),
        _ => {
            if let Some(kind) = process.kind(topic) {
                kind_detail(kind)
            } else if let Some(phase) = default_profile().phase_by_gate(topic) {
                gate_detail(default_profile(), phase, tier)
            } else {
                bail!("unknown topic `{topic}`\n\n{TOPICS}")
            }
        }
    })
}

fn kinds() -> String {
    let mut out =
        String::from("Document kinds (front-matter `kind`, ID code, ISO/IEC/IEEE 15289 type):\n\n");
    for k in &Process::get().kinds {
        out.push_str(&format!(
            "  {:<5} {:<5} {:<14} {}\n",
            k.key, k.code, k.doc_type, k.title
        ));
    }
    out.push_str("\n`jig explain <kind>` describes one kind.");
    out
}

fn kind_detail(kind: &crate::process::Kind) -> String {
    let naming = match kind.naming {
        Naming::Single => format!("one per project, at {}", kind.path),
        Naming::Numbered => format!("numbered, in {}/{}-NNN-slug.md", kind.path, kind.code),
        Naming::Gate => format!("one per gate, in {}/{}-<GATE>.md", kind.path, kind.code),
        Naming::Date => format!("one per day, in {}/YYYY-MM-DD.md", kind.path),
    };
    let mut out = format!(
        "{} ({}, kind `{}`)\n\nPurpose: {}\nType:    {} (ISO/IEC/IEEE 15289)\nBasis:   {}\nFiles:   {}\n",
        kind.title, kind.code, kind.key, kind.purpose, kind.doc_type, kind.basis, naming
    );
    if kind.procedure {
        out.push_str("Style:   numbered imperative steps are expected.\n");
    }
    if kind.end_user() {
        out.push_str("Style:   written for end users, who may be addressed as \"you\".\n");
    }
    out.push_str("\nRequired by:\n");
    let mut any = false;
    for profile in &Process::get().profiles {
        for phase in &profile.phases {
            for req in phase.require.iter().filter(|r| r.kind == kind.key) {
                any = true;
                let state = if req.state == DocState::Released {
                    "released"
                } else {
                    "exists"
                };
                let tier = req
                    .min_tier
                    .as_deref()
                    .or(phase.min_tier.as_deref())
                    .map(|t| format!(", {t} tier and up"))
                    .unwrap_or_default();
                out.push_str(&format!(
                    "  {:<9} {} gate: {state}{tier}\n",
                    profile.kind,
                    phase.gate.as_deref().unwrap_or("-")
                ));
            }
        }
    }
    if !any {
        out.push_str("  no gate; written when needed\n");
    }
    out
}

fn profiles() -> String {
    let mut out = String::from("Project kinds:\n\n");
    for p in &Process::get().profiles {
        let tiers = if p.tiers.is_empty() {
            String::new()
        } else {
            format!(" Tiers: {}.", p.tiers.join(", "))
        };
        out.push_str(&format!(
            "  {:<9} {}. {}{}\n",
            p.kind, p.title, p.description, tiers
        ));
    }
    out
}

fn phases(profile: &Profile, tier: Option<&str>) -> String {
    let tier_note = tier.map(|t| format!(", {t} tier")).unwrap_or_default();
    let mut out = format!("Lifecycle of a {} project{tier_note}:\n\n", profile.kind);
    for phase in profile.phases_for(tier) {
        match (&phase.gate, &phase.gate_title) {
            (Some(gate), Some(title)) => out.push_str(&format!(
                "  {:<3} {:<38} gate {gate}: {title}\n      {}\n      Equivalent: {}\n",
                phase.id,
                phase.name,
                phase.question.as_deref().unwrap_or(""),
                phase.equivalent.as_deref().unwrap_or("")
            )),
            _ => out.push_str(&format!("  {:<3} {}\n", phase.id, phase.name)),
        }
    }
    out
}

fn gate_detail(profile: &Profile, phase: &crate::process::Phase, tier: Option<&str>) -> String {
    let mut out = format!(
        "{}: {} ({} project)\nCloses phase {}.\nQuestion:   {}\nEquivalent: {}\n\nRequired documents:\n",
        phase.gate.as_deref().unwrap_or(""),
        phase.gate_title.as_deref().unwrap_or(""),
        profile.kind,
        phase.label(),
        phase.question.as_deref().unwrap_or(""),
        phase.equivalent.as_deref().unwrap_or("")
    );
    for req in profile.requirements(phase, tier) {
        let title = Process::get()
            .kind(&req.kind)
            .map(|k| k.title.clone())
            .unwrap_or_default();
        let state = if req.state == DocState::Released {
            "released"
        } else {
            "exists"
        };
        out.push_str(&format!("  {} ({}): {state}\n", title, req.kind));
    }
    if !phase.checks.is_empty() {
        out.push_str(&format!(
            "\nAutomated checks: {}\n",
            phase.checks.join(", ")
        ));
    }
    out.push_str("\nEntry criteria (confirmed in the gate review record):\n");
    for c in profile.criteria(phase, tier) {
        out.push_str(&format!("  - {c}\n"));
    }
    out
}

fn separation() -> String {
    format!(
        "Separation between project repositories and the vault

A project repository holds what someone needs to build, verify, operate or
change the product. Everything written to build the reader's own skills goes
in the vault. When two documents cover the same subject, their purpose
decides: a procedure for this product belongs to the project, a tutorial
belongs to the vault.

  Diataxis type                     Location
  Tutorial                          vault
  Explanation of general knowledge  vault   (\"how I2C works\")
  Explanation of this design        project (a decision record)
  How-to procedure for this product project (a test procedure)
  How-to for a general skill        vault   (\"using a logic analyzer\")
  Reference                         project (pin map, interfaces, BOM)

References run one way: vault documents may cite project documents by ID;
project documents never mention the vault, its paths or its contents.

`jig check` enforces this on the files git tracks or would add, minus the
paths under `[check] exclude` in project.toml:
  error    teaching phrases in Markdown: {}
  warning  teaching words in Markdown: {}
  warning  second person (you, your) in engineering documents under docs/
  error    in any text file: the vault's path, a private term listed in the
           vault's jig.toml, or an absolute path under a home directory
  error    in Markdown: a link to an absolute path or out of the repository
           (inline, reference definition, or HTML href and src)
Teaching phrases in code blocks, code spans and HTML comments are ignored, so
they can be quoted in backticks. Private references are reported everywhere.",
        TEACHING_PHRASES
            .join(", ")
            .replace("(?:'|’)", "'")
            .replace("(?:", "(")
            .replace(")?", ")"),
        TEACHING_WORDS.join(", ").replace("(?:", "(")
    )
}

fn requirements() -> String {
    format!(
        "Requirements (ISO/IEC/IEEE 29148, INCOSE Guide to Writing Requirements, EARS)

Each requirement is a heading, a one-sentence statement and attributes:

  ### REQ-001 Encipher keystrokes

  When a letter key is pressed, the machine shall show the enciphered letter within 50 ms.

  - **Verification:** Test
  - **Priority:** Must
  - **Rationale:** Operators expect immediate feedback.
  - **Source:** N-01

EARS patterns:
  Ubiquitous          The <system> shall <response>.
  Event-driven        When <trigger>, the <system> shall <response>.
  State-driven        While <state>, the <system> shall <response>.
  Unwanted behaviour  If <condition>, then the <system> shall <response>.
  Optional feature    Where <feature is included>, the <system> shall <response>.

Rules checked by `jig check`:
  - exactly one `shall` per statement (singular)
  - a verification method: {}
  - no vague terms: {}

Test cases in the V&V plan use the same shape:

  ### TC-001 Keystroke latency

  - **Verifies:** REQ-001
  - **Method:** Test
  - **Level:** System
  - **Procedure:** TP-002
  - **Pass criteria:** The lamp lights within 50 ms in 20 of 20 trials.

Test reports record `- **Result:** Pass | Fail | Blocked | Not run` under the
same TC heading; `jig trace` joins all three into the traceability matrix.",
        METHODS.join(", "),
        VAGUE_TERMS.join(", ")
    )
}

fn ids() -> String {
    let letters: Vec<String> = REVISION_LETTERS.chars().map(String::from).collect();
    format!(
        "Document IDs, revisions and statuses

IDs are <PROJECT CODE>-<KIND CODE>, with a suffix for kinds that have many
documents: EM4-SRS, EM4-ADR-004, EM4-GR-SRR, EM4-LOG-2026-10-01.

Revisions are letters per ASME Y14.35: {}, then AA, AB and so on.
A document starts at revision A as a draft. `jig doc release` marks it
released and adds a revision-history row; `jig doc revise` opens the next
letter as a draft. Git holds every intermediate change.

Statuses: draft, in-review, released, superseded. A decision record in draft
is proposed; released is accepted; superseded is replaced by a newer record.",
        letters.join(" ")
    )
}

pub fn unknown_kind(name: &str) -> anyhow::Error {
    anyhow!("unknown document kind `{name}`; `jig explain kinds` lists them")
}
