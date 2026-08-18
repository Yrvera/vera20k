# GOAL: finish the random map generator (RMG) in vera20k

**Success condition (one sentence):** every option the Create Random Map dialog
can produce — all five map types, all theaters, all seeds — generates a complete,
playable, deterministic map through the normal load path, with no stage in the
pipeline left as a no-op and no residual from the three RMG plans unclosed or
unrecorded.

Work item by item, in the order below. Each item is done when its acceptance
criterion holds **and you have pasted the literal command output that proves it**.
Do not batch items; commit each one separately.

---

## Step 0 — set up an isolated worktree (do this first, before reading anything else)

Parallel sessions run on `dev` in the primary checkout. Work on a branch in a
dedicated worktree so nothing collides. From
`.`:

    git worktree add ../ra2-rust-game-rmg -b feature/rmg-p3 dev

A fresh worktree is missing every gitignored dependency this work needs — without
`ini/` the crate does not compile at all (`include_str!`), and without `docs/` none
of the plans or research reports cited below exist. Link them in (junctions, not
copies, so research written during this work is visible from every checkout):

    cd ../ra2-rust-game-rmg
    cmd //c mklink /J ini      ..\ra2-rust-game\ini
    cmd //c mklink /J docs     ..\ra2-rust-game\docs
    cmd //c mklink /J .claude  ..\ra2-rust-game\.claude
    cp ../ra2-rust-game/CLAUDE.md ../ra2-rust-game/.mcp.json .

Gate before starting item 1 — all three must hold, paste the output:
1. `ls ini/rulesmd.ini docs/research/skirmish-ui docs/plans` resolves.
2. `cargo check -p vera20k` completes (first build is cold, several minutes —
   run it in the background and wait on it; never interrupt a cargo build).
3. `git branch --show-current` prints `feature/rmg-p3`.

Then read `CLAUDE.md` in full before touching code.

---

## Context: where the generator stands

`src/map/rmg/` is a complete, deterministic pipeline for **map types 0–2, all
theaters**, verified in-game (commit `cacc073f`: a temperate 74×82 2-player map
loaded, all 319 tiles resolved, sim ran 1440 ticks on 573 generated ore cells).
The Create Random Map dialog (`0x105`), the `.SED` writer, the saved-seed browser,
worker-thread generation and the progressive preview all landed (commits
`c371c6d0` … `928644d4`).

Authoritative documents — read the relevant one before each item, do not work
from this prompt's summaries alone:

- Design + tiny-detail ledger (items 1–66):
  `docs/plans/2026-07-19-random-map-generator-design.md`
- Pipeline plan, per-task status:
  `docs/plans/2026-07-20-rmg-plan2-terrain-phases-plan.md`
- Dialog plan and its review (§9 has three open backend gates):
  `docs/plans/2026-07-21-random-map-setup-dialog-plan.md`,
  `…-plan-REVIEW.md`
- Progressive preview + the progress-bar follow-up:
  `docs/plans/2026-07-22-incremental-random-map-generation-plan.md`
- Ghidra reports: `docs/research/skirmish-ui/RMG_*.md`,
  `SKIRMISH_RANDOM_MAP_*.md`, `SKIRMISH_CREATE_RANDOM_MAP_*.md`

Search them with the `research-index` MCP server (`research_search`,
`research_related`, `research_brief`), not raw grep.

---

## Work items

### 1. Map types 3/4 are not generated at all — the largest gap

`src/map/rmg/pipeline.rs:183` fires the `IslandPasses` stage observer and does
nothing else. Choosing "inland" or "mountainous" in the dialog silently produces a
map shaped like map type 0/1/2 — wrong terrain, no rivers, no lakes, no bridges,
no terracing. This fires on any run where the player picks those types, and on
~40% of Randomize rolls (`RandomRanged(1,4)`).

Note the naming trap: `IslandPasses` is a **misnomer**. Types 3/4 are *inland*
and *mountainous*; archipelago is map type **0** and gets its water from the
normal water path. Renaming the stage is cosmetic — do it, but do not let the
name mislead the implementation.

