---
id: JIG-SPC-001
title: Engineering process
kind: spc
revision: E
status: draft
date: 2026-10-01
author: dR4y54m5
gate: TRR
---

# Engineering process

## 1. Purpose and scope

This specification defines the engineering process for every project on a bench: the repository layout, the separation of engineering and learning material, project kinds and lifecycles, gate reviews, the document set, document control, writing rules, diagrams, rendering and configuration management.

It applies to hardware products, software, reverse-engineering projects and learning exercises run by one engineer, usually working with an AI pair. jig implements the process, and `jig explain` prints its rules from the same data jig enforces (JIG-ADR-001).

## 2. Normative references

| Reference | Used for |
|---|---|
| ISO/IEC/IEEE 15288:2023, System life cycle processes | The technical and management processes behind the lifecycle |
| ISO/IEC/IEEE 15289, Content of life-cycle information items | Generic document types and their content |
| ISO/IEC/IEEE 29148:2018, Requirements engineering | Characteristics of good requirements |
| INCOSE Guide to Writing Requirements | Requirement writing rules |
| EARS, Easy Approach to Requirements Syntax (Mavin et al., 2009) | Requirement sentence patterns |
| ISO/IEC/IEEE 29119-3, Test documentation | Test plans, procedures and completion reports |
| ISO/IEC/IEEE 42010 and arc42 | Architecture descriptions |
| MADR 4 | Decision records |
| AIAG & VDA FMEA Handbook (2019) | Design FMEA and action priority |
| NASA SP-2016-6105 Rev 2 and NPR 7123.1 | Life-cycle phases, technical reviews and tailoring |
| R. G. Cooper, Stage-Gate | Go and kill decisions at gates, scaled by risk |
| EVT, DVT and PVT practice | Hardware validation builds |
| ASME Y14.35 | Revision letters |
| Diátaxis | Documentation types, the basis of the separation rule |
| DIN SPEC 3105-1 and the OSHWA definition | Documentation of public hardware |

## 3. Terms and definitions

| Term | Definition |
|---|---|
| Bench | The directory holding every project repository, the vault and reference repositories |
| Project repository | One git repository per project, public or private |
| Vault | One private repository holding learning material, notes, ideas, inventory and the project registry |
| Profile | The lifecycle of a project kind: phases, gates, required documents, checks and entry criteria |
| Tier | How far a hardware product goes: desk, batch or retail |
| Gate | The review that closes a phase and decides whether the project continues |
| Baseline | The released documents and design files at a gate, tagged in git |
| Controlled document | A Markdown document under `docs/` with document-control front matter |
| Verification | Confirmation that the product meets its specified requirements: built right |
| Validation | Confirmation that the product meets its intended use: the right thing built |

## 4. The bench

### 4.1 Layout

```text
<bench>/
  projects/<name>/   one git repository per project, public or private
  vault/             the private companion repository
  refs/              third-party repositories kept for reference and never edited
```

Paths never encode a project's kind, language or year; those live in each project's `project.toml`, because projects change kind (a software prototype becomes a product).

### 4.2 The vault

| Path | Contents |
|---|---|
| `projects/<name>/` | Learning roadmap, `primers/`, `labs/`, `walkthroughs/`, `retros/` and `notes/` for one project |
| `knowledge/` | Notes and lessons that span projects |
| `bench/` | Guides for agents and the bench's own state; `bench/CLAUDE.md` is installed at the bench root by `jig init` |
| `ideas/` | One file per idea not yet started as a project |
| `inventory/` | Parts, tools and suppliers |
| `templates/` | Templates for learning documents |
| `workspaces/` | A VS Code workspace per project, pairing its repository with its vault folder |
| `registry.toml` | Every project's name, path and remote; `jig sync` clones missing projects |
| `archive/` | Material kept for reference only |
| `jig.toml` | Private terms that must never appear in a project repository |

### 4.3 Agent context

An agent learns the process from files, never from an earlier conversation: its automatic memory is per directory and per machine, so durable context lives in these layers.

| Layer | Location | Loaded | Content | Maintained by |
|---|---|---|---|---|
| Bench guide | `<bench>/CLAUDE.md`, installed by `jig init` from `vault/bench/CLAUDE.md` | Every session under the bench | The engineer's conventions, the bench map, rules and antipatterns | The engineer, in the vault |
| Project instructions | `<project>/CLAUDE.md`, drafted by `jig new` and `jig adopt` | Every session in the project | Project facts and the rules for changing the repository | The project, committed |
| Local settings | `<project>/.claude/settings.local.json`, written by `jig setup` and never committed | Every session in the project | Access to the project's vault folder | jig |
| Skill | `skill/SKILL.md` in the jig repository, linked into the agent's skills | When the task needs it | Procedures: starting a project, writing documents, gate reviews | jig |
| Process rules | `jig explain` | On request | The rules jig enforces | jig's process data |

