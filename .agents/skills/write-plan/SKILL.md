---
name: write-plan
description: >
  Use when you have an approved design spec and need to create an
  implementation plan before writing code. Breaks specs into
  bite-sized tasks with exact file paths, complete code, and
  verification steps. Usage: '/write-plan topic' or '/write-plan'
  (auto-detects latest design doc). Trigger only on an explicit request to write,
  create, or review an implementation plan after design approval; do not chain it
  automatically after brainstorming.
---

# Write Plan — Architecture-Aware Implementation Planning

Create an implementation plan for: **$ARGUMENTS**

If `$ARGUMENTS` is empty, look for the most recent `docs/plans/*-design.md` file.
If none exists, tell the user to run `/brainstorm` first.

Do NOT write any implementation code during this skill. Plans and research only.

Current delivery lens: plan for a retail-convincing ordinary stock-skirmish
scenario. Preserve deterministic architecture and evidence honesty, but do not
turn bounded expert-only exactification residuals into implementation blockers.
Use Ghidra and exhaustive detail when an uncertainty can change common behavior,
authority, lifecycle, RNG, persistence, or shared architecture.

---

## Hard Gate

- Do NOT start writing the plan until you have read and understood the full design doc.
- If no design doc exists, tell the user to run `/brainstorm` first and stop.
- If the design doc is missing **Architecture Context** or **Impact Analysis**, flag it
  and ask whether to proceed or go back to brainstorming.
- Under an explicitly autonomous `/goal`, a design with recorded adversarial
  self-approval satisfies the approval gate. Repair missing grounding directly
  when it is discoverable; pause only for a material unrecoverable ambiguity.

---

## Research Grounding Phase

Use the research-index MCP first as the source map. Prefer `research_brief` and
`research_handoff`; if unavailable, use the repo-local CLI fallback:

```text
python tools/research_index/brief.py "<topic>" --limit 8
```

If the exact system is known, add `--system <system>` (examples: `bridges`,
`miner`, `skirmish-ui`, `chrono`). If the result reports zero docs, rerun without
`--system`, then rerun with the inferred exact system. Use `validate.py`,
`handoff.py`, `map.py`, `graph.py implementation`, and `search.py` to find
verified docs, stale warnings, handoff rows, and Rust touchpoints. The plan must
still cite primary evidence and current Rust lines; index output is navigation.

Before Pre-Planning, gather grounding from the sources below, following
AGENTS.md priority order for behavioral truth. Skipping this is the #1 cause of
plans that get rewritten after `/review-plan`.

### R1. docs/research/ — existing RE research

Search `docs/research/` for prior Ghidra reports on this system. You
have 1,700+ files there; most systems already have struct layouts, field
offsets, and behavioral analysis documented.

- List every relevant report by filename
- Extract confidence levels the original report stated (verified / inferred)
- Note any TS-legacy warnings flagged in the report
- Every cited report goes in the plan's **Sources & References** section

**Do not redo research that's already been done.**

### R2. gamemd.exe via Ghidra MCP — live binary verification

Use Ghidra MCP to fill gaps in the docs and verify claims:

- Decompile functions referenced in the docs to confirm they still match
- Trace xrefs for any function you plan to reimplement — confirm the code
  path is actually reachable in a standard YR skirmish (watch for TS ghosts)
- Check param_1 types before extracting struct offsets (int vs int* —
  see AGENTS.md "Decompilation pitfall")
- Capture binary addresses here; they go in the plan's Sources section,
  not in Rust code comments

State confidence for every claim: **verified-from-binary** vs **inferred**.

### R3. Repo code — existing patterns to mirror

Find the closest existing pattern in the repo for what you're building:

- Which module owns similar behavior today?
- Which structs/traits does new code need to integrate with?
- List specific files + line ranges the new code should mirror

### R4. INI files — constants and source of truth

Grep `ini/rulesmd.ini` and `ini/artmd.ini` for every constant the feature
needs. Never hardcode values the INI defines.

- List the `[Section]` Key=Value entries the plan depends on
- Flag any INI keys not currently parsed by `src/rules/` — those become
  plan tasks

### Grounding Summary

Produce a short summary (~10-20 lines) before Pre-Planning that the plan
will reference. It should answer:

- What do the docs already tell us? (cite reports)
- What did Ghidra verification confirm or contradict? (cite addresses)
- What existing repo pattern does this follow?
- What INI keys drive the behavior?
- What's still unknown after grounding? (→ Deferred Open Questions)

