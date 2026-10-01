---
id: JIG-ADR-009
title: Check the files git would commit
kind: adr
revision: A
status: draft
date: 2026-10-01
author: dR4y54m5
gate: TRR
---

# Check the files git would commit

## Context and problem statement

`jig check` walked the repository and skipped hidden directories, `target` and `node_modules`. Vendored and generated directories with other names were checked as the project's own text, which blocked the adoption of existing repositories. Files other than Markdown were not checked at all, so a private path in source code went unseen.

## Decision drivers

- The check covers what could be published, and nothing else.
- No second ignore list to keep in step with `.gitignore`.
- Third-party content that is tracked can be left out by a recorded decision.

## Considered options

1. A longer built-in list of directory names to skip.
2. An ignore file of jig's own.
3. The files git tracks or would add, with an exclude list in `project.toml` for tracked third-party paths.

## Decision outcome

Chosen option: 3. Git already knows which files a commit can contain: `git ls-files --cached --others --exclude-standard` lists them, honouring `.gitignore`, the repository's local exclude file and the global one. Private references are searched in every text file on that list, and the Markdown rules apply to its Markdown files. `exclude` under `[check]` in `project.toml` names tracked paths that are not the project's own text, such as vendored code. Outside a git repository the check walks the directory as before.

### Consequences

- Good, because what is checked is what can be committed.
- Good, because a repository whose dependencies are ignored needs no configuration to be adopted.
- Bad, because every `jig check` runs git, which costs a few milliseconds.
- Bad, because an excluded path is not searched for private references either, so each exclusion has to be deliberate.

## Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
