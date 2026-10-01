---
id: JIG-ADR-010
title: Make gates cumulative
kind: adr
revision: A
status: draft
date: 2026-10-01
author: dR4y54m5
gate: TRR
---

# Make gates cumulative

## Context and problem statement

A gate checked only the documents its own phase requires. A rehearsal of adopting a finished ten-file tool at P5 Sustain and close showed the consequence: the Closeout Review requires no document of its own, so the project could have closed with none of its six documents written. A baseline document revised after its gate went unseen too: a software project could reach its Test Readiness Review with its requirements specification back in draft. The question is what a gate requires of the phases before it.

## Decision drivers

- A go means the baseline up to that gate is in place, not only the part its phase adds.
- A project can still be adopted mid-life: an earlier gate that was never held cannot be held after the fact.
- The rule comes from the profile data, so readiness, `jig explain` and the gate package agree.
- No new record for the engineer to keep, or for an AI pair to write.

## Considered options

1. Keep gates per phase, and list the earlier documents again by hand in each phase of each profile.
2. Cumulative documents: a gate requires the documents of its phase and of every phase before it.
3. Cumulative documents and automated checks.
4. Option 3, and every earlier gate held, or waived on record.

## Decision outcome

Chosen option: 3. `process` merges the requirements of every phase up to the gate: each document kind once, released if any of those phases asks for a release and existing otherwise, and each automated check once. Phases above the project's tier are left out. Gate readiness, `jig explain <GATE>` and the contents of the gate package all read that list. The entry criteria stay those of the gate's own phase, and a gate that was never held stays recorded in the project plan's tailoring table. The profiles are unchanged; a document that a later phase lists again in the same state is now redundant and does no harm.

### Consequences

- Good, because a project adopted at a late phase has to write and release the documents of the earlier phases before its first gate can record a go.
- Good, because a baseline document revised after its gate blocks the next gate until it is released again.
- Good, because the checks, the explanation and the package come from one list.
- Bad, because adopting a small finished project costs more: its earlier documents can no longer stay drafts. The process has no lighter lifecycle for small software yet.
- Bad, because a late gate lists many documents: 14 at the Closeout Review of a retail product.
- Neutral, because `--force` still records a go past failed checks, and the record states that it was forced.

## Pros and cons of the options

### Option 1

- Good, because it needs no change to jig.
- Bad, because every profile repeats its earlier documents by hand, and one forgotten line reopens the gap.

### Option 2

- Good, because it closes the gap the rehearsal found.
- Bad, because a check an earlier gate ran is not run again where a later phase does not list it: the software Release Readiness Review listed only the verification check, not well-formed requirements.

### Option 4

- Good, because the record shows that every gate was decided.
- Bad, because an adopted project needs a waiver for each gate it never held, which is a new record that can be written to pass a gate (RISK-006 in [JIG-RSK](../risks.md)).

## Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