A new machine gets every layer back from `jig init` and `jig sync`, which run `jig setup` for each cloned project.

## 5. Separation of engineering and learning material

### 5.1 Rule

A project repository contains only information needed to build, verify, operate or change that project's product. Material written to build the reader's own skills belongs in the vault. The rule applies to private project repositories too, because a private project may be published later.

### 5.2 Classification

The rule follows the Diátaxis documentation types. When two documents cover the same subject, their purpose decides where each belongs.

| Diátaxis type | Location | Example |
|---|---|---|
| Tutorials | Vault | Measuring current with a multimeter, using this board as the example |
| Explanation of general knowledge | Vault | How I2C works |
| Explanation of this design | Project | A decision record: why the display bus runs at 400 kHz |
| How-to procedures for this product | Project | A test procedure: measure standby current |
| How-to guides for a general skill | Vault | Using a logic analyzer |
| Reference | Project | Pin map, interface control document, bill of materials |

### 5.3 References

References run one way. Vault documents cite project documents by ID, such as `EM4-SRS` or `EM4-ADR-004`. Project documents never mention the vault, its paths or its contents.

### 5.4 Enforcement

`jig check` enforces the rule in a project repository, and the pre-commit hook blocks a commit with errors. The check covers the files git tracks or would add: a file that git ignores is skipped, and so is any path listed under `exclude` in the `[check]` table of `project.toml`, which records the engineer's decision to leave third-party content out. Outside a git repository the check covers every file except hidden directories and build output.

| Rule | Severity | Applies to | Detects |
|---|---|---|---|
| `separation.teaching` | Error | Markdown | Teaching phrases: `you'll learn`, `you will learn`, `we'll learn`, `let's`, `let us`, `notice that`, `notice how`, `your turn`, `exercise for the reader`, `predict, then`, `decode box`, `as a beginner`, `learning goal`, `learning objective`, `quiz`, `homework` |
| `separation.teaching` | Warning | Markdown | Words that usually signal teaching: `tutorial`, `walkthrough`, `try it`, `try this`, `step-by-step guide` (not reported in end-user documents) |
| `separation.second-person` | Warning | Markdown under `docs/` | `you` and `your` in engineering documents |
| `separation.private-reference` | Error | Every text file | The vault's path, any private term listed in the vault's `jig.toml`, and any absolute path under a home directory (`/Users/` or `/home/` followed by a user name) |
| `separation.private-reference` | Error | Markdown | A link to an absolute path or to a path outside the repository, as an inline link, a reference definition or the `href` or `src` attribute of HTML |

Text in code blocks, code spans and HTML comments is exempt from the teaching and second-person rules, so a phrase can be quoted in backticks. Private references are reported wherever they appear, code included. Link syntax quoted in a code span is not a link.

## 6. Project kinds and tiers

| Kind | For | Tiers |
|---|---|---|
| `product` | A physical product: electronics, firmware and an enclosure | desk, batch, retail |
| `software` | An application, library, CLI or service | None |
| `re` | Reverse engineering existing hardware or software | None |
| `exercise` | A kata, course or practice project | None |

| Tier | Meaning | Lifecycle |
|---|---|---|
| desk | Built for the engineer's own use | P0 to P5, then closeout |
| batch | Small runs built for other people | Adds a production readiness phase |
| retail | Sold commercially | Adds certification, packaging and support |

## 7. Lifecycles

### 7.1 Hardware product

| Phase | Gate | The gate answers | Industry equivalent |
|---|---|---|---|
| P0 Concept | CR: Concept Review | Is the project worth doing, and is it clear what done looks like? | NASA Pre-Phase A (MCR); Stage-Gate gates 1 and 2 |
| P1 Definition | SRR: System Requirements Review | Are the requirements complete and verifiable, and can the riskiest parts work? | NASA Phase A; Stage-Gate gate 3 |
| P2 Preliminary design | PDR: Preliminary Design Review | Does the architecture meet every requirement with margin, shown by a works-like prototype? | NASA Phase B |
| P3 Detailed design | CDR: Critical Design Review | Is the design complete and reviewed, and ready to fabricate? | NASA Phase C |
| P4 Engineering validation | EVT: EVT exit | Does the built design work? | EVT; NASA Phase D integration |
| P5 Design validation | DVT: DVT exit | Does the product meet every requirement in real conditions, and does it satisfy the user? | DVT; NASA Phase D verification |
| P6 Production validation and release (batch and retail) | PRR: Production Readiness Review | Can the product be built repeatably, and is it compliant? | PVT; Stage-Gate gate 5 |
| P7 Sustain and close | CLOSE: Closeout Review | Is the product working in use, and what did the project learn? | NASA Phases E and F |

