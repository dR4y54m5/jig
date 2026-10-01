---
id: JIG-VVP
title: Verification and validation plan
kind: vvp
revision: D
status: draft
date: 2026-10-01
author: dR4y54m5
gate: TRR
---

# Verification and validation plan

## 1. Purpose and scope

This plan defines how JIG is verified against its [system requirements](requirements.md) and validated against the stakeholder needs in the [concept brief](concept.md).

## 2. Strategy

| Level | What it covers | How it runs |
|---|---|---|
| Unit | Front matter, IDs and revisions, Markdown parsing, lint rules, traceability, process data | `cargo test --bin jig` |
| System | The `jig` binary driven end to end against a temporary bench | `cargo test --test cli` |
| Inspection | Rendered PDF pages and properties of the source code | Procedures in section 4 |
| Validation | Real projects run through the process | Section 5 |

Every automated test runs with `cargo test`. The PDF test renders only when Pandoc and Typst are installed and reports that it skipped otherwise.

## 3. Environments and equipment

| Item | Version |
|---|---|
| Development machine | Apple M4, macOS |
| Rust toolchain | 1.96 |
| Pandoc | 3.11 |
| Typst | 0.15.1 |

## 4. Test cases

### TC-001 Scaffold a product and pass its first gate

- **Verifies:** REQ-001, REQ-013, REQ-014
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli a_new_product_walks_through_its_first_gate`
- **Pass criteria:** The test passes: the project, vault folder, workspace and hook exist, readiness fails before release, and a go moves the project to P1 with its SRS drafted.

### TC-002 Adoption leaves existing files unchanged

- **Verifies:** REQ-002, REQ-039
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli adopting_an_existing_repository_keeps_its_files`
- **Pass criteria:** The test passes: the existing README and `CLAUDE.md` are byte-for-byte unchanged, `.gitignore` gains only the build directory, and the existing `CLAUDE.md` becomes the agent instructions in the project's vault folder.

### TC-003 A dry run writes nothing

- **Verifies:** REQ-003
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli dry_run_creates_nothing`
- **Pass criteria:** The test passes: the project directory does not exist afterwards.

### TC-004 Document control is enforced

- **Verifies:** REQ-004, REQ-005
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli check_requires_document_control`
- **Pass criteria:** The test passes: a missing status, revision I and a document without front matter are each reported.

### TC-005 Teaching material and private references are rejected

- **Verifies:** REQ-006, REQ-007, REQ-035
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli -- check_rejects_teaching_material_and_private_references quoted_phrases_in_code_spans_are_not_teaching check_finds_private_references_in_every_text_file check_follows_every_kind_of_link`
- **Pass criteria:** The four tests pass: a teaching phrase is an error and the same phrase in a code span is not; the vault path, a private term and a home path are errors in files that are not Markdown; a link out of the repository is an error as an inline link, an absolute path, an HTML attribute and a reference definition, in a file that also holds a template placeholder.

### TC-006 Requirement rules are enforced

- **Verifies:** REQ-008, REQ-009, REQ-010, REQ-011
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli check_enforces_requirement_rules`
- **Pass criteria:** The test passes: a missing `shall`, a statement with two `shall`s, a missing verification method, a vague term and an undefined requirement reference are each reported, the first two as errors.

### TC-007 The traceability matrix joins requirements, tests and results

- **Verifies:** REQ-012
- **Method:** Test
- **Level:** Unit
- **Procedure:** `cargo test --bin jig trace::`
- **Pass criteria:** The tests pass: the latest result wins, missing results leave a requirement planned, and uncovered requirements are reported.

### TC-008 A premature go is refused

- **Verifies:** REQ-015
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli -- a_new_product_walks_through_its_first_gate a_go_needs_every_check_unless_it_is_forced`
- **Pass criteria:** Both tests pass: a go is refused while an entry criterion is unconfirmed and while an automated check fails, the phase and the record stay unchanged, and the same go is recorded when `--force` is given.

### TC-009 Rendered document layout

- **Verifies:** REQ-016
- **Method:** Inspection
- **Level:** System
- **Procedure:** Render a draft and a released document with `jig doc pdf <ID> -o 'page-{p}.png'` and inspect every page.
- **Pass criteria:** Each document starts with a title block; every page carries the document ID and revision in its header and "Page X of Y" in its footer; draft pages carry a watermark and released pages do not.

### TC-010 Gate package contents

- **Verifies:** REQ-017
- **Method:** Demonstration
- **Level:** System
- **Procedure:** Run `jig doc pack` for a project with a released SRS and inspect the package.
- **Pass criteria:** The package opens with a cover listing its contents, then the gate review record, the baseline documents up to the gate, and the traceability matrix.

### TC-011 Diagrams render in process

- **Verifies:** REQ-018
- **Method:** Test
- **Level:** Unit
- **Procedure:** `cargo test --bin jig render::`
- **Pass criteria:** The tests pass: a Mermaid flowchart and a WaveDrom signal render to SVG with no external program.

### TC-012 JSON output parses

- **Verifies:** REQ-019
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli every_command_prints_json`
- **Pass criteria:** The test passes: the output of every command run with `--json` parses as one JSON document.

### TC-013 No command reads standard input

