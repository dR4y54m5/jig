---
id: JIG-ADR-001
title: Encode the process as data embedded in the binary
kind: adr
revision: A
status: released
date: 2026-09-30
author: dR4y54m5
gate: TRR
---

# Encode the process as data embedded in the binary

## Context and problem statement

The engineering process (phases, gates, required documents, checks and entry criteria) is enforced by several commands and described in a specification. If each command encoded the rules in code, and the specification restated them in prose, the three would drift apart.

## Decision drivers

- One source of truth for the rules the tool enforces and explains.
- New project kinds and tiers without code changes.
- A single self-contained binary.

## Considered options

1. Rules in Rust code.
2. Rules in TOML files read from disk at run time.
3. Rules in TOML files embedded in the binary at compile time.

## Decision outcome

Chosen option: 3, because it keeps the rules as reviewable data while shipping one binary that cannot run with mismatched rule files. Unit tests validate the data: every kind a profile references exists, gates are unique and tiers are known.

### Consequences

- Good, because `jig explain` prints exactly the rules `jig gate check` enforces.
- Good, because a new profile is a new TOML file and one line in `process.rs`.
- Bad, because changing a rule needs a rebuild and reinstall.

## Pros and cons of the options

### Option 1: rules in code

- Good, because the compiler checks them.
- Bad, because they are hard to review as a process and drift from the specification.

### Option 2: rules read at run time

- Good, because rules change without a rebuild.
- Bad, because the binary and its rule files can fall out of step on another machine.

## Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-09-30 | Initial baseline | dR4y54m5 |
