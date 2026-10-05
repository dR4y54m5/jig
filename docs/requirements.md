---
id: JIG-SRS
title: System requirements specification
kind: srs
revision: E
status: released
date: 2026-10-01
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

If a text file in a project repository mentions the vault path, a private term or an absolute home-directory path, then `jig check` shall report an error.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** A public repository must never reveal private material or its location.
- **Source:** N-03

### REQ-008 Singular requirement statements

`jig check` shall report an error for each requirement statement that does not contain exactly one `shall`.

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

When a project is created, adopted or set up, jig shall install the agent instructions kept in the project's vault folder as a read-only `CLAUDE.md` at the repository root, unless adoption finds a `CLAUDE.md` that jig did not install.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** An agent started in a project repository learns the rules before it changes anything, and the instructions never enter the repository or its history.
- **Source:** N-06

### REQ-026 Local agent access

When a project is created, adopted or set up, jig shall add the project's vault folder to the additional directories in the repository's local agent settings, keeping every other setting.

- **Verification:** Test
- **Priority:** Should
- **Rationale:** Learning material is written to the vault folder without permission prompts, and personal settings survive.
- **Source:** N-06

### REQ-027 Untracked agent files

When jig installs agent instructions or writes local agent settings, jig shall list the file in the repository's local exclude file.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** Neither file is ever committed, and no tracked file names them.
- **Source:** N-08

### REQ-028 Bench guide

When `jig init` runs and the vault holds `bench/CLAUDE.md`, jig shall install it at the bench root.

- **Verification:** Test
- **Priority:** Should
- **Rationale:** Every session under the bench loads the guide, and the vault keeps it versioned.
- **Source:** N-07

### REQ-029 Criteria from the profile

If an entry criterion of the gate's profile is missing from the review record, then jig shall treat the criterion as unconfirmed.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** A gate cannot be passed by deleting a criterion from its record.
- **Source:** N-02

### REQ-030 Unfinished review record

If a gate decision is requested while the review record contains template guidance, then jig shall refuse the decision.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** A released record that still holds guidance fails `jig check`, which blocks the commit of the decision.
- **Source:** N-02

### REQ-031 Decided review record

If a gate decision is requested while the review record is released, then jig shall refuse the decision.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** A decision is recorded once; another review starts with `jig gate open`.
- **Source:** N-02

### REQ-032 Refused decisions

If jig refuses a gate decision, then jig shall leave the review record and `project.toml` unchanged.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** A released record is never edited in place, and a refusal has no side effects.
- **Source:** N-02

### REQ-033 Confirmations cleared on reopening

When `jig gate open` reopens a released review record, jig shall clear every entry-criterion confirmation in the new revision.

- **Verification:** Test
- **Priority:** Should
- **Rationale:** Each review confirms its criteria afresh.
- **Source:** N-02

### REQ-034 Forced decisions

When a go is recorded with `--force` while a check fails or an entry criterion is unconfirmed, jig shall state in the review record that the decision was forced.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** A reader of the record sees that the gate was passed against its criteria.
- **Source:** N-02

### REQ-035 Links that leave the repository

If a Markdown file links to an absolute path or to a path outside the repository, by an inline link, a reference definition or an HTML attribute, then `jig check` shall report an error.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** A link out of the repository reveals where private material is kept, whatever syntax it uses.
- **Source:** N-03

### REQ-036 Files outside the check

`jig check` shall skip the files that git ignores and the paths listed under `exclude` in `project.toml`.

- **Verification:** Test
- **Priority:** Should
- **Rationale:** Dependencies and third-party text are not the project's own documents, and an exclusion is a recorded decision.
- **Source:** N-03

### REQ-037 Adoption report

When `jig adopt` is run, jig shall report the errors that `jig check` finds in the repository.

- **Verification:** Test
- **Priority:** Should
- **Rationale:** The engineer sees what must change before the first commit, which the pre-commit hook would refuse.
- **Source:** N-01

### REQ-038 Tracked agent files

If git tracks a project's agent instructions or local agent settings, then `jig check` shall report an error.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** The pre-commit hook then refuses the commit that would publish them.
- **Source:** N-08

### REQ-039 First agent instructions

If a project's vault folder holds no agent instructions when the project is created, adopted or set up, then jig shall create them from the repository's `CLAUDE.md`, or from the template when the repository has none.

