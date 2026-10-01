---
id: JIG-TR-003
title: Verification of the review fixes
kind: tr
revision: A
status: released
date: 2026-10-01
author: dR4y54m5
gate: RRR
---

# Verification of the review fixes

## 1. Purpose and scope

This report records a full run of the test cases in [JIG-VVP](../vv-plan.md) Rev D against [JIG-SRS](../requirements.md) Rev C, after the changes that followed the review of 2026-10-01: REQ-029 to REQ-050 are new, and REQ-007, REQ-008, REQ-025 and REQ-027 are revised. It supersedes the results in JIG-TR-002 for every test case it lists.

## 2. Build under test

| Item | Version |
|---|---|
| jig | 0.1.0, source at commit `8924eb1` on `main` |
| Rust toolchain | 1.96.0 |
| Pandoc | 3.11 |
| Typst | 0.15.1 |
| git | 2.50.1 |
| Machine | Apple M4, macOS |

## 3. Results

Automated tests: `cargo test` ran 90 tests (59 unit, 31 system); 90 passed, and none failed or was skipped. `cargo clippy --all-targets` reported no warning.

### TC-001 Scaffold a product and pass its first gate

- **Result:** Pass
- **Evidence:** `a_new_product_walks_through_its_first_gate` passed.
- **Basis:** 34023e2d6c

### TC-002 Adoption leaves existing files unchanged

- **Result:** Pass
- **Evidence:** `adopting_an_existing_repository_keeps_its_files` passed.
- **Basis:** 082f872f67

### TC-003 A dry run writes nothing

- **Result:** Pass
- **Evidence:** `dry_run_creates_nothing` passed.
- **Basis:** d9cc02ee09

### TC-004 Document control is enforced

- **Result:** Pass
- **Evidence:** `check_requires_document_control` passed.
- **Basis:** 5218603654

### TC-005 Teaching material and private references are rejected

- **Result:** Pass
- **Evidence:** `check_rejects_teaching_material_and_private_references`, `quoted_phrases_in_code_spans_are_not_teaching`, `check_finds_private_references_in_every_text_file` and `check_follows_every_kind_of_link` passed.
- **Basis:** 3114821c47

### TC-006 Requirement rules are enforced

- **Result:** Pass
- **Evidence:** `check_enforces_requirement_rules` passed.
- **Basis:** 3be41489a9

### TC-007 The traceability matrix joins requirements, tests and results

- **Result:** Pass
- **Evidence:** The six `trace::` unit tests passed.
- **Basis:** d36fe5b9c3

### TC-008 A premature go is refused

- **Result:** Pass
- **Evidence:** `a_new_product_walks_through_its_first_gate` and `a_go_needs_every_check_unless_it_is_forced` passed.
- **Basis:** b34ce9d0aa

### TC-009 Rendered document layout

- **Result:** Pass
- **Evidence:** JIG-CON Rev A (released, 2 pages) and JIG-SRS Rev C (draft, 8 pages) were rendered as page images and all ten pages inspected. Each document starts with a title block; every page carries the document ID and revision in its header and "Page X of Y" in its footer; every page of the draft carries the watermark and neither page of the released document does.
- **Basis:** 8f605d634e

### TC-010 Gate package contents

- **Result:** Pass
- **Evidence:** `jig doc pack TRR` produced one package of 58 pages and 21 documents. The cover lists the contents; the gate review record comes first, then the baseline documents, the specification, both earlier test reports, the spike report, nine decision records and the traceability matrix. The cover and the first page of the matrix were inspected.
- **Basis:** e0cd92d59f

### TC-011 Diagrams render in process

- **Result:** Pass
- **Evidence:** `mermaid_flowcharts_render` and `wavedrom_signals_render` passed.
- **Basis:** 1ccf165e19

### TC-012 JSON output parses

- **Result:** Pass
- **Evidence:** `every_command_prints_json` passed for init, new, adopt, status, check, doc list, doc new, doc release, doc revise, doc pdf, doc pack, trace, explain with and without a topic, gate check, gate open, gate close, phase next, vault path, vault new, setup and sync.
- **Basis:** ebee168f5f

### TC-013 No command reads standard input

- **Result:** Pass
- **Evidence:** `grep -rn "stdin" src/` returned no matches.
- **Basis:** 98373fc10c

### TC-014 Sync clones missing projects

- **Result:** Pass
- **Evidence:** `sync_clones_missing_projects_from_their_remotes` passed.
- **Basis:** 05b814e66b

### TC-015 Check time on 50 documents

- **Result:** Pass
- **Evidence:** `check_stays_within_its_time_budget` passed. With the release build, the mean of 100 runs on the same project was 20.5 ms.
- **Basis:** be92e14477