### 7.2 Software

| Phase | Gate | The gate answers |
|---|---|---|
| P0 Concept | CR: Concept Review | Is the project worth doing, and is it clear what done looks like? |
| P1 Definition | SRR: System Requirements Review | Are the requirements complete and verifiable, and can the riskiest parts work? |
| P2 Design | DR: Design Review | Does the design meet every requirement, with the risky parts proven? |
| P3 Build | TRR: Test Readiness Review | Is the implementation complete and ready for formal verification? |
| P4 Verify and release | RRR: Release Readiness Review | Is every requirement verified, and is the release ready to ship? |
| P5 Sustain and close | CLOSE: Closeout Review | Is the software working in use, and what did the project learn? |

### 7.3 Reverse engineering

| Phase | Gate | The gate answers |
|---|---|---|
| P0 Scoping | SR: Scoping Review | Is the target, the goal and the set of legal and safety limits clear? |
| P1 Investigation and characterization | FR: Findings Review | Is the target understood well enough to meet the goal, and is that knowledge reproducible? |
| P2 Close | CLOSE: Closeout Review | Is the goal met, and is any follow-on project decided? |

### 7.4 Learning exercise

Goal, Work and Retro, with no gates. The repository holds code and a README; goals, notes and the retrospective live in the vault.

### 7.5 Verification and validation pairing

Every defining document is closed by evidence later in the lifecycle.

| Defines | Closed by |
|---|---|
| Concept brief: stakeholder needs | Validation in real use (DVT) |
| System requirements specification | Verification of every requirement by its method (DVT) |
| Architecture and interfaces | Integration tests (EVT) |
| Detailed design | Bring-up and unit tests (CDR to EVT) |

## 8. Gate reviews

### 8.1 Procedure

1. `jig gate check` shows the gate's required documents, automated checks and entry criteria.
2. `jig gate open` creates the gate review record with the entry criteria as a checklist, the automated checks and the evidence list. Run again, it refreshes the generated sections and restores any criterion missing from the checklist.
3. The summary and the open actions are written, usually by the AI pair. The engineer confirms each entry criterion by ticking it.
4. `jig doc pack` renders the gate package for a reviewer.
5. `jig gate close <GATE> --outcome <OUTCOME>` records the decision, releases the review record and, on a go, moves the project to its next phase and drafts that phase's documents.
6. The baseline is committed and tagged `gate/<gate>`.

The entry criteria come from the profile. A criterion that is missing from the record counts as unconfirmed, so a checklist cannot be shortened to pass a gate. A line added to the checklist is a criterion too.

A go is refused while any automated check fails or any criterion is unconfirmed, unless `--force` records it anyway; the record then states that the go was forced and what it overrode. No decision is recorded while the record contains template guidance, or once the record is released. A refused decision changes neither the record nor the project.

### 8.2 Outcomes

| Outcome | Meaning |
|---|---|
| Go | The phase is complete; the project moves to the next phase |
| Go with actions | As go, with open actions recorded in the review record |
| Iterate | The phase continues; `jig gate open` starts the next review in a new revision of the record, with the outcome pending and every criterion unconfirmed |
| Kill | The project stops; its status becomes killed |

### 8.3 Required documents, hardware product

R means released, E means the document exists in any status.

| Gate | Required documents | Automated checks |
|---|---|---|
| CR | PLN R, CON R, RSK E | None |
| SRR | SRS R, VVP E, RSK E | Requirements well formed |
| PDR | ARC R, VVP E, RSK E; from batch: ICD R, FMEA E | Requirements well formed |
| CDR | SRS R, ARC R, VVP R, TP E; from batch: ICD R, FMEA R | Requirements well formed; every requirement has a test case |
| EVT | TP R, TR R | Every requirement has a test case |
| DVT | TR R; from batch: USR E | Every requirement verified in a released test report |
| PRR | MFG R, USR R, REL R; retail: CMP R | None |
| CLOSE | None | None |

Every gate also requires `jig check` to report no errors. `jig explain <GATE> --kind <KIND>` lists the documents and criteria for any profile.

