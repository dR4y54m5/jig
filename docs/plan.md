---
id: JIG-PLN
title: Project plan
kind: pln
revision: C
status: released
date: 2026-10-01
author: dR4y54m5
gate: CR
---

# Project plan

## 1. Purpose and scope

This plan defines how JIG is run: its lifecycle, how the bench engineering process is tailored for it, the schedule and the gate reviews.

## 2. Project summary

| Attribute | Value |
|---|---|
| Project | jig |
| Kind | Software |
| Tier | n/a |
| Visibility | public |

jig is the command-line tool that implements the bench engineering process defined in [JIG-SPC-001](specs/SPC-001-engineering-process.md). It scaffolds projects, checks their documents, runs gate reviews and renders documents and gate packages to PDF. It is the reference implementation of the process and its first user.

## 3. Lifecycle and gates

| Phase | Gate | The gate answers |
|---|---|---|
| P0 Concept | CR: Concept Review | Is the project worth doing, and is it clear what done looks like? |
| P1 Definition | SRR: System Requirements Review | Are the requirements complete and verifiable, and can the riskiest parts work? |
| P2 Design | DR: Design Review | Does the design meet every requirement, with the risky parts proven? |
| P3 Build | TRR: Test Readiness Review | Is the implementation complete and ready for formal verification? |
| P4 Verify and release | RRR: Release Readiness Review | Is every requirement verified, and is the release ready to ship? |
| P5 Sustain and close | CLOSE: Closeout Review | Is the software working in use, and what did the project learn? |

## 4. Tailoring

| Item | Tailoring | Reason |
|---|---|---|
| CR, SRR and DR | Not held. The project was adopted at P3 Build on 2026-09-30; the concept brief, requirements and architecture were written after the first implementation. | The process specification and the tool were developed together, with the specification as the design input. |
| Test procedures | The verification and validation plan holds every procedure; there are no separate TP documents. | Each test case is either a `cargo test` command or an inspection or demonstration short enough to state in the test case itself. |
| Continuous integration | A GitHub Actions workflow runs `cargo test` and `jig check` on every push to `main` (REQ-051). The inspections and demonstrations of the verification and validation plan run on the development machine. | They read rendered pages or source code and are not automated. |

## 5. Schedule

| Milestone | Target |
|---|---|
| TRR: formal verification of 0.1.0 | October 2026 |
| RRR: release of 0.1.0 | October 2026 |

## 6. Resources and constraints

One engineer, part time, working with an AI pair. PDF output depends on Pandoc 3 and Typst 0.15 or later.

## 7. Configuration management

Documents, source code and design files are version-controlled in this repository. Each document records its revision letter (A, B, C and so on, per ASME Y14.35) and status in its front matter and revision history. Each gate baseline is tagged `gate/<gate>`, and each release `v<version>`.

## 8. Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-09-30 | Initial baseline | dR4y54m5 |
| B | 2026-10-01 | Test procedures: each test case is a cargo test command or an inspection or demonstration stated in the test case | dR4y54m5 |
| C | 2026-10-01 | Tailoring: continuous integration runs the automated tests; inspections and demonstrations stay on the development machine | dR4y54m5 |