### TC-016 Package time on 10 documents

- **Result:** Pass
- **Evidence:** `a_gate_package_stays_within_its_time_budget` passed. With the release build, three runs took 0.41 s, 0.43 s and 0.43 s.
- **Basis:** 7cbb25c4c9

### TC-017 External programs

- **Result:** Pass
- **Evidence:** `grep -rn "Command::new" src/` found ten matches, in `docs`, `check`, `render`, `scaffold` and `main`; each runs git, Pandoc or Typst.
- **Basis:** 030c0eb028

### TC-018 Agent files are installed from the vault and stay untracked

- **Result:** Pass
- **Evidence:** `agent_instructions_are_installed_from_the_vault` passed.
- **Basis:** 9111db6e2b

### TC-019 The bench guide is installed from the vault

- **Result:** Pass
- **Evidence:** `init_installs_the_bench_guide_from_the_vault` passed.
- **Basis:** 036ba304cf

### TC-020 A gate takes its criteria from the profile

- **Result:** Pass
- **Evidence:** `a_gate_takes_its_criteria_from_the_profile` passed.
- **Basis:** 995b697457

### TC-021 A review record is finished and decided once

- **Result:** Pass
- **Evidence:** `a_review_record_is_finished_and_decided_once` passed.
- **Basis:** c3d2e78e2c

### TC-022 A forced go is marked

- **Result:** Pass
- **Evidence:** `a_go_needs_every_check_unless_it_is_forced` passed.
- **Basis:** 0f244b4df6

### TC-023 Ignored and excluded files are skipped

- **Result:** Pass
- **Evidence:** `check_skips_ignored_and_excluded_files` passed.
- **Basis:** dab04cb776

### TC-024 Adoption reports what the existing files fail

- **Result:** Pass
- **Evidence:** `adoption_reports_what_the_existing_files_fail` passed.
- **Basis:** 3b0ecb9231

### TC-025 Tracked agent files are errors

- **Result:** Pass
- **Evidence:** `check_rejects_tracked_agent_files` passed.
- **Basis:** b8059a21af

### TC-026 The hook refuses what fails or cannot be checked

- **Result:** Pass
- **Evidence:** `the_hook_blocks_a_commit_that_fails_or_cannot_be_checked` and `setup_reports_hooks_that_are_managed_elsewhere` passed.
- **Basis:** 2ffad6de3f

### TC-027 New repositories start on main

- **Result:** Pass
- **Evidence:** `new_repositories_start_on_main` passed.
- **Basis:** 132a662169

### TC-028 Results are stamped and go stale

- **Result:** Pass
- **Evidence:** The six `trace::` unit tests and `a_test_report_is_stamped_and_its_results_go_stale` passed.
- **Basis:** a614400688

### TC-029 Phases without gates move by command

- **Result:** Pass
- **Evidence:** `a_project_without_gates_moves_with_phase_next` passed.
- **Basis:** be31e2c66a

### TC-030 Explain prints the rules that are enforced

- **Result:** Pass
- **Evidence:** `explain_prints_the_rules_the_other_commands_enforce` passed.
- **Basis:** 50762f2950

### TC-031 Templates hold guidance and no content

- **Result:** Pass
- **Evidence:** `release_is_refused_while_template_guidance_remains`, `templates_hold_guidance_and_no_sample_content` and the five `templates::` unit tests passed.
- **Basis:** 219d7df46e


## 4. Anomalies

| ID | Description | Severity | Disposition |
|---|---|---|---|
| AN-01 | The review of 2026-10-01 found defects that the test cases of JIG-TR-002 did not reach: a gate passed once its criteria were deleted from the record, a gate closed on a record that still held template guidance, a refused close rewrote a released record, a template placeholder switched off the link checks of its file, and private references were searched in Markdown only | High | Fixed in this build; TC-005, TC-008 and TC-020 to TC-025 now cover each case |
| AN-02 | Flowcharts with many labelled edges place labels loosely and shrink wide diagrams, as in section 5 of JIG-CON | Low | Open; a limitation of the Mermaid renderer, recorded against RISK-002 |
| AN-03 | A page break can separate a requirement's statement from its attribute list, or a heading and its introductory line from the table they introduce. A heading itself is never left alone: it stays with the block that follows it | Low | Open |

## 5. Summary

All 31 test cases pass, and every requirement in JIG-SRS Rev C is verified by its stated method. Each result carries the basis stamp of its test case and requirements, so a later change to either shows the result as stale. The timing requirements, measured by hand in earlier reports, are now verified by automated tests, with the release build measured as well.

## 6. Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-10-01 | Full run of all 31 test cases after the review fixes | dR4y54m5 |
