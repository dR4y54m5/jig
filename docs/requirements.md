---
id: JIG-SRS
title: System requirements specification
kind: srs
revision: B
status: released
date: 2026-09-30
author: dR4y54m5
gate: SRR
---

# System requirements specification

## 1. Purpose and scope

This specification states the system requirements for JIG. Each requirement traces to a stakeholder need in the [concept brief](concept.md) and has one verification method. It is the baseline for the [verification and validation plan](vv-plan.md).

## 2. Conventions

Requirements follow the EARS patterns:

| Pattern | Form |
|---|---|
| Ubiquitous | The *system* shall *response*. |
| Event-driven | When *trigger*, the *system* shall *response*. |
| State-driven | While *state*, the *system* shall *response*. |
| Unwanted behaviour | If *condition*, then the *system* shall *response*. |
| Optional feature | Where *feature is included*, the *system* shall *response*. |

Each requirement has one verification method: Inspection, Analysis, Demonstration or Test. Priorities are Must, Should and Could. "The vault" is the private companion repository described in [JIG-SPC-001](specs/SPC-001-engineering-process.md).

## 3. Functional requirements

### REQ-001 Scaffold a project

When `jig new` is run with a name, kind, code and title, jig shall create the project repository, its configuration, the documents required by every phase up to the starting phase, its vault folder, its workspace file and its registry entry.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** Starting a project correctly must take one step.
- **Source:** N-01

### REQ-002 Adopt without modifying existing files

When `jig adopt` is run on an existing repository, jig shall leave the content of every existing file unchanged, apart from adding the `/build/` entry to `.gitignore`.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** Existing projects must be adoptable without risk to their content.
- **Source:** N-01

### REQ-003 Dry run

Where `--dry-run` is given, jig shall list the planned changes without writing to the file system.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** An AI pair shows the plan to the engineer before acting.
- **Source:** N-06

### REQ-004 Document-control fields

`jig check` shall report an error for each document under `docs/` that lacks an `id`, `title`, `kind`, `revision`, `status` or `date` field.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** Every controlled document carries the fields a reviewer relies on.
- **Source:** N-05

### REQ-005 Revision letters

`jig check` shall report an error for each revision that is not an ASME Y14.35 revision letter.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** Letters I, O, Q, S, X and Z are confused with digits.
- **Source:** N-05

### REQ-006 Teaching phrases

If a project repository contains a teaching phrase outside code blocks, code spans and comments, then `jig check` shall report an error.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** Learning material belongs in the vault.
- **Source:** N-03

### REQ-007 Private references

If a project repository mentions the vault path or a private term, or links to an absolute path or outside the repository, then `jig check` shall report an error.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** A public repository must never reveal private material or its location.
- **Source:** N-03

### REQ-008 Singular requirement statements

`jig check` shall report each requirement statement that does not contain exactly one `shall`.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** A singular requirement has one verification outcome.
- **Source:** N-04

### REQ-009 Verification method

`jig check` shall report an error for each requirement without a verification method of Inspection, Analysis, Demonstration or Test.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** An unverifiable requirement cannot pass a gate.
- **Source:** N-04

### REQ-010 Vague terms

`jig check` shall report a warning for each requirement statement that contains a term from the vague-terms list in JIG-SPC-001.

- **Verification:** Test
- **Priority:** Should
- **Rationale:** Vague terms make requirements ambiguous or unverifiable.
- **Source:** N-04

### REQ-011 Test case references

`jig check` shall report an error for each test case that verifies a requirement the SRS does not define.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** Broken references make the traceability matrix wrong.
- **Source:** N-04

### REQ-012 Traceability matrix

`jig trace` shall list each requirement with its test cases, their latest results and its verification status.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** Gates from CDR on depend on coverage and results.
- **Source:** N-04

### REQ-013 Gate readiness

`jig gate check` shall report the state of each document, automated check and entry criterion the gate requires.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** The engineer needs to see what stands between the project and its next gate.
- **Source:** N-02

