---
id: JIG-TR-006
title: Verification of the storage folder
kind: tr
revision: A
status: released
date: 2026-10-16
author: dR4y54m5
gate: RRR
---

# Verification of the storage folder

## 1. Purpose and scope

This report records a run of the test cases in [JIG-VVP](../vv-plan.md) Rev G that the storage folder ([JIG-ADR-011](../decisions/ADR-011-keep-large-and-personal-data-in-a-storage-folder.md)) adds or touches, against [JIG-SRS](../requirements.md) Rev F: REQ-054 to REQ-056 are new and TC-034 verifies them; the test of TC-012 now also runs `jig storage path`. The results in [JIG-TR-005](TR-005-verification-of-cumulative-gates.md) stand for every other test case.

## 2. Build under test

| Item | Version |
|---|---|
| jig | 0.1.0, source at commit `6245a71` on `main` with the storage change in the working tree |
| Rust toolchain | 1.99.0, stable MSVC |
| git | 2.50.1.windows.1 |
| Machine | The second development machine, Windows 11 Pro |

## 3. Results

Automated tests: `cargo test` ran 97 tests (64 unit, 33 system); 94 passed and 3 failed. `cargo clippy --all-targets` reported no warning. The 3 failures are `agent_instructions_are_installed_from_the_vault`, `sync_clones_missing_projects_from_their_remotes` and `the_hook_blocks_a_commit_that_fails_or_cannot_be_checked`; they fail in the same places on this machine without the change (anomaly A-1), and neither TC-012 nor TC-034 depends on them.

### TC-012 JSON output parses

- **Result:** Pass
- **Evidence:** `every_command_prints_json` passed, with `storage path` among the commands run.
- **Basis:** ebee168f5f

### TC-034 Each project has a storage folder outside the bench

- **Result:** Pass
- **Evidence:** `each_project_has_a_storage_folder_outside_the_bench` passed, including the home directory taken from `USERPROFILE` when `HOME` is unset. Before the change was implemented it failed at its first assertion, the storage root created by `jig init`. The installed build also printed both folders from a PowerShell session on this machine, where `HOME` is not set.
- **Basis:** b1892457e7

## 4. Anomalies

| ID | Description | Severity | Disposition |
|---|---|---|---|
| A-1 | Three system tests fail on Windows: a path compared with forward slashes, and two tests that run `/bin/sh` or assume Unix paths. The same three fail on the unchanged source at `6245a71` | Minor | Open; known before this change. The tests pass on macOS (JIG-TR-005); continuous integration runs on macOS |

## 5. Summary

The storage folder meets REQ-054 to REQ-056: `jig init` creates the root, `jig new` and `jig setup` create the project's folder, `jig storage path` prints it inside a project and the root elsewhere, and the root defaults to `jig-storage` in the home directory unless `JIG_STORAGE` is set. The home directory is `HOME`, else `USERPROFILE`, which the first trial on Windows showed is needed; the bench root uses the same lookup. JSON output still parses for every command. The three failures on Windows predate the change and are unrelated to it.

## 6. Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-10-16 | Initial baseline | dR4y54m5 |