Decoded already, in
`docs/research/skirmish-ui/RMG_MODE34_WATER_BRIDGES_TECH_GHIDRA_REPORT.md` and
design-doc ledger items 54–58: driver `0x0059C580`, river carver `0x0059D510`,
lake grower `0x0059C920`, the `MapSeed+0x310` river-bridge enable draw, region
rebuild + terracing `0x0058EBC0`/`0x0058D620`, bridge pass `0x0058EF10` /
`0x0058F0C0` / `0x005905D0`, cliff drops `0x005A19E0`, water re-anchor
`0x005A17F0`/`0x005A1350`.

**Two things are explicitly still undecoded** and must be closed in Ghidra before
implementation: the low-bridge **deck-helper internals** (ledger 56 marks them
deferred) and the region-rebuild ordering relative to the connector pass.

Sequence for this item — do not skip a phase:
1. `/verify-doc RMG_MODE34_WATER_BRIDGES_TECH` — that report is from 2026-07-20
   and everything else here rests on it. Fix what it gets wrong before building.
2. Close the two undecoded holes in Ghidra. Label what you prove (function names,
   globals, plate comments citing the `decompile_function` call) and
   `save_program` — per CLAUDE.md, do this without being asked.
3. `/write-plan` for the implementation, then implement it.

**Acceptance:** map types 3 and 4 run their own phase module(s) in `STAGE_ORDER`;
generation for both types is deterministic across repeat runs; a generated type-3
and type-4 map each load through `build::generate_map` → `emit::populate` with
every emitted tile index in range; RNG draws are consumed at the documented points
even where the result is discarded (the `U[0,100]` connector roll, the 0.25f
bridge-enable draw).

### 2. Three backend gates from the dialog-plan review §9

Verify each in the binary this session before changing code — the review's own
headline was that one "mismatch" turned out to be a false alarm.

1. The `water_amount != 0` test is applied by the native **only** on the 3/4
   branch. Check where the port applies it.
2. Tech buildings run **only when `map_type != 0`**.
3. Rocks are theater-gated (`+0x38 == 0`), not map-type-gated — the port already
   matches; confirm and leave alone.

**Acceptance:** each gate matches the binary, with the `decompile_function`
address cited in the commit message; a test pins each gate's on/off behaviour.

### 3. Neutral tech buildings never appear on a generated map

`src/app_init.rs:392` passes `&[]` for `tech_types`. `phases/tech_buildings.rs`
is fully implemented and tested but is being handed an empty list, so every
generated map ships without oil derricks, hospitals or any other neutral
structure. Fires on every generated map with `map_type != 0`.

Resolve `NeutralTechBuildings` (`ini/rulesmd.ini:3082`) into `TechType` values,
including each type's footprint from `Foundation=` in `artmd.ini` (art INI owns
all footprint data — never hardcode), and wire the real list at the callsite.

**Acceptance:** a generated non-type-0 map contains `[Structures]` entries owned
by the Neutral house whose names all appear in `NeutralTechBuildings`, and whose
footprints do not overlap terrain the phase rejects. Test asserts non-empty.

### 4. Hills slope-fixup half is deferred

`src/map/rmg/phases/lat_fixup.rs:19` documents it: the native per-cell slope-type
byte (`+0x11C`, values 0..4) is a **different quantity** from the port's 0..18
ramp-variant index, so the dispatch was left out. RE the slope-type byte's
producer and consumers, then implement the dispatch.

**Acceptance:** the slope-fixup dispatch runs in `RecalcAfter*`; a fixture with
known cliff/ramp adjacency produces the documented tile choices; the existing 12
`lat_fixup` tests still pass.

### 5. The dialog's progress bar is drawn empty for the entire generate block

Now visible for the whole (multi-second) generation since the worker landed.
`src/ui/skirmish_shell/…/modals.rs` draws a bevelled outline and never fills it.

