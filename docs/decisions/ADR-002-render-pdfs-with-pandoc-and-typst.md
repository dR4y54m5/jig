---
id: JIG-ADR-002
title: Render PDFs with Pandoc and Typst
kind: adr
revision: B
status: released
date: 2026-10-01
author: dR4y54m5
gate: TRR
---

# Render PDFs with Pandoc and Typst

## Context and problem statement

Reviewers receive documents and gate packages as PDF. The source is GitHub-flavored Markdown with YAML front matter, which must render on GitHub unchanged. The PDF needs a title block, document-control headers, page numbering, watermarks, cross-document links and packages of many documents.

## Decision drivers

- Faithful conversion of GitHub-flavored Markdown, including tables and alerts.
- Precise, scriptable page layout.
- Fast rendering and a small installation.

## Considered options

1. Pandoc with a LaTeX engine.
2. Quarto.
3. HTML with a print engine such as WeasyPrint or headless Chrome.
4. Pandoc for conversion to Typst fragments, and Typst for layout.

## Decision outcome

Chosen option: 4, because Pandoc reads GitHub-flavored Markdown accurately, tables and alerts included, and Typst offers a layout language that is readable, fast and small. A Lua filter adapts Pandoc's output (callouts, diagrams, links between documents, unique labels), and one Typst library (`assets/bench.typ`) defines the page design for single documents and packages. [JIG-SPK-001](../spikes/SPK-001-pdf-pipeline.md) confirmed the pipeline.

### Consequences

- Good, because a 10-document package renders in under half a second.
- Good, because the page design lives in one short Typst file.
- Bad, because two external programs must be installed for PDF output.

## Pros and cons of the options

### Option 1: Pandoc with LaTeX

- Good, because it is mature and widely used.
- Bad, because a TeX distribution is large and its templates are hard to maintain.

### Option 2: Quarto

- Good, because it bundles Pandoc and supports Typst.
- Bad, because it adds a large runtime and its own project model on top of Pandoc.

### Option 3: HTML to PDF

- Good, because CSS is familiar.
- Bad, because paged media support varies and a browser engine is heavy.

## Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-09-30 | Initial baseline | dR4y54m5 |
| B | 2026-10-01 | Reword the reason for choosing Pandoc: it reads GitHub-flavored Markdown accurately | dR4y54m5 |