---

## Pre-Planning Phase

Complete all four steps before writing any tasks.

### A. Read the Design Doc

Load the design spec from `docs/plans/YYYY-MM-DD-<topic>-design.md`. Extract:
- Goal, architecture context, impact analysis
- Chosen approach, interfaces/contracts, testing strategy

### A.1. Re-verify the design's premise against current git state

Design docs go stale. Parallel sessions or background commits can land
between the brainstorm and the plan, silently invalidating chunks of the
design. Catch this BEFORE writing tasks against a stale view.

For every file the design names as "modify" or "depend on":

```
git log --oneline -10 -- <path>
```

Read the commit subjects. Any commit landed since the design doc's date
that touches the same system is a red flag — re-read the file and confirm
the design's "current state" claims still hold. If the system has been
restructured, refactored, or partially implemented since the design,
**stop and re-scope**: most of the plan's tasks may now be obsolete or
wrong-direction.

This step takes seconds and catches the single most common failure mode:
writing a plan against the world as the design saw it, not the world as
it is now.

### B. Map the File Structure

From the design, list every file that will be created or modified. For each file:
- What is its single responsibility?
- Does it land in the right place in the project structure?
  - `sim/` — deterministic game logic only, never depends on render/ui/audio/net
  - `render/`, `sidebar/`, `ui/` — presentation, sits above sim
  - `assets/`, `util/` — low-level, reusable
  - `rules/`, `map/` — data used by gameplay and rendering
- Does it stay within its current responsibility (for modifications)?
- Does it follow existing patterns for file organization?
- Will any file exceed ~600 lines? If so, plan submodule splits.

### C. Identify Interface Boundaries and Risk Areas

- Which tasks create or modify public interfaces (APIs, exports, contracts, event
  handlers, config schemas)? These need extra care — order them before implementations.
- Pull risk areas from the Impact Analysis. Which files have the highest blast radius?
  Which tasks are most likely to break existing functionality? Plan regression tests.

**For sim/ changes, additionally flag:**
- Does this affect tick ordering in `Simulation::advance_tick`?
- Does this add new state that must be included in the deterministic state hash?
- Does any new math use `fixed`-point types? (f32/f64 forbidden in sim logic)
- Does the EntityStore (`BTreeMap<u64, GameEntity>`) iteration order matter here?

---

## Task Design Rules

### Granularity

Keep tasks bounded and independently verifiable. One action per step:
- "Define the struct/types" — step
- "Write the implementation" — step
- "Add tests" — step
- "Run `cargo test`, verify pass" — step
- "Commit" appears only when the user explicitly requested commits; otherwise
  omit all staging/commit steps.

### Ordering Principles

1. **Interfaces and types first** — define structs, traits, and contracts before implementations
2. **Foundation before features** — infrastructure/plumbing before business logic
3. **Tests where testable** — unit tests for pure logic and data transforms;
   for RE-driven behavior, verification against gamemd.exe replaces traditional TDD
4. **Independent tasks early** — things with no dependencies get done first
5. **High-risk changes early** — surface problems before building on top
6. **Integration tasks last** — wire things together after the pieces work

### Verification Strategy

Not all tasks can be TDD'd. Use the right verification for the task type:

- **Pure logic / data transforms** — unit tests with `#[cfg(test)] mod tests`
  in the same file (Rust convention for unit tests)
- **INI parsing** — test that parsed values match expected INI entries from
  `rules(md).ini` / `art(md).ini`
- **RE-driven behavior** — verify against gamemd.exe via Ghidra decompilation,
  verified research, in-game observation, or `/trace-action`. State the expected behavior and
  how to confirm it matches the original.
- **Rendering / visual** — describe the expected visual result and how to verify
  by running the game and comparing to original YR
- **Integration tests** — `tests/` directory for cross-module tests if needed

### No Placeholders — Ever

These are plan failures. Never write:
- "TBD", "TODO", "implement later", "fill in details"
- "Add appropriate error handling" / "add validation" / "handle edge cases"
- "Write tests for the above" (without actual test code)
- "Similar to Task N" (repeat the content — tasks must be self-contained)
- "See the design doc for details" (the task must contain everything needed)
- Steps that describe *what* to do without showing *how* (code required)

### Architecture Compliance

Each task must note:
- Which existing pattern it follows (if any)
- If it creates a new pattern, call that out explicitly
- If it modifies an interface/contract, list what depends on it and confirm nothing breaks

