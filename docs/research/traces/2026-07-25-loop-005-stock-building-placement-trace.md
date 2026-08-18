# LOOP-005 Stock Building Placement Trace

Date: 2026-07-25  
Loop: `LOOP-005-BUILD-PLACE`  
Owner: `GSI-09.11`  
Bounded fixture: a completed stock Allied `GAPOWR`, an existing stock
`GACNST` build-area provider, one rejected foundation, then one valid adjacent
foundation.

## Verdict

**The earliest ordinary stock placement divergences were in the shared Rust
per-cell predicate. They are closed for this bounded slice.**

Before this slice, both preview and commit could accept:

1. a `GAPOWR` foundation cell occupied by a marked Ground-list unit or
   infantry; and
2. a live nonblocking overlay such as ore or gems.

Active `gamemd.exe` rejects both cases in the normal human sidebar-ready path.
The production Rust predicate now reads the existing Ground occupancy list and
live `OverlayGrid` authority in addition to its prior structure and terrain
checks. Rejection preserves the completed building, stable-ID allocator,
entities, occupancy, overlay, and resource state. Clearing the blocker permits
the same command to create the building through the normal lifecycle, mark its
four foundation cells, enter buildup, and contribute stock power.

This is not a full-loop parity certification. Mouse input, placement-ghost
pixels, rejection EVA timing, special overlay replacement branches, and an
active-retail input/pixel differential remain unverified.

## Evidence levels

- **VERIFIED-BINARY**: current active `gamemd.exe` body, disassembly, callsites,
  receiver flow, or vtable path was inspected in Ghidra.
- **RETAIL-DERIVED**: sealed retail INI, art, theater, or map input.
- **RUST-PRODUCTION-VERIFIED**: the real Rust command/lifecycle path was
  executed by an automated check.
- **RUST-REGRESSION**: a Rust-only focused test; useful as a ratchet, not native
  parity proof.
- **UNVERIFIED**: no executable native oracle or exhaustive proof exists.

## Stock fixture values

`rulesmd.ini` patches base `rules.ini`, and `artmd.ini` patches base `art.ini`.

| Item | Stock value | Evidence |
|---|---:|---|
| `GACNST.Foundation` | `4x4` | RETAIL-DERIVED after art merge |
| `GACNST.BaseNormal` | yes | RETAIL-DERIVED |
| `GAPOWR.Foundation` | `2x2` | RETAIL-DERIVED after art merge |
| `GAPOWR.Adjacent` | 2 | RETAIL-DERIVED |
| `GAPOWR.Strength` | 750 | RETAIL-DERIVED |
| `GAPOWR.Power` | +200 | RETAIL-DERIVED |
| `GAPOWR.Buildup` | `GAPOWRMK` | RETAIL-DERIVED |
| retail map | `Dustbowl.mmx`, 125,288 bytes, CRC32 `75B73654` | RETAIL-DERIVED |
| retail map SHA-256 | `46B07F8968BE4C267CBDEC5B99CF36E9BDE98F4AC0D23B7D634ABF86E9165A79` | sealed-file check |

## End-to-end mechanism trace

### 1. Player entry and preview

The app arms `TargetingMode::BuildingPlacement`, converts the cursor to a
foundation origin, and calls
`production::placement_preview_for_owner`. The preview and commit both call
the same `evaluate_building_placement`/`cell_placeable` policy.

Native preview uses the active per-cell placement predicate at `0x0047C620`
through `BuildingPlacement_per_cell_draw @ 0x0047EC90`. It walks the base
`Foundation=` offsets and applies ordinary cell land, slope, overlay,
occupation, and blocking policy.

Result after this slice: **RUST-PRODUCTION-VERIFIED** for the bounded
foundation decision. Exact cursor origin, ghost composition, tint, and pixels
remain **UNVERIFIED**.

### 2. Command scheduling and authority

On a locally valid click, `src/app_commands.rs` schedules
`Command::PlaceReadyBuilding` using the preview's stored origin. At its
deterministic execution tick,
`src/sim/world/world_commands.rs` resolves owner/type IDs and calls
`production::place_ready_building`.

The sim revalidates the completed item and every foundation cell before any
spawn or ready-queue mutation. The command therefore does not trust a stale
preview.

Native player-ready placement enters
`HouseClass::Place_Production @ 0x004FB0E0`, obtains the held ready object, and
calls its unlimbo/place virtual. The active chain is:

`HouseClass::Place_Production @ 0x004FB0E0`
-> `BuildingClass::Unlimbo @ 0x00440580`
-> `BuildingClass::Can_Enter_Cell @ 0x00449440`
-> building-type vtable `+0xA8` / `0x00716150`
-> `Cell_passability_building_placement @ 0x0047C620`.

