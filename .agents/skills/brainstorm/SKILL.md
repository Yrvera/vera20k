---
name: brainstorm
description: >
  Develop a grounded design for a VERA20k feature, refactor, or behavior change
  when the user asks to brainstorm or when design work would help a substantial
  implementation. Explains the chosen approach, ownership, evidence and how the
  result will be checked. A design-only request does not authorize implementation.
---

# Brainstorm

Design the requested outcome in the current repository. Follow
[ENGINE.md](../../../ENGINE.md); this skill is design help, not a mandatory stage
before every edit. Use the conversation to identify the topic when arguments are empty.

Understand the affected production path and its state owners before proposing a
change. Read current source and relevant native evidence, research and retail data.
Synthesis documents can locate evidence; their conclusions do not replace it.
Resolve uncertainties that would change the design, and name what remains unknown.

Recommend a Rust-native approach that preserves the established behavior. Explain
alternatives when they represent a real decision; there is no required number.
Choose boundaries and abstractions from the mechanism and its consumers, not a
preferred file count, type count or native class hierarchy.

A useful design covers, in proportion to the work:

- The player scenario, scope and observable completion condition.
- Current owners and the proposed data, control and state flow, including production
  integration and any old authority that must retire.
- Evidence for consequential behavior and the constraints it places on the design.
- Dependencies, migration effects, necessary foundations and unresolved decisions.
- Concrete validation scenarios and the limitations of their coverage.

For visual work, establish the complete relevant composition: parent draw order,
active flags, asset/frame selection, anchors, clipping and palette. A helper or a
loaded asset alone does not establish what appears on screen. For simulation work,
follow the surrounding gameplay loop and downstream readers, not just the local helper.

Use a short explanation for a small design. For a substantial or requested artifact,
save `docs/plans/YYYY-MM-DD-<topic>-design.md` in the task's worktree. Keep evidence
close to the decision it supports; avoid a second completion ledger.

A design-only request ends with the design. If implementation is already authorized,
make routine design choices and continue that work without a separate approval ritual.
Use [independent review](../_shared/review.md) when the risk warrants it; a reviewer
may challenge the approach and missing scope, not merely approve a template.
