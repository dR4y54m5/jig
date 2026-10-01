---
id: JIG-SPK-001
title: PDF pipeline
kind: spk
revision: A
status: released
date: 2026-09-30
author: dR4y54m5
gate: TRR
---

# PDF pipeline

## 1. Purpose and scope

This report records a timeboxed investigation for JIG into rendering controlled documents and gate packages to PDF.

## 2. Question

Can GitHub-flavored Markdown with document-control front matter and Mermaid diagrams be rendered to a reviewer-grade PDF without a web browser, and which diagram types does the Rust Mermaid renderer handle? This addresses RISK-002.

## 3. Timebox and method

Timebox: one session on 2026-09-30. Tools: mermaid-rs-renderer 0.3.1, Pandoc 3.11, Typst 0.15.1.

1. Render the six diagram types the process uses and lay the SVGs out on one page with Typst.
2. Convert a sample requirements specification to a Typst fragment with Pandoc, and lay it out with a draft Typst library.
3. Combine two documents into one package with a cover page and a link between them.
4. Inspect every page as a PNG image.

## 4. Results

### 4.1 Diagram types

| Diagram | Use in the process | Result |
|---|---|---|
| Flowchart, left to right | System context | Correct |
| Flowchart, top to bottom | Power tree, module structure | Correct |
| Flowchart with subgraphs | Block diagram | Correct; wide layouts shrink the text |
| Sequence diagram | Boot, pairing and key flows | Correct |
| Gantt chart | Schedule | Correct |
| State diagram | Operating modes | Correct up to about five transitions; edge labels overlap beyond that |
| block-beta | Block diagram | Defective: a quoted label containing spaces splits into several blocks, and edges have no arrowheads |

### 4.2 Conversion and layout

| Finding | Resolution |
|---|---|
| The Markdown H1 duplicates the title from front matter | `--shift-heading-level-by=-1` promotes it out of the body |
| `--id-prefix` does not apply to Typst labels, so combined documents would share labels | The Lua filter prefixes heading identifiers and in-document links per document |
| Pandoc wraps each table in a centered, unbreakable figure | A Typst show rule unwraps it, aligns cells left and lets tables break across pages |
| Proportional column widths wrapped short codes such as "CON N-01" | Typst's automatic column sizing keeps short columns whole and wraps only long ones; kept |
| GitHub alerts arrive as untyped nested blocks | The Lua filter turns them into styled callouts |
| Links between documents point at Markdown paths | jig rewrites them to document IDs and the Lua filter links them inside the package |

### 4.3 Packages

A metadata marker at the start of each document lets the page header, footer and draft watermark follow the document a page belongs to, so one PDF can hold released and draft documents. The cover lists the contents with links to each document.

## 5. Conclusion

The pipeline meets the need and is adopted ([JIG-ADR-002](../decisions/ADR-002-render-pdfs-with-pandoc-and-typst.md), [JIG-ADR-003](../decisions/ADR-003-render-diagrams-in-process-with-rust-renderers.md)). Block diagrams are drawn as flowcharts with subgraphs, state diagrams are kept to about five transitions, and `jig check` warns when block-beta is used. The block-beta label defect is a candidate for an upstream fix in mermaid-rs-renderer.

## 6. Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-09-30 | Initial baseline | dR4y54m5 |