`BuildingTypeClass::CanBePlacedAt @ 0x0045EE70` is not the normal human
sidebar-ready commit validator. Its complete verified direct-caller census is
unit deploy, AI deploy scheduling, and factory exit. Consequently, its allied
scatter side effect must not be imported into the ordinary ready-building
path: a marked unit or infantry rejects this fixture.

### 3. Per-cell terrain, overlay, and occupancy

For each base-foundation cell, active `0x0047C620`:

- enforces the ordinary in-playfield/on-screen and land/speed gates;
- reads slope and cell blocking state;
- reads Ground occupation state; and
- rejects a nonempty overlay after earlier special-building exceptions.

The Rust predicate already covered resolved terrain, slope, bridge, path, and
stored structures. It diverged in two load-bearing authorities:

- `structure_occupies_cell` scanned only structures, so a marked tank or
  infantry on a non-origin foundation cell could be accepted;
- `ResolvedTerrainCell::overlay_blocks` is false for ordinary resources such
  as ore/gems, so a live `OverlayGrid` resource could remain under a newly
  placed building.

The bounded correction:

- walks only `MovementLayer::Ground` cell membership;
- rejects every resolved non-structure member without a health, dying, or
  activity filter;
- fails closed on a stale occupancy ID, with a debug assertion;
- skips structure members because the existing structure predicate retains
  its separate wall/lifecycle policy; and
- rejects any nonempty live `OverlayGrid` cell in the ordinary path.

A dying infantry remains marked until lifecycle `UnInit`, so it continues to
block exactly as the cell-list authority requires. Air and underground layers
are not read by this predicate. A bridge object is represented by the bridge
layer/facts and does not become a false Ground-mobile blocker.

### 4. Rejection transaction

Native failed `Place_Production` does not call
`FactoryClass::CompletedProduction`; the held ready object remains available.
For the local player it plays `EVA_CannotDeployHere` and clears placement UI
state.

The Rust command now returns before `spawn_object` or ready-queue removal. The
automated production checks prove that rejection preserves:

- entity count;
- next stable object ID;
- occupancy generation and membership;
- the owner's completed `GAPOWR`;
- live overlay ID/data; and
- seeded resource-node state on the retail ore footprint.

The tick/session clock still advances because the rejected command is executed
inside `Simulation::advance_tick`; preservation is therefore asserted over
the placement transaction's authoritative objects rather than by falsely
requiring a whole-world hash to remain unchanged.

### 5. Accepted placement and lifecycle

After the blocker is removed, the same valid adjacent foundation passes.
`place_ready_building` calls the real `Simulation::spawn_object` lifecycle,
then attaches `BuildingUp` and removes exactly one completed item.

The sealed retail oracle proves the resulting stock `GAPOWR`:

- has category Structure and health 750/750;
- has `2x2` foundation state;
- is object-alive, out of limbo, cell-marked, and in logic order;
- owns all four Ground occupancy cells;
- remains in visible buildup; and
- contributes 200 power with zero drain in the bounded fixture.

This validates the production Rust handoff through lifecycle, occupancy, and a
real downstream consumer. The exact native buildup frame count/cadence is not
certified by this slice; Rust still has a separate fixed-duration
`BuildingUp` residual.

### 6. Presentation consumers

The app observes `spawned_entities`, refreshes derived presentation state,
triggers construction-yard animation, renders the building/buildup, and
updates the power/sidebar consumers. Those paths were mapped but not driven
with desktop input in this slice.

No Computer Use, window focus, injected input, live screen capture, or Oracle
mutation was used. Placement mouse feel, rejection audio/EVA, exact frame
ordering, building sprite pixels, and placement-cell pixels remain
**UNVERIFIED** and require the explicitly joint visual gate or a future
noninteractive capture driver.

## Ordered stock-loop status

| Stage | Expected handoff | Current bounded result | Status |
|---:|---|---|---|
| 1-6 | Queue, spend, complete, and expose ready `GAPOWR` | Existing production path; not re-audited in this slice | PARTIAL / residual |
| 7 | Arm placement mode | Existing app path mapped | UNVERIFIED interactively |
| 8 | Draw placement ghost | Existing render path mapped | UNVERIFIED pixels |
| 9-11 | Walk foundation; read terrain, overlay, and Ground occupancy | Shared Rust predicate now consumes all three scoped authorities | CLOSED for fixture |
| 12a | Reject marked tank/infantry | Preview and real command reject; ready item and world authorities preserved | RUST-PRODUCTION-VERIFIED |
| 12b | Reject ore/gems | Retail `Dustbowl.mmx` ore footprint rejects; overlay/resource preserved | RETAIL-DERIVED RUST oracle |
| 12c | Accept clear adjacent `2x2` foundation | Real command succeeds | RUST-PRODUCTION-VERIFIED |
| 13 | Reveal and mark four cells | Real lifecycle and Ground membership asserted | RUST-PRODUCTION-VERIFIED |
| 14 | Enter visible buildup | Buildup present; exact cadence unverified | PARTIAL |
| 15 | Publish stock power | 200 output during buildup | RUST-PRODUCTION-VERIFIED |
| 16 | Render exact stock result | Not captured | UNVERIFIED |

