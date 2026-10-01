---
id: JIG-ADR-005
title: Keep private terms in the vault
kind: adr
revision: A
status: released
date: 2026-09-30
author: dR4y54m5
gate: TRR
---

# Keep private terms in the vault

## Context and problem statement

`jig check` must reject any mention of private material in a project repository, including the names of private repositories. The first implementation listed such a name in the source code, which would publish it in jig's own public repository.

## Decision drivers

- A public tool must not contain private names.
- The check must know every private term on a given bench.

## Considered options

1. Private terms in the source code.
2. Private terms in an environment variable.
3. Private terms in `jig.toml` at the root of the vault.

## Decision outcome

Chosen option: 3, because the vault is private by definition and travels with the bench to a new machine. The vault's own path is always private and needs no entry.

### Consequences

- Good, because jig's source contains no private information.
- Bad, because a bench without a vault checks only for absolute paths and links outside the repository.

## Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-09-30 | Initial baseline | dR4y54m5 |
