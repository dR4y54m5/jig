---
id: JIG-TR-002
title: Verification of agent context changes
kind: tr
revision: A
status: released
date: 2026-09-30
author: dR4y54m5
gate: RRR
---

# Verification of agent context changes

## 1. Purpose and scope

This report records a full rerun of the test cases in [JIG-VVP](../vv-plan.md) Rev C, after the changes for REQ-025 to REQ-028 and the broadened JSON output for REQ-019. It supersedes the results in JIG-TR-001 for every test case it lists.

## 2. Build under test

| Item | Version |
|---|---|
| jig | 0.1.0, working tree of 2026-09-30, after the agent context changes |
| Rust toolchain | 1.96.0 |
| Pandoc | 3.11 |
| Typst | 0.15.1 |
| Machine | Apple M4, macOS |

## 3. Results

Automated tests: `cargo test` ran 57 tests (43 unit, 14 system); 57 passed, none failed or were skipped.

### TC-001 Scaffold a product and pass its first gate

- **Result:** Pass
- **Evidence:** `a_new_product_walks_through_its_first_gate` passed.

### TC-002 Adoption leaves existing files unchanged

- **Result:** Pass
- **Evidence:** `adopting_an_existing_repository_keeps_its_files` passed.

### TC-003 A dry run writes nothing

- **Result:** Pass
- **Evidence:** `dry_run_creates_nothing` passed.

### TC-004 Document control is enforced

- **Result:** Pass
- **Evidence:** `check_requires_document_control` passed.

### TC-005 Teaching material and private references are rejected

- **Result:** Pass
- **Evidence:** `check_rejects_teaching_material_and_private_references` and `quoted_phrases_in_code_spans_are_not_teaching` passed.

### TC-006 Requirement rules are enforced

- **Result:** Pass
- **Evidence:** `check_enforces_requirement_rules` passed.

### TC-007 The traceability matrix joins requirements, tests and results

- **Result:** Pass
- **Evidence:** The three `trace::` unit tests passed.

### TC-008 A premature go is refused

- **Result:** Pass
- **Evidence:** In `a_new_product_walks_through_its_first_gate`, closing CR before the criteria were confirmed exited with status 2.

### TC-009 Rendered document layout

- **Result:** Pass
- **Evidence:** No rendering asset changed since JIG-TR-001; the JIG TRR package (17 documents) re-rendered without errors.

### TC-010 Gate package contents

- **Result:** Pass
- **Evidence:** The JIG TRR package lists and contains the gate review record, the baseline, JIG-SPC-001, both test reports, the spike report, six decision records and the traceability matrix.

### TC-011 Diagrams render in process

- **Result:** Pass
- **Evidence:** `mermaid_flowcharts_render` and `wavedrom_signals_render` passed.

### TC-012 JSON output parses

- **Result:** Pass
- **Evidence:** `every_command_prints_json` passed for init, new, status, check, doc list, doc new, doc release, doc revise, doc pdf, doc pack, trace, explain, gate check, gate open, vault path, vault new, setup and sync.

### TC-013 No command reads standard input

- **Result:** Pass
- **Evidence:** `grep -rn "stdin" src/` returned no matches.

### TC-014 Sync clones missing projects

- **Result:** Pass
- **Evidence:** `sync_clones_missing_projects_from_their_remotes` passed.

### TC-015 Check time on 50 documents

- **Result:** Pass
- **Evidence:** Mean 7.1 ms over 100 runs with the release build.

### TC-016 Package time on 10 documents

- **Result:** Pass
- **Evidence:** 0.41 s, 0.40 s and 0.41 s over three runs with the release build.

### TC-017 External programs

- **Result:** Pass
- **Evidence:** `grep -rn "Command::new" src/` found only git, Pandoc and Typst.

### TC-018 Agent files are installed without overwriting

- **Result:** Pass
- **Evidence:** `setup_installs_agent_files_without_overwriting` passed.

### TC-019 The bench guide is installed from the vault

- **Result:** Pass
- **Evidence:** `init_installs_the_bench_guide_from_the_vault` passed.

## 4. Anomalies

| ID | Description | Severity | Disposition |
|---|---|---|---|
| AN-01 | JIG-TR-001 recorded REQ-019 as verified although only `jig check` printed JSON; several commands printed text under `--json` | Medium | Fixed: every command now honours `--json`, and TC-012 covers all of them |
| AN-02 | Flowcharts with many labelled edges place labels loosely and shrink wide diagrams | Low | Open; a limitation of mermaid-rs-renderer, recorded against RISK-002 |

## 5. Summary

All 19 test cases pass, and every requirement in JIG-SRS Rev B is verified by its stated method. The review behind this run found that REQ-019 had been reported as verified on too narrow a test; the implementation and the test case were corrected.

## 6. Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-09-30 | Full rerun after the agent context changes | dR4y54m5 |