## Literal validation

Red-first evidence before the correction:

- marked Ground mobile:
  `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4778 filtered out`;
  failure: `MTNK must reject the preview`;
- nonblocking overlay:
  `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4778 filtered out`;
  failure: `any ordinary nonempty overlay must reject`.

Feature-worktree results after the correction:

- the two red-first tests together:
  `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 4777 filtered out`;
- ordinary empty-cell wall / wall-on-overlay boundary:
  `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4778 filtered out`;
- full production placement module:
  `test result: ok. 36 passed; 0 failed; 2 ignored; 0 measured; 4746 filtered out`;
- existing ignored retail `Dustbowl.mmx` overlay diagnostic:
  `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out`;
- new ignored sealed-retail production-load oracle:
  `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4783 filtered out`;
- `cargo check -q -p vera20k`: exit 0, warnings only;
- full library:
  `test result: FAILED. 4760 passed; 1 failed; 20 ignored; 0 measured; 0 filtered out`.
  The only failure is the already-established
  `global_skirmish_replay_is_deterministic_and_baseline_stable` assertion,
  again with final hash `B86BAFD0F6AAACE0`; this slice did not rebaseline it;
- full library skipping only that known assertion:
  `test result: ok. 4760 passed; 0 failed; 20 ignored; 0 measured; 1 filtered out`.

Feature commits:

- gameplay and retail oracle:
  `12948d89c7e68d0dac14f7e0ed58d9212e844a2e`;
- System Map closeout:
  `fd4e311f3a21c8d1ba51a43a3d8f361bdd717e23`.
- incorporated-current-dev feature merges:
  `3507bc0712bcf526266d449e32afd23188cb249e` and
  `825a2cc13923a3cdc9227c9488253802b94a7a21`;
- guarded local `dev` integration:
  `7b326d71d0fc0854d88818f0030c833f679cb6f5`.

Post-merge validation on `dev`, after the disjoint RMG retail-test descendant
`dca769f95a1034ce3960dbb47eaacce7f4f0b0c0`:

- production placement module:
  `test result: ok. 36 passed; 0 failed; 2 ignored; 0 measured; 4746 filtered out`;
- ignored sealed-retail production-load oracle:
  `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4783 filtered out`;
- `cargo check -q -p vera20k`: exit 0, warnings only;
- `python -m tools.system_map check --ci`:
  `errors=0`, `warnings=12`, `systems=336`, `edges=53`, `loops=12`.

## Residuals

These are real residuals, not parity downgrades or permission to simplify
working behavior:

1. **Production authority:** current Rust queue/ready representation advances
   factory state earlier than native's held `FactoryClass::Object`. The bounded
   rejected placement preserves Rust ready state, but the complete
   factory-object lifetime remains separate work.
2. **Delayed-command rejection UI:** a locally valid preview can become invalid
   before its delayed command executes. Rust has already cleared placement
   mode and the sim rejection does not yet publish the native
   `EVA_CannotDeployHere`/UI handoff.
3. **Command trust boundary:** rare malformed/mismatched owner payload behavior
   was not expanded.
4. **Spawn/ready transaction:** failure after a successful spawn but before
   ready removal is not modeled as an atomic rollback.
5. **Special overlays:** same-owner damaged matching-wall replacement,
   `ToTile`, LaserFence, upgrades, and gate-specific exceptions are not
   implemented by the generic ordinary-overlay gate. Empty-cell stock wall
   placement remains green; current wall-on-wall behavior was already a known
   drift.
6. **Wall autofill authority:** app-only autofill segments are not uniformly
   committed to `OverlayGrid`, so later sim placement can miss those segments.
7. **Structure authority:** structures still use a raw `EntityStore` scan while
   non-structures use cell-list membership. Consolidating that split requires a
   separate lifecycle proof.
8. **Bounds:** exact native playfield/on-screen boundary semantics remain
   broader than this fixture.
9. **Buildup/pixels/audio:** exact asset-driven buildup timing, placement
   graphics, sprite output, cursor feel, and rejection audio remain
   unverified.

