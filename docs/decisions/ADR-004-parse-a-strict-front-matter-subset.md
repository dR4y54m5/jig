---
id: JIG-ADR-004
title: Parse a strict front-matter subset
kind: adr
revision: A
status: released
date: 2026-09-30
author: dR4y54m5
gate: TRR
---

# Parse a strict front-matter subset

## Context and problem statement

Every controlled document starts with YAML front matter: flat `key: value` fields such as `id`, `revision` and `status`. jig reads the fields and rewrites them on release and revision. GitHub shows the same front matter as a table, and shows an error box when the YAML is invalid.

## Decision drivers

- Output that is always valid YAML, so GitHub renders it.
- Stable field order and formatting when jig rewrites a document.
- Few dependencies.

## Considered options

1. A general YAML library.
2. A parser and writer for the flat subset jig uses.

## Decision outcome

Chosen option: 2, because document control needs only flat string fields, and a small parser keeps field order, quotes values only when YAML requires it, and reports the line of any malformed entry. The widely used Rust YAML library is deprecated, which removes its main advantage.

### Consequences

- Good, because a rewrite changes only the fields jig sets.
- Bad, because nested YAML in front matter is rejected; document control does not need it.

## Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-09-30 | Initial baseline | dR4y54m5 |
