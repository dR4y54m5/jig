---
id: JIG-GR-TRR
title: Test Readiness Review (TRR)
kind: gr
revision: A
status: draft
date: 2026-09-30
author: dR4y54m5
gate: TRR
---

# Test Readiness Review (TRR)

## 1. Purpose and scope

This record documents the Test Readiness Review (TRR) of JIG, which closes phase P3 Build. The review answers one question: Is the implementation complete and ready for formal verification?

## 2. Summary

jig 0.1.0 implements the engineering process of [JIG-SPC-001](../specs/SPC-001-engineering-process.md). The project was adopted into that process at P3 Build on 2026-09-30, so no earlier gate was held ([JIG-PLN](../plan.md) section 4), and this is its first gate review.

A review of the first build on 2026-10-01 found 23 defects and gaps. The gravest were a gate that could be passed by deleting its entry criteria from the review record, and private-reference checks that read Markdown only. All were corrected through the change process: the requirements specification grew from 28 to 50 requirements, the verification and validation plan from 19 to 31 test cases, and three decisions were recorded (JIG-ADR-007 to JIG-ADR-009). A read of every document against the code then corrected five statements the code did not support, in the plan, the risk register and two decision records.

Every requirement is implemented and verified. 90 automated tests pass, and [JIG-TR-003](../reports/TR-003-verification-of-the-review-fixes.md) records a full run of all 31 test cases. One entry criterion is not met: automated tests do not yet run in continuous integration, because the repository has no remote; until then the tests run on the development machine.

Decision requested: once the repository is published and continuous integration runs the tests, go to P4 Verify and release.

## 3. Entry criteria

- [ ] Every requirement is implemented.
- [ ] Automated tests run in continuous integration.

## 4. Automated checks

<!-- jig:begin checks -->
| Check | Result | Detail |
|---|---|---|
| verification and validation plan released | Fail | JIG-VVP Rev D draft |
| requirements are well formed | Pass | 50 requirements, 0 errors, 0 warnings |
| every requirement has a test case | Pass | 50 of 50 covered |
| `jig check` reports no errors | Pass | 0 errors |
<!-- jig:end checks -->

## 5. Evidence

<!-- jig:begin evidence -->
| Document | Title | Revision | Status |
|---|---|---|---|
| JIG-PLN | Project plan | B | draft |
| JIG-CON | Concept brief | B | draft |
| JIG-RSK | Risk register | B | draft |
| JIG-SRS | System requirements specification | C | draft |
| JIG-ARC | Architecture description | C | draft |
| JIG-SPC-001 | Engineering process | E | draft |
| JIG-VVP | Verification and validation plan | D | draft |
| JIG-TR-001 | Verification of jig 0.1.0 | A | released |
| JIG-TR-002 | Verification of agent context changes | A | released |
| JIG-TR-003 | Verification of the review fixes | A | draft |
| JIG-SPK-001 | PDF pipeline | A | released |
| JIG-ADR-001 | Encode the process as data embedded in the binary | A | released |
| JIG-ADR-002 | Render PDFs with Pandoc and Typst | B | draft |
| JIG-ADR-003 | Render diagrams in process with Rust renderers | A | released |
| JIG-ADR-004 | Parse a strict front-matter subset | A | released |
| JIG-ADR-005 | Keep private terms in the vault | A | released |
| JIG-ADR-006 | Plan every change before applying it | B | draft |
| JIG-ADR-007 | Keep agent instructions in the vault and install them untracked | A | draft |
| JIG-ADR-008 | Stamp each test result with what it verified | A | draft |
| JIG-ADR-009 | Check the files git would commit | A | draft |
<!-- jig:end evidence -->

## 6. Open actions

| # | Action | Owner | Due |
|---|---|---|---|
| 1 | Review and release the revised documents: JIG-PLN, JIG-CON and JIG-RSK Rev B, JIG-SRS Rev C, JIG-ARC Rev C, JIG-SPC-001 Rev E, JIG-VVP Rev D, JIG-ADR-002 and JIG-ADR-006 Rev B, JIG-ADR-007 to JIG-ADR-009, and JIG-TR-003 | Engineer | Before this gate closes |
| 2 | Publish the repository and run `cargo test` and `jig check` in continuous integration | Engineer | Before this gate closes |
| 3 | Add the licence files for the MIT and Apache-2.0 licences that `Cargo.toml` declares | Engineer | Before publishing |

## 7. Decision

**Outcome:** Pending

## 8. Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
