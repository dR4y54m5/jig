---
id: JIG-TR-001
title: Verification of jig 0.1.0
kind: tr
revision: A
status: released
date: 2026-09-30
author: dR4y54m5
gate: RRR
---

# Verification of jig 0.1.0

## 1. Purpose and scope

This report records the results of running the test cases in [JIG-VVP](../vv-plan.md) against one build of JIG.

## 2. Build under test

| Item | Version |
|---|---|
| jig | 0.1.0, working tree of 2026-09-30 |
| Rust toolchain | 1.96.0 |
| Pandoc | 3.11 |
| Typst | 0.15.1 |
| Machine | Apple M4, macOS |

## 3. Results

Automated tests: `cargo test` ran 54 tests (43 unit, 11 system); 54 passed, none failed or were skipped.

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
- **Evidence:** In `a_new_product_walks_through_its_first_gate`, closing CR before the criteria were confirmed exited with status 2 and left the phase at P0.

### TC-009 Rendered document layout

- **Result:** Pass
- **Evidence:** The JIG TRR package (41 pages) and JIG-ARC were rendered to PNG and every page type inspected: title blocks, headers with ID and revision, "Page X of Y" footers, draft watermark only on JIG-GR-TRR, tables breaking across pages with repeated headers.

### TC-010 Gate package contents

- **Result:** Pass
- **Evidence:** The JIG TRR package opens with a cover listing 16 documents, followed by JIG-GR-TRR, the baseline in lifecycle order, JIG-SPC-001, the spike report, six decision records and the traceability matrix.

### TC-011 Diagrams render in process

- **Result:** Pass
- **Evidence:** `mermaid_flowcharts_render` and `wavedrom_signals_render` passed.

### TC-012 JSON output parses

- **Result:** Pass
- **Evidence:** `check_requires_document_control`, `check_enforces_requirement_rules` and `check_rejects_teaching_material_and_private_references` parse `jig check --json` output.

### TC-013 No command reads standard input

- **Result:** Pass
- **Evidence:** `grep -rn "stdin" src/` returned no matches.

### TC-014 Sync clones missing projects

- **Result:** Pass
- **Evidence:** `sync_clones_missing_projects_from_their_remotes` passed.

### TC-015 Check time on 50 documents

- **Result:** Pass
- **Evidence:** Mean 7.0 ms over 100 runs with the release build.

### TC-016 Package time on 10 documents

- **Result:** Pass
- **Evidence:** 0.43 s, 0.42 s and 0.42 s over three runs with the release build (10 documents and the traceability matrix).

### TC-017 External programs

- **Result:** Pass
- **Evidence:** `grep -rn "Command::new" src/` found only git, Pandoc and Typst.

## 4. Anomalies

| ID | Description | Severity | Disposition |
|---|---|---|---|
| AN-01 | The singular-requirement rule counted a `shall` quoted in a code span (JIG-SRS REQ-008) | Low | Fixed before this run; unit test added |
| AN-02 | The template-guidance rule reported a guidance marker quoted in a code span (JIG-SPC-001) | Low | Fixed before this run; unit test added |
| AN-03 | Gate packages left out specifications, errata and change orders | Medium | Fixed before this run; JIG-SPC-001 updated |
| AN-04 | Unchecked criteria boxes rendered as placeholder glyphs | Medium | Fixed before this run with symbol-font fallbacks |
| AN-05 | Long tables moved whole to the next page instead of breaking | Medium | Fixed before this run |
| AN-06 | Flowcharts with many labelled edges place labels loosely and shrink wide diagrams | Low | Open; a limitation of mermaid-rs-renderer, recorded against RISK-002 |

## 5. Summary

All 17 test cases pass, and every requirement in JIG-SRS is verified by its stated method. Five anomalies found while checking and rendering jig's own documents were fixed before the run; one renderer limitation remains open.

## 6. Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-09-30 | Results for jig 0.1.0 | dR4y54m5 |