### REQ-014 Gate decision

When a gate is closed with a go, jig shall record the outcome, release the review record and move the project to its next phase.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** A gate decision is recorded once, in one place.
- **Source:** N-02

### REQ-015 Premature go

If a go is requested while a check fails or an entry criterion is unconfirmed, then jig shall refuse the decision unless `--force` is given.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** A gate protects the next phase from unfinished work.
- **Source:** N-02

### REQ-016 Document PDF

`jig doc pdf` shall render a document with a title block, a document-control header, page numbers and, for an unreleased revision, a status watermark.

- **Verification:** Inspection
- **Priority:** Must
- **Rationale:** Reviewers expect controlled documents to identify themselves on every page.
- **Source:** N-05

### REQ-017 Gate package

`jig doc pack` shall render the gate review record, the baseline documents up to the gate and the traceability matrix into one PDF.

- **Verification:** Demonstration
- **Priority:** Must
- **Rationale:** A reviewer receives one file per gate.
- **Source:** N-05

### REQ-018 Diagrams without a browser

jig shall render Mermaid and WaveDrom diagrams without a web browser or network access.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** Rendering must work offline and without a Node.js or Chromium installation.
- **Source:** N-05

### REQ-019 JSON output

Where `--json` is given, jig shall print its result as one JSON document.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** An AI pair parses results instead of scraping text.
- **Source:** N-06

### REQ-020 Non-interactive commands

jig shall complete every command without reading from standard input.

- **Verification:** Inspection
- **Priority:** Must
- **Rationale:** An AI session cannot answer prompts.
- **Source:** N-06

### REQ-021 Bench rebuild

When `jig sync` is run, jig shall clone every registered project that is missing locally and has a remote.

- **Verification:** Test
- **Priority:** Should
- **Rationale:** A new machine is set up from the vault's registry.
- **Source:** N-07

### REQ-025 Agent instructions

When a project is created or adopted, jig shall write a `CLAUDE.md` stating the process rules for agents, unless the file exists.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** An agent started in a project repository learns the rules before it changes anything.
- **Source:** N-06

### REQ-026 Local agent access

When a project is created, adopted or set up, jig shall add the project's vault folder to the additional directories in the repository's local agent settings, keeping every other setting.

- **Verification:** Test
- **Priority:** Should
- **Rationale:** Learning material is written to the vault folder without permission prompts, and personal settings survive.
- **Source:** N-06

### REQ-027 Untracked local settings

When jig writes local agent settings, jig shall list the settings file in the repository's `.gitignore`.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** The settings hold a private path that must never be committed.
- **Source:** N-03

### REQ-028 Bench guide

When `jig init` runs and the vault holds `bench/CLAUDE.md`, jig shall install it at the bench root.

- **Verification:** Test
- **Priority:** Should
- **Rationale:** Every session under the bench loads the guide, and the vault keeps it versioned.
- **Source:** N-07

## 4. Performance requirements

### REQ-022 Check time

`jig check` shall complete within 1 s for a project of 50 documents on the development machine.

- **Verification:** Test
- **Priority:** Should
- **Rationale:** The check runs on every commit.
- **Source:** N-02

### REQ-023 Package time

`jig doc pack` shall render a gate package of 10 documents within 5 s on the development machine.

- **Verification:** Test
- **Priority:** Should
- **Rationale:** Packages are regenerated as documents change.
- **Source:** N-05

## 5. Interface requirements

### REQ-024 External programs

jig shall run no external programs other than git, Pandoc and Typst.

- **Verification:** Inspection
- **Priority:** Must
- **Rationale:** Few dependencies keep the bench easy to rebuild.
- **Source:** N-07

## 6. Environmental and physical requirements

None: jig is a command-line program.

## 7. Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-09-30 | Initial baseline | dR4y54m5 |
| B | 2026-09-30 | Add REQ-025 to REQ-028 for agent context | dR4y54m5 |
