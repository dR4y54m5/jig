# {{name}}: instructions for agents

{{code}}: {{title}}. A learning exercise on the bench, managed with `jig`. An exercise has no gates and no engineering documents: the repository holds code and a README.

## Start of every session

1. Run `jig status` for the current phase: Goal, Work or Retro.

## Rules

- The aims of the exercise, notes and the retrospective live in the project's vault folder (`jig vault path`), never in this repository, and nothing here refers to where they are kept.
- Run `jig check` before finishing a change. The pre-commit hook runs it too; never bypass it with `--no-verify`.
- Move the exercise to its next phase with `jig phase next`, and only when the engineer asks.
- Commit, push or tag only when asked.
- This file is installed by `jig setup` from the project's vault folder and is never committed. To change it, edit the copy in the vault folder (`jig vault path`), then run `jig setup`.
