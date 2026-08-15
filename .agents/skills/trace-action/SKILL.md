---
name: trace-action
description: "Trace a game mechanic or player action end-to-end through the engine pipeline, verifying each stage against gamemd.exe. Usage: '/trace-action shroud reveal when unit moves' or '/trace-action attack Grizzly vs Rhino at 5 cells' or '/trace-action ore growth spread' or '/trace-action power goes offline'"
---

# Trace Action — End-to-End Pipeline Diagnostic

Trace `$ARGUMENTS` through every stage of the engine — from trigger to sim state to render to screen. At each stage, compute the concrete values our code produces and verify them against gamemd.exe.

**The goal: "Does this complete production path behave convincingly in an
ordinary stock skirmish, where does it diverge from gamemd.exe, and which
divergences actually block the milestone?"**

This covers ANY game mechanic — not just player clicks. Shroud updates, power calculations, ore spreading, building placement rules, superweapon charging, veterancy, crate pickups, etc.

Use active gamemd behavior, retail inputs, and production Rust to trace every
load-bearing boundary. Preserve honest exact verdicts: an unproven internal
difference may remain DRIFT/UNCHECKED. Delivery impact is separate. Focus exact
mechanism work on differences that can affect common behavior, outcomes,
determinism, authority, lifecycle, RNG, persistence, or shared architecture.

---

## Iron Law — FOLLOW EVERY LOAD-BEARING STAGE

The point of an end-to-end trace is to find where the player journey actually
breaks or begins to feel wrong. Do not stop at helper functions or a
high-level summary. Check details in proportion to how they are consumed:

- The 1-tick delay between trigger and effect (does our code apply on the
  same tick or the next one?)
- The order of two writes to the same field within one tick
- The fixed-point rounding direction (truncate? floor? round-to-nearest?
  banker's rounding?)
- The `<` vs `<=` in a range check that flips behavior at the boundary
  cell
- The clamp that saturates one unit short of where gamemd's clamps
- The signed-vs-unsigned compare that flips at the wraparound
- The animation frame that gamemd starts at frame 1, not frame 0
- The sound cue that fires *before* the visual, not after
- The pixel offset that shifts the sprite by 1 in screen Y
- The Z-order tie-breaker when two sprites are on the same cell
- The field read that picks up the *previous* tick's value instead of
  this tick's (because it's read before its writer ran)

For every stage ask: what enters, what state changes, who consumes it next, what
the player experiences, and whether a difference is milestone-blocking,
compounding, an exactification residual, or unknown-risk. Literal equality is
required only for an exact `MATCH` claim. A stage may be `MILESTONE-PASS` from
production evidence while retaining named exact residuals.

## Step 0 — Parse the scenario

Interpret `$ARGUMENTS` as a game mechanic or scenario. If unclear, ask the user to clarify.

Examples of valid inputs:
- `shroud reveal when unit moves` — trace vision/shroud update pipeline
- `attack Grizzly vs Rhino at 5 cells` — trace the full combat sequence
- `move Harvester from (50,50) to (55,55)` — trace movement pipeline
- `power goes offline` — trace what happens when power drops below demand
- `ore growth spread` — trace ore growth tick logic + render
- `deploy MCV` — trace the deploy state machine
- `building placement GAWEAP` — trace placement validation
- `chrono miner teleport` — trace the teleport movement system
- `prism tower chain` — trace prism forwarding logic
- `gap generator shroud` — trace gap generator shroud re-covering
- `bridge repair` — trace bridge HP and state transitions
- `paradrop over target` — trace paradrop spawning + parachute descent

Extract what's needed:
- Any unit/building types involved — look up in `ini/rulesmd.ini` and `ini/artmd.ini`
- Relevant INI values for the mechanic (Speed, ROT, ROF, Sight, Power, etc.)
- Initial conditions (positions, health, power state, etc.)
- Use the research-index MCP first to find prior trace/research docs and Rust
  touchpoints. Prefer `research_brief`; if unavailable, use the repo-local CLI
  fallback:

  ```text
  python tools/research_index/brief.py "<scenario>" --limit 8
  ```

  If the exact system is known, add `--system <system>` (examples: `bridges`,
  `miner`, `skirmish-ui`, `chrono`). If the result reports zero docs, rerun
  without `--system`, then rerun with the inferred exact system. Use `search.py`,
  `graph.py evidence`, and `graph.py implementation` for focused anchors. The
  index is a source map; the trace verdict still requires concrete Rust values
  and gamemd evidence.