## Approval question

**Why should this be approved?**

The slice starts from a real stock player action, follows preview and commit
through the native ready-object/unlimbo/per-cell chain, fixes the first
shared-policy divergence, exercises the deterministic production command, and
then proves lifecycle, occupancy, buildup, and power using sealed retail
inputs. It neither imports the wrong `CanBePlacedAt` scatter semantics nor
removes working placement behavior to make the oracle easy.

**What evidence could still make it wrong?**

- An active-retail executable differential could reveal a cell-list category
  or special-overlay branch not represented by the bounded fixture.
- A scheduler trace could expose a different exact frame for rejection EVA,
  ready-object release, buildup, or power publication.
- A joint pixel/input comparison could expose cursor-origin or render
  composition drift even though the sim decision is now correct.
- A future lifecycle consolidation could prove that the raw structure scan and
  occupancy cache can disagree in an ordinary stock case.

Those uncertainties keep the full loop `UNVERIFIED`; they do not overturn the
verified ordinary Ground-occupant and ore/gem rejections closed here.

## 2026-07-25 Yuri generic sidebar/progress prerequisite update

The loop's stage-6 presentation re-audit found an earlier load-bearing
prerequisite drift under `GSI-02.01`. Before the correction, Rust constructed
the Yuri sidebar atlas from only `sidec02md.mix` and used `RADARYURI.PAL` for
every role. Retail `GCLOCK2.SHP` and the verified generic sidebar pieces are
absent from that archive; the old global fallback could therefore select
duplicated Allied bytes.

### Native producer, authority, and consumer chain

Active-YR evidence establishes this bounded chain:

1. `InitSideMixFiles @ 0x00534FA0` installs side archive stacks.
2. `LoadFileFromMIX @ 0x005B40B0` resolves generic sidebar files in side order:
   side patch/MD archive, side base archive, then side neutral archive.
3. Yuri maps to side 2, so its generic route is
   `sidec02md.mix -> sidec02.mix -> sidenc02.mix`.
4. Generic chrome and `GCLOCK2.SHP` consume `SIDEBAR.PAL`.
5. Yuri radar/background art retains its separate `sidec02md.mix` /
   `RADARYURI.PAL` authority; the bounded generic resolver does not claim or
   rewrite that route.
6. `SidebarClass::LoadSHPs @ 0x006A5840` and the sidebar/strip consumers use
   the resolved frames for production presentation.

### Production Rust chain after the correction

`AssetManager` still owns the existing global archive set and insertion order.
The sidebar atlas now adds an immutable, local `SidebarSideRoute` for only the
verified generic roles. `build_theme_atlas` selects a theme-specific
radar/background palette separately from the side-resolved generic
`SIDEBAR.PAL`. `build_gclock_cpu_atlas` decodes every stored retail frame
without shifting frame indices, and the GPU wrapper consumes that exact CPU
result.

At the app consumer boundary, `build_sidebar_cameo_instances` retains the
existing in-progress gate and calls the production-shared
`build_gclock_instance`. A 55-frame Yuri/side-2 retail atlas at progress `0.5`
selects stored frame 28 and preserves the current geometry, UV, depth, tint,
and alpha fields. The queue/house-to-theme path was mapped but was not driven
end to end by this prerequisite.

### Verification and residual boundary

The ignored sealed-retail production-load-path oracle verifies:

- Allied generic tabs resolve to side 1;
- Soviet and Yuri generic pieces resolve to side 2;
- Yuri generic palette authority is `SIDEBAR.PAL` from the side-2 route;
- Yuri radar/background files remain available in `sidec02md.mix`;
- Yuri `GCLOCK2` decodes as 60x48 with 55 stored frames, transparent frame 0,
  and visible later frames; and
- the production instance helper emits the expected midpoint instance.

This is **VERIFIED_RETAIL provenance and production CPU/instance execution**,
not full-loop or pixel parity. Queue/house-to-theme execution, GPU upload/draw,
swapchain pixels, exact RGB/ConvertClass conversion, packed `0x404` blending,
zero/progress rounding and cadence, native cache/earlier-global timing,
excluded `TABS`/`POWER`/`ADDON`/unknown roles, and repair/sell missing-state
layout remain `UNVERIFIED` or `DRIFT`.

Production commit `7448a5a55d8ee9a7c9daf7b45254d250f4acc69e`
merged through `2a9056b18dca6d752cef83ba43e242c3027b96db`;
the affected System Map metadata is commit
`898285e633f10d6908d400a6d1cefebc291b0cec`, merged at validated `dev`
`2af10dbe819b4c6e61dadf9b2ffdc6b22adc38d5`.