## 9. Document set

### 9.1 Kinds

| Code | Document | 15289 type | Basis |
|---|---|---|---|
| PLN | Project plan | Plan | ISO/IEC/IEEE 15288 project planning; NASA SEMP, tailored |
| CON | Concept brief | Description | NASA concept of operations; Stage-Gate business case |
| RSK | Risk register | Record | ISO/IEC/IEEE 15288 risk management |
| SRS | System requirements specification | Specification | ISO/IEC/IEEE 29148; INCOSE guide; EARS |
| SPK | Spike report | Report | ISO/IEC/IEEE 15288 system analysis |
| ADR | Decision record | Record | MADR 4 |
| ARC | Architecture description | Description | arc42; ISO/IEC/IEEE 42010 |
| ICD | Interface control document | Specification | NASA interface control practice |
| SPC | Specification | Specification | ISO/IEC/IEEE 15289 |
| FMEA | Design FMEA | Report | AIAG & VDA FMEA Handbook |
| VVP | Verification and validation plan | Plan | ISO/IEC/IEEE 29119-3 |
| TP | Test procedure | Procedure | ISO/IEC/IEEE 29119-3 |
| TR | Test report | Report | ISO/IEC/IEEE 29119-3 |
| FND | Findings report | Report | ISO/IEC/IEEE 15289 |
| ERR | Errata | Record | Hardware errata practice |
| ECO | Engineering change order | Request | EIA-649 configuration management |
| REL | Release record | Record | EIA-649 configuration management |
| GR | Gate review record | Record | NPR 7123.1 review criteria; Stage-Gate |
| LOG | Engineering notebook | Record | Engineering notebook practice |
| MFG | Manufacturing and test procedure | Procedure | PVT practice |
| USR | User guide | Description | End-user documentation |
| CMP | Compliance file | Report | FCC, CE and UN38.3 as applicable |

### 9.2 File locations

| Naming | Location | Example |
|---|---|---|
| One per project | A fixed path | `docs/requirements.md` |
| Numbered | `<dir>/<CODE>-NNN-<slug>.md` | `docs/decisions/ADR-004-use-i2c.md` |
| One per gate | `docs/reviews/GR-<GATE>.md` | `docs/reviews/GR-SRR.md` |
| One per day | `docs/notebook/YYYY-MM-DD.md` | `docs/notebook/2026-10-01.md` |

## 10. Document control

### 10.1 Front matter

| Field | Required | Content |
|---|---|---|
| `id` | Yes | Document ID |
| `title` | Yes | Title |
| `kind` | Yes | Document kind, such as `srs` |
| `revision` | Yes | Revision letter |
| `status` | Yes | `draft`, `in-review`, `released` or `superseded` |
| `date` | Yes | Date of the last status change, YYYY-MM-DD |
| `author` | No | Author |
| `gate` | No | The gate at which the document is baselined |
| `supersedes`, `superseded-by` | No | IDs of related decision records |

### 10.2 IDs

An ID is the project code and the kind code, with a suffix for kinds that have many documents: `EM4-SRS`, `EM4-ADR-004`, `EM4-GR-SRR`, `EM4-LOG-2026-10-01`. Items inside documents have their own IDs: requirements `REQ-NNN`, test cases `TC-NNN`, risks `RISK-NNN`, stakeholder needs `N-NN` and interfaces `IF-NN`.

### 10.3 Revisions and statuses

Revisions are letters per ASME Y14.35: A to Y without I, O, Q, S, X and Z, then AA, AB and so on. A document starts at revision A as a draft. `jig doc release` marks it released and adds a row to its revision history; `jig doc revise` opens the next letter as a draft. Git records every change in between. A decision record in draft is proposed, released is accepted, and superseded has been replaced by a newer record.

### 10.4 Structure

Every document except a decision record opens with a Purpose and scope section and ends with a Revision history table. Decision records follow MADR. Section numbers are written in the headings, so references are the same on GitHub and in PDF. Templates carry `<!-- guide: -->` comments, which must be replaced with content before a release.

## 11. Writing rules

### 11.1 Requirements

