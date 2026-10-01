---
id: JIG-ADR-003
title: Render diagrams in process with Rust renderers
kind: adr
revision: A
status: released
date: 2026-09-30
author: dR4y54m5
gate: TRR
---

# Render diagrams in process with Rust renderers

## Context and problem statement

Documents contain Mermaid diagrams (context, block, state, sequence, Gantt) and WaveDrom timing diagrams as text. GitHub renders Mermaid itself; the PDF needs them as SVG. The reference Mermaid renderer, mermaid-cli, runs Mermaid.js in headless Chrome.

## Decision drivers

- Rendering without Node.js, a browser or network access (REQ-018).
- Speed: `jig check` renders every diagram to validate it.
- The ability to fix and improve the renderer upstream.

## Considered options

1. mermaid-cli and wavedrom-cli, both Node.js programs.
2. The Rust crates mermaid-rs-renderer and wavedrom, linked into jig.

## Decision outcome

Chosen option: 2, because the diagrams render in milliseconds inside jig, with no external programs, and fixes can be contributed upstream and used through a Cargo `[patch]` entry before they are released. [JIG-SPK-001](../spikes/SPK-001-pdf-pipeline.md) found flowchart, sequence, state and Gantt diagrams usable, and block-beta labels with spaces defective.

### Consequences

- Good, because `jig check` can render every diagram on every commit.
- Bad, because the Rust renderer is younger than Mermaid.js; block diagrams are drawn as flowcharts until block-beta is fixed.

## Pros and cons of the options

### Option 1: Node.js renderers

- Good, because they are the reference implementations.
- Bad, because they need Node.js and a headless browser, and take seconds per diagram.

## Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-09-30 | Initial baseline | dR4y54m5 |