## Step 1 — Identify the trigger and trace the full chain

**What causes this mechanic to activate?** Map from the trigger event all the way to the visual result.

Every mechanic has a pipeline. Identify each stage by reading the code:

```
TRIGGER:     What initiates it (player click, tick timer, state change, etc.)
DATA:        What INI/rules values feed into it
SIM LOGIC:   The core computation (formulas, state machines, conditions)
STATE CHANGE: What sim state gets modified (entity fields, map state, etc.)
PROPAGATION: Does the state change trigger further effects? (chain reactions, updates)
RENDER:      How the state change becomes visual (sprites, animations, overlays)
SCREEN:      What the player actually sees
```

**Adapt the stages to the mechanic.** A shroud system has different stages than combat:

Shroud example:
```
TRIGGER:     Unit moves to new cell -> vision system runs
DATA:        Sight= from rules.ini, cell positions, height levels
SIM LOGIC:   Vision radius calculation, cell iteration, LOS checks
STATE CHANGE: ShroudMap cells marked as revealed
PROPAGATION: Revealed cells expose enemy units, update minimap
RENDER:      Shroud overlay sprites updated (black -> visible)
SCREEN:      Terrain/units appear where black shroud was
```

Power example:
```
TRIGGER:     Building destroyed / sold / built -> power recalculated
DATA:        Power= from rules.ini per building
SIM LOGIC:   Sum all Power values, compare drain vs supply
STATE CHANGE: House power state (normal/low), building powered flags
PROPAGATION: Unpowered buildings lose abilities (radar, production speed)
RENDER:      Powered anims stop, sidebar power bar updates
SCREEN:      Buildings go dark, radar goes offline
```

Identify the actual functions in our codebase for each stage. Note file and line.

## Step 1.5 — Enumerate ALL entry points (lifecycle coverage)

**This step prevents missed code paths.** For the mechanic's trigger event, grep/search the codebase for EVERY code path that can cause it. Don't rely on the task description or your assumptions — search exhaustively.

Ask: **"What are ALL the ways this state change can happen?"**

Examples:
- "Building enters the world" → search for `spawn_object`, `spawn_from_map`, `spawn_at_height`, `entities.insert` — find ALL spawn paths
- "Building leaves the world" → search for `entities.remove`, `dying = true`, `despawn`, `sell_building` — find ALL removal paths
- "Entity changes owner" → search for `owner =`, `capture`, ownership transfer paths
- "Power balance changes" → building placed, sold, destroyed, captured, powered on/off

**Format:**
```
ENTRY POINTS for "<trigger event>":
  1. <path> — <file>:<line> — <when it fires>
  2. <path> — <file>:<line> — <when it fires>
  ...
  COVERAGE: Which paths have the hook? Which are missing?
```

If any path is missing the necessary hook/call, flag it as a gap BEFORE implementation begins. This is the #1 source of "it works for production buildings but not map buildings" bugs.

## Step 2 — Trace concrete values through each stage

For the specific scenario, compute the actual value at each pipeline stage with real numbers.

**Format each stage as:**
```
STAGE N — <name> (<file>:<line>)
  Input:   <what enters this stage>
  Formula: <the computation, with actual numbers>
  Output:  <what this stage produces>
  gamemd:  <what the original produces — from Ghidra or docs>
  Verdict: PASS / FAIL / UNCHECKED
```

**Rules:**
- Use actual INI values from `ini/rulesmd.ini` and `ini/artmd.ini`, not hypothetical ones.
- Compute with real numbers — show `Sight=8, radius=8 cells, area checked=201 cells`.
- For gamemd verification, search in this order (exhaust each before the next):
  1. **`docs/`** and **`<main-checkout>/docs/research/`** — written research reports, including the ignored in-repo research corpus
  2. **INI files** — `ini/rulesmd.ini`, `ini/artmd.ini`
  3. **Ghidra MCP** — last resort, only for gaps or spot-checks the docs don't cover
- Include intermediate values that could diverge (fixed-point truncation, integer division, rounding).
- **Show the tiny stuff explicitly.** For each stage, write down — even if
  "obviously the same":
  - Which tick the stage runs on, relative to the trigger (`tick T` or
    `tick T+1`)
  - Order of operations within the stage (which field is read or written
    first, second, third)
  - Rounding mode for any division or fixed-point conversion
  - Clamp bounds, inclusive vs exclusive
  - Whether comparisons are signed or unsigned
  - Whether the stage reads pre-update or post-update values from the
    fields it depends on
  These are the things that look "obvious" and are wrong half the time.
  Listing them forces you to actually check.