Known already (tail of the incremental-generation plan, verified 2026-07-23 and
labelled in Ghidra): control id is **0x639**;
`ProgressMeterClass__SetPercent @ 0x00643C50`,
`__Redraw @ 0x00643AE0`, `__DrawFill @ 0x00643400`; fill rect starts at
(x+3, y+3), width is `Math__ftol(fraction × frame-0 width)`; repaint happens only
when the stored value actually changes.

**Two facts are still missing and both are pixels** — recover them from xrefs to
the meter singleton at `0x00AC4F58`: which SHP `this+0x54` holds, and whether the
solid-colour path (`this+0x71`) is on for this dialog and with what colour.
Do **not** fill the bar before you have both; inventing a visual is worse than the
current drift. If the SHP cannot be identified, stop and report that — do not
guess a shape.

Percent ladder to drive it, from the confirmed preview points:
55, 60, 70, 80, 85, 90, 95.

**Acceptance:** the bar fills during generation using the real SHP, at the seven
documented percentages, with a screenshot of a live generate as evidence.

### 6. Dead skeleton: `map::rmg::generate`

`src/map/rmg/mod.rs:206` still returns `emit::empty_map_file` and only records
stage names. It has **no callers outside its own tests** — the real entry point is
`build::generate_map`. An exported function that silently returns an empty map is
a trap for the next session. Delete it (moving the stage-order tests to whatever
still owns `STAGE_ORDER`) or make it delegate to `build::generate_map`.

**Acceptance:** no path can obtain an empty `GeneratedMap` from a successful call;
`cargo check -p vera20k` clean.

### 7. Plan-2 Task 16 — the verification pass that was never run

Read Task 16 in `2026-07-20-rmg-plan2-terrain-phases-plan.md` and execute all
five steps:
1. Determinism matrix: seeds {0, 1234, 0xFFFF} × map types {0,1,2,3,4} ×
   theaters {0,1} — generate twice, assert byte-equal cells, overlays, terrain
   objects and waypoints (full content hashing, not just a cell count).
2. Headless e2e: feed a generated map through `load_map_from_initial`; assert the
   loader accepts it and waypoint capacity ≥ NumPlayers.
3. Per-phase draw-stream ledger: one test per phase asserting the total RNG draws
   consumed on a fixed tiny fixture.
4. An AUDIT_LOG line recording per-phase status — every phase labelled either
   "formula-verified vs doc <name>" or **UNVERIFIED-pending-instrument**.
5. Full `cargo test -p vera20k`.

**Acceptance:** all five done, with the literal `test result:` line pasted.

### 8. Two owed tests, blocked on a missing `AppState` harness

`test_preview_snapshot_matches_direct_rasterise` and
`test_closing_mid_generate_discards_late_frames` (incremental-generation plan,
§"Owed tests"). Both need an `AppState`, which owns a `Window` and a
`GpuContext`. One harness unblocks both and makes the whole poll/close path —
currently the least-covered code in this feature — testable.

If building that harness turns out to require restructuring app init, stop and
report the cost instead of doing it; it is the one item here where the price is
genuinely unknown.

### 9. Debug-build panic reachable from a real seed

`src/map/rmg/phases/starts.rs:307` and `:332` can `debug_assert!(false)` on
start-starved maps. **Acceptance:** a start-starved configuration returns a
recoverable error or retries the way the native does (check which), and a test
covers it. Establish what the native does before choosing.

### 10. Tail items (do last, or report as deliberately deferred)

`[Lighting]` emit from the theater ambient vectors, and the ore-patch-lamp lists
in `ini/rmgmd.ini` (Task 15 step 1). Ledger §10.4 records the lamps as unused —
confirm that before writing code for them.

---

## The certification gap — read before making any parity claim

Every RMG test in the repo today is Rust-vs-prior-Rust. Per CLAUDE.md those are
**regression ratchets, not parity evidence**: nothing currently proves a generated
map equals what gamemd.exe produces from the same `.SED`. Do not write
"matches gamemd" about any generator output.

At the end, **propose** (do not build unasked) a gamemd-derived golden-map oracle:
fixed `RandMap.Sed` → capture the original's generated map → byte-compare cells,
overlays, terrain objects and waypoints. Until it exists, every RMG parity
statement is `UNVERIFIED-pending-instrument` and must be labelled that way.

