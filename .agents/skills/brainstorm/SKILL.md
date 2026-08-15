---
name: brainstorm
description: >
  Architecture-aware brainstorming before any feature work, new component,
  refactor, or behavioral change. Explores user intent, maps existing
  architecture, evaluates fit, and produces a design spec before any code
  is written. Usage: '/brainstorm topic', e.g. '/brainstorm ore refinery
  docking system' or '/brainstorm refactor movement into sub-modules'.
---

# Brainstorm — Architecture-Aware Design Skill

Brainstorm a design for: **$ARGUMENTS**

If `$ARGUMENTS` is empty, ask the user what they want to brainstorm.

---

## Hard Gate

**NO code, scaffolding, or implementation until the design is approved.** In an
interactive task, approval comes from the user. In an autonomous `/goal` that
explicitly grants self-approval, the coordinator may approve its own design
after an adversarial review that asks why it should be approved, what could
still make ordinary skirmish feel wrong, and what could create expensive later
rework. Record the decision; do not pause for routine approval.

Do NOT invoke any implementation, editing, or code-writing tools during this skill.
Research and read tools only.

Native-to-Rust translation rule: do not design raw pointer vectors,
COM/vtable plumbing, global mutable singleton style, or the full native
inheritance tree just because the binary uses them. Design Rust-native
ownership boundaries that preserve retail-convincing behavior and load-bearing
semantics: storage owners, scheduler/order owners, lifecycle helper APIs, plain
subsystem functions, and ordered commit points. Explain where deterministic
state, authority, lifecycle, and important player-visible effects live.

Current delivery target: an experienced YR player should be able to play an
ordinary 30–60 minute stock skirmish without repeatedly noticing differences.
Exact findings remain honest residuals, but only milestone-blocking or
compounding differences block the design.

Label-adversarial Ghidra rule: local Ghidra names, labels, comments, xref
labels, and decompiler-assigned symbol names are navigation hints only. This
project may contain polluted or stale labels from earlier scripts. Do not let a
convenient Ghidra name become the design premise unless the function body,
callsites, receiver/`this` pointer, vtable/data references, and active-YR
reachability prove that role. Designs should use address plus verified role when
labels are suspect.

---

## Process — Follow these 8 steps in order

### Step 1: Explore Project Context

- Read AGENTS.md, recent git log (last 10 commits), and any relevant docs/ files
- Use the research-index MCP before broad manual doc search. Prefer
  `research_brief(query=<topic>, system=<system>)`; use the repo-local CLI only
  when the corresponding MCP capability is unavailable:

  ```text
  python tools/research_index/brief.py "<topic>" --limit 8
  ```

  If the exact system is known, add `--system <system>` (examples: `bridges`,
  `miner`, `skirmish-ui`, `chrono`). If the result reports zero docs, rerun
  without `--system`, then rerun with the inferred exact system. Use
  `graph.py implementation`, `handoff.py`, and `search.py` for focused Rust
  touchpoints and mechanism anchors. Treat index output as navigation; verified
  docs, Ghidra, INI/assets, current Rust reads, and tests still decide parity.
- Search `<main-checkout>/docs/research/` for any relevant
  existing `*_SYSTEM_MODEL_SYNTHESIS.md` files.
  Use them as orientation maps for current understanding, stale-doc warnings,
  implementation-safe facts, and unresolved research gaps.
- Do not treat synthesis files as primary evidence. For parity constraints,
  follow their source ledger back to the cited Ghidra report, trace, INI key,
  address, or Rust surface and cite that underlying source in the design.
- Treat contradictions as primary evidence. If a screenshot, runtime result, test
  failure, trace, asset dump, or user observation disagrees with the current
  model, mark the design input incomplete and resolve the contradiction before
  proposing implementation approaches.
- Stale docs are warnings and source maps, not blockers. If old research conflicts
  with primary evidence, design from the primary evidence and call out the stale
  claim as documentation follow-up.
- Identify relevant parts of the tech stack for this feature
- Note project conventions that apply (architecture rules, module size limits, etc.)

### Step 2: Map Existing Architecture

This is the most important step. Understand before proposing.

- Identify the major components/modules the proposed work touches
- Map the dependency graph — what calls what, what imports what
- Identify **boundaries** — where does this system end and another begin?
- Identify **interfaces/contracts** — public APIs, data formats, event flow
- Identify **patterns already in use** — how are similar things done in this codebase?
  (e.g., how other sim systems are structured, how INI data flows, how entities interact)
