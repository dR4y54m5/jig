---
id: JIG-ADR-011
title: Keep large and personal data in a storage folder outside the bench
kind: adr
revision: A
status: released
date: 2026-10-16
author: dR4y54m5
gate: RRR
---

# Keep large and personal data in a storage folder outside the bench

## Context and problem statement

Projects produce data that belongs neither in a project repository nor in the vault: telemetry recordings, audio captures, recordings of a person's voice, downloaded models and datasets. Such data is large, often personal, and recaptured or downloaded again rather than versioned. Each project had picked its own folder in the home directory, so neither agents nor tools could find a project's data without being told where it was.

## Decision drivers

- Data of this kind never enters a git repository, public or private.
- An agent finds a project's data folder from jig, as it finds the vault folder.
- No project repository names a home path or a private location (JIG-ADR-005).
- The location can differ between machines, for example a second drive.

## Considered options

1. Each project chooses its own folder and documents it in its agent instructions.
2. A `storage/` folder inside the bench, beside `projects/`, `vault/` and `refs/`.
3. A storage folder outside the bench, `jig-storage` in the home directory, overridden by `JIG_STORAGE`, with one folder per project.

## Decision outcome

Chosen option: 3, because it gives every project the same, discoverable place while keeping gigabytes of untracked data away from the repositories and from any tool that scans the bench. `jig storage path` prints the folder; `jig init` creates the root, and `jig new`, `jig adopt` and `jig setup` create the project's folder.

### Consequences

- Good, because agents and the engineer find a project's data with one command, on every machine.
- Good, because the folder can move to another drive by setting `JIG_STORAGE`.
- Bad, because the bench neither backs up nor synchronises the data between machines; moving it is a manual step.
- Bad, because a product's code cannot name the folder; it reads a location from its own environment variable or option instead.

## Pros and cons of the options

### Option 1: each project chooses

- Good, because nothing changes in jig.
- Bad, because every project invents a location, and only its own instructions say where it is.

### Option 2: inside the bench

- Good, because the whole bench lives under one root.
- Bad, because large binary files sit next to repositories, where a misplaced command or an editor's search indexes or copies them.

### Option 3: outside the bench, one folder per project

- Good, because the location is uniform, discoverable and independent of the repositories.
- Bad, because it adds a second root to set up on a new machine.

## Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-10-16 | Initial baseline | dR4y54m5 |