---

## Constraints — boundaries on this work

- **Do not touch `sim/`.** The generator is map-layer, pre-sim. The
  `sim_does_not_reference_the_generator` guard in `emit.rs` must keep passing.
- **Do not change `generate_map`'s internals for cosmetic reasons.** The x87
  math, the RNG consumption order and `STAGE_ORDER` are what all generation
  parity rests on. Item 1 adds phases; it does not restructure existing ones.
- **Do not hand-compute a golden value.** Machine-derived only (binary emulation,
  live capture, retail file bytes). Hand-computed goldens have produced wrong
  references in this repo before.
- **Do not re-baseline any committed golden or ratchet hash** if the tree carries
  another session's unmerged changes. Record the delta in
  `docs/scans/PENDING_REBASELINES.md` and leave the test red.
- **Never run crate-wide `cargo fmt`.** Format only files you edited
  (`rustfmt --edition 2024 <file>`), never a `mod.rs` (it recurses into
  submodules and churns untouched files).
- **Never interrupt a cargo build or test.** A build killed during codegen
  corrupts `target/debug/incremental` and the next build fails with
  `unresolved external symbol anon.*.llvm.*` in files you never touched.
- **No pushes, no PRs, no `main`.** Commit to `feature/rmg-p3`; merge to `dev`
  only when an item is complete and tested.
- **Out of scope:** anything not on the list above — AI behaviour, unrelated
  parity findings noticed in passing (flag them, don't fix them), refactors of
  code you aren't otherwise changing, style cleanups alongside substantive fixes.

## How to work each item

1. **Explore before planning, plan before coding.** For items 1, 4, 5 and 9 the
   mechanism is not fully known — use plan mode / a subagent to investigate, and
   `/write-plan` before writing code. Items 2, 3 and 6 are small enough to do
   directly.
2. **Re-read the current file before editing it**, and `git log --grep` the item.
   Parallel sessions work this repo and RMG findings have decayed within a day.
3. **Verify every address, offset and vtable slot live in Ghidra this session** —
   prior docs are hints, not ground truth. If the bridge is down (connection
   refused, empty `list_instances`, "No program loaded"), invoke `/ghidra-up`
   before retrying.
4. **Label what you prove, then `save_program`.** Function names, global names +
   types, plate comments citing the verifying call (e.g. "Verified 2026-07-25:
   `decompile_function 0x0059D510`"). Record negative results too. A read-only
   session throws its own work away.
5. **Use subagents for investigation** so exploration doesn't consume the main
   context, and `/clear` between items.
6. **Before calling an item done, run `/code-review`** on its diff.

## Verification — the check to run

- Per edit: `cargo check -p vera20k`
- Per item: `cargo test -p vera20k --lib rmg`
- Before any merge to `dev`: full `cargo test -p vera20k`, started once in the
  background and waited on with a single `until grep -q 'test result:' …` loop —
  it takes minutes, and concurrent cargo invocations fight over the target lock.

**Show evidence, never assert success.** Paste the literal `test result:` line.
A wrong `-p` name exits 101 without running anything, so read the real output
before reporting a pass. Never report counts or SHAs from memory.

## Stop conditions

- **Per item:** stop when its acceptance criterion holds and the check passes;
  commit; report; move to the next item.
- **On repeated failure:** if the same check fails three times, stop, and report
  the failure with the output rather than layering more changes.
- **On design ambiguity:** if the RE turns up something that changes the design
  (e.g. the 3/4 driver does not match the report's structure), stop and report
  before implementing — one sentence of concern, then wait.
- **Whole goal:** done when items 1–9 are complete or explicitly reported as
  blocked with the reason, item 10 is done or deferred with a stated reason, and
  the golden-map oracle proposal has been written.

## Report after each item

Four things, no preamble: what changed (files), the command you ran and its
literal output line, what the acceptance criterion was and whether it holds, and
what is left. Flag anything you deliberately did not do.
