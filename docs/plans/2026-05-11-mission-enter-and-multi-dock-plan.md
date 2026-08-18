# Multi-Pad DockingOffset + Pad-Aware AirfieldDocks — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.
> Each commit must independently `cargo test` clean before the next begins.

**Goal:** Parse all `DockingOffset%d` entries from art.ini and make `AirfieldDocks` pad-aware so GAAIRC/AMRADR (NumberOfDocks=4) aircraft visibly land on 4 distinct pads instead of stacking on one cell.

**Architecture:** Pure additive changes inside `sim/docking/` + `rules/`. No new top-level modules. No FSM unification (out of scope per design). Six existing entry-into-building FSMs stay untouched; the parity gap is concentrated in the aircraft path, and refinery/depot become multi-pad-capable-but-always-pad-0.

**Design Doc:** [docs/plans/2026-05-11-mission-enter-and-multi-dock-design.md](docs/plans/2026-05-11-mission-enter-and-multi-dock-design.md)

---

## Grounding Summary

- **Docs say** `DockingOffset%d` is parsed from art.ini by `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`, loop at `0x004649AF`, struct at `BuildingTypeClass+0x1788` stride 12, 3×int32 per entry. `NumberOfDocks` at `+0x1780`. Verified live in Stage 1 audit of [BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md](docs/research/BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md).
- **Ghidra confirmed** `AircraftClass::Mission_Enter @ 0x00419C80` (renamed this session from `Mission_Sticky`) state 7 reads per-pad coords via `(*(building+0xA8))(coords_out, aircraft)` — aircraft never directly calls `FindDockSlot`. Pad allocation is internal to building/RadioClass, first-empty-slot in `Contacts[]@+0xE4`. See [MISSION_ENTER_CROSSWALK_AND_GAPS_GHIDRA_REPORT.md](docs/research/MISSION_ENTER_CROSSWALK_AND_GAPS_GHIDRA_REPORT.md) §3.
- **Repo pattern** to mirror: `AirfieldDocks` ([aircraft_dock.rs:85-204](src/sim/docking/aircraft_dock.rs#L85-L204)) — two-phase snapshot pattern (snapshot → mutate → apply), `BTreeMap` for determinism. Refinery's `refinery_pad_cell` ([miner_dock_sequence.rs:57-74](src/sim/miner/miner_dock_sequence.rs#L57-L74)) is the existing +128 half-cell rounding template.
- **INI keys** verified in `artmd.ini`: GAAIRC has `DockingOffset0=0,-128,0` through `DockingOffset3=256,128,0` (a 2×2 cell spread). AMRADR uses `Image=GAAIRC` so inherits the same 4-pad layout. Refineries/depots have NumberOfDocks=1 with no offset (default to building center) or a single `DockingOffset0`.
- **Unknown after grounding**: gamemd's exact tie-breaker when two aircraft simultaneously arrive at the same building has been confirmed as "first-empty-slot in linear scan order". Our `BTreeMap`-keyed iteration already provides deterministic order. Nothing left to research before implementing.
- **Late finding (2026-05-11 verification pass)**: `BuildingClass::GetDockCoord @ 0x00447B20` adds DockingOffset to building **geometric center**, not origin top-left. The previous design's `pad_cell_for` formula was origin-relative — corrected here. New helper signature: `pad_cell_for(origin, foundation, pad)`. Refinery wrapper updated to pass foundation through. All retail refineries hit the fallback branch (DockingOffset0 commented out), so no observable behavior change for refineries; the fix only affects multi-pad airfields where the bug would have placed aircraft 1 cell NW of retail.

## Key Technical Decisions

| Decision | Rationale | Confidence | Source |
|---|---|---|---|
| Replace `Option<(i32,i32,i32)>` with `Vec<DockPad>` on `ObjectType` | Single source of truth; matches gamemd's array layout exactly; vec index IS pad index | high | Design doc Section "Chosen Approach" |
| Pad index is implicit (vec position), not stored in DockPad | Smallest data model; no consistency hazards | high | Design doc Components section |
| Zero-init missing pad indices when NumberOfDocks > offsets specified | Mirrors gamemd memory layout per Stage 1 audit | high | BUILDING_DOCKING_SYSTEM verification audit 2026-05-11 |
| `AirfieldDocks::slots: BTreeMap<u64, Vec<Option<u64>>>` keyed by airfield_sid → per-pad occupant vec | First-empty linear scan = gamemd `RadioClass::FindDockSlot @ 0x0065AD90`; BTreeMap iteration deterministic | high | Stage 2 doc + Agent D pre-flight |
| `try_reserve` returns `Option<u8>` (pad_index) instead of `bool` | Caller needs pad_index to compute per-pad cell coords | high | Design doc Interfaces section |
| `AircraftAmmo` gains `target_pad: Option<u8>` (NOT a new GameEntity field) | Avoids bloating GameEntity for non-aircraft entities | high | Design doc Components section |
| `AircraftMission::Docking` and `DockedIdle` gain `pad_index: u8` | Save-game compat: pad index needs to round-trip; AircraftMission is already serde | high | Design doc Data Flow section |
| Refinery and depot read `pads.first()` for their single pad | Multi-pad-capable but always pad 0 in retail | high | art.ini grep: refineries NumberOfDocks=1 |
| Extract `pad_geometry::pad_cell_for` into new file `sim/docking/pad_geometry.rs` | Shared by refinery_pad_cell + aircraft tick; one source of truth for +128 half-cell rounding | medium | Design doc — user explicitly approved the new file |
| `pad_cell_for` uses **building-center-relative** offset math, not origin-relative | gamemd's `BuildingClass::GetDockCoord @ 0x00447B20` adds DockingOffset to `GetCoords()` (building center). Origin-relative would put GAAIRC pads 1 cell northwest of where retail puts them — visible parity drift | high | Live decompile 2026-05-11 — caught the formula bug pre-implementation |
| New file `sim/docking/pad_geometry.rs` (small) | Sibling to `aircraft_dock.rs` and `building_dock.rs`; follows existing sim/docking/ layout | medium | User chose this over "inline the helper" in brainstorm Q |

## Open Questions

### Resolved During Planning

- "How does pad-index assignment happen in gamemd?" → Answered Stage 2: first-empty-slot scan inside `RadioClass::Transmit_Radio_Impl @ 0x0065A970` cmd 2 (HELLO). Allocation is internal, caller doesn't pre-compute.
- "Does the design need to handle Carryalls?" → Answered Stage 2 re-verification: Carryall passenger pickup uses same `AircraftClass::Mission_Enter` function but a separate `Type+0xDFC` branch that doesn't touch multi-pad data. Orthogonal.
- "Refinery exit facing 0x47 and offset (-0x80, +0x80) — does this design preserve them?" → Yes, refinery_pad_cell stays in miner_dock_sequence.rs; only its lepton→cell conversion is extracted.

### Deferred to Implementation

- **Determinism replay test exact assertion** — the test will run two simultaneous aircraft and assert pad-assignment is stable across two runs. The exact assertion code depends on `Simulation::advance_tick` step granularity which we'll measure during Commit 4.
- **AMRADR test fixture** — AMRADR uses `Image=GAAIRC`, so it inherits the offsets via the existing art→rules merge. Worth a small assertion in the merge tests, but the test data depends on what fixture setup exists already.

## File Map

| Action | Path | Responsibility | Commit |
|---|---|---|---|
| Create | `src/sim/docking/pad_geometry.rs` | `DockPad` struct (re-exported), `pad_cell_for` helper | C3 |
| Modify | `src/sim/docking/mod.rs` | Add `pub mod pad_geometry;` | C3 |
| Modify | `src/rules/object_type.rs:305-308` | Replace `docking_offset: Option<(i32,i32,i32)>` with `pads: Vec<DockPad>` | C1, C2 |
| Modify | `src/rules/art_data.rs:89-95, 265-285` | Same field swap + parser loop 0..N | C1, C2 |
| Modify | `src/rules/ruleset.rs:1630-1640` | Art→rules merge takes whole vec | C1, C2 |
| Modify | `src/rules/shp_vehicle_sequence.rs:129` | Test fixture default | C1, C2 |
| Modify | `src/sim/miner/miner_dock_sequence.rs:57-74` | `refinery_pad_cell` reads `pads.first()`; later calls `pad_cell_for` | C2, C3 |
| Modify | `src/sim/docking/aircraft_dock.rs:32-204, 290-575` | Pad-keyed reservation + per-pad cell + tick threading | C4 |
| Modify | `src/sim/aircraft/mod.rs:70-85` | Add `pad_index: u8` to Docking + DockedIdle variants | C4 |
| Modify | `src/sim/aircraft/mod.rs:392-501` | 7 Docking ctor/destructure sites + 1 DockedIdle destructure thread pad_index. Also `try_reserve` call at :429 captures `Option<u8>` and re-targets descent to per-pad cell. | C4 |
| Modify | `src/sim/production/production_queue.rs:519-538` | `try_reserve` at :537 captures `Option<u8>`; DockedIdle ctor at :522 gets `pad_index: 0` (helipads always single-pad). | C4 |
| Modify | `src/sim/miner/miner_tests.rs` | Test fixture updates | C2 |

## Interface Changes

- **`ObjectType.pads: Vec<DockPad>`** — replaces `docking_offset: Option<(i32,i32,i32)>`. Every reader of the old field must migrate to `pads.first().map(|p| p.lepton_offset)` (single-pad consumers) or iterate the vec (multi-pad consumers). Searches: `grep "docking_offset"` in `src/`.
- **`AirfieldDocks::try_reserve(af, ac, num_pads) -> Option<u8>`** — was `bool`. Callers: `tick_aircraft_docks` in [aircraft_dock.rs:466](src/sim/docking/aircraft_dock.rs#L466). One callsite. Update to capture the returned pad index.
- **`AircraftMission::Docking` and `::DockedIdle`** — gain `pad_index: u8` field. Variant pattern-matches break across the codebase. Searches: `grep "AircraftMission::Docking\|AircraftMission::DockedIdle"`.
- **`AircraftAmmo.target_pad: Option<u8>`** — new serde-derived field. Save-game format changes by one field. Pre-1.0 project, acceptable.

## Sim Checklist

This plan touches `sim/` extensively. Confirmed:

- [x] All math uses `i32`/`u16` integer types — no f32/f64 in new logic. `pad_cell_for` is pure integer.
- [x] New state (target_pad, pad_index) is `serde::Serialize/Deserialize` — included in save-game and state hash automatically.
- [x] No new dependencies on render/ui/sidebar/audio/net.
- [x] No new RNG draws. Determinism preserved via `BTreeMap` + `Vec` linear scan.
- [x] No tick-order changes. `tick_aircraft_docks` keeps its existing slot.
- [x] `EntityStore` iteration order unaffected.
- [x] State hash (`sim/world_hash.rs`) automatically picks up new serde fields via derive — but verify by checking `world_hash.rs` includes the field's parent component in its hash visitor. (Verification step in Task 4.10.)

## Risk Areas

From design doc Impact Analysis:

- **HIGH:** `AirfieldDocks` refactor — touches the queue/promote/cancel/cleanup logic. Regression risk for existing aircraft reload flows.
- **MEDIUM:** `tick_aircraft_docks` — now must thread pad_index through descent → reloading → launching. Per-pad cell coord used for `air_move_to`.
- **MEDIUM:** Save-game format break — old saves won't load. Pre-1.0; acceptable.
- **LOW:** Refinery + depot path changes are mechanical (`pads.first()` substitution).
- **LOW:** New `pad_geometry.rs` file is pure-integer math.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|---|---|---|---|
| C1.3, C4.6 | GAAIRC 4 pads on a 2×2 cell spread | Player sees 4 distinct aircraft landing positions on Allied Airforce Command, not a stack | art.ini grep + in-game observation after C4 |
| C3.1 | Building-center-relative pad math + `+128` half-cell rounding | gamemd's GetDockCoord adds DockingOffset to building center. Origin-relative would put GAAIRC pads 1 cell NW of retail | Unit test with explicit GAAIRC expected cells (rx+1, ry+0)..(rx+2, ry+1) |
| C4.2 | First-empty-slot allocation in linear scan order | Two simultaneous aircraft must get pads 0 and 1 deterministically (matches gamemd's Contacts[] scan) | New unit test asserts pad order = arrival order |
| C4.6 | AircraftAmmo descent targets per-pad cell, not building center | Aircraft visibly lands on its assigned pad's cell, not the building's geometric center | Live in-game test post-C4 |
| C1.4 | Zero-init for missing DockingOffset%d when N > K | A 4-pad building with only 2 offsets specified does not crash; missing pads parked at building origin | Unit test: NumberOfDocks=4 + only DockingOffset0,1 |
| C4.7 | AirfieldDocks cleanup_dead promotes queue across pad reallocation | When pad 1 is released, queued aircraft N gets pad 1 specifically (not just "a slot") | Updated cleanup_dead test |

---

## Tasks

### Commit 1 — Add `pads: Vec<DockPad>` alongside `docking_offset` (no-op duplication)

**Goal:** Add the new field, parser, and merge logic. Keep the old `docking_offset` field temporarily populated from `pads.first()` so no consumers break.

#### Task 1.1: Define `DockPad` struct in `src/rules/object_type.rs`

**Why:** First piece of the new data model. Place it next to the existing `ObjectType` struct it'll be a field of.

**Files:**
- Modify: `src/rules/object_type.rs` — insert above the `pub struct ObjectType` definition.

**Pattern:** Mirror other small data structs already in this file (e.g., `ObjectCategory`).

**Step 1: Find the line just above `pub struct ObjectType` (around line 280-290; confirm via `grep -n "pub struct ObjectType" src/rules/object_type.rs`).** Insert this struct above it:

```rust
/// One docking pad on a building. Stored in `ObjectType.pads` as a Vec
/// whose index IS the pad index (0-based, matching `DockingOffset0..N-1`).
///
/// `lepton_offset` is the building-origin-relative offset in leptons
/// (256 leptons per cell). Parsed from art.ini `DockingOffset%d=X,Y,Z`.
/// Zero-initialized entries are valid (e.g. when `NumberOfDocks=4` but
/// only `DockingOffset0..1` are specified — gamemd treats unspecified
/// pads as zero-offset, verified in Stage 1 audit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DockPad {
    pub lepton_offset: (i32, i32, i32),
}
```

**Step 2: Verify the file compiles.**
Run: `cargo check -p ra2-rust-game`
Expected: PASS (struct added but not yet used).

**Step 3: No commit yet** — bundle with 1.2 + 1.3 + 1.4 + 1.5.

#### Task 1.2: Add `pads: Vec<DockPad>` field to `ObjectType` (keep `docking_offset` for now)

**Why:** Duplicate the data temporarily. Step 1 of the no-op duplication strategy from the design.

**Files:**
- Modify: `src/rules/object_type.rs:305-308` — add field after `docking_offset`.

**Step 1: Read [object_type.rs:300-315](src/rules/object_type.rs#L300-L315) to confirm the exact context.** The block reads:

```rust
    pub docking_offset: Option<(i32, i32, i32)>,
    /// Cells added to the rectangular foundation (from art.ini AddOccupy1..N).
```

**Step 2: Replace that block by inserting the new field between `docking_offset` and `add_occupy`:**

```rust
    pub docking_offset: Option<(i32, i32, i32)>,
    /// All docking pads on this building, parsed from art.ini `DockingOffset0..NumberOfDocks-1`.
    /// Index in vec IS the pad index. `len() == number_of_docks` after merge.
    /// Refineries / service depots / single-pad helipads use only `pads[0]`.
    /// Airfields (GAAIRC, AMRADR) use all 4. Empty vec = building has no docking offsets defined.
    pub pads: Vec<DockPad>,
```

**Step 3: Update the `Default` impl for `ObjectType` (or the constructor) so `pads` defaults to `Vec::new()`.** Search [object_type.rs:820](src/rules/object_type.rs#L820) for `docking_offset: None,` and add the new line below it:

```rust
            docking_offset: None, // merged from art.ini later
            pads: Vec::new(),     // merged from art.ini later
```

**Step 4: Verify.**
Run: `cargo check -p ra2-rust-game`
Expected: PASS.

#### Task 1.3: Add `pads: Vec<DockPad>` field to `ArtData` entry + parser loop

**Why:** The parser is the source of the multi-pad data. art.ini → ArtData → merged into ObjectType.

**Files:**
- Modify: `src/rules/art_data.rs:89-95` (struct field)
- Modify: `src/rules/art_data.rs:265-285` (parser block)

**Step 1: In [art_data.rs:89-95](src/rules/art_data.rs#L89-L95), the existing block reads:**

```rust
    pub queueing_cell: Option<(u16, u16)>,
    /// First docking offset from art.ini (DockingOffset0=X,Y,Z).
    /// Lepton offset from building origin where units dock. 256 leptons = 1 cell.
    /// e.g. GAREFN has `DockingOffset0=0,-128,0`.
    pub docking_offset: Option<(i32, i32, i32)>,
    /// Pixel offsets where fire/smoke overlays appear when building health < ConditionYellow.
```

**Step 2: Insert the new field after `docking_offset`:**

```rust
    pub docking_offset: Option<(i32, i32, i32)>,
    /// All docking pads parsed from art.ini `DockingOffset0..N-1` where N is the building's
    /// `NumberOfDocks` from rules.ini. Indices missing from art.ini get zero-init entries
    /// (matches gamemd memory layout per Stage 1 audit of BUILDING_DOCKING_SYSTEM doc).
    /// Length is set during art→rules merge to match `number_of_docks`.
    pub pads: Vec<crate::rules::object_type::DockPad>,
    /// Pixel offsets where fire/smoke overlays appear when building health < ConditionYellow.
```

**Step 3: Update the parser block at [art_data.rs:272-282](src/rules/art_data.rs#L272-L282).** The existing block reads:

```rust
            let docking_offset: Option<(i32, i32, i32)> =
                section.get("DockingOffset0").and_then(|s| {
                    let mut parts = s.split(',');
                    let x = parts.next()?.trim().parse::<i32>().ok()?;
                    let y = parts.next()?.trim().parse::<i32>().ok()?;
                    let z = parts
                        .next()
                        .and_then(|v| v.trim().parse::<i32>().ok())
                        .unwrap_or(0);
                    Some((x, y, z))
                });
```

**Step 4: Add a new block after that, reading up to 8 docking offsets (4 in retail, 8 ceiling for mod safety):**

```rust
            // Multi-pad parser: read DockingOffset0..7 from art.ini.
            // We over-read here; the actual pad count comes from rules.ini NumberOfDocks
            // and gets truncated/padded during art→rules merge in ruleset.rs.
            let pads: Vec<crate::rules::object_type::DockPad> = (0..8)
                .filter_map(|i| {
                    let key = format!("DockingOffset{}", i);
                    section.get(&key).and_then(|s| {
                        let mut parts = s.split(',');
                        let x = parts.next()?.trim().parse::<i32>().ok()?;
                        let y = parts.next()?.trim().parse::<i32>().ok()?;
                        let z = parts
                            .next()
                            .and_then(|v| v.trim().parse::<i32>().ok())
                            .unwrap_or(0);
                        Some(crate::rules::object_type::DockPad {
                            lepton_offset: (x, y, z),
                        })
                    })
                })
                .collect();
```

**Step 5: Add `pads` to the `ArtEntry` constructor block** (find the spot near [art_data.rs:387-392](src/rules/art_data.rs#L387-L392) where `docking_offset` is bound into the struct literal):

```rust
                    queueing_cell,
                    docking_offset,
                    pads,
                    damage_fire_offsets,
```

**Step 6: Verify.**
Run: `cargo check -p ra2-rust-game`
Expected: PASS.

#### Task 1.4: Art→rules merge — propagate `pads` with NumberOfDocks-aware sizing

**Why:** Make `ObjectType.pads.len() == number_of_docks` post-merge, zero-padding when art has fewer entries than rules declares.

**Files:**
- Modify: `src/rules/ruleset.rs:1630-1640` — merge block.

**Step 1: Read the existing merge block at [ruleset.rs:1630-1640](src/rules/ruleset.rs#L1630-L1640).** Find the lines that merge `docking_offset`:

```rust
                // Merge DockingOffset0 from art.ini (TibSun legacy dock system).
                if entry.docking_offset.is_some() {
                    obj.docking_offset = entry.docking_offset;
                }
```

**Step 2: Insert the multi-pad merge block right after:**

```rust
                // Merge DockingOffset0 from art.ini (TibSun legacy dock system).
                if entry.docking_offset.is_some() {
                    obj.docking_offset = entry.docking_offset;
                }
                // Merge multi-pad data: size to NumberOfDocks (rules.ini), zero-pad missing.
                // Truncates art entries beyond NumberOfDocks (defensive against modders).
                {
                    let n = obj.number_of_docks as usize;
                    obj.pads = entry.pads.iter().take(n).copied().collect();
                    while obj.pads.len() < n {
                        obj.pads.push(crate::rules::object_type::DockPad {
                            lepton_offset: (0, 0, 0),
                        });
                    }
                }
```

**Step 3: Verify.**
Run: `cargo check -p ra2-rust-game`
Expected: PASS.

#### Task 1.5: Update `shp_vehicle_sequence.rs` test fixture default

**Why:** Test fixture currently initializes `docking_offset: None,` — needs `pads: Vec::new(),` too or compilation fails.

**Files:**
- Modify: `src/rules/shp_vehicle_sequence.rs:129`

**Step 1: Read [shp_vehicle_sequence.rs:120-135](src/rules/shp_vehicle_sequence.rs#L120-L135) for context.**

**Step 2: Find the line `docking_offset: None,` (around line 129) and add the new field below it:**

```rust
            docking_offset: None,
            pads: Vec::new(),
```

**Step 3: Verify.**
Run: `cargo check -p ra2-rust-game --tests`
Expected: PASS.

#### Task 1.6: Add unit tests for the multi-pad parser

**Why:** Lock in the parity-critical behaviors before any consumer changes.

**Files:**
- Modify: `src/rules/art_data.rs` — append to the existing `#[cfg(test)] mod tests` block.

**Step 1: Find the `#[cfg(test)] mod tests` block in [art_data.rs](src/rules/art_data.rs).** Append these tests:

```rust
    #[test]
    fn parse_gaairc_four_pads() {
        let ini = "\
[GAAIRC]\n\
DockingOffset0=0,-128,0\n\
DockingOffset1=0,128,0\n\
DockingOffset2=256,-128,0\n\
DockingOffset3=256,128,0\n\
";
        let art = ArtData::from_ini_str(ini);
        let entry = art.get("GAAIRC").expect("GAAIRC entry");
        assert_eq!(entry.pads.len(), 4, "should parse all 4 offsets");
        assert_eq!(entry.pads[0].lepton_offset, (0, -128, 0));
        assert_eq!(entry.pads[1].lepton_offset, (0, 128, 0));
        assert_eq!(entry.pads[2].lepton_offset, (256, -128, 0));
        assert_eq!(entry.pads[3].lepton_offset, (256, 128, 0));
    }

    #[test]
    fn parse_no_docking_offsets_yields_empty_vec() {
        let ini = "[GAHPAD]\nHeight=1\n";
        let art = ArtData::from_ini_str(ini);
        let entry = art.get("GAHPAD").expect("GAHPAD entry");
        assert!(entry.pads.is_empty(), "no offsets → empty pads vec");
    }

    #[test]
    fn parse_partial_offsets_collects_what_exists() {
        // art has only DockingOffset0 and DockingOffset2 (gap at 1).
        // Parser collects what's present. The art→rules merge handles sizing to NumberOfDocks.
        let ini = "\
[ODD]\n\
DockingOffset0=64,0,0\n\
DockingOffset2=192,0,0\n\
";
        let art = ArtData::from_ini_str(ini);
        let entry = art.get("ODD").expect("ODD entry");
        assert_eq!(entry.pads.len(), 2, "filter_map skips missing index 1");
        assert_eq!(entry.pads[0].lepton_offset, (64, 0, 0));
        assert_eq!(entry.pads[1].lepton_offset, (192, 0, 0));
    }
```

**Step 2: Run the tests.**
Run: `cargo test -p ra2-rust-game art_data::tests --lib`
Expected: 3 new tests PASS; all existing art_data tests still PASS.

**Step 3: Add a merge test in ruleset.rs.** Find the `#[cfg(test)] mod tests` block in [ruleset.rs](src/rules/ruleset.rs) and append:

```rust
    #[test]
    fn merge_pads_zero_pads_missing_indices() {
        // ObjectType has NumberOfDocks=4 but art only has DockingOffset0,1.
        // Merge must produce pads.len() == 4 with indices 2,3 zero-init.
        let rules_ini = "[GAAIRC]\nNumberOfDocks=4\n";
        let art_ini = "\
[GAAIRC]\n\
DockingOffset0=0,-128,0\n\
DockingOffset1=0,128,0\n\
";
        let rules = parse_ruleset_from_strs(rules_ini, art_ini);
        let obj = rules.object_case_insensitive("GAAIRC").expect("obj");
        assert_eq!(obj.pads.len(), 4, "pads sized to NumberOfDocks");
        assert_eq!(obj.pads[0].lepton_offset, (0, -128, 0));
        assert_eq!(obj.pads[1].lepton_offset, (0, 128, 0));
        assert_eq!(obj.pads[2].lepton_offset, (0, 0, 0), "missing index 2 zero-init");
        assert_eq!(obj.pads[3].lepton_offset, (0, 0, 0), "missing index 3 zero-init");
    }

    #[test]
    fn merge_pads_truncates_excess_offsets() {
        // ObjectType has NumberOfDocks=2 but art has 4 offsets. Truncate.
        let rules_ini = "[GAAIRC]\nNumberOfDocks=2\n";
        let art_ini = "\
[GAAIRC]\n\
DockingOffset0=0,0,0\n\
DockingOffset1=128,0,0\n\
DockingOffset2=256,0,0\n\
DockingOffset3=384,0,0\n\
";
        let rules = parse_ruleset_from_strs(rules_ini, art_ini);
        let obj = rules.object_case_insensitive("GAAIRC").expect("obj");
        assert_eq!(obj.pads.len(), 2, "truncated to NumberOfDocks=2");
    }
```

**Note:** If `parse_ruleset_from_strs` doesn't already exist as a test helper, search [ruleset.rs tests](src/rules/ruleset.rs) for the existing test-fixture pattern (e.g., `RuleSet::from_inis` or a helper) and adapt.

**Step 4: Run the merge tests.**
Run: `cargo test -p ra2-rust-game ruleset::tests --lib`
Expected: 2 new tests PASS; all existing pass.

#### Task 1.7: Commit 1

Run all parser tests: `cargo test -p ra2-rust-game --lib`
Expected: All PASS.

**Commit message:**
```
rules: parse multi-pad DockingOffset%d alongside legacy single-pad field

Adds DockPad struct + ObjectType.pads: Vec<DockPad> populated from art.ini
DockingOffset0..7 then sized during art→rules merge to NumberOfDocks
(zero-pad missing indices, truncate excess). Legacy docking_offset field
retained for now; consumers migrate in next commit.

GAAIRC's 4-pad layout (verified in art.ini) now reaches ObjectType.pads:
  [0]=(0,-128,0)  [1]=(0,128,0)  [2]=(256,-128,0)  [3]=(256,128,0)

Tests:
- parse_gaairc_four_pads
- parse_no_docking_offsets_yields_empty_vec
- parse_partial_offsets_collects_what_exists
- merge_pads_zero_pads_missing_indices
- merge_pads_truncates_excess_offsets
```

---

### Commit 2 — Drop `docking_offset`; consumers read `pads.first()`

**Goal:** Single source of truth. Remove the temporarily-duplicated old field. Refinery + depot read `pads.first()`.

#### Task 2.1: Migrate `refinery_pad_cell` to read from `pads`

**Why:** Refinery is the only sim/ consumer of `docking_offset`. Switch it to `pads.first()` so the old field can be deleted.

**Files:**
- Modify: `src/sim/miner/miner_dock_sequence.rs:57-74` (the `refinery_pad_cell` helper)
- Modify: `src/sim/miner/miner_dock_sequence.rs:99-118` (the `resolve_refinery_cells` caller)

**Step 1: Read [miner_dock_sequence.rs:57-74](src/sim/miner/miner_dock_sequence.rs#L57-L74).** The current signature is:

```rust
pub(super) fn refinery_pad_cell(
    rx: u16,
    ry: u16,
    width: u16,
    height: u16,
    docking_offset: Option<(i32, i32, i32)>,
) -> (u16, u16) {
    if let Some((dx, dy, _)) = docking_offset {
        ...
```

**Step 2: Keep the signature the same** (caller passes the option). Don't change `refinery_pad_cell` itself yet — that's Task 3.3.

**Step 3: Update the caller at [miner_dock_sequence.rs:99-118](src/sim/miner/miner_dock_sequence.rs#L99-L118).** Find this block:

```rust
    let dock_off = obj.and_then(|o| o.docking_offset);
```

**Step 4: Replace with:**

```rust
    let dock_off = obj.and_then(|o| o.pads.first().map(|p| p.lepton_offset));
```

**Step 5: Verify.**
Run: `cargo check -p ra2-rust-game`
Expected: PASS.

#### Task 2.2: Search for any other readers of `docking_offset`

**Why:** Defensive — remove ALL consumers before removing the field.

**Step 1: Run:**
```
grep -rn "docking_offset" src/ --include='*.rs'
```

**Step 2: For each hit, replace `.docking_offset` access with `.pads.first().map(|p| p.lepton_offset)`** or `.pads.first().is_some()` depending on context. Expect ~3-5 hits: the ObjectType field, the ArtData field, ruleset merge, miner_dock_sequence (already done), and possibly tests.

**Step 3: Verify.**
Run: `cargo check -p ra2-rust-game --tests`
Expected: PASS (any test still referencing `docking_offset` will fail; fix as needed).

#### Task 2.3: Delete the `docking_offset` field from `ObjectType` and `ArtData`

**Why:** Single source of truth.

**Files:**
- Modify: `src/rules/object_type.rs:305-308` — delete field and its doc comment.
- Modify: `src/rules/object_type.rs:821` — delete `docking_offset: None,` from constructor. (Note: this is the line AFTER `queueing_cell: None,` at :820 — sharp-eyed executors checking :820 should look one line below.)
- Modify: `src/rules/art_data.rs:92-95` — delete field and doc comment.
- Modify: `src/rules/art_data.rs:272-282` — delete the old single-pad parser block.
- Modify: `src/rules/art_data.rs:387-392` — delete `docking_offset,` from the ArtEntry constructor.
- Modify: `src/rules/ruleset.rs:1633-1636` — delete the legacy merge block (`if entry.docking_offset.is_some() ...`).
- Modify: `src/rules/shp_vehicle_sequence.rs:129` — delete `docking_offset: None,`.

**Step 1: Delete each spot. Use Edit (search-replace) for each, NOT global delete — verify each context first.**

**Step 2: Verify.**
Run: `cargo build -p ra2-rust-game --tests`
Expected: PASS.

#### Task 2.4: Update any tests that referenced `docking_offset`

**Why:** Tests may break from the field rename.

**Step 1: Run:**
```
cargo test -p ra2-rust-game --lib 2>&1 | head -100
```

**Step 2: For each failing test that references `docking_offset`, replace with `pads`:**
- `docking_offset: None` → `pads: Vec::new()`
- `docking_offset: Some((x,y,z))` → `pads: vec![DockPad { lepton_offset: (x,y,z) }]`

**Step 3: Re-run.**
Run: `cargo test -p ra2-rust-game --lib`
Expected: All PASS.

#### Task 2.5: Commit 2

Run: `cargo test -p ra2-rust-game`
Expected: All PASS.

**Commit message:**
```
rules+sim/miner: drop legacy docking_offset, read from pads.first()

Removes the redundant single-pad ObjectType/ArtData field now that all
consumers (refinery_pad_cell, future aircraft tick) read from pads: Vec<DockPad>.

Single-source-of-truth migration; no behavior change for single-pad
refinery/depot consumers (pads.first() == old docking_offset).
```

---

### Commit 3 — Extract `pad_geometry::pad_cell_for` helper

**Goal:** One implementation of lepton→cell conversion, shared by miner refinery and aircraft tick.

#### Task 3.1: Create `src/sim/docking/pad_geometry.rs`

**Why:** Shared helper. Pure function, easily testable.

**Files:**
- Create: `src/sim/docking/pad_geometry.rs`

**Step 1: Create the file with this content:**

```rust
//! Pad geometry — lepton→cell conversion for docking pad cells.
//!
//! Single source of truth for converting a building's (origin + foundation) +
//! a pad's lepton offset into the cell where the docked unit parks. Shared by
//! refinery (miner_dock_sequence::refinery_pad_cell) and aircraft docking
//! (tick_aircraft_docks).
//!
//! ## Critical: BUILDING-CENTER-RELATIVE offsets
//!
//! `DockingOffset%d` in art.ini is a lepton offset from the building's
//! geometric **center**, NOT its origin top-left. gamemd's
//! `BuildingClass::GetDockCoord @ 0x00447B20` computes:
//! ```
//! pad_lepton = GetCoords() + DockingOffset[i]
//! ```
//! where `GetCoords()` returns the building geometric center, equal to
//! `origin_lepton + ((W-1)*128, (H-1)*128)`. We replicate this here.
//!
//! ## Dependency rules
//! - Part of sim/ — depends only on rules/.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::rules::object_type::DockPad;

/// Convert a building's (origin + foundation) + a pad's lepton offset into the pad's cell.
///
/// `foundation` is `(width, height)` in cells. Used to compute the building's
/// geometric center per gamemd's `BuildingClass::GetCoords` convention.
///
/// The `+128` half-cell rounding ensures lepton coordinates near cell boundaries
/// snap to the visually correct cell (e.g. lepton 128 → cell 1, not 0).
pub fn pad_cell_for(origin: (u16, u16), foundation: (u16, u16), pad: &DockPad) -> (u16, u16) {
    let (rx, ry) = origin;
    let (w, h) = foundation;
    // Building geometric center offset (in leptons) from origin cell's top-left.
    // Matches gamemd's BuildingClass::GetCoords: (W-1)*128, (H-1)*128.
    let center_off_x = (w as i32 - 1) * 128;
    let center_off_y = (h as i32 - 1) * 128;
    let (dx, dy, _dz) = pad.lepton_offset;
    // Total lepton offset from origin cell's top-left, plus +128 half-cell rounding.
    let cx = (center_off_x + dx + 128).div_euclid(256);
    let cy = (center_off_y + dy + 128).div_euclid(256);
    (
        (rx as i32 + cx).max(0) as u16,
        (ry as i32 + cy).max(0) as u16,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(x: i32, y: i32, z: i32) -> DockPad {
        DockPad {
            lepton_offset: (x, y, z),
        }
    }

    #[test]
    fn one_by_one_zero_offset_returns_origin() {
        // For a 1x1 building, center is at the origin cell, so offset (0,0) = origin.
        assert_eq!(pad_cell_for((10, 10), (1, 1), &pad(0, 0, 0)), (10, 10));
    }

    #[test]
    fn one_by_one_positive_offset_one_cell() {
        // 1x1 building: center == origin. +256 leptons = +1 cell.
        assert_eq!(pad_cell_for((10, 10), (1, 1), &pad(256, 0, 0)), (11, 10));
        assert_eq!(pad_cell_for((10, 10), (1, 1), &pad(0, 256, 0)), (10, 11));
    }

    #[test]
    fn negative_offset_is_clamped_to_zero() {
        // 1x1 at (1,1) with offset (-512, 0) would land at (-1, 1); clamp to (0, 1).
        assert_eq!(pad_cell_for((1, 1), (1, 1), &pad(-512, 0, 0)), (0, 1));
    }

    #[test]
    fn gaairc_four_pads_match_gamemd_center_relative() {
        // GAAIRC: Foundation=3x2, art.ini offsets verified live 2026-05-11.
        // Building center is at (W-1)*128, (H-1)*128 = (256, 128) leptons from origin.
        let origin = (20, 20);
        let foundation = (3, 2);
        // DockingOffset0=0,-128,0:
        //   center_off=(256, 128), pad_off=(0, -128), total=(256+0+128, 128-128+128)=(384, 128)
        //   cell offset = (1, 0). pad cell = (21, 20).
        assert_eq!(pad_cell_for(origin, foundation, &pad(0, -128, 0)), (21, 20));
        // DockingOffset1=0,128,0:
        //   total=(384, 384), cell offset=(1, 1). pad cell = (21, 21).
        assert_eq!(pad_cell_for(origin, foundation, &pad(0, 128, 0)), (21, 21));
        // DockingOffset2=256,-128,0:
        //   total=(640, 128), cell offset=(2, 0). pad cell = (22, 20).
        assert_eq!(pad_cell_for(origin, foundation, &pad(256, -128, 0)), (22, 20));
        // DockingOffset3=256,128,0:
        //   total=(640, 384), cell offset=(2, 1). pad cell = (22, 21).
        assert_eq!(pad_cell_for(origin, foundation, &pad(256, 128, 0)), (22, 21));
    }

    #[test]
    fn nadept_single_pad_matches_gamemd_for_4x3_depot() {
        // NADEPT: Foundation=4x3, art.ini DockingOffset0=128,0,0 verified live.
        // Center offset = ((4-1)*128, (3-1)*128) = (384, 256).
        // Total = (384+128+128, 256+0+128) = (640, 384).
        // Cell offset = (640/256, 384/256) = (2, 1). pad cell = (origin+2, origin+1).
        // gamemd's expected position: building center + offset = matches.
        assert_eq!(pad_cell_for((30, 30), (4, 3), &pad(128, 0, 0)), (32, 31));
    }

    #[test]
    fn z_coord_does_not_affect_cell() {
        // Z is for rendering altitude only; cell is X/Y based.
        assert_eq!(pad_cell_for((10, 10), (1, 1), &pad(0, 0, 999)), (10, 10));
    }
}
```

**Note on the formula:** the helper takes `foundation: (u16, u16)` because gamemd's per-pad cell math is building-center-relative, and center = origin + ((W-1)*128, (H-1)*128) leptons. Verified via live decompile of `BuildingClass::GetDockCoord @ 0x00447B20` on 2026-05-11.

**Step 2: Verify.**
Run: `cargo check -p ra2-rust-game`
Expected: PASS (file is independent so far).

#### Task 3.2: Export `pad_geometry` from `src/sim/docking/mod.rs`

**Files:**
- Modify: `src/sim/docking/mod.rs`

**Step 1: Read [src/sim/docking/mod.rs](src/sim/docking/mod.rs).** Current content:

```rust
//! Docking systems — repair depot docks and airfield landing pads.

pub mod aircraft_dock;
pub mod building_dock;
```

**Step 2: Replace with:**

```rust
//! Docking systems — repair depot docks and airfield landing pads.

pub mod aircraft_dock;
pub mod building_dock;
pub mod pad_geometry;
```

**Step 3: Verify.**
Run: `cargo test -p ra2-rust-game pad_geometry --lib`
Expected: 5 tests PASS.

#### Task 3.3: Refactor `refinery_pad_cell` to call `pad_cell_for`

**Why:** Single source of truth for the lepton→cell conversion. Refinery's existing helper becomes a thin wrapper.

**Files:**
- Modify: `src/sim/miner/miner_dock_sequence.rs:57-74`

**Step 1: Replace the entire `refinery_pad_cell` body with:**

```rust
/// Pad cell — on the refinery platform inside the building footprint.
///
/// Uses art.ini `DockingOffset0=` when available (merged into ObjectType.pads),
/// converting from lepton offset to cell offset via `pad_geometry::pad_cell_for`.
/// Otherwise falls back to rightmost foundation column, vertically centred.
///
/// `pad_geometry::pad_cell_for` treats the offset as building-center-relative
/// (matches gamemd `BuildingClass::GetDockCoord`). The previous origin-relative
/// formula was bugged but never fired in retail (all refineries have
/// `DockingOffset0` commented out).
pub(super) fn refinery_pad_cell(
    rx: u16,
    ry: u16,
    width: u16,
    height: u16,
    docking_offset: Option<(i32, i32, i32)>,
) -> (u16, u16) {
    if let Some((dx, dy, dz)) = docking_offset {
        let pad = crate::rules::object_type::DockPad {
            lepton_offset: (dx, dy, dz),
        };
        crate::sim::docking::pad_geometry::pad_cell_for((rx, ry), (width, height), &pad)
    } else {
        (rx + width.saturating_sub(1), ry + height / 2)
    }
}
```

**Note:** The function still takes `docking_offset: Option<(i32,i32,i32)>` to preserve its caller signature in this commit. The caller (`resolve_refinery_cells`) already reads from `pads.first()` post-Commit-2, so the tuple is sourced from pads. A future cleanup (post-this-plan) could replace this signature with `pad: Option<&DockPad>` but that's not necessary to close the parity gap.

**Note on the formula change:** existing refinery integration tests should still pass because retail refineries hit the `else` branch (DockingOffset0 commented out in all RA2/YR refinery art entries). If a test breaks because it constructed a refinery with explicit `docking_offset: Some(...)`, that test was relying on the old wrong formula; update its expected output to match the corrected building-center-relative math.

**Step 2: Verify the refactor preserves behavior.** The math is identical (`pad_geometry::pad_cell_for` uses the exact +128 / 256 expression).

Run: `cargo test -p ra2-rust-game miner --lib`
Expected: All existing refinery tests PASS.

#### Task 3.4: Commit 3

Run: `cargo test -p ra2-rust-game --lib`
Expected: All PASS.

**Commit message:**
```
sim/docking: extract pad_geometry::pad_cell_for shared helper

New small module sim/docking/pad_geometry.rs centralizes the lepton→cell
conversion (+128 half-cell rounding) used by both refinery dock approach
and (next commit) aircraft pad-aware descent. refinery_pad_cell becomes
a thin wrapper.

Tests: zero offset, positive offset, negative-clamped, GAAIRC 4-pad,
z-doesnt-affect-cell.
```

---

### Commit 4 — Pad-keyed `AirfieldDocks` + thread `pad_index` through aircraft tick

**Goal:** Close the actual parity gap. `try_reserve` returns assigned pad_index; aircraft descent targets per-pad cell.

This is the biggest commit. Take it slowly.

#### Task 4.1: Refactor `AirfieldDocks` internals to be pad-keyed

**Why:** The reservation store must know which pad each aircraft holds.

**Files:**
- Modify: `src/sim/docking/aircraft_dock.rs:85-204`

**Step 1: Read [aircraft_dock.rs:85-204](src/sim/docking/aircraft_dock.rs#L85-L204).** Current struct:

```rust
pub struct AirfieldDocks {
    /// Maps airfield stable_id → (occupied_count, max_slots).
    slots: BTreeMap<u64, (u8, u8)>,
    /// Maps airfield stable_id → FIFO queue of waiting aircraft stable_ids.
    queues: BTreeMap<u64, VecDeque<u64>>,
    /// Maps aircraft stable_id → airfield stable_id (reverse lookup for cancel).
    aircraft_to_airfield: BTreeMap<u64, u64>,
}
```

**Step 2: Replace the struct definition with the pad-keyed version:**

```rust
/// Pad-aware multi-slot dock reservation manager for airfields.
///
/// Each airfield has `NumberOfDocks` pads. The `slots` map stores a per-airfield
/// `Vec<Option<u64>>` where index = pad index and value = occupant aircraft
/// stable_id (None = empty). First-empty-slot allocation matches gamemd's
/// `RadioClass::Transmit_Radio_Impl @ 0x0065A970` cmd 2 (HELLO) which scans
/// Contacts[]@+0xE4 linearly for the first empty slot.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AirfieldDocks {
    /// Per-airfield occupancy: pad_index → occupant aircraft (None = empty).
    /// Vec length equals NumberOfDocks for the airfield.
    slots: BTreeMap<u64, Vec<Option<u64>>>,
    /// Per-airfield FIFO queue of aircraft waiting for a free pad.
    queues: BTreeMap<u64, VecDeque<u64>>,
    /// Reverse lookup: aircraft → (airfield, pad_index).
    aircraft_to_pad: BTreeMap<u64, (u64, u8)>,
}
```

**Step 3: Replace `ensure_registered` (around line 97):**

```rust
    /// Register an airfield with its max dock count.
    /// Called lazily when an aircraft first tries to dock. Idempotent.
    fn ensure_registered(&mut self, airfield_sid: u64, num_pads: u8) {
        self.slots
            .entry(airfield_sid)
            .or_insert_with(|| vec![None; num_pads as usize]);
    }
```

**Step 4: Replace `try_reserve` (around line 105-126):**

```rust
    /// Try to reserve a pad slot for `aircraft_sid` at `airfield_sid`.
    ///
    /// Returns `Some(pad_index)` if a pad was assigned (immediately granted).
    /// Returns `None` if all pads are full — the aircraft is enqueued.
    ///
    /// First-empty-slot policy: scans `slots[airfield][0..num_pads]` left-to-right,
    /// returns first index where `Option<u64>::None`. Matches gamemd's
    /// linear scan of `Contacts[]@RadioClass+0xE4`.
    pub fn try_reserve(&mut self, airfield_sid: u64, aircraft_sid: u64, num_pads: u8) -> Option<u8> {
        self.ensure_registered(airfield_sid, num_pads);

        // Already docked here? Return existing pad index (idempotent).
        if let Some((af, pad)) = self.aircraft_to_pad.get(&aircraft_sid) {
            if *af == airfield_sid {
                return Some(*pad);
            }
        }

        let pads = self.slots.get_mut(&airfield_sid).expect("registered above");
        for (idx, slot) in pads.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(aircraft_sid);
                let pad_index = idx as u8;
                self.aircraft_to_pad
                    .insert(aircraft_sid, (airfield_sid, pad_index));
                return Some(pad_index);
            }
        }

        // All pads full — enqueue.
        let queue = self.queues.entry(airfield_sid).or_default();
        if !queue.contains(&aircraft_sid) {
            queue.push_back(aircraft_sid);
        }
        None
    }
```

**Step 5: Replace `release` (around line 130-148):**

```rust
    /// Release the aircraft's pad. Returns the next aircraft promoted from
    /// the queue, if any. The promoted aircraft gets the just-freed pad index.
    pub fn release(&mut self, aircraft_sid: u64) -> Option<u64> {
        let (airfield_sid, pad_index) = self.aircraft_to_pad.remove(&aircraft_sid)?;
        if let Some(pads) = self.slots.get_mut(&airfield_sid) {
            if let Some(slot) = pads.get_mut(pad_index as usize) {
                *slot = None;
            }
        }
        // Promote next aircraft from queue into the freed pad.
        if let Some(next) = self.queues.get_mut(&airfield_sid).and_then(|q| q.pop_front()) {
            if let Some(pads) = self.slots.get_mut(&airfield_sid) {
                if let Some(slot) = pads.get_mut(pad_index as usize) {
                    *slot = Some(next);
                }
            }
            self.aircraft_to_pad.insert(next, (airfield_sid, pad_index));
            return Some(next);
        }
        None
    }
```

**Step 6: Replace `has_free_slot`:**

```rust
    /// Check if an airfield has at least one free pad. Read-only probe.
    pub fn has_free_slot(&self, airfield_sid: u64, num_pads: u8) -> bool {
        match self.slots.get(&airfield_sid) {
            Some(pads) => pads.iter().any(|s| s.is_none()),
            None => num_pads > 0, // Not yet registered = all pads free.
        }
    }

    /// Look up which pad an aircraft is parked on, if any.
    pub fn pad_for(&self, aircraft_sid: u64) -> Option<(u64, u8)> {
        self.aircraft_to_pad.get(&aircraft_sid).copied()
    }
```

**Step 7: Replace `cancel`:**

```rust
    /// Cancel an aircraft's reservation or queue position.
    /// If cancellation frees a pad, promotes the next queued aircraft into it.
    pub fn cancel(&mut self, aircraft_sid: u64) {
        if let Some((airfield_sid, pad_index)) = self.aircraft_to_pad.remove(&aircraft_sid) {
            if let Some(pads) = self.slots.get_mut(&airfield_sid) {
                if let Some(slot) = pads.get_mut(pad_index as usize) {
                    *slot = None;
                }
            }
            // Promote next from queue into the freed pad.
            if let Some(next) = self.queues.get_mut(&airfield_sid).and_then(|q| q.pop_front()) {
                if let Some(pads) = self.slots.get_mut(&airfield_sid) {
                    if let Some(slot) = pads.get_mut(pad_index as usize) {
                        *slot = Some(next);
                    }
                }
                self.aircraft_to_pad.insert(next, (airfield_sid, pad_index));
            }
        } else {
            // Not docked anywhere — remove from any queue.
            for queue in self.queues.values_mut() {
                queue.retain(|&sid| sid != aircraft_sid);
            }
        }
    }
```

**Step 8: Replace `cleanup_dead`:**

```rust
    /// Remove dead entities (aircraft or airfields). Promotes queue as pads free.
    pub fn cleanup_dead(&mut self, alive: &BTreeSet<u64>) {
        // Remove dead airfields entirely.
        self.slots.retain(|sid, _| alive.contains(sid));
        self.queues.retain(|sid, _| alive.contains(sid));

        // Snapshot dead aircraft from pad assignments and release each.
        let dead_aircraft: Vec<u64> = self
            .aircraft_to_pad
            .keys()
            .filter(|sid| !alive.contains(sid))
            .copied()
            .collect();
        for sid in dead_aircraft {
            self.release(sid);
        }

        // Remove dead aircraft from queues.
        for queue in self.queues.values_mut() {
            queue.retain(|sid| alive.contains(sid));
        }
        self.queues.retain(|_, q| !q.is_empty());
    }
```

**Step 9: Verify the structural changes compile.**
Run: `cargo check -p ra2-rust-game`
Expected: Fails — `tick_aircraft_docks` still calls `try_reserve` expecting `bool`. Fix in Task 4.6.

#### Task 4.2: Add `target_pad` field to `AircraftAmmo`

**Files:**
- Modify: `src/sim/docking/aircraft_dock.rs:32-45`

**Step 1: Read [aircraft_dock.rs:32-45](src/sim/docking/aircraft_dock.rs#L32-L45).** Current struct:

```rust
pub struct AircraftAmmo {
    pub current: i32,
    pub max: i32,
    pub dock_phase: Option<AircraftDockPhase>,
    pub target_airfield: Option<u64>,
    pub reload_timer: u32,
    pub rescan_cooldown: u16,
}
```

**Step 2: Add `target_pad: Option<u8>`:**

```rust
pub struct AircraftAmmo {
    pub current: i32,
    pub max: i32,
    pub dock_phase: Option<AircraftDockPhase>,
    pub target_airfield: Option<u64>,
    /// Pad index assigned by `AirfieldDocks::try_reserve` for the current dock attempt.
    /// Set when transitioning from WaitForDock to Descending; cleared on launch.
    pub target_pad: Option<u8>,
    pub reload_timer: u32,
    pub rescan_cooldown: u16,
}
```

**Step 3: Update the `AircraftAmmo::new` constructor:**

```rust
impl AircraftAmmo {
    pub fn new(max_ammo: i32) -> Self {
        Self {
            current: max_ammo,
            max: max_ammo,
            dock_phase: None,
            target_airfield: None,
            target_pad: None,
            reload_timer: 0,
            rescan_cooldown: 0,
        }
    }
}
```

**Step 4: Verify.**
Run: `cargo check -p ra2-rust-game`
Expected: Same `tick_aircraft_docks` error remains; proceed to 4.3.

#### Task 4.3: Add `pad_index` to `AircraftMission::Docking` and `DockedIdle`

**Files:**
- Modify: `src/sim/aircraft/mod.rs:69-85`

**Step 1: Read [aircraft/mod.rs:69-85](src/sim/aircraft/mod.rs#L69-L85).** Current variants:

```rust
    Docking {
        airfield_id: u64,
        sub_state: u8,
        reload_timer: u32,
    },
    DockedIdle {
        airfield_id: u64,
    },
```

**Step 2: Add `pad_index: u8` to both:**

```rust
    Docking {
        airfield_id: u64,
        sub_state: u8,
        reload_timer: u32,
        /// Pad index this aircraft is docked on (0-based). Populated when
        /// transitioning from sub_state=wait_for_dock to descending.
        pad_index: u8,
    },
    DockedIdle {
        airfield_id: u64,
        /// Pad index this aircraft is parked on (0-based).
        pad_index: u8,
    },
```

**Step 3: Update every construction / pattern-match site.** Reviewed pre-implementation; the grep returns 13 hits across 3 files. Apply the change described for each row:

| # | Site | Kind | Change |
|---|---|---|---|
| 1 | `aircraft/mod.rs:392` | `Docking` ctor (transition from `ReturnToBase` when `dist <= 2`) | Add `pad_index: 0,` (fresh attempt; sub_state 0 will overwrite when try_reserve succeeds) |
| 2 | `aircraft/mod.rs:402` | `Docking` destructure (binds `airfield_id, sub_state, reload_timer`) | Add `pad_index,` to the pattern so it's bound. Used by ctors at lines 444/459/465/472 below. |
| 3 | `aircraft/mod.rs:434` | `Docking` ctor (sub_state 0→1, just after `try_reserve` returns success) | Add `pad_index: <returned_u8>,` using the `Option<u8>` value from Task 4.4b's `try_reserve` call. See Task 4.4b for the let-binding. |
| 4 | `aircraft/mod.rs:444` | `Docking` ctor (sub_state 1→2 on landing) | Add `pad_index: *pad_index,` (preserve from outer destructure at line 402) |
| 5 | `aircraft/mod.rs:459` | `Docking` ctor (sub_state 2→3 launch) | Add `pad_index: *pad_index,` |
| 6 | `aircraft/mod.rs:465` | `Docking` ctor (sub_state 2 stay) | Add `pad_index: *pad_index,` |
| 7 | `aircraft/mod.rs:472` | `Docking` ctor (sub_state 2 timer tick) | Add `pad_index: *pad_index,` |
| 8 | `aircraft/mod.rs:501` | `DockedIdle` destructure (binds `airfield_id` only) | Add `pad_index,` to the pattern even if unused (`pad_index: _` is acceptable until needed). |
| 9 | `aircraft/mod.rs:604` | `Docking` pattern with `sub_state: 1, ..` | **No change** — uses `..` |
| 10 | `aircraft/mod.rs:611` | `Docking` pattern with `sub_state: 3, ..` | **No change** — uses `..` |
| 11 | `world_commands.rs:1212` | `DockedIdle` pattern with `{ .. }` | **No change** — uses `..` |
| 12 | `production_queue.rs:522` | `DockedIdle` ctor (helipad spawn) | Add `pad_index: 0,` initially; Task 4.4b will update this to consume the `try_reserve` return at line 537. |
| 13 | matches!() macros at `aircraft/mod.rs:122, 128` | matches! patterns with `{ .. }` | **No change** — uses `..` |

**Step 4: Verify.**
Run: `cargo check -p ra2-rust-game --tests`
Expected: Compiles. (Build still has unresolved `try_reserve` boolean callers at `aircraft/mod.rs:429` and `production_queue.rs:537`; fixed in Task 4.4 and new Task 4.4b.)

#### Task 4.4: Update `tick_aircraft_docks` to use pad-aware reservation

**Why:** This is the actual parity fix — aircraft descend to per-pad cells, not building center.

**Files:**
- Modify: `src/sim/docking/aircraft_dock.rs:290-575`

**Step 1: Find the call to `try_reserve` at [aircraft_dock.rs:466-470](src/sim/docking/aircraft_dock.rs#L466-L470).** Current:

```rust
                if sim
                    .production
                    .airfield_docks
                    .try_reserve(af_sid, snap.id, max_slots)
                {
                    m.new_dock_phase = Some(Some(AircraftDockPhase::Descending));
                    m.set_air_phase = Some(AirMovePhase::Descending);
                    m.clear_movement = true;
                }
                // Otherwise keep waiting.
```

**Step 2: Replace with pad-aware logic that captures pad_index AND computes per-pad cell:**

```rust
                if let Some(pad_index) = sim
                    .production
                    .airfield_docks
                    .try_reserve(af_sid, snap.id, max_slots)
                {
                    m.new_dock_phase = Some(Some(AircraftDockPhase::Descending));
                    m.set_air_phase = Some(AirMovePhase::Descending);
                    m.clear_movement = true;

                    // Compute per-pad cell coords. Resolve the airfield's object so
                    // we can index pads[pad_index] and pass its foundation dims to
                    // pad_cell_for (gamemd's pad math is building-center-relative).
                    // If pads is empty (no DockingOffset in art.ini), fall back to
                    // building center (which is what find_nearest_airfield targeted).
                    let pad_cell = sim
                        .entities
                        .get(af_sid)
                        .and_then(|af| {
                            let obj = rules.object(sim.interner.resolve(af.type_ref))?;
                            let foundation = crate::sim::production::foundation_dimensions(&obj.foundation);
                            obj.pads.get(pad_index as usize).map(|pad| {
                                crate::sim::docking::pad_geometry::pad_cell_for(
                                    (af.position.rx, af.position.ry),
                                    foundation,
                                    pad,
                                )
                            })
                        });

                    if let Some((px, py)) = pad_cell {
                        m.air_move_to = Some((px, py));
                    }
                    // Else: stay on the building-center target set during ReturnToBase.

                    m.new_target_pad = Some(Some(pad_index));
                }
                // Otherwise keep waiting (pad busy, aircraft remains in WaitForDock).
```

**Step 3: Add `new_target_pad` field to the `AircraftMutation` struct.** Find the struct around [aircraft_dock.rs:350-365](src/sim/docking/aircraft_dock.rs#L350-L365) and add:

```rust
    struct AircraftMutation {
        id: u64,
        new_dock_phase: Option<Option<AircraftDockPhase>>,
        new_target_airfield: Option<Option<u64>>,
        new_target_pad: Option<Option<u8>>,         // NEW
        new_reload_timer: Option<u32>,
        new_rescan_cooldown: Option<u16>,
        restore_ammo: i32,
        clear_attack_target: bool,
        set_air_phase: Option<AirMovePhase>,
        air_move_to: Option<(u16, u16)>,
        clear_movement: bool,
    }
```

**Step 4: Initialize `new_target_pad` in the per-aircraft mutation struct (around line 365-380):**

```rust
        let mut m = AircraftMutation {
            id: snap.id,
            new_dock_phase: None,
            new_target_airfield: None,
            new_target_pad: None,           // NEW
            new_reload_timer: None,
            new_rescan_cooldown: None,
            restore_ammo: 0,
            clear_attack_target: false,
            set_air_phase: None,
            air_move_to: None,
            clear_movement: false,
        };
```

**Step 5: Apply target_pad in the mutation-apply loop** (around [aircraft_dock.rs:519-548](src/sim/docking/aircraft_dock.rs#L519-L548)):

```rust
    for m in &mutations {
        if let Some(entity) = sim.entities.get_mut(m.id) {
            if let Some(ref mut ammo) = entity.aircraft_ammo {
                if let Some(new_phase) = m.new_dock_phase {
                    ammo.dock_phase = new_phase;
                }
                if let Some(new_af) = m.new_target_airfield {
                    ammo.target_airfield = new_af;
                }
                if let Some(new_pad) = m.new_target_pad {     // NEW
                    ammo.target_pad = new_pad;
                }
                if let Some(new_timer) = m.new_reload_timer {
                    ammo.reload_timer = new_timer;
                }
                if let Some(new_cooldown) = m.new_rescan_cooldown {
                    ammo.rescan_cooldown = new_cooldown;
                }
                ammo.current = (ammo.current + m.restore_ammo).min(ammo.max);
            }
            // ...rest unchanged
        }
    }
```

**Step 6: When ammo is fully restored and the aircraft launches** (around [aircraft_dock.rs:493-497](src/sim/docking/aircraft_dock.rs#L493-L497)), clear `target_pad`:

Find:
```rust
                    if new_ammo >= snap.max_ammo {
                        m.new_dock_phase = Some(Some(AircraftDockPhase::Launching));
                        m.set_air_phase = Some(AirMovePhase::Ascending);
                        sim.production.airfield_docks.release(snap.id);
                    } else {
```

Replace with:
```rust
                    if new_ammo >= snap.max_ammo {
                        m.new_dock_phase = Some(Some(AircraftDockPhase::Launching));
                        m.set_air_phase = Some(AirMovePhase::Ascending);
                        m.new_target_pad = Some(None);                 // clear pad index
                        sim.production.airfield_docks.release(snap.id);
                    } else {
```

**Step 7: Same cleanup in the `ReturnToBase → None` (no airfield available) and any other path that releases the dock.** Search for `airfield_docks.cancel` and `airfield_docks.release` in this file and ensure each sets `m.new_target_pad = Some(None);` so the field is cleared.

**Step 8: Verify.**
Run: `cargo check -p ra2-rust-game --tests`
Expected: PASS.

#### Task 4.4b: Update parallel `try_reserve` call sites (`aircraft/mod.rs` + `production_queue.rs`)

**Why:** Task 4.1 changed `AirfieldDocks::try_reserve` to return `Option<u8>` instead of `bool`. Three callers exist; Task 4.4 covered the one in `aircraft_dock.rs::tick_aircraft_docks`. Two more callers remain in:
- `aircraft/mod.rs:429` — used by the `AircraftMission::Docking` state machine (the *active* aircraft mission path; `aircraft_dock.rs::tick_aircraft_docks` only handles aircraft *without* an `aircraft_mission`).
- `production_queue.rs:537` — used when a freshly-produced aircraft spawns at a helipad and needs to reserve its dock slot.

Without this task, Commit 4 fails to compile.

**Files:**
- Modify: `src/sim/aircraft/mod.rs:429-440`
- Modify: `src/sim/production/production_queue.rs:519-538`

**Pattern:** Same as Task 4.4 — capture `Option<u8>` return, thread `pad_index` into AircraftMission construction.

**Step 1: Update `aircraft/mod.rs:429-439`.** Current:

```rust
if sim.production.airfield_docks.try_reserve(
    *airfield_id,
    snap.id,
    max_slots,
) {
    m.new_mission = AircraftMission::Docking {
        airfield_id: *airfield_id,
        sub_state: 1,
        reload_timer: 0,
    };
}
```

**Step 2: Replace with:**

```rust
if let Some(pad_index) = sim.production.airfield_docks.try_reserve(
    *airfield_id,
    snap.id,
    max_slots,
) {
    m.new_mission = AircraftMission::Docking {
        airfield_id: *airfield_id,
        sub_state: 1,
        reload_timer: 0,
        pad_index,
    };
    // Per design: thread per-pad cell into AircraftAmmo.target_pad and
    // re-target descent toward the assigned pad cell.
    if let Some(entity) = sim.entities.get_mut(snap.id) {
        if let Some(ref mut ammo) = entity.aircraft_ammo {
            ammo.target_pad = Some(pad_index);
        }
    }
    // Issue air-move to per-pad cell (replaces building-center target set during ReturnToBase).
    if let Some((px, py)) = sim
        .entities
        .get(*airfield_id)
        .and_then(|af| {
            let obj = rules.object(sim.interner.resolve(af.type_ref))?;
            let foundation = crate::sim::production::foundation_dimensions(&obj.foundation);
            obj.pads.get(pad_index as usize).map(|pad| {
                crate::sim::docking::pad_geometry::pad_cell_for(
                    (af.position.rx, af.position.ry),
                    foundation,
                    pad,
                )
            })
        })
    {
        m.move_to = Some((px, py));
    }
}
```

**Note:** the snapshot/mutation pattern in `aircraft/mod.rs` differs slightly from `aircraft_dock.rs` (uses `m.move_to` vs `m.air_move_to`, and mutates entity directly inside the conditional). Adapt the field names to match — check the existing `AircraftMissionMutation` struct definition nearby for the exact field name (`move_to` likely, but verify).

**Step 3: Update `production_queue.rs:535-537`.** Current:

```rust
sim.production
    .airfield_docks
    .try_reserve(af_id, stable_id, max_slots);
```

This call ignored the return value before; now it returns `Option<u8>`. The helipad-spawned aircraft needs its `pad_index` set on the just-constructed `AircraftMission::DockedIdle`.

**Step 4: Replace with:**

```rust
let pad_index = sim
    .production
    .airfield_docks
    .try_reserve(af_id, stable_id, max_slots)
    .unwrap_or(0); // Helipads have NumberOfDocks=1, so pad 0 always available on fresh spawn.
// Patch the DockedIdle we just constructed to carry the assigned pad_index.
if let Some(entity) = sim.entities.get_mut(stable_id) {
    if let Some(crate::sim::aircraft::AircraftMission::DockedIdle {
        ref mut pad_index: stored_pad,
        ..
    }) = entity.aircraft_mission
    {
        *stored_pad = pad_index;
    }
}
```

**Note:** The cleaner approach would be to construct the `DockedIdle` AFTER calling `try_reserve`. If the existing code structure allows, swap the order in lines 521-524 and 535-537 to construct `DockedIdle { ..., pad_index }` directly. Use whichever is less disruptive to existing logic.

**Step 5: Verify.**
Run: `cargo build -p ra2-rust-game --tests`
Expected: PASS. All three `try_reserve` callers consume `Option<u8>`. Both `AircraftMission::Docking` and `DockedIdle` constructions thread `pad_index`.

#### Task 4.5: Update existing AirfieldDocks tests

**Why:** Old tests assume `try_reserve` returns `bool` and check `aircraft_to_airfield` field.

**Files:**
- Modify: `src/sim/docking/aircraft_dock.rs:584-645` (existing test module)

**Step 1: Replace each of the 5 existing tests with pad-aware versions:**

```rust
    #[test]
    fn airfield_docks_basic_reserve() {
        let mut docks = AirfieldDocks::default();
        // 2-pad airfield: first two aircraft get pads 0 and 1.
        assert_eq!(docks.try_reserve(100, 1, 2), Some(0));
        assert_eq!(docks.try_reserve(100, 2, 2), Some(1));
        // 3rd aircraft queues.
        assert_eq!(docks.try_reserve(100, 3, 2), None);
        assert_eq!(docks.queues[&100].len(), 1);
    }

    #[test]
    fn airfield_docks_release_promotes() {
        let mut docks = AirfieldDocks::default();
        docks.try_reserve(100, 1, 1);
        docks.try_reserve(100, 2, 1); // queued
        docks.try_reserve(100, 3, 1); // queued
        let promoted = docks.release(1);
        assert_eq!(promoted, Some(2));
        assert_eq!(docks.pad_for(2), Some((100, 0)), "promoted into pad 0");
    }

    #[test]
    fn airfield_docks_cancel() {
        let mut docks = AirfieldDocks::default();
        docks.try_reserve(100, 1, 2);
        docks.try_reserve(100, 2, 2);
        docks.try_reserve(100, 3, 2); // queued
        docks.cancel(1);
        // Pad 0 freed, queued #3 promoted into it.
        assert_eq!(docks.pad_for(3), Some((100, 0)));
        assert_eq!(docks.pad_for(2), Some((100, 1)));
    }

    #[test]
    fn airfield_docks_cleanup_dead() {
        let mut docks = AirfieldDocks::default();
        docks.try_reserve(100, 1, 2);
        docks.try_reserve(100, 2, 2);
        docks.try_reserve(100, 3, 2); // queued
        let alive: BTreeSet<u64> = [100, 2, 3].into_iter().collect();
        docks.cleanup_dead(&alive);
        // Aircraft 1 died — pad 0 freed, #3 promoted into pad 0.
        assert_eq!(docks.pad_for(1), None);
        assert_eq!(docks.pad_for(3), Some((100, 0)));
    }

    #[test]
    fn airfield_docks_idempotent_reserve() {
        let mut docks = AirfieldDocks::default();
        assert_eq!(docks.try_reserve(100, 1, 2), Some(0));
        assert_eq!(docks.try_reserve(100, 1, 2), Some(0), "still pad 0");
    }
```

**Step 2: Verify.**
Run: `cargo test -p ra2-rust-game aircraft_dock --lib`
Expected: 5 existing tests PASS with new pad-aware assertions.

#### Task 4.6: Add new tests for parity-critical pad behaviors

**Files:**
- Modify: `src/sim/docking/aircraft_dock.rs` test module — append.

**Step 1: Append:**

```rust
    #[test]
    fn airfield_docks_four_pad_allocation_order() {
        // GAAIRC has 4 pads. First 4 aircraft get pads 0..3 in arrival order.
        let mut docks = AirfieldDocks::default();
        assert_eq!(docks.try_reserve(100, 11, 4), Some(0));
        assert_eq!(docks.try_reserve(100, 12, 4), Some(1));
        assert_eq!(docks.try_reserve(100, 13, 4), Some(2));
        assert_eq!(docks.try_reserve(100, 14, 4), Some(3));
        // 5th queues.
        assert_eq!(docks.try_reserve(100, 15, 4), None);
        assert_eq!(docks.queues[&100].len(), 1);
    }

    #[test]
    fn airfield_docks_release_pad_1_promotes_into_pad_1() {
        // Parity test: when pad 1 (specifically) is released, the queued
        // aircraft takes pad 1, not pad 0 or "the next free pad". This
        // matches gamemd's RadioClass::FindDockSlot which scans for the
        // first empty slot — but release frees a specific slot.
        let mut docks = AirfieldDocks::default();
        docks.try_reserve(100, 11, 4); // pad 0
        docks.try_reserve(100, 12, 4); // pad 1
        docks.try_reserve(100, 13, 4); // pad 2
        docks.try_reserve(100, 14, 4); // pad 3
        docks.try_reserve(100, 15, 4); // queued
        docks.release(12); // free pad 1
        assert_eq!(docks.pad_for(15), Some((100, 1)), "queued promoted into pad 1");
    }

    #[test]
    fn airfield_docks_single_pad_helipad() {
        // Helipads (NAHPAD/GAHPAD) have NumberOfDocks=1.
        let mut docks = AirfieldDocks::default();
        assert_eq!(docks.try_reserve(200, 21, 1), Some(0));
        assert_eq!(docks.try_reserve(200, 22, 1), None, "queued");
        docks.release(21);
        assert_eq!(docks.pad_for(22), Some((200, 0)));
    }

    #[test]
    fn airfield_docks_pad_assignment_is_deterministic() {
        // Determinism check: two independent runs with same input produce
        // identical pad assignments. Required for replay/lockstep.
        let mut run_a = AirfieldDocks::default();
        let mut run_b = AirfieldDocks::default();
        // Same arrival order in both runs.
        for ac in [11_u64, 12, 13, 14] {
            let pa = run_a.try_reserve(100, ac, 4);
            let pb = run_b.try_reserve(100, ac, 4);
            assert_eq!(pa, pb, "aircraft {} got same pad in both runs", ac);
        }
    }
```

**Step 2: Run.**
Run: `cargo test -p ra2-rust-game aircraft_dock --lib`
Expected: 4 new tests PASS, all existing PASS.

#### Task 4.7: Confirm state-hash interaction (NO update needed)

**Why:** Verified pre-implementation that `world_hash.rs` is manual `Hash` impl (not serde-derive). Specifically: `grep aircraft_ammo|aircraft_mission src/sim/world/world_hash.rs` returns **zero matches**. Neither `AircraftAmmo` nor `AircraftMission` is currently part of the deterministic state hash — pre-existing gap, out of scope for this work.

**Consequence:** adding `AircraftAmmo.target_pad` and `AircraftMission::Docking.pad_index` requires NO update to `world_hash.rs`. The new fields are not hashed today (because the parent components aren't hashed today), and that's fine for this commit.

**DO NOT** add aircraft hashing in this commit. That's a separate, larger fix (would change every existing replay's state hash → invalidate save-game compatibility). It belongs in a dedicated determinism-coverage commit, not bundled into the multi-pad work.

**Step 1: Confirm by grep.**
Run:
```
grep -n "aircraft_ammo\|aircraft_mission" src/sim/world/world_hash.rs
```
Expected: no output (zero matches). If matches DO appear, the codebase has changed since this plan was written — STOP and reassess.

**Step 2: No code change.** Move on to Task 4.8.

#### Task 4.8: Commit 4

Run: `cargo test -p ra2-rust-game`
Expected: All PASS.

**Commit message:**
```
sim/docking: pad-keyed AirfieldDocks closes GAAIRC 4-pad parity gap

AirfieldDocks now tracks per-pad occupancy via Vec<Option<u64>> per airfield.
try_reserve returns Option<u8> (assigned pad index, None = queued). Aircraft
descend to the per-pad cell computed via pad_geometry::pad_cell_for(origin,
&pads[pad_index]) instead of the building center.

AircraftAmmo gains target_pad: Option<u8>; AircraftMission::Docking and
DockedIdle gain pad_index: u8 for save-game roundtrip. First-empty-slot
allocation order matches gamemd RadioClass::FindDockSlot (linear Contacts[]
scan).

Visible result: GAAIRC and AMRADR (both NumberOfDocks=4) now land aircraft
on 4 distinct pad cells (2x2 spread within foundation), matching gamemd.
Helipads, refineries, depots (all NumberOfDocks=1) keep existing single-pad
behavior with pads[0].

Tests:
- four_pad_allocation_order: pads 0..3 assigned in arrival order
- release_pad_1_promotes_into_pad_1: pad-specific promotion semantics
- single_pad_helipad: NumberOfDocks=1 round-trip
- pad_assignment_is_deterministic: replay/lockstep correctness
```

---

### Commit 5 — Documentation + dead-code cleanup

**Goal:** Update module doc comments, remove any transitional code, run clippy clean.

#### Task 5.1: Update module doc comments

**Files:**
- Modify: `src/sim/docking/aircraft_dock.rs:1-15` (module header)
- Modify: `src/sim/docking/pad_geometry.rs:1-12` (already correct)
- Modify: `src/sim/miner/miner_dock_sequence.rs:1-12` (note the pad_cell_for delegation)

**Step 1: Update aircraft_dock.rs header:**

```rust
//! Aircraft ammo tracking and airfield docking system.
//!
//! Aircraft with finite `Ammo=` (from rules.ini) deplete ammo on each weapon
//! fire. When ammo reaches 0, the aircraft auto-returns to the nearest
//! helipad/airfield owned by the same player, descends onto its assigned
//! pad cell, reloads, and re-launches.
//!
//! Multi-pad airfields (GAAIRC, AMRADR: NumberOfDocks=4) allocate pad indices
//! via `AirfieldDocks::try_reserve` (first-empty-slot, matching gamemd's
//! RadioClass::FindDockSlot at 0x0065AD90). Aircraft descends to the per-pad
//! cell computed by `pad_geometry::pad_cell_for`.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/, sim/components, sim/air_movement,
//!   sim/docking/pad_geometry.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.
```

**Step 2: Update miner_dock_sequence.rs header — add the pad_geometry reference:**

Add this line in the module doc:
> `refinery_pad_cell` is now a thin wrapper over `sim::docking::pad_geometry::pad_cell_for`. The single +128 half-cell rounding implementation lives there.

#### Task 5.2: Run clippy clean

**Step 1:** Run: `cargo clippy -p ra2-rust-game --all-targets --no-deps -- -D warnings`
**Expected:** No warnings. Fix any introduced during this work.

#### Task 5.3: Search for any remaining "docking_offset" references

**Step 1:** Run: `grep -rn "docking_offset" src/ docs/ --include='*.rs' --include='*.md'`
**Expected:** No `.rs` matches. Only `.md` matches in design doc context — leave those.

**Step 2: If any `.rs` reference remains, fix.**

#### Task 5.4: Final commit

**Commit message:**
```
sim/docking: docs + cleanup post-multi-pad

Updates module headers to reflect the new pad-aware data flow. Confirms
no stale references to docking_offset remain in src/. Clippy clean.
```

---

## Verification After All 5 Commits

After Commit 5, run the full suite:

```
cargo test -p ra2-rust-game
cargo clippy -p ra2-rust-game --all-targets --no-deps -- -D warnings
cargo build --release
```

**Manual in-game test:**
1. Build GAAIRC (Allied Airforce Command).
2. Train 4 aircraft (Harriers/Black Eagles).
3. Wait for them to land or order them to RTB.
4. Confirm visually: aircraft land on 4 distinct cells in a 2×2 grid, not stacked.
5. Repeat with AMRADR (American variant, uses Image=GAAIRC).
6. Sanity: helipads (NAHPAD/GAHPAD) still land 1 aircraft per pad.

Compare against retail gamemd.exe by running the same scenario in the original game and screenshotting.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-11-mission-enter-and-multi-dock-design.md](docs/plans/2026-05-11-mission-enter-and-multi-dock-design.md)
- **Investigation plan:** [docs/plans/2026-05-11-mission-enter-and-multi-dock-investigation-plan.md](docs/plans/2026-05-11-mission-enter-and-multi-dock-investigation-plan.md)
- **Ghidra reports:**
  - [BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md](docs/research/BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md) — `NumberOfDocks` at `BuildingTypeClass+0x1780`, DockingOffset array at `+0x1788` stride 12. Verification audit appended 2026-05-11.
  - [MISSION_ENTER_CROSSWALK_AND_GAPS_GHIDRA_REPORT.md](docs/research/MISSION_ENTER_CROSSWALK_AND_GAPS_GHIDRA_REPORT.md) — `AircraftClass::Mission_Enter @ 0x00419C80` full decompile (state 7 reads per-pad coords via building vtable[+0xA8]). First-empty-slot allocation verified in `RadioClass::Transmit_Radio_Impl @ 0x0065A970` cmd 2 (HELLO).
  - [HARVESTER_DOCK_UNLOAD.md](docs/research/HARVESTER_DOCK_UNLOAD.md) — refinery exit facing 0x47 + (-0x80, +0x80) offset; preserved by leaving refinery_pad_cell signature unchanged.
- **gamemd.exe addresses (kept here, not in Rust comments):**
  - `0x0045FE50` (`BuildingTypeClass_ReadINI_Water` — parser entry)
  - `0x004649AF`–`0x00464A41` (DockingOffset loop)
  - `0x0065AD90` (`RadioClass::FindDockSlot`)
  - `0x0065A970` (`RadioClass::Transmit_Radio_Impl` cmd 2 HELLO)
  - `0x00419C80` (`AircraftClass::Mission_Enter`)
  - `0x004595C0` (`BuildingClass::ReleaseDockedHarvester` — per-cycle harvester release)
- **INI keys (from ini/rulesmd.ini and ini/artmd.ini):**
  - `NumberOfDocks` — section default 1; GAAIRC=4, AMRADR=4
  - `DockingOffset0..3` — GAAIRC: (0,-128,0), (0,128,0), (256,-128,0), (256,128,0); NADEPT: (128,0,0); others single or absent
  - `Image=GAAIRC` on AMRADR — inherits 4-pad art
  - `Refinery=yes`, `UnitRepair=yes`, `Helipad=yes`, `UnitReload=yes` — gating flags (unchanged)
- **Related code (current paths):**
  - [src/rules/object_type.rs:305-308, 820, 950](src/rules/object_type.rs)
  - [src/rules/art_data.rs:89-95, 265-285, 387-392](src/rules/art_data.rs)
  - [src/rules/ruleset.rs:1630-1640](src/rules/ruleset.rs)
  - [src/sim/miner/miner_dock_sequence.rs:57-74](src/sim/miner/miner_dock_sequence.rs)
  - [src/sim/docking/aircraft_dock.rs:32-204, 290-575](src/sim/docking/aircraft_dock.rs)
  - [src/sim/aircraft/mod.rs:69-85](src/sim/aircraft/mod.rs)
- **Prior commits:** none directly relevant; refinery dock work landed earlier in `2026-05-06-refinery-dock-gamemd-parity-{design,plan}.md`.
