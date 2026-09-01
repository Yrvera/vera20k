---
name: goal-prompt-fable
description: >
  Write a short, direction-only goal prompt for an autonomous Fable 5.1 run on
  VERA20k — reason, goal at full ambition, a real bar, and a builder /
  fresh-critic loop, with no method prescribed. Use for any Fable goal prompt,
  builder/critic prompt, minimal goal prompt, or "just give it the direction"
  request. The older /goal-prompt over-prescribes for Fable. Never launches
  the run.
---

# Goal Prompt for Fable

Destination, not route, in 60–150 words.

**The Task.** Why it matters, and the goal at full ambition. A real bar —
named, fetchable, comparable: the verified gamemd decompile for sim, hash or
byte identity for refactors and tooling. One boundary. No real bar? Propose
two or three and stop.

**The Build Method.** Divide into the smallest pieces that can be improved
and judged independently. Each gets a builder and its own fresh-context
critic that inspects the real output against the bar, names the biggest
remaining gap, and sends it back until no gap is worth a round. Gate:
`cargo test -p vera20k --lib`, `/rust-scan --changed`, then the PR.

Say "Read ENGINE.md first"; it carries every other rule. Prescribe no files,
steps, or self-verification. Output a fenced prompt with word count and
effort. Example: [references/profiler-example.md](references/profiler-example.md).
