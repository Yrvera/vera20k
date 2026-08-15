---
name: rust-scan
description: >
  Project-scoped, read-only VERA20k Rust review for confirmed
  determinism/desync, authoritative-state and lifecycle, architecture,
  correctness/safety, and measured hot-path risks. Use for /rust-scan [path],
  changed-Rust reviews, Rust anti-pattern audits, determinism reviews, or
  code-health scans, including macro-expanded and trait-dispatched behavior
  when relevant. Defaults to src/sim/; reports evidence and never edits or
  auto-fixes.
---

# Rust Scan

Read and follow `../../../.agents/skills/rust-scan/SKILL.md` completely. It is
the canonical procedure shared by both agent frontends; its bundled
`references/` lenses and `scripts/` helpers resolve relative to that canonical
directory. Where it writes `$rust-scan`, read `/rust-scan`.

If the relative path does not exist (worktree checkout), read
`<main-checkout>/.agents/skills/rust-scan/SKILL.md` instead.