- **Verification:** Test
- **Priority:** Should
- **Rationale:** Instructions a repository already has are kept, and move to the vault with their content.
- **Source:** N-08

### REQ-040 Pre-commit check

When a project is created, adopted or set up, jig shall install a pre-commit hook that refuses a commit when `jig check` reports an error or jig cannot be run.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** The separation is enforced on every commit, and a check that cannot run must not pass.
- **Source:** N-03

### REQ-041 Hooks managed elsewhere

If a repository's hooks are managed through `core.hooksPath` or by a pre-commit hook that jig did not install, then jig shall report that its hook was not installed.

- **Verification:** Test
- **Priority:** Should
- **Rationale:** The engineer adds the check to the hook in use instead of believing it runs.
- **Source:** N-03

### REQ-042 Initial branch

When jig creates a git repository, jig shall name its initial branch `main`.

- **Verification:** Test
- **Priority:** Should
- **Rationale:** Every repository on the bench has the same default branch, whatever the machine's git configuration.
- **Source:** N-01

### REQ-043 Result basis

When `jig doc new tr` creates a test report, jig shall list each test case of the verification and validation plan with a basis stamp derived from the test case and the requirements it verifies.

- **Verification:** Test
- **Priority:** Should
- **Rationale:** A result records what it was obtained against, without the author computing anything.
- **Source:** N-04

### REQ-044 Stale results

If a test case or a requirement it verifies has changed since the basis stamp of the test case's latest result, then `jig trace` shall report the requirement as stale.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** A result proves the requirement as it read when the test was run, not as it reads now.
- **Source:** N-04

### REQ-045 Results not run

`jig trace` shall disregard a result of Not run when it selects the latest result of a test case.

- **Verification:** Test
- **Priority:** Should
- **Rationale:** A case that was not run says nothing about the requirement and must not hide an earlier result.
- **Source:** N-04

### REQ-046 Phases without gates

When `jig phase next` is run in a phase that has no gate, jig shall move the project to its next phase, or mark the project closed after its last phase.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** A lifecycle without gates, such as an exercise, moves through its phases by a command and not by a hand edit.
- **Source:** N-02

### REQ-047 Phases with gates

If `jig phase next` is run in a phase that has a gate, then jig shall refuse and name the gate.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** A gated phase is left only through its gate review.
- **Source:** N-02

### REQ-048 Process rules on request

`jig explain` shall print the document kinds, lifecycles, gate requirements and writing rules from the data the other commands enforce.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** An AI pair asks the tool for a rule instead of recalling it, and gets the rule that is enforced.
- **Source:** N-06

### REQ-049 Template guidance blocks a release

If a document contains template guidance, then `jig doc release` shall refuse to release it.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** A released document has every guided section written.
- **Source:** N-05

### REQ-050 Guidance in every template

jig shall provide for every document kind a template that contains template guidance.

- **Verification:** Test
- **Priority:** Should
- **Rationale:** A document left as its template can then never be released.
- **Source:** N-05

### REQ-051 Continuous integration

When a commit is pushed to the `main` branch of the jig repository, continuous integration shall run `cargo test` and `jig check` on that commit.

- **Verification:** Demonstration
- **Priority:** Must
- **Rationale:** A new machine builds jig from its repository. A run on a machine that holds only the repository and its tools shows, for every change, that the build and the tests depend on nothing else.
- **Source:** N-07

### REQ-052 Cumulative documents

jig shall require at a gate each document that the gate's phase or an earlier phase of the lifecycle requires, as released where any of those phases requires it released.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** A project adopted at a late phase could otherwise pass its gate with every earlier document unwritten, and a baseline document revised after its gate would go unseen at the next one.
- **Source:** N-02

### REQ-053 Cumulative checks

jig shall run at a gate each automated check that the gate's phase or an earlier phase of the lifecycle lists.

- **Verification:** Test
- **Priority:** Must
- **Rationale:** What an earlier gate checked, such as well-formed requirements, still has to hold at a later gate.
- **Source:** N-02

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
| C | 2026-10-01 | Add REQ-029 to REQ-050; revise REQ-007, REQ-008, REQ-025 and REQ-027 | dR4y54m5 |
| D | 2026-10-01 | Add REQ-051: continuous integration runs the tests and the check on every push to main | dR4y54m5 |
| E | 2026-10-01 | Add REQ-052 and REQ-053: a gate requires the documents and checks of earlier phases | dR4y54m5 |
