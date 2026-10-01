---
id: JIG-RSK
title: Risk register
kind: rsk
revision: A
status: released
date: 2026-09-30
author: dR4y54m5
gate: TRR
---

# Risk register

## 1. Purpose and scope

This register records the risks to JIG: what could make the project fail, how likely and severe each risk is, and what is being done about it. It is reviewed at every gate.

## 2. Rating scales

Likelihood (L) and impact (I) are rated from 1 to 5, and the score is L × I. A score of 15 or more is high, 8 to 14 is medium, and below 8 is low.

## 3. Register

| ID | Risk | L | I | Score | Mitigation | Status |
|---|---|---|---|---|---|---|
| RISK-001 | The separation check misses a leak or rejects legitimate engineering text | 3 | 4 | 12 | Conservative phrase lists; code spans and comments are exempt; private terms are configured in the vault ([JIG-ADR-005](decisions/ADR-005-keep-private-terms-in-the-vault.md)); findings are reviewed at each gate | Open |
| RISK-002 | The Rust Mermaid renderer mis-renders diagram types the process needs | 3 | 3 | 9 | [JIG-SPK-001](spikes/SPK-001-pdf-pipeline.md) found flowchart, sequence, state and Gantt diagrams usable and block-beta defective; the specification uses flowcharts for block diagrams and `jig check` warns on block-beta | Mitigated |
| RISK-003 | The process rules in the specification and in the tool diverge | 3 | 3 | 9 | The process is data embedded in the binary and printed by `jig explain` ([JIG-ADR-001](decisions/ADR-001-encode-the-process-as-data-embedded-in-the-binary.md)) | Mitigated |
| RISK-004 | The document-control overhead is more than a solo engineer sustains | 2 | 4 | 8 | Tiered profiles, generated documents and one-command releases keep the effort per gate small; revisit at the first closeout | Open |
| RISK-005 | New Pandoc or Typst releases change the output and break rendering | 2 | 3 | 6 | An automated test renders a document and a gate package whenever both tools are installed | Open |

## 4. Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
| A | 2026-09-30 | Initial baseline | dR4y54m5 |
