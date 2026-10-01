---
id: JIG-ADR-006
title: Plan every change before applying it
kind: adr
revision: A
status: released
date: 2026-09-30
author: dR4y54m5
gate: TRR
---

# Plan every change before applying it

## Context and problem statement

`jig new`, `jig adopt` and `jig init` create directories, write files, initialize repositories, update the registry and install hooks. An AI pair runs these commands and should show the engineer what will change before it changes.

## Decision drivers

- A dry run that is exactly what the real run does (REQ-003).
- No overwritten files (REQ-002).

## Considered options

1. Write directly, with `if dry_run` checks at each step.
2. Build a list of actions, then either describe it or apply it.

## Decision outcome

Chosen option: 2, because one code path produces the plan, so the dry run cannot diverge from the real run. Applying a plan skips any file that already exists unless the action explicitly allows overwriting, which only the regenerated workspace file does.

### Consequences

- Good, because `--dry-run --json` gives an AI pair the complete list of changes.
- Bad, because actions that depend on earlier results must be expressed as data.

## Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-09-30 | Initial baseline | dR4y54m5 |
