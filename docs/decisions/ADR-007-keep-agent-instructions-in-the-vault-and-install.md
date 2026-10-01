---
id: JIG-ADR-007
title: Keep agent instructions in the vault and install them untracked
kind: adr
revision: A
status: released
date: 2026-10-01
author: dR4y54m5
gate: TRR
---

# Keep agent instructions in the vault and install them untracked

## Context and problem statement

Every project has agent instructions, a `CLAUDE.md` at the repository root, and local agent settings. The first design committed the instructions and named the settings file in `.gitignore`. Both put the AI pair on show in every project repository, public ones included, although the engineer keeps it out of the commit history. The instructions must still reach an agent in every session and survive a move to a new machine.

## Decision drivers

- A project repository shows nothing of the agent files: no content, no file, and no name in a tracked file.
- An agent started in the repository loads the instructions without any step by the engineer.
- A new machine gets the instructions back.
- The agent's file tools do not write through symbolic links.

## Considered options

1. Commit `CLAUDE.md` in each project repository.
2. Keep `CLAUDE.md` in the repository and untracked through `.gitignore`.
3. Keep the master in the project's vault folder and install a copy that the repository's local exclude file hides.
4. Link the repository root to the vault copy with a symbolic link.

## Decision outcome

Chosen option: 3. The vault is private and versioned, reaches a new machine first, and already holds a folder per project that the project's agents may read and write. `jig setup` installs the copy, and `jig sync` runs it for every project it clones. Git reads the local exclude file like `.gitignore` and never commits it, so, unlike option 2, no tracked file names an agent file. `jig check` reports an agent file that git tracks, so the pre-commit hook stops the one way such a file could still be published. The installed copy is read-only: an edit fails and is made to the vault copy instead, which gives the single source that option 4 offers without a link that tools refuse to write through.

### Consequences

- Good, because a published repository and its history hold no agent file.
- Good, because the instructions are versioned with the engineer's other private material.
- Bad, because a change to the instructions takes effect only after `jig setup`.
- Bad, because a clone made without `jig sync` has neither the instructions nor the exclude entries until `jig setup` runs there; `jig check` still refuses a commit that tracks an agent file.

## Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-10-01 | Initial baseline | dR4y54m5 |