- **Verifies:** REQ-020
- **Method:** Inspection
- **Level:** Unit
- **Procedure:** `grep -rn "stdin" src/`
- **Pass criteria:** No matches.

### TC-014 Sync clones missing projects

- **Verifies:** REQ-021
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli sync_clones_missing_projects_from_their_remotes`
- **Pass criteria:** The test passes: the project is cloned from its registered remote.

### TC-015 Check time on 50 documents

- **Verifies:** REQ-022
- **Method:** Test
- **Level:** System
- **Procedure:** Create a software project at P3 with 44 decision records (50 documents), then time 100 runs of `jig check --quiet` with the release build.
- **Pass criteria:** The mean run time is under 1 s.

### TC-016 Package time on 10 documents

- **Verifies:** REQ-023
- **Method:** Test
- **Level:** System
- **Procedure:** Create a software project at P1 with 5 decision records (10 documents), then time three runs of `jig doc pack SRR` with the release build.
- **Pass criteria:** Every run completes in under 5 s.

### TC-017 External programs

- **Verifies:** REQ-024
- **Method:** Inspection
- **Level:** Unit
- **Procedure:** `grep -rn "Command::new" src/`
- **Pass criteria:** Every match runs `git`, `pandoc` or `typst`.

### TC-018 Agent files are installed from the vault and stay untracked

- **Verifies:** REQ-025, REQ-026, REQ-027
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli agent_instructions_are_installed_from_the_vault`
- **Pass criteria:** The test passes: a new project has its agent instructions in its vault folder and a read-only copy at its root; no tracked file names an agent file and git reports neither; `jig setup` installs an edited vault copy, restores a deleted copy, and adds the vault folder to the local settings once while keeping the other settings.

### TC-019 The bench guide is installed from the vault

- **Verifies:** REQ-028
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli init_installs_the_bench_guide_from_the_vault`
- **Pass criteria:** The test passes: `jig init` installs the vault's guide at the bench root, read-only, and updates it when the vault copy changes.

### TC-020 A gate takes its criteria from the profile

- **Verifies:** REQ-029, REQ-032
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli a_gate_takes_its_criteria_from_the_profile`
- **Pass criteria:** The test passes: with every criterion deleted from the record, readiness reports each of the profile's criteria as missing, a go is refused with the record and the phase unchanged, and `jig gate open` restores the criteria unconfirmed.

### TC-021 A review record is finished and decided once

- **Verifies:** REQ-030, REQ-031, REQ-032, REQ-033
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli a_review_record_is_finished_and_decided_once`
- **Pass criteria:** The test passes: no outcome is recorded while the summary is template guidance, even with `--force`; a second decision on a released record is refused and leaves it unchanged; reopening gives Rev B with the outcome pending and no criterion confirmed.

### TC-022 A forced go is marked

- **Verifies:** REQ-034
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli a_go_needs_every_check_unless_it_is_forced`
- **Pass criteria:** The test passes: the outcome line and the revision history of the record both state that the go was forced, and the outcome names the failed check.

### TC-023 Ignored and excluded files are skipped

- **Verifies:** REQ-036
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli check_skips_ignored_and_excluded_files`
- **Pass criteria:** The test passes: a teaching phrase is reported in a tracked directory and not in one that git ignores, and it is no longer reported once the directory is listed under `exclude`.

### TC-024 Adoption reports what the existing files fail

- **Verifies:** REQ-037
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli adoption_reports_what_the_existing_files_fail`
- **Pass criteria:** The test passes: the dry run and the adoption both report the two errors in the existing files, and the dry run writes nothing.

### TC-025 Tracked agent files are errors

- **Verifies:** REQ-038
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli check_rejects_tracked_agent_files`
- **Pass criteria:** The test passes: `jig check` reports no agent file in a new project, and reports the agent instructions and the local settings as errors once both are added to git.

### TC-026 The hook refuses what fails or cannot be checked

- **Verifies:** REQ-040, REQ-041
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli -- the_hook_blocks_a_commit_that_fails_or_cannot_be_checked setup_reports_hooks_that_are_managed_elsewhere`
- **Pass criteria:** Both tests pass: the installed hook exits 0 on a clean repository, 1 when jig is not on the PATH and 1 when `jig check` reports an error; in a repository with `core.hooksPath` set, no hook is installed and the result carries a note that names the setting.

### TC-027 New repositories start on main

- **Verifies:** REQ-042
- **Method:** Test
- **Level:** System
- **Procedure:** `cargo test --test cli new_repositories_start_on_main`
- **Pass criteria:** The test passes: with git configured for another default branch, the vault and a new project are both on `main`.

## 5. Validation

| Need | Validation activity | Result |
|---|---|---|
| N-01 | Start the first hardware and software projects with `jig new` and `jig adopt` | Pending |
| N-02 | Use `jig status` and `jig gate check` through a full gate on a real project | Pending |
| N-03 | Run the pre-commit check on every commit of the first real projects | Pending |
| N-05 | Send a gate package to a reviewer and record the feedback | Pending |
| N-06 | Drive every command from an AI session | Pending |

## 6. Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-09-30 | Initial baseline | dR4y54m5 |
| B | 2026-09-30 | TC-005: pass both test filters after -- | dR4y54m5 |
| C | 2026-09-30 | TC-012 covers every command; add TC-018 and TC-019 | dR4y54m5 |
