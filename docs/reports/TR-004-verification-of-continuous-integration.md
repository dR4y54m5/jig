---
id: JIG-TR-004
title: Verification of continuous integration
kind: tr
revision: A
status: released
date: 2026-10-01
author: dR4y54m5
gate: RRR
---

# Verification of continuous integration

## 1. Purpose and scope

This report records the demonstration of TC-032 in [JIG-VVP](../vv-plan.md) Rev E against REQ-051 in [JIG-SRS](../requirements.md) Rev D: the first run of the continuous integration workflow, `.github/workflows/ci.yml`. It covers no other test case. The results in [JIG-TR-003](TR-003-verification-of-the-review-fixes.md) stand for TC-001 to TC-031, which the change did not touch: it added the workflow and changed no source file.

## 2. Build under test

| Item | Version |
|---|---|
| jig | 0.1.0, source at commit `133eaad` on `main` |
| Workflow run | `ci`, run 36926526086, started by the push of `133eaad` on 2026-10-01 |
| Runner | GitHub-hosted, image `macos-26-arm64` version 20260907.0351.1 |
| Rust toolchain | 1.99.0 |
| Pandoc | 3.11 |
| Typst | 0.15.1 |
| git | 2.55.0 |

## 3. Results

Before the workflow was pushed, `cargo fmt --check`, `cargo clippy --all-targets` and `cargo test` were run on the development machine with Rust 1.96.0: no difference, no warning, and 90 of 90 tests passed.

The workflow was pushed in commit `b14e970`, and its first run, 36926155628, passed. That run showed the gap recorded as AN-02, which commit `133eaad` closed. The result below is the run of `133eaad`.

### TC-032 Continuous integration runs the tests and the check

- **Result:** Pass
- **Evidence:** `gh run list --branch main --limit 1` lists run 36926526086 of the `ci` workflow, started by the push event for commit `133eaad` and completed with success in 2 min 19 s, with no manual step. `gh run view 36926526086 --log` shows Pandoc 3.11 and Typst 0.15.1 installed before the tests, `cargo test --locked` running 90 tests (59 unit, 31 system) with 90 passed and none failed or ignored, and `jig check` reporting 0 errors and 0 warnings.
- **Basis:** 98d664e500

## 4. Anomalies

| ID | Description | Severity | Disposition |
|---|---|---|---|
| AN-01 | The runner has no vault, so `jig check` searches for no private term there, and its output does not say so | Low | Open. The pre-commit hook runs the full check on the development machine before a commit can reach the repository |
| AN-02 | `cargo test` hides the output of a passing test, so the log cannot show whether a PDF test skipped itself. In the first run the step that prints the Pandoc version could not fail, because its output was piped | Low | Closed in `133eaad`: the workflow installs Pandoc and Typst, and the step that prints their versions before the tests fails when either is missing |

## 5. Summary

TC-032 passes, and REQ-051 is verified by demonstration. With the results of JIG-TR-003, every requirement in JIG-SRS Rev D is verified. The tests now run on every push to `main`, on a machine that holds only the repository and its tools.

## 6. Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-10-01 | Demonstration of TC-032: the first runs of the continuous integration workflow | dR4y54m5 |
