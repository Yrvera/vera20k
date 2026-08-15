---
name: architecture-scan
description: >
  Project-scoped, read-only VERA20k review of repository organization, module
  ownership, dependency direction, API visibility, naming, cohesion, change
  locality, test placement, and Cargo target boundaries. Use only when
  explicitly invoked as /architecture-scan to assess where existing Rust code
  belongs or whether current repository boundaries are healthy. Reports
  evidence-backed findings and options; never edits, auto-refactors, performs
  gameplay parity research, or imposes generic Rust layout preferences.
---

# Architecture Scan

Read and follow `../../../.agents/skills/architecture-scan/SKILL.md`
completely. It is the canonical procedure shared by both agent frontends; its
bundled `references/` lenses resolve relative to that canonical directory.
Where it writes `$architecture-scan`, read `/architecture-scan`.

If the relative path does not exist (worktree checkout), read
`<main-checkout>/.agents/skills/architecture-scan/SKILL.md` instead.