- Locate where the proposed work would **fit** in this picture

Use local search/read tools such as `rg` and direct file reads. Only use Codex
subagents if the user explicitly asked for delegated or
parallel work. Be thorough — read the actual code, don't guess from file names.

**Output:** an "Architecture Context" section summarizing what you found. Can be a few
sentences for small features, a proper component map for larger ones.

### Step 3: Ask Clarifying Questions

Present questions to the user **one at a time** (or in small focused batches).

- Prefer multiple choice when possible
- Focus on: purpose, constraints, success criteria, edge cases
- Ask about **scope** — is this one thing or should it be decomposed?
- Ask about **priorities** — what matters most (correctness, performance, simplicity)?
- If the gamemd.exe binary is relevant, ask whether RE investigation is needed first
- If the feature involves gamemd.exe RE findings, ask: **has the TS vs YR distinction
  been verified?** gamemd.exe serves both Tiberian Sun and YR — systems, fields, and
  code paths may be TS legacy (disabled/repurposed in YR). Don't design around a
  mechanic that isn't actually active in a standard YR skirmish.

**Scope-recommendation rule.** Never narrow scope in a way that leaves the
selected ordinary-skirmish loop visibly broken, changes outcomes, creates
determinism/authority/lifecycle debt, or pushes the defect into a neighboring
common path.

A difference may be deferred when it is TS-legacy/inactive, blocked by an
external prerequisite, or classified as an `EXACTIFICATION-RESIDUAL`: bounded,
uncommon in ordinary play, mainly expert-probed, non-compounding, and safe for
later architecture. Record its trigger, player effect, expected frequency, and
downstream risk. If those are unknown, classify it `UNKNOWN-RISK` and
investigate only enough to decide whether it blocks the milestone.

Do not proceed while a material ambiguity remains. In an interactive task, ask
the user. In an explicitly autonomous goal, resolve discoverable questions,
choose the smallest robust assumption for the representative scenario, record
it, and continue; stop only when the choice would materially change intent and
cannot be recovered safely.

### Step 4: Dependency & Impact Analysis

Based on Steps 1-3, analyze:

- What existing code does this feature **touch or depend on**?
- What existing code **depends on what we're changing**?
- What might **break**? What is the blast radius?
- Are there migration concerns (data format changes, config, API compatibility)?
- For sim/ changes: does this affect tick ordering, determinism, or state hashing?

**Output:** an "Impact Analysis" section listing touched files/modules and risk areas.

### Step 4.5: Player-Experience Detail Ledger

**Before proposing approaches**, list details that could silently make the
selected ordinary-skirmish scenario feel wrong or create expensive later
rework. Include timing, constants, ordering, RNG, state/lifecycle, asset lookup,
draw composition, audio triggers, and edge results in proportion to their risk.

Classify each item:

- `MILESTONE-BLOCKING`
- `COMPOUNDING`
- `EXACTIFICATION-RESIDUAL`
- `UNKNOWN-RISK`

Pull evidence from research docs, synthesis outputs as source maps, INI/assets,
current Rust, production observations, and Ghidra when needed. Do not invent
details. An unknown does not automatically block design: investigate enough to
learn whether it can affect the representative scenario, deterministic state,
shared architecture, or downstream systems.

Trace from the behavior owner, not a helper. For UI, start at the paint/message/
render owner; for gameplay, start at the tick/update/action owner; for audio,
start at the event trigger path; for parsing, start at the actual load/apply
path; for input, start at dispatch and follow through to command effect. A helper
belongs in the design only after the target owner is proven to call it.

Always prove activation for the target scenario. Record the game, mode, dialog,
map, unit, house, difficulty, or setting that the design targets, plus the
flag/key/case/default that enables the behavior. Code existing in gamemd.exe is
not enough to make it a design constraint for standard YR.

Inspect variants required by the representative scenario before generalizing.
Check relevant SHP frames, YR `*md` overrides, dialog cases, difficulty,
faction, map, and optional branches. Uninspected variants outside the current
match matrix become named residuals rather than automatic blockers.

Format as a bulleted ledger. Examples of what belongs in it:

- Exact timing: "trigger fires on tick T; effect applies on tick T+0
  (same tick), not T+1"
