# Calibration example

## The prompt (133 words)

Written 2026-09-01 for the VERA20k per-phase sim profiler, after a nine-agent
research and adversarial-review pass had produced a detailed contract-style
recommendation. This is that recommendation with every "how" removed.

```
We're targeting 20,000 units and nothing in this repo measures where sim tick
time goes. Build a per-phase profiler suitable for VERA20k: research the
options yourself and pick. Read ENGINE.md first; fresh feature branch. Bar:
zero cost when disabled; state hash identical enabled vs disabled; per-phase
timings at rising unit counts on a real map with combat. Out of scope:
optimising what it finds. Divide the goal into the smallest pieces that can be
built and judged independently; each gets a builder and its own fresh-context
critic that inspects the real output against the bar, names the biggest
remaining gap, and sends it back. Keep going until no gap is worth a round;
`cargo test -p vera20k --lib` green and `/rust-scan --changed` clean before
opening the PR. Operate autonomously; note learnings in
docs/plans/profiler-notes.md.
```

Run at **high**: it has to research tooling, read the tick spine, and make a
design call.

## Annotated

| Sentence | Part | Why it earns its place |
|---|---|---|
| "We're targeting 20,000 units and nothing in this repo measures where sim tick time goes." | reason | Fable does measurably better told why; also tells it the *scale* that matters without prescribing a benchmark size |
| "Build a per-phase profiler suitable for VERA20k: research the options yourself and pick." | goal | outcome only; "research yourself" is permission, not method |
| "Read ENGINE.md first; fresh feature branch." | rulebook + git | one line carries every project rule; the branch clause is the single git rule most likely to be tripped |
| "zero cost when disabled" | bar (property) | fetchable: build without the feature and inspect |
| "state hash identical enabled vs disabled" | bar (the load-bearing one) | named, fetchable, comparable — the critic runs both and diffs; it cannot be waved through |
| "produces per-phase timings at rising unit counts on a real map with combat" | bar (representativeness) | encodes the replication failure "optimised a synthetic benchmark" as a property of the artifact, not as advice |
| "Out of scope: optimising what it finds." | boundary | without it Fable starts fixing the first hot spot it sees |
| "Divide the goal into the smallest pieces … each gets a builder and its own fresh-context critic … names the biggest remaining gap, and sends it back." | Build Method | the decomposition instruction and the per-piece loop — the only structural instruction the prompt gives, and it leaves the split to the agent |
| "Keep going until no gap is worth a round; … clean before opening the PR." | keep going + gate | no round count; the gate is the one place project process belongs |
| "Operate autonomously; …" | autonomy | Fable otherwise asks permission it does not need deep in a run |
| "Note learnings in docs/plans/profiler-notes.md." | memory | tracked, so present in every worktree |

What was removed from the contract version, and why each was method: the
`[profile.release]` line and the dependency opt-level (settings Fable will
discover); `src/util/perf.rs` and the `profile` feature name (layout); the CSV
column list (schema — the artifact the critic reads, not a prescription);
"place new scope pairs, don't reuse `trace_master_frame_rung`" (a trap the
adversarial pass found — real, but Fable reading the code will find it too,
and an instruction about it anchors the design to one approach).