Each requirement is a heading `REQ-NNN Title` (level three, or level four inside a subsection), a one-sentence statement with exactly one `shall`, and an attribute list: Verification (Inspection, Analysis, Demonstration or Test), Priority (Must, Should or Could), Rationale and Source (a stakeholder need). Statements follow the EARS patterns: ubiquitous, event-driven (When), state-driven (While), unwanted behaviour (If, then) and optional feature (Where). `jig check` reports an error for a statement without exactly one `shall` and for a missing or unknown verification method, and a warning for each of these vague terms: appropriate, adequate, as applicable, as appropriate, easy, easily, user-friendly, fast, quickly, sufficient, robust, flexible, approximately, etc, and/or, if possible, as far as possible, minimize, maximize, optimal, best, typical, several and many. The gate check that requirements are well formed fails on the errors and states the number of warnings.

### 11.2 Test cases and results

Each test case in the verification and validation plan is a heading `TC-NNN Title` with the attributes Verifies (requirement IDs), Method, Level, Procedure (a TP document or an automated test command) and Pass criteria. Test reports repeat the heading for each case run, with a Result of Pass, Fail, Blocked or Not run and its Evidence. `jig trace` joins requirements, test cases and the latest result of each case; only results in released test reports count toward the DVT and RRR gates.

### 11.3 Risks

The risk register rates likelihood and impact from 1 to 5; the score is their product, high from 15, medium from 8. Each risk names its mitigation: a spike, a decision record or an action.

### 11.4 Style

Engineering documents are impersonal, in the present tense, with measurable statements. Procedures use numbered imperative steps. User guides address the reader directly.

## 12. Diagrams

Diagrams are text in the document, which is their source of truth; GitHub and the PDF render the same source. Only the design sources of schematics, layouts and mechanical parts (KiCad and build123d) are not text diagrams; their exports go into release packages.

| Diagram | Tool | Notes |
|---|---|---|
| System context, block diagram, power tree, module structure | Mermaid flowchart | Block diagrams use flowcharts with subgraphs; block-beta is not used until its label defect is fixed |
| Operating modes | Mermaid state diagram | About five transitions per diagram; split larger machines |
| Boot, pairing and key flows | Mermaid sequence diagram | |
| Schedule | Mermaid Gantt chart | |
| Signal timing | WaveDrom | |
| Schematic, PCB, enclosure | KiCad, build123d | Exported to PDF, SVG and STEP for releases |

`jig check` renders every diagram and reports those that fail.

## 13. Rendering and distribution

`jig doc pdf <ID>` renders one document, and `jig doc pack [GATE]` renders a gate package, into `build/pdf/`, which git ignores. A `.png` output path containing `{p}` renders one image per page for previews.

Every page shows the document ID and revision in its header, and the project and "Page X of Y" in its footer. Each document starts with a title block of its document-control fields. Draft, in-review and superseded documents carry a watermark. A gate package opens with a cover page stating the gate, its question and the contents, followed by the gate review record, the documents required up to the gate in lifecycle order, any specifications, test reports, findings reports, errata, change orders, spike reports and decision records, and, from SRR, the requirements traceability matrix.

## 14. Configuration management

- Every project repository is a git repository. A gate baseline is tagged `gate/<gate>`, each fabricated hardware revision `hw/rev-<letter>`, and each firmware or software release `v<version>`.
- After a baseline, a change to a released design is proposed in an engineering change order and recorded in the affected documents' revision histories.
- Known defects of a hardware revision are recorded in its errata.
- A release record lists every released artifact with its version and SHA-256 checksum.

## 15. Tailoring

### 15.1 Per project

Each project records its tailoring in section 4 of its project plan, such as documents merged at its tier or gates not held when a project is adopted mid-life, as NASA practice requires for significant tailoring.

### 15.2 Of the references

| Omitted | Reason |
|---|---|
| SysML and model-based systems engineering tools | Mermaid diagrams and tables cover the architecture views a solo engineer needs |
| A separate stakeholder requirements specification (ISO/IEC/IEEE 29148) | Stakeholder needs are listed in the concept brief |
| NASA System Definition, System Integration and Test Readiness reviews for products | Folded into PDR and the EVT and DVT exits |
| NASA operational and flight readiness reviews | Not applicable |
| Dedicated requirements tools such as StrictDoc or sphinx-needs | IDs in Markdown and a generated traceability matrix cover solo scale |
| Separate measurement and quality assurance processes (ISO/IEC/IEEE 15288) | Folded into gate criteria and the pre-commit check |

## 16. Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-09-30 | Initial baseline | dR4y54m5 |
| B | 2026-09-30 | Requirement headings may sit one level deeper inside a subsection | dR4y54m5 |
| C | 2026-09-30 | Gate packages include test and findings reports that exist | dR4y54m5 |
| D | 2026-09-30 | Section 4.3 agent context; bench and archive in the vault layout | dR4y54m5 |