- Exact constants: clamps, thresholds, frame counts, ROT values, ROF
  cooldowns, animation lengths
- Order of operations within a tick (which write happens before which)
- Rounding direction for any fixed-point / integer math
- Inclusive vs exclusive bounds in range checks
- Signed vs unsigned comparisons
- Default values when a field is uninitialized
- Edge cases: what happens at zero, at max, at empty container, at null
- Which animation frame an effect starts on (0 vs 1)
- Sub-tick ordering between paired events (sound vs visual, write vs read)
- Z-order tie-breakers between overlapping sprites
- Pixel-level draw offsets / anchor points
- Side effects: writes that look dead but are read by another system

For UI, shell, sidebar, menu, viewport, sprite, and other visual work, the
ledger must be a composition ledger, not a summary. Start from the full top-level
paint/message/render path and include:

- Draw order from entry point to return, including post-background and optional
  helper calls
- Per-dialog or per-mode flags that enable each optional draw layer
- Exact asset names and frames; dump or inspect all SHP frames before choosing a
  design around an asset
- Source and destination rects, anchors, clipping/scissor, palette/convert path,
  and z-order
- Asset role classification: content/preview, chrome/container, overlay,
  transition-only, loaded-but-inactive, or not used for the scoped role

If a user screenshot, reference capture, or asset dump contradicts the current
research conclusion, the research is incomplete. Do not design around the
contradictory conclusion; reopen the paint path and find the missing layer,
frame, flag, rect, or palette first.

Each line cites a source: `[GHIDRA <addr>]`, `[doc: X_GHIDRA_REPORT.md §3]`,
`[ini: rulesmd.ini Speed=]`, or `[UNKNOWN — needs RE]`.

The ledger does not need to be exhaustive of everything the system does. It
must cover details that could break the representative scenario, compound
through a normal match, or create costly architecture debt. Every approach
must explain how it preserves blocking/compounding items and where it records
exactification residuals.

### Step 5: Propose 2-3 Approaches

For each approach, include:

- **How it works** — brief description
- **Architectural fit** — does it follow existing patterns or introduce new ones?
- **Experience fit** — does this make the selected production scenario
  retail-convincing without unsafe shortcuts?
- **Detail coverage** — state where every milestone-blocking or compounding
  item lives and list residuals explicitly.
- **Trade-offs** — complexity, performance, maintainability, tech debt
- **What it touches** — which modules/files/interfaces change
- **Risk areas** — what could go wrong

**Retail-convincing behavior is a first-class evaluation criterion.** For each
approach, answer:

- What active-gamemd mechanisms does the original have here? (branching,
  state bytes, timing, visuals, input feel, draw order, audio cues, cursor
  behavior near boundaries)
- Which details are required for the representative production path,
  deterministic state, and shared architecture?
- Which differences remain honest DRIFT but are safe exactification residuals?
- Would any small difference be frequent, noticeable, compounding, or
  outcome-changing? If so, keep it in scope regardless of implementation size.
- **Be suspicious of "clean" designs that don't mention any of the
  ledger items.** Cleanness is often achieved by abstracting away exactly
  the details that drive behavior. If an approach reads as elegant but its
  player-experience ledger is empty, that's a red flag, not a strength.

Lead with your recommendation and explain why. Call out if an approach introduces
architectural inconsistency, tech debt, OR parity drift — and whether that's acceptable.

In interactive work, present these to the user and wait for a choice. In an
explicitly autonomous goal, choose the recommendation after adversarial
self-review and continue.

### Step 6: Present Design

Scale each section to the complexity of the task. A small feature might need a few
paragraphs; a major system needs full treatment.

Cover as appropriate:
- Architecture / component layout
- Data structures and flow
- Interfaces and contracts
- Integration points with existing code
- Error handling strategy
- Testing strategy
- Determinism considerations (for sim/ work)

**Explicitly state:**
- How the design integrates with existing architecture
- Which existing patterns it follows or deviates from (and why)

Present in sections and get approval after each major section during interactive
work. Under an explicitly autonomous goal, review each section internally,
record load-bearing objections and their resolution, then continue.

### Step 7: Write Design Doc

Save to `docs/plans/YYYY-MM-DD-<topic>-design.md` using this structure:

