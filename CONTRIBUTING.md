# Contributing to jig

How jig is organised, built and tested, and how it is changed without breaking the process it implements. jig is also a project that follows that process: its requirements, architecture, verification plan and test reports are in [`docs/`](docs/), and every behaviour change updates them.

## 1. Organisation

| Path | Contents |
|---|---|
| `src/main.rs` | Command-line interface and output (text or `--json`) |
| `src/process.rs` | Loads and validates the embedded process data |
| `src/project.rs`, `src/bench.rs` | `project.toml`, the bench layout, the registry and the vault's `jig.toml` |
| `src/frontmatter.rs` | The document-control subset of YAML ([JIG-ADR-004](docs/decisions/ADR-004-parse-a-strict-front-matter-subset.md)) |
| `src/markdown.rs` | Fences, prose, headings and links, aware of code blocks, comments and code spans |
| `src/docs.rs` | Documents, IDs, paths, revision letters, drafting, release and revise |
| `src/trace.rs` | Requirements, test cases, results and the traceability matrix |
| `src/check.rs` | Every `jig check` rule |
| `src/gate.rs` | Gate readiness, review records and decisions |
| `src/render.rs` | Diagrams, Markdown to Typst, PDF and gate packages |
| `src/scaffold.rs` | Plans for `init`, `new`, `adopt` and `setup` ([JIG-ADR-006](docs/decisions/ADR-006-plan-every-change-before-applying-it.md)) |
| `src/explain.rs` | `jig explain` text |
| `process/kinds.toml`, `process/profiles/*.toml` | The process as data ([JIG-ADR-001](docs/decisions/ADR-001-encode-the-process-as-data-embedded-in-the-binary.md)) |
| `templates/docs/` | One body template per document kind |
| `templates/project/` | README, CLAUDE.md and directory READMEs for new projects |
| `assets/bench.typ`, `assets/bench.lua` | Page design and the Pandoc filter ([JIG-ADR-002](docs/decisions/ADR-002-render-pdfs-with-pandoc-and-typst.md)) |
| `skill/SKILL.md` | The agent skill, linked into the agent's skills directory |
| `tests/cli.rs` | System tests that drive the binary against a temporary bench |

`process/`, `templates/` and `assets/` are compiled into the binary. After changing any of them, rebuild and reinstall; the installed `jig` otherwise keeps the old rules.

## 2. Build, test and install

```sh
cargo fmt
cargo clippy --all-targets    # no warnings
cargo test                    # unit and system tests; the PDF test runs when pandoc and typst are installed
cargo install --path .        # updates the jig on the PATH
jig check                     # jig's own documents
```

System tests set `JIG_BENCH` to a temporary directory, so they never touch a real bench.

## 3. Changing behaviour

Every change to what jig does follows the process jig enforces:

1. `jig doc revise SRS`, then add or change the requirement. New requirements take the next free ID; IDs are never renumbered or reused.
2. `jig doc revise VVP`, then add or change the test case that verifies it, naming the automated test in its procedure.
3. Write the test first: a system test in `tests/cli.rs` for behaviour seen through the command line, a unit test in the module otherwise.
4. Implement it in the module that owns the responsibility (section 1). When a requirement lands in a module for the first time, revise the architecture's allocation table.
5. When the change touches the process itself (phases, gates, document kinds, rules, layout), revise [JIG-SPC-001](docs/specs/SPC-001-engineering-process.md) in the same change. The specification and `process/*.toml` must never disagree.
6. Run section 2 in full.
7. Record the run in a new test report (`jig doc new tr`) and release it. A test report records one run and is never revised to describe a later one.
8. Release every revised document with a note saying what changed.

Record significant design choices as decision records (`jig doc new adr`).

## 4. Common changes

### 4.1 A document kind

1. Add an entry to `process/kinds.toml`: key, code, title, ISO/IEC/IEEE 15289 type, basis, purpose, naming, path, and `procedure` or `audience` where they apply.
2. Add `templates/docs/<key>.md`. It starts with `# {{title}}`, opens with a Purpose and scope section, ends with the revision history table, numbers its sections, and puts authoring guidance in `<!-- guide: -->` comments.
3. Require it in a profile if a gate needs it.
4. Add it to `PACK_ORDER` in `src/render.rs`. If it is evidence at any gate once it exists, add it to the list in `pack_kinds` too.
5. Add it to the kinds table in JIG-SPC-001 section 9.1.

The unit tests reject a kind without a template, a template without guidance or without the sections the process specifies, and a profile that references an unknown kind or check.

### 4.2 A lifecycle

Profiles live in `process/profiles/<kind>.toml` and are listed in `PROFILES` in `src/process.rs`. Phases carry their gate, question, industry equivalent, required documents (`require`), automated checks (`checks`) and entry criteria. `min_tier` limits a phase, requirement or criterion to a tier and above. Update JIG-SPC-001 sections 7 and 8.3.

### 4.3 A `jig check` rule

Add a method to `Checker` in `src/check.rs` with a rule ID of the form `area.name`, and call it from `check`. Scan prose with `markdown::prose`, which already skips code blocks, HTML comments and code spans. Run `jig check` in this repository to catch false positives against jig's own documents, add a system test, and describe the rule in JIG-SPC-001 and in `src/explain.rs` when it enforces the process.

### 4.4 Rendering

The page design is `assets/bench.typ`; the Pandoc filter is `assets/bench.lua`; the pipeline is `src/render.rs`. Preview changes as images with `jig doc pack -o '<dir>/page-{p}.png'` and inspect every page type: cover, title block, tables that break across pages, diagrams, the traceability matrix. [JIG-SPK-001](docs/spikes/SPK-001-pdf-pipeline.md) records the Pandoc and Typst behaviours the design works around.

### 4.5 New-project files

`templates/project/` holds what `jig new` and `jig adopt` write: the README, `CLAUDE.md` and the directory READMEs, with the placeholders `{{name}}`, `{{code}}`, `{{title}}` and `{{layout}}`. An exercise has its own, shorter README and `CLAUDE.md`. Existing projects keep their files. The `CLAUDE.md` template seeds the agent instructions in a project's vault folder, from which `jig setup` installs them; after that the vault copy belongs to the project.

### 4.6 The skill

`skill/SKILL.md` holds the procedures an agent follows. Keep its command reference in step with the command line, and keep rules in jig, not in the skill: the skill points at `jig explain`.

## 5. Rules for this repository

- It is public. No private name, path or content goes into any file, code and tests included ([JIG-ADR-005](docs/decisions/ADR-005-keep-private-terms-in-the-vault.md)).
- Front matter is written by jig. Released documents change only after `jig doc revise`.
- Commit messages follow Conventional Commits: `type(scope): summary`.

## 6. Working on the Mermaid renderer

Diagrams render through mermaid-rs-renderer ([JIG-ADR-003](docs/decisions/ADR-003-render-diagrams-in-process-with-rust-renderers.md)). To test a fix before it is released upstream, clone the renderer into the bench's `refs/` and point Cargo at it:

```toml
[patch.crates-io]
mermaid-rs-renderer = { path = "../../refs/mermaid-rs-renderer" }
```

The known defect worth fixing first is block-beta labels that contain spaces (JIG-SPK-001).