- If a stage is **not implemented** in our engine, mark it explicitly: `NOT IMPLEMENTED`.
- If you didn't actually compute both numbers (ours AND gamemd's), the
  stage is `UNCHECKED`, not `PASS`. Don't fill in `PASS` from intuition.

## Step 3 — Verify critical logic against gamemd.exe (targeted)

**Ghidra MCP is the last resort.** Most answers should already be in the research docs from Step 2. Only use Ghidra when:
- The docs don't cover a specific formula or threshold
- A doc's claim seems suspect and needs spot-checking
- The 1-2 most gameplay-critical computations need hard confirmation

When you do use Ghidra:
- Decompile the relevant function, extract constants and branching
- Compute the expected output for the scenario's inputs
- Compare against our code's output
- Check `param_1` type: if `int` offsets are direct bytes; if `int *` multiply by 4
- **TS vs YR:** gamemd.exe serves both Tiberian Sun and Yuri's Revenge. Verify that
  code you're tracing is actually active in YR, not dormant TS legacy. Don't trust
  field/flag names — trace what they actually gate. TS-era names like "Tiberium"
  may control something different in YR context (e.g., `Tiberium=` on warheads only
  gates vein destruction, not ore destruction).
- **Labels are navigation hints.** Confirm a cited function from its body, callers,
  receiver flow, and vtable binding. Vtable identity requires a
  COL→TypeDescriptor owner walk plus slot read and body decompile. If identity is
  unproven, cite the address and mark the stage `UNCHECKED`, not PASS/FAIL.
- When live Ghidra is used, record annotation candidates separately. Include the
  address/source, current metadata, proposed label/comment/reference, and exact proof.
  A worker reports candidates only; it never mutates Ghidra.

A single matching trace is evidence, not proof of equivalence. Downgrade a
different mechanism from DRIFT only with algebraic proof, exhaustive vectors over
a finite domain, or a suitable gamemd/retail-derived executable oracle.

## Step 4 — Check the visual result

Trace from sim state to what the player sees:

- **Does the right thing appear on screen?** (shroud clears, building goes dark, unit moves, etc.)
- **At the right position?** (coordinate conversion, offsets, anchoring)
- **At the right time?** (immediate vs delayed, animation timing)
- **With the right appearance?** (correct sprite/overlay/color, correct animation frame)

If any part of the render pipeline is missing or wrong, note it.

## Step 5 — Timing and sequencing

For mechanics that unfold over time:

- **How many ticks from trigger to visible result?** (Is there a 1-tick delay? A multi-second animation?)
- **What's the sequence of events?** (What happens first, second, third?)
- **Does timing feel and behave correctly in the representative scenario?**
  Record exact tick/frame/order differences and classify their milestone impact.

For instantaneous mechanics (like shroud reveal), verify it happens on the same tick as the trigger, not a tick later.

## Step 5.5 — Optional Ghidra annotation sync

By default, stop after reporting annotation candidates. If `--sync-ghidra-labels` was
provided or the user directly requested synchronization, the root/standalone trace waits
for every reader to stop and follows ENGINE.md's serial sync protocol. A read-only
request or `--no-sync-ghidra-labels` disables synchronization. Workers remain read-only.

## Step 6 — Report

Present:

1. **Pipeline diagram** — the full chain from trigger to screen, one line per stage
2. **Milestone failures** — stages that break or repeatedly distort ordinary play
3. **Not implemented** — stages gamemd has that we're missing entirely
4. **Residuals** — exact differences and unknowns that do not currently block
5. **Timing** — noticeable/compounding timing errors first, then bounded residuals

6. **Ghidra annotations** — candidate/applied/deferred count and exact addresses, or `None`

For each failure state:
- Which stage, what our code does vs gamemd
- Root cause
- What consumed state, rendered pixel, sound, decision, or timing can differ

**Keep it concise.** Lead with milestone failures and their earliest root cause.
Preserve exact differences in the residual section without letting them bury the
production result.

Do NOT implement gameplay/source fixes — only report findings unless the user explicitly
asks. Ghidra synchronization is optional under Step 5.5, not a default side effect.
