# {{title}}

## 1. Purpose and scope

This plan defines how {{code}} is run: its lifecycle, how the bench engineering process is tailored for it, the schedule and the gate reviews.

## 2. Project summary

| Attribute | Value |
|---|---|
| Project | {{name}} |
| Kind | {{profile_title}} |
| Tier | {{tier}} |
| Visibility | {{visibility}} |

<!-- guide: One paragraph stating what is being built and why. -->

## 3. Lifecycle and gates

{{lifecycle}}

## 4. Tailoring

<!-- guide: Every deviation from the standard process for this kind and tier, with its reason. For example: "Interfaces are a section of the architecture description (desk tier)." Significant tailoring is written down, never implied. -->

| Item | Tailoring | Reason |
|---|---|---|

## 5. Schedule

<!-- guide: A milestone per gate. A Mermaid gantt chart suits this section. Allow one to two weeks of lead time for every board or part order. -->

## 6. Resources and constraints

<!-- guide: Budget, equipment and the time available per week. -->

## 7. Configuration management

Documents, source code and design files are version-controlled in this repository. Each document records its revision letter (A, B, C and so on, per ASME Y14.35) and status in its front matter and revision history. Each gate baseline is tagged `gate/<gate>`, each hardware revision `hw/rev-<letter>` and each firmware or software release `v<version>`.

## 8. Revision history

| Rev | Date | Description | Author |
|---|---|---|---|
