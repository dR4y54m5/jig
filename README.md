# jig

A command-line tool that scaffolds, checks and renders engineering projects run through a staged process modelled on industrial practice, from concept through gate reviews to production. It is built for a solo engineer working with an AI pair: every command is non-interactive, prints JSON on request, and states the process rules through `jig explain` instead of relying on memory.

## What it does

- **Scaffolds** a project for its kind (hardware product, software, reverse engineering or exercise) and tier, drafting the documents its phases require.
- **Checks** document control, requirement quality (one `shall` per statement, a verification method, no vague terms), traceability references, diagrams, and the separation between a project's engineering record and private material, in every text file git would commit.
- **Runs gate reviews**: readiness against the lifecycle profile, a review record with entry criteria and evidence, and the decision that moves the project to its next phase.
- **Renders** documents and gate review packages to PDF, with a title block, document-control header, page numbering, draft watermarks and a generated requirements traceability matrix.

## The process

A hardware product moves through eight phases, each closed by a gate review named after its industry equivalent:

| Phase | Gate | The gate answers |
|---|---|---|
| P0 Concept | CR: Concept Review | Is the project worth doing, and is it clear what done looks like? |
| P1 Definition | SRR: System Requirements Review | Are the requirements complete and verifiable, and can the riskiest parts work? |
| P2 Preliminary design | PDR: Preliminary Design Review | Does the architecture meet every requirement with margin, shown by a works-like prototype? |
| P3 Detailed design | CDR: Critical Design Review | Is the design complete and reviewed, and ready to fabricate? |
| P4 Engineering validation | EVT exit | Does the built design work? |
| P5 Design validation | DVT exit | Does the product meet every requirement in real conditions, and does it satisfy the user? |
| P6 Production validation | PRR: Production Readiness Review | Can the product be built repeatably, and is it compliant? |
| P7 Sustain and close | Closeout Review | Is the product working in use, and what did the project learn? |

The lifecycle, document set, document control and writing rules are defined in the [engineering process specification](docs/specs/SPC-001-engineering-process.md). They draw on ISO/IEC/IEEE 15288, 15289, 29148 and 29119-3, the NASA Systems Engineering Handbook, Stage-Gate, EVT/DVT/PVT practice, arc42, MADR, the AIAG & VDA FMEA handbook and ASME Y14.35.

## Install

```sh
cargo install --path .
brew install pandoc typst   # needed for PDF output only
```

## Quick start

```sh
jig init                                   # bench directories and an empty vault
jig new air-remote --kind product --tier desk --code AR --title "Air mouse remote"
cd ~/bench/projects/air-remote
jig status                                 # phase, documents, gate readiness
jig check                                  # document control, separation, requirements
jig gate open                              # review record for the current gate
jig doc pack                               # the gate review package as one PDF
```

## Commands

| Command | Purpose |
|---|---|
| `jig init` | Create the bench directories and an empty vault |
| `jig new`, `jig adopt` | Create a project, or bring an existing repository under the process |
| `jig status` | The current project's status, or every registered project |
| `jig check` | Lint documents; exits non-zero on errors (installed as a pre-commit hook) |
| `jig doc new`, `release`, `revise` | Create documents from templates and manage their revisions |
| `jig doc pdf`, `jig doc pack` | Render one document, or a gate review package |
| `jig gate check`, `open`, `close` | Gate readiness, the review record and the decision |
| `jig phase next` | Move a project whose phase has no gate, such as an exercise, to its next phase |
| `jig trace` | The requirements traceability matrix |
| `jig explain` | The process rules: kinds, phases, gates, separation, requirements, IDs |
| `jig vault new` | Learning documents in the private vault for the current project |
| `jig sync` | Clone registered projects missing from this machine |
| `jig setup` | Install a project's pre-commit hook, agent instructions and local agent settings |

Every command accepts `--json` and then prints one JSON document. `jig new`, `jig adopt`, `jig init` and `jig sync` accept `--dry-run`, which lists every planned change without making it.

## Working with agents

Every project jig creates or adopts gets agent instructions: a `CLAUDE.md` with the process rules an agent must follow. The master copy lives in the project's folder in the vault. `jig setup` installs a read-only copy at the repository root and lists it in the repository's local exclude file, so the instructions never enter the repository or its history; `jig check` reports an error if git tracks them. Local settings that give agents access to the project's vault folder are handled the same way. `jig init` installs the bench-wide agent guide from `vault/bench/CLAUDE.md` at the bench root, where every session under the bench loads it. The `jig` skill in `skill/` holds the procedures: starting a project, writing documents and holding gate reviews.

## Documentation

- [Engineering process specification](docs/specs/SPC-001-engineering-process.md)
- [Concept brief](docs/concept.md), [requirements](docs/requirements.md), [architecture](docs/architecture.md)
- [Decision records](docs/decisions/) and the [PDF pipeline spike](docs/spikes/SPK-001-pdf-pipeline.md)
- [Contributing](CONTRIBUTING.md): how jig is built, tested and changed

## License

MIT or Apache-2.0, at your option.
