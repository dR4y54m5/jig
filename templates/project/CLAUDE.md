# {{name}}: instructions for agents

{{code}}: {{title}}. This repository follows the bench engineering process and is managed with `jig`. These rules apply to every change.

## Start of every session

1. Run `jig status` for the current phase, the next gate and what the gate still needs.
2. Load the `jig` skill before creating or changing anything under `docs/`. Ask `jig explain <topic>` instead of guessing a rule.

## Rules

- `docs/` holds controlled engineering documents only. Create them with `jig doc new`, release them with `jig doc release`, and change a released document only after `jig doc revise`. Never edit `id`, `kind`, `revision`, `status` or `date` by hand.
- Run `jig check` before finishing a change. The pre-commit hook runs it too; never bypass it with `--no-verify`.
- This repository holds only what is needed to build, verify, operate or change the product. Learning material never goes here, and nothing here refers to where it is kept; `jig vault path` shows its location.
- IDs are permanent: never renumber or reuse a requirement, test case, risk, need or document ID.
- When a V&V test case names an automated test, renaming, moving or deleting that test means updating the plan in the same change.
- A change to behaviour that a requirement covers updates the requirement (after `jig doc revise`) and its test case in the same change.
- Gate entry criteria and gate decisions belong to the engineer: never tick a criterion or run `jig gate close` unasked.
- `build/` holds generated PDFs and is never committed.
- Commit, push or tag only when asked.

## Repository layout

{{layout}}

Build and test commands are in the README.
