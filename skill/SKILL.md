---
name: jig
description: Run engineering projects on the bench with the jig CLI. Use when the user wants to start a new project (for example "start a project called air-remote-2"), adopt an existing repository, write or review engineering documents (concept brief, requirements, architecture, decision records, V&V plans, test reports), check a project, prepare or hold a gate review, render documents or a gate package to PDF, or add learning material to the vault.
---

# jig: engineering projects on the bench

jig implements a staged engineering process: phases closed by gate reviews, controlled documents, requirements traced to verification, and a strict separation between a project's engineering record and private learning material. The tool is the source of truth for every rule. When unsure, ask it (`jig explain <topic>`) instead of recalling.

## Ground rules

1. Run jig non-interactively with flags. Read results with `--json`. Run `jig new` and `jig adopt` with `--dry-run` first and show the plan before applying it.
2. After creating or editing any document in a project repository, run `jig check` and fix every error before finishing the turn.
3. Decide where content belongs before writing it (`jig explain separation`). Project repositories hold what is needed to build, verify, operate or change the product. Explanations of general concepts, labs, code explanations written to teach, retrospectives and roadmaps go to the vault (`jig vault path`). Nothing in a project repository reveals where the vault is or what it holds.
4. Create documents with `jig doc new`, never by hand. Change released documents only after `jig doc revise`. Release with `jig doc release <ID> --note "<what changed>"`. Never edit `revision`, `status` or `date` by hand.
5. Confirming gate entry criteria and deciding gate outcomes belong to the user. Never tick a criterion or close a gate on your own judgment.
6. Commit, tag and push only when the user asks.

## Starting a project

1. Ask in one round, with the question tool: project kind (product, software, re, exercise), tier for a product (desk, batch, retail) and visibility (public, private). Propose a code of 2 to 6 capitals and a one-line title derived from the name, and let the user adjust them.
2. Run `jig new <name> --kind <kind> [--tier <tier>] --code <CODE> --title "<title>" --visibility <v> --dry-run`, show the plan, then run it without `--dry-run`.
3. Interview the user briefly (at most 8 questions) for the concept brief: problem, users, stakeholder needs, alternatives, non-goals and measurable targets. Then write `docs/concept.md`, the top risks with a planned spike each in `docs/risks.md`, and the summary, tailoring and schedule in `docs/plan.md`. Replace every template guidance comment.
4. Record what the user wants to learn from the project in the vault roadmap, not in the project repository.
5. Run `jig check` and `jig status`, and tell the user what the Concept Review needs next. Mention the workspace file (`jig vault path` shows the vault; workspaces are in its `workspaces/` folder).

To bring an existing repository under the process, use `jig adopt <path>` with `--phase` set to where the project really is. Earlier phases' documents are drafted so they can be written retroactively; record the gates not held in the plan's tailoring table.

## Writing documents

- `jig explain <kind>` gives each kind's purpose, basis and location. Follow the template's numbered sections.
- Requirements (`jig explain requirements`): a `### REQ-NNN Title` heading, one sentence in an EARS pattern with exactly one `shall`, then Verification (Inspection, Analysis, Demonstration or Test), Priority, Rationale and Source (a need ID). No vague terms. Quantify.
- Test cases go in the V&V plan as `### TC-NNN Title` with Verifies, Method, Level, Procedure and Pass criteria. Results go in test reports under the same heading with Result and Evidence. `jig doc new tr` lists every test case with a Basis stamp: keep each stamp, and remove or leave as Not run the cases the report does not cover. `jig trace` builds the matrix and reports a requirement as stale when it or its test case changed after the run.
- Decision records follow MADR: context, drivers, options, outcome, consequences. Supersede, never rewrite.
- Diagrams are Mermaid (flowchart, sequence, state, Gantt) or WaveDrom. Draw block diagrams as flowcharts with subgraphs, never block-beta. Keep state diagrams to about five transitions.
- Style: impersonal, present tense, measurable. No second person except in user guides. Procedures use numbered imperative steps.
- Refer to other documents by ID and link them with relative paths inside `docs/`.

## Gate reviews

1. `jig gate check` and summarize what stands between the project and its gate.
2. Close the gaps the user wants closed: write documents, then release them.
3. `jig gate open`, then write the record's summary and open actions. No decision can be recorded while the record still holds template guidance.
4. Ask the user to confirm each entry criterion (multi-select question). Tick only what the user confirms.
5. `jig doc pack` and give the user the PDF path. For a quick visual check, render pages with `-o '<dir>/page-{p}.png'`.
6. When the user decides, `jig gate close <GATE> --outcome go|go-with-actions|iterate|kill --note "<reason>"`. Then offer to commit and tag `gate/<gate>`.

## Learning material

`jig vault new primer|lab|walkthrough|retro|note "<title>"` creates a vault document for the current project from the vault's templates. Labs follow predict, measure, explain. Code explanations follow tests first, an attempt, then comparison. Cite project documents by ID, never by path.

## Command reference

| Command | Use |
|---|---|
| `jig status [--all]` | Phase, documents, check results, traceability, gate readiness |
| `jig check [--quiet] [--strict]` | Lint; exit 1 on errors |
| `jig doc list`, `new`, `release`, `revise` | Documents and revisions |
| `jig doc pdf <ID>`, `jig doc pack [GATE]` | PDF output in `build/pdf/` |
| `jig gate check`, `open`, `close` | Gate reviews |
| `jig phase next` | Next phase of a project without gates, such as an exercise |
| `jig trace` | Requirements traceability matrix |
| `jig explain [kinds, <kind>, phases, <GATE>, separation, requirements, ids]` | The rules |
| `jig vault new`, `jig vault path` | Learning material |
| `jig new`, `adopt`, `init`, `sync`, `setup` | Bench and project setup; `setup` installs a project's hook, CLAUDE.md and local agent settings on this machine |