**Sim/ boundary rule:** If a task touches `sim/`, verify it introduces NO dependency on
`render/`, `ui/`, `sidebar/`, `audio/`, or `net/`. This is the #1 architectural invariant.

### Code Style Checklist

All code written in plan tasks must follow these patterns:

- **Ownership in signatures** — `&str` not `&String`, `&[T]` not `&Vec<T>`, `Arc<str>` not `Arc<String>`
- **Hot path** — no per-tick allocations (Vec::new, String::from, .clone() on non-Copy).
  Prefer reusable scratch buffers, interned IDs, or pre-allocated collections.
- **Edition 2024** — `#[unsafe(no_mangle)]`, `#[expect(lint)]` over `#[allow(lint)]`,
  inner `unsafe {}` blocks inside `unsafe fn`, `let-else` for early returns,
  let chains for nested `if let`
- **Iterators over index loops** — `.iter().filter().map()` over `for i in 0..vec.len()`
- **Error handling** — `?` for propagation, `.unwrap()` only on internal invariants,
  never on external input (INI, assets, files)
- **Every `unsafe` block** gets a `// SAFETY:` comment

---

## Plan Document Template

Save to `docs/plans/YYYY-MM-DD-<topic>-plan.md`:

````markdown
# [Feature Name] Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** [One sentence from design doc]

**Architecture:** [2-3 sentences — how this fits into the existing system]

**Design Doc:** [path to the design spec]

---

## Grounding Summary

