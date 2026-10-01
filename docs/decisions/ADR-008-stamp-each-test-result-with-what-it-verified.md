---
id: JIG-ADR-008
title: Stamp each test result with what it verified
kind: adr
revision: A
status: draft
date: 2026-10-01
author: dR4y54m5
gate: TRR
---

# Stamp each test result with what it verified

## Context and problem statement

`jig trace` joined each requirement to the latest result of its test cases by test case ID alone. A requirement or a test case that was reworded after the run kept its Verified status, so the matrix could claim verification of text that had never been tested. Requirements tools call such a link suspect, and detect it with a stored fingerprint of the linked item.

## Decision drivers

- A change to a requirement or a test case shows in the matrix without anyone remembering to mark it.
- A change elsewhere leaves a result alone: a manual hardware test is expensive to repeat.
- A released test report is never edited, so the comparison uses what the report recorded at the time.
- The documents remain the only database.

## Considered options

1. Leave results as they are and rely on a full rerun after every change.
2. Tie each test report to the revision letters of the requirements specification and the verification and validation plan.
3. Stamp each result with a fingerprint of its test case and of the requirements that test case verifies.

## Decision outcome

Chosen option: 3. `jig doc new tr` writes a Basis stamp for every test case: a 40-bit hash of the test case's method, level, procedure and pass criteria, and of the statement and verification method of each requirement it names. `jig trace` computes the stamp again and reports a requirement as Stale when the stamp of a latest result no longer matches. Option 2 would make every result stale at each revision of either document, whichever requirement changed. Titles, priorities, rationales and line breaks are left out of the stamp, because they do not change what was tested.

### Consequences

- Good, because the matrix cannot report a requirement as verified on a result that predates its current text.
- Good, because results for untouched requirements survive a revision of the specification.
- Bad, because a result written without a stamp is taken as it stands, so reports older than this decision never go stale.
- Bad, because a rewording that does not change the meaning still makes a result stale, and the test is run again.

## Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