```markdown
# [Feature Name] Design

## Goal
One sentence.

## Architecture Context
How the existing system works in the area this feature touches.
Key components, interfaces, data flow.

## Impact Analysis
What this changes, what depends on it, risk areas.

## Chosen Approach
What we're building and why this approach over alternatives.

## Player-Experience Detail Ledger
Milestone-blocking, compounding, residual, and unknown-risk details. Cite the
source for claims and state why each item does or does not block the selected
ordinary-skirmish scenario.

## Design

### Components
### Interfaces / Contracts
### Data Flow
### Error Handling
### Testing Strategy

## Architectural Decisions
Patterns followed, patterns deviated from (and why).
Tech debt introduced (if any) and plan to address it.

## Alternatives Considered
Brief summary of rejected approaches and why.
```

### Step 8: Hand Off

For interactive work, tell the user the design is ready and ask whether to plan,
refine, or park it. Under an explicitly autonomous goal, hand the approved
design directly into proportional planning and implementation.

---

## Anti-Patterns to Flag

During brainstorming, actively call out these anti-patterns if you spot them in a
proposed design (including your own proposals):

- **"Just bolt it on"** — adding a feature without considering where it fits
  architecturally. Ask: where does this belong in the module hierarchy?
- **"New pattern for no reason"** — introducing a different way of doing something
  the codebase already has a convention for. Ask: how is this done elsewhere?
- **"Hidden coupling"** — a design that looks clean but creates invisible dependencies
  between modules. Ask: what breaks if we change X?
- **"Scope creep"** — the feature quietly growing beyond what was asked for.
  Apply YAGNI ruthlessly. Ask: do we actually need this part?
- **"Testing afterthought"** — a design where testability wasn't considered upfront.
  Ask: how do we test this without spinning up the whole engine?
- **"TS ghost"** — designing around a mechanic found in gamemd.exe that is actually
  dormant Tiberian Sun legacy, not active in Yuri's Revenge. Ask: is this code
  actually executed in a standard YR skirmish? Don't trust field names — verify
  what they gate in the binary. (Example: WarheadTypeClass `Tiberium=` does NOT
  control ore destruction — it only gates vein destruction.)
- **"Convenience-disguised deferral"** — labelling an item "edge case", "rare",
  or "low priority" without naming its trigger, ordinary-play frequency, player
  effect, compounding risk, and downstream consumers. Effort is not a valid
  impact measure.
- **"First-helper fallacy"** - treating one helper's draw stack as the whole UI
  composition. Ask: what does the parent paint handler draw after this returns?
- **"Frame-zero tunnel vision"** - assuming SHP frame 0 is the relevant visual.
  Ask: were all frames dumped or did Ghidra prove the exact frame?
- **"Negative-role overreach"** - converting "not used as X" into "not visible at
  all." Ask: which visual roles were actually ruled out?
- **"Literal C++ port reflex"** - proposing to recreate native inheritance,
  raw pointer vectors, COM/vtable plumbing, or global singleton mutation in Rust
  just because gamemd uses those mechanisms internally. Ask: what exact behavior
  contract must be preserved, and which Rust-native owner should preserve it?
- **"Clean-Rust behavior loss"** - proposing an elegant Rust abstraction that drops
  native ordering, lifecycle, RNG, timer, or same-tick consequences. Ask: where
  does each native tiny-detail ledger item live in the design?

---

## Key Principles

- **One question at a time** — don't overwhelm the user with 10 questions
- **Architecture first** — understand the system before proposing changes
- **YAGNI for speculative and out-of-matrix work** — preserve known residuals
  in the ledger instead of implementing every possible native branch now.
- **Complete player loops beat exhaustive local scope** — a smaller slice is
  wrong when it leaves the selected production journey visibly broken or
  creates deterministic/architectural debt. Expert-only exactification may
  remain recorded for later.
- **Visual parity requires composition closure** - for UI/render work, trace from
  the paint/render entry point to return before deciding what the player sees.
- **Explore alternatives** — always propose 2-3 approaches, never just one
- **Name the trade-offs** — especially architectural ones
- **Incremental validation** — present design in chunks, get approval
- **Patterns matter** — follow existing conventions unless there's a good reason not to,
  and document the deviation
- **Determinism is sacred** — for any sim/ work, prove the design preserves lockstep
  correctness
- **Rust-native structure, retail-convincing behavior** - preserve verified
  authority, deterministic state, common-path ordering, RNG use, lifecycle, and
  player-visible effects; record bounded exact residuals honestly