[~10-20 lines — what the docs say, what Ghidra confirmed, which repo
pattern this mirrors, which INI keys drive behavior, what's still unknown]

## Key Technical Decisions

- [Decision]: [Rationale] — **Confidence:** high | medium | low
  - **Source:** docs/research/XYZ.md | Ghidra FUN_00abcdef | repo pattern src/sim/foo.rs | inferred

Low-confidence decisions MUST be flagged for `/review-plan` to verify
before implementation starts.

## Open Questions

### Resolved During Planning

- [Question]: [Resolution + source]

### Deferred to Implementation

- [Question]: [Why it can't be answered until we run the code — e.g.,
  "exact tick count depends on frame rate we observe in-game"]

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/sim/new_module.rs` | Handles X |
| Modify | `src/sim/existing.rs` | Add Y interface |

## Interface Changes

List any public APIs, traits, structs, or config schemas that are created or
modified. Note what depends on them.

## Sim Checklist

(Include only for tasks touching sim/)

- [ ] All math uses `fixed`-point — no f32/f64 in game logic
- [ ] New state included in deterministic state hash
- [ ] No dependencies on render/ui/sidebar/audio/net
- [ ] Tick ordering impact noted (if any)
- [ ] BTreeMap iteration order considered (if relevant)

## Risk Areas

From impact analysis — what might break, what needs regression tests.

## Player-Experience Critical Items

List milestone-blocking, compounding, exactification-residual, and unknown-risk
details from the design. Include the representative production scenario and why
each item does or does not block it.

| Task # | Class | Item | Why it matters | Verification |
|--------|-------|------|----------------|--------------|
| e.g. Task 5 | COMPOUNDING | Scroll pan interpolation | Visible repeatedly during radar navigation | Production side-by-side and focused timing check |
| e.g. Task 8 | MILESTONE-BLOCKING | Completed cameo becomes selectable | Blocks ordinary production commands | Retail-input production test |

The section may be compact, but it cannot omit authority, lifecycle,
determinism, command, or common player-visible risks. Exactification residuals
must state trigger, frequency, player effect, and downstream risk.

---

## Tasks

### Task 1: [Component/Interface Name]

**Why:** [One sentence — what this accomplishes and why it's ordered here]

**Files:**
- Create: `src/sim/new_module.rs`
- Modify: `src/sim/existing.rs:123-145`

**Pattern:** [Which existing codebase pattern this follows, or "new pattern"]

**Step 1: Define types**
```rust
// src/sim/new_module.rs
pub struct NewThing {
    pub field: FixedI32<U16>,
}
```

**Step 2: Write implementation**
```rust
impl NewThing {
    pub fn new(value: FixedI32<U16>) -> Self {
        Self { field: value }
    }
}
```

**Step 3: Add tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_thing_creation() {
        let thing = NewThing::new(FixedI32::from_num(42));
        assert_eq!(thing.field, FixedI32::from_num(42));
    }
}
```

**Step 4: Verify**
Run: `cargo test new_thing -- --nocapture`
Expected: PASS

**Step 5: Optional commit (only when explicitly requested by the user)**

### Task N: [Integration / Wiring]

(Later tasks wire components together and run full regression)

### Task N+1: [Verification against gamemd.exe]

**Why:** Confirm the implementation matches original engine behavior.

**Verify:**
- [Specific behavior to check against gamemd.exe]
- [How to verify — Ghidra decompilation, in-game test, /trace-action]
- [Expected result from original engine]

## Sources & References

- **Design doc:** docs/plans/YYYY-MM-DD-<topic>-design.md
- **Ghidra reports:** docs/research/XYZ.md, docs/research/ABC.md
- **gamemd.exe addresses:** FUN_00abcdef (name), 0x00123456 (struct offset)
  — kept here, not in Rust code comments
- **INI keys:** rulesmd.ini `[Section]` Key=..., artmd.ini `[Section]` Key=...
- **Related code:** src/sim/path.rs, src/rules/parse.rs
- **Prior PRs / commits:** [sha or PR #] if relevant
````

---

## Post-Plan Self-Review (mandatory)

Run this 10-point checklist before presenting to the user. Fix any gaps.

1. **Spec coverage** — every requirement in the design doc traceable to a task?
2. **Placeholder scan** — any TBD/TODO/vague steps? (search the plan text)
3. **Architecture check** — do tasks follow the patterns from the design doc?
4. **Interface ordering** — are contracts defined before they're consumed?
5. **Risk coverage** — do high-risk areas from impact analysis have regression tests?
6. **Self-containment** — could each task be done by someone with zero codebase context?
7. **Sim/ compliance** — if any task touches sim/, does the sim checklist pass?
8. **Grounding coverage** — did the plan cite the relevant combination of
   current code, production observation, research, INI/assets, and Ghidra?
   Ghidra is required when a consequential native uncertainty remains, not as
   ritual for every localized fix.
9. **Confidence tagging** — every Key Technical Decision has a confidence level
   and a source? Low-confidence decisions flagged for /review-plan?
10. **Deferred questions** — execution-time unknowns explicitly listed, not
    hidden as fake certainty in tasks?
11. **Player-experience items populated** — did you classify blocking,
    compounding, residual, and unknown-risk details and include a real production
    scenario?

---

## Present to User

In interactive work, show the plan and ask for review. Under an explicitly
autonomous goal, run the skeptical plan review, repair load-bearing findings,
record approval, and proceed without a routine user pause.

After approval, offer execution options:
- **Subagent-driven** — dispatch one task at a time to fresh subagents, review between
  tasks (recommended for complex plans)
- **Batch execution** — execute in this session in batches of 3-5 tasks with human
  checkpoints
- **Manual** — user takes the plan and runs it themselves or in a separate session

Do NOT start implementing. The user decides what happens next.

---

## Anti-Patterns to Flag

- **"God task"** — a task that does too many things (split it)
- **"Implicit dependency"** — Task 5 assumes something Task 3 did but doesn't say so
  (make it explicit)
- **"Test desert"** — a stretch of pure-logic tasks with no tests (add them)
- **"Architecture drift"** — the plan quietly deviates from design doc patterns
  (flag it, get approval)
- **"Interface surprise"** — a task changes a public interface without listing what
  depends on it (add the impact)
- **"Float in sim"** — any f32/f64 usage in sim/ code (must be fixed-point)
- **"Sim depends on render"** — any sim/ task that imports from render/ui/sidebar/audio/net

---

## Key Principles

- **Plans are for dumb executors** — assume zero codebase context, zero taste, zero
  judgment. Spell everything out.
- **Architecture-aware** — every task knows where it fits and which patterns it follows
- **Interfaces first** — define structs, traits, and contracts before implementations
- **Test where testable, verify where not** — unit tests for pure logic; gamemd.exe
  comparison for RE-driven behavior
- **YAGNI** — if it's not in the design doc, it's not in the plan
- **Self-contained tasks** — repeat information across tasks; they must stand alone
- **Intentional commits** — include commit steps only when the user explicitly
  requested commits; keep them small and atomic when authorized.
- **Risk-aware ordering** — surface problems early, don't build on a shaky foundation
- **Determinism is sacred** — sim/ tasks must preserve lockstep correctness
