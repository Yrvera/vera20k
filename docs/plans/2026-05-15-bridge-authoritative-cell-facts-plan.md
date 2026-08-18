# Bridge Authoritative Cell Facts Implementation Plan

> For Claude: Execute this plan task-by-task. Each task is self-contained.

**Goal:** Replace broad high-bridge terrain inference with first-class
`SetBridgeDirection`-equivalent bridge facts produced from map overlay data.

**Architecture:** This is a `map/` data correction with compatibility consumers
in `sim/`. The authoritative bridge facts are built during map/resolved-terrain
construction, then consumed by bridge runtime, pathfinding, movement, and render
through existing map data surfaces. `sim/` must not gain any dependency on
`render/`, `ui/`, `sidebar/`, `audio/`, or `net/`.

**Design Input:** No separate `*-design.md` exists. This plan treats the audited
Ghidra report plus the Priority 1 list as the approved design input:

- `docs/plans/2026-05-15-bridge-parity-fix-priority-list.md`
- `docs/research/BRIDGE_SETBRIDGEDIRECTION_STAMPING_GHIDRA_REPORT.md`

---

## Grounding Summary

The parent bridge reports establish that high-bridge map-load facts in
`gamemd.exe` are stamped from overlay object marking through
`OverlayClass::Mark` into `CellClass::SetBridgeDirection_*`, not inferred by a
resolved-terrain flood fill. The audited stamping report confirms the exact
high-bridge overlay IDs, call arguments, cell slots, flag writes, anchor pointer
writes, and the later `[OverlayDataPack]` overwrite of `cell+0x11E`.

Live Ghidra verification in this session confirmed the same addresses and
control flow: `SetBridgeDirection_NESW @ 0x0047E040`,
`SetBridgeDirection_NWSE @ 0x0047E470`, `OverlayClass::Mark @ 0x005FC570`,
`ReadMapOverlayPacks @ 0x005FD2E0`, `CellClass::RecalcAttributes @
0x0047D2B0`, and `MapClass::Resize @ 0x00565C10`. The one static caveat is that
the direction-offset table at `0x0089F688` is runtime-populated; the direction
index convention `0=N, 2=E, 4=S, 6=W` comes from prior audited direction
encoding, not static bytes.

Current Rust parses overlay entries in `src/map/overlay.rs`, but only preserves
the `[OverlayDataPack]` byte on cells that also have an overlay. This plan must
preserve an overlay-data byte grid because the binary writes `cell+0x11E` for
every in-bounds cell after stamping. Current Rust then derives bridge facts in
`src/map/resolved_terrain.rs` through broad overlay classification, side-cell
expansion, connected-component deck-level normalization, broad
`BridgeSet/WoodBridgeSet` bridgehead detection, and gap fill. Those are the
main Priority 1 removal targets.

INI grounding: high-bridge overlay IDs are hardcoded position IDs, not flags.
In `rulesmd.ini`, `[OverlayTypes]` entries `25=BRIDGE1`, `26=BRIDGE2`,
`241=BRIDGEB1`, and `242=BRIDGEB2` become 0-based map IDs `0x18`, `0x19`,
`0xED`, and `0xEE`. Low bridge overlays remain out of scope for this plan.

---

## Key Technical Decisions

- Build raw bridge facts in `map/`, not `sim/`. **Confidence: high.**
  **Source:** `BRIDGE_SETBRIDGEDIRECTION_STAMPING_GHIDRA_REPORT.md`;
  layering rule in `AGENTS.md`.

- Preserve `[OverlayDataPack]` as a 512 by 512 byte grid plus an explicit
  presence flag on `MapFile`. **Confidence: high.** **Source:**
  `ReadMapOverlayPacks @ 0x005FD2E0` writes every in-bounds `cell+0x11E`
  after `[OverlayPack]`, but only when the `[OverlayDataPack]` section is
  present and decodes successfully.

- Add `BridgeCellFacts` alongside existing flattened bridge fields, then derive
  compatibility fields from it. **Confidence: high.** **Source:** current
  consumers still depend on `has_bridge_deck`, `bridge_walkable`,
  `bridge_transition`, and `bridge_layer`.

- Implement high-bridge map-load stamping only for IDs `0x18`, `0x19`,
  `0xED`, and `0xEE`. **Confidence: high.** **Source:** `OverlayClass::Mark`
  xrefs at `0x005FC5FE`, `0x005FC60A`, `0x005FC62C`.

- Do not implement low-bridge tube semantics in this plan. **Confidence: high.**
  **Source:** `BRIDGE_LOW_AND_ZONE_RECORDS_GHIDRA_SUPPLEMENT.md` proves low
  bridges are tube/land-type backed and should not be folded into high-bridge
  facts.

- Keep bridgehead/ramp exact tile predicates deferred to Priority 2, but stop
  relying on broad `BridgeSet/WoodBridgeSet` as an authoritative high-bridge
  fact source. **Confidence: medium.** **Source:** parent report verifies broad
  detection is too coarse; exact bridgehead replacement is the next priority.

---

## Open Questions

### Resolved During Planning

- **Is another Ghidra pass required before coding Priority 1?** No. The audited
  report found the high-bridge stamping table safe to use. A data dump of stock
  maps is useful later, but not a blocker for implementing the stamping model.

- **Can `OverlayEntry.frame` alone represent `cell+0x11E`?** No. The binary
  writes `[OverlayDataPack]` bytes to every in-bounds cell, including stamped
  neighbor cells with no overlay entry. Preserve a map-level data byte grid.

### Deferred to Implementation

- **How many current tests depend on side expansion/gap fill?** The plan
  includes targeted test updates after the raw facts are introduced. Any failing
  test must be rewritten to assert stamped binary facts, not the old inferred
  shape.

- **Which stock maps expose bridge facts that current Rust invented?** A map
  dump/check can be added after the code path exists. It is a validation task,
  not a prerequisite for the pure stamping function.

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/map/mod.rs` | Export new bridge facts module. |
| Create | `src/map/bridge_facts.rs` | Raw bridge flag constants, stamp data types, and pure `SetBridgeDirection` map-load stamping. |
| Modify | `src/map/overlay.rs` | Preserve overlay data bytes separately from overlay entries. |
| Modify | `src/map/map_file.rs` | Store the overlay data byte grid and expose coordinate lookup. |
| Modify | `src/map/overlay_types.rs` | Split high-bridge anchor ID helpers from broad bridge overlay helpers. |
| Modify | `src/map/resolved_terrain.rs` | Add raw bridge facts to cells, run high-bridge stamping, derive compatibility fields, and remove high-bridge side/gap inference. |
| Modify | `src/sim/bridge_state/mod.rs` | Read raw facts for anchor spans and roles instead of re-inferring anchors from broad bridge layers. |
| Modify | `src/sim/bridge_specs.rs` | Keep runtime `set_bridge_direction` behavior aligned with the same slot table and cover the alignment with tests. |
| Modify | `src/sim/pathfinding/core.rs` | No traversal rewrite; only adjust tests or compatibility assumptions broken by corrected facts. |
| Modify | `src/sim/movement/movement_bridge.rs` | No traversal rewrite; only adjust tests or compatibility assumptions broken by corrected facts. |

---

## Interface Changes

- `MapFile` gains a full overlay data byte grid, a presence query, and a lookup
  method: `has_overlay_data_pack() -> bool` and `overlay_data_at(rx, ry) -> u8`.
- `ResolvedTerrainCell` gains `bridge_facts: BridgeCellFacts`.
- `src/map/bridge_facts.rs` exposes pure data types and helpers:
  `BridgeCellFacts`, `BridgeStampFamily`, `BridgeStampSlot`,
  `BridgeAnchorRelation`, high-bridge overlay classifier, and
  `stamp_set_bridge_direction`.
- `BridgeRuntimeState::from_resolved_terrain` reads `bridge_facts` when
  constructing anchor spans and roles.

These are map/sim-facing APIs. No render or UI types are introduced.

---

## Sim Checklist

- [ ] All math uses integer cell coordinates and bytes; no `f32` or `f64` in sim logic.
- [ ] No new deterministic runtime state is added outside existing bridge state construction.
- [ ] No dependencies on `render/`, `ui/`, `sidebar/`, `audio/`, or `net`.
- [ ] Tick ordering is unchanged; this is map-load initialization plus existing rebuild consumers.
- [ ] `BTreeMap` iteration remains deterministic for existing `anchor_spans` and `group_cells`.

---

## Risk Areas

- `ResolvedTerrainCell` has many test fixtures. Adding `bridge_facts` will cause
  compile failures until all literals are updated with `BridgeCellFacts::default()`.
- Removing high-bridge side expansion and gap fill can break tests that encoded
  the old inferred behavior. Those tests must be changed to assert the binary
  stamped cells.
- `BridgeRuntimeState::from_resolved_terrain` currently groups by
  `has_bridge_deck`. Once that field derives from raw `0x100`, anchor span
  construction must use `bridge_facts` to avoid losing direction, slot, and
  anchor relation.
- Rendering code may still use `bridge_layer` to select overlay art. This plan
  keeps `bridge_layer` as compatibility metadata for overlay-bearing cells while
  moving authoritative cell facts into `bridge_facts`.
- Priority 2 bridgehead/ramp exact detection is not implemented here. This plan
  must not silently reintroduce broad bridgehead facts as authoritative data.

---

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 1 | Preserve `[OverlayDataPack]` for every cell when present | Final bridge state byte can exist on stamped cells with no overlay entry, but maps without `[OverlayDataPack]` keep SetBridgeDirection defaults | Unit tests for present data byte lookup, missing-pack fallback, and out-of-range lookup |
| 2 | Exact high-bridge overlay ID dispatch | Wrong IDs stamp the wrong cells or fold low bridges into high bridge pathing | Unit tests for `0x18`, `0x19`, `0xED`, `0xEE`, and low IDs ignored |
| 3 | Exact per-slot flag table | A one-bit mismatch changes traversal, damage, render height, or bridge repair inputs | Unit tests for dir 0 and dir 6 intact stamps |
| 5 | Remove side expansion, normalization, and gap fill from high-bridge facts | Invented deck cells cause A* and locomotion to debug against fake input | Resolved terrain tests prove unstamped side/gap cells remain unstamped |
| 6 | Runtime anchor spans read stamped direction and relation | Damage/collapse side effects must hit the same relative cells as gamemd.exe | Bridge runtime tests check slots 0-5 and blow-up slots |

---

## Tasks

### Task 1: Preserve OverlayDataPack Bytes

**Why:** `gamemd.exe` writes `[OverlayDataPack]` to `cell+0x11E` for every
in-bounds cell after overlay stamping only when the section exists. Current Rust
only keeps frame bytes on cells that have an overlay entry.

**Files:**
- Modify: `src/map/overlay.rs`
- Modify: `src/map/map_file.rs`

**Pattern:** Follow existing `parse_overlays` and `OverlayEntry` parsing in
`src/map/overlay.rs`.

**Step 1: Add an overlay-data grid type**

```rust
// src/map/overlay.rs
#[derive(Debug, Clone)]
pub struct OverlayDataPack {
    bytes: Vec<u8>,
    present: bool,
}

impl OverlayDataPack {
    pub fn from_decoded(bytes: Vec<u8>) -> Self {
        let mut normalized = bytes;
        normalized.resize(OVERLAY_TOTAL_CELLS, 0);
        Self {
            bytes: normalized,
            present: true,
        }
    }

    pub fn missing() -> Self {
        Self {
            bytes: vec![0; OVERLAY_TOTAL_CELLS],
            present: false,
        }
    }

    pub fn is_present(&self) -> bool {
        self.present
    }

    pub fn byte_at(&self, rx: u16, ry: u16) -> u8 {
        if rx as usize >= OVERLAY_GRID_SIZE || ry as usize >= OVERLAY_GRID_SIZE {
            return 0;
        }
        let idx = ry as usize * OVERLAY_GRID_SIZE + rx as usize;
        self.bytes[idx]
    }
}
```

**Step 2: Return entries and data together**

Add:

```rust
#[derive(Debug, Clone)]
pub struct ParsedOverlayPacks {
    pub entries: Vec<OverlayEntry>,
    pub data: OverlayDataPack,
}

pub fn parse_overlay_packs(ini: &IniFile) -> ParsedOverlayPacks {
    // Decode OverlayPack as today.
    // Decode OverlayDataPack once, using OverlayDataPack::missing() when absent.
    // Build entries in the same idx order as current parse_overlays.
}
```

Keep `parse_overlays(ini)` as a compatibility wrapper returning
`parse_overlay_packs(ini).entries`.

**Step 3: Store the data grid in MapFile**

Add to `MapFile`:

```rust
pub overlay_data: overlay::OverlayDataPack,
```

In `MapFile::from_bytes`, call `parse_overlay_packs` once, assign
`overlays = parsed.entries`, and assign `overlay_data = parsed.data`.

Add:

```rust
pub fn overlay_data_at(&self, rx: u16, ry: u16) -> u8 {
    self.overlay_data.byte_at(rx, ry)
}

pub fn has_overlay_data_pack(&self) -> bool {
    self.overlay_data.is_present()
}
```

**Step 4: Add tests**

Add tests in `src/map/overlay.rs` that build a tiny fake decoded data vector
through `OverlayDataPack::from_decoded`:

- `overlay_data_pack_returns_byte_for_empty_overlay_cell`
- `overlay_data_pack_returns_zero_out_of_range`
- `overlay_data_pack_missing_reports_absent_and_returns_zero`
- `parse_overlays_still_returns_entries_for_existing_callers`

**Step 5: Verify**

Run:

```powershell
cargo test map::overlay -- --nocapture
```

Expected: overlay parser tests pass.

Do not commit unless the user explicitly asks for commits.

### Task 2: Add Raw Bridge Fact Types And Stamp Helper

**Why:** The rest of the implementation needs a pure, testable representation
of binary bridge bits before resolved terrain starts consuming it.

**Files:**
- Create: `src/map/bridge_facts.rs`
- Modify: `src/map/mod.rs`

**Pattern:** Use small map-owned data helpers like `src/map/overlay_types.rs`;
do not import from `sim/`.

**Step 1: Export the module**

```rust
// src/map/mod.rs
pub mod bridge_facts;
```

**Step 2: Define constants and types**

```rust
// src/map/bridge_facts.rs
//! Authoritative bridge cell facts stamped from map overlay data.

pub const BRIDGE_FLAG_ANCHOR_SELF: u32 = 0x80;
pub const BRIDGE_FLAG_STRUCTURAL: u32 = 0x100;
pub const BRIDGE_FLAG_TRANSITION: u32 = 0x200;
pub const BRIDGE_FLAG_DESTROYED_OR_RAMP: u32 = 0x400;
pub const BRIDGE_FLAG_DIRECTION_ZERO: u32 = 0x800;
pub const BRIDGE_FLAG_FORWARD_SIDE: u32 = 0x1000;
pub const BRIDGE_FLAG_EXTRA_SIDE: u32 = 0x10000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BridgeStampFamily {
    #[default]
    None,
    Nesw,
    Nwse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeStampSlot {
    Anchor,
    Forward1,
    Forward2,
    Forward3,
    Opposite,
    ExtraDir6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeAnchorRelation {
    pub anchor: (u16, u16),
    pub slot: BridgeStampSlot,
    pub family: BridgeStampFamily,
    pub direction: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BridgeCellFacts {
    pub raw_flags: u32,
    pub state_byte: u8,
    pub overlay_id: Option<u8>,
    pub family: BridgeStampFamily,
    pub direction: Option<u8>,
    pub anchor: Option<BridgeAnchorRelation>,
}
```

**Step 3: Add overlay classifier**

```rust
pub fn high_bridge_stamp_for_overlay(id: u8) -> Option<(BridgeStampFamily, u8)> {
    match id {
        0x18 => Some((BridgeStampFamily::Nesw, 0)),
        0x19 => Some((BridgeStampFamily::Nesw, 6)),
        0xED => Some((BridgeStampFamily::Nwse, 0)),
        0xEE => Some((BridgeStampFamily::Nwse, 6)),
        _ => None,
    }
}
```

**Step 4: Add pure stamping**

Implement:

```rust
pub fn stamp_set_bridge_direction(
    cells: &mut [BridgeCellFacts],
    width: u16,
    height: u16,
    anchor: (u16, u16),
    family: BridgeStampFamily,
    direction: u8,
    set: bool,
) {
    // Compute the same slots as the audited table.
    // Apply masks and OR bits exactly per slot.
    // Write state_byte for slots where SetBridgeDirection writes +0x11E.
    // Bounds-check every target cell before writing.
}
```

Use helper functions:

```rust
fn index(width: u16, height: u16, rx: u16, ry: u16) -> Option<usize>;
fn step(cell: (u16, u16), direction: u8) -> Option<(u16, u16)>;
fn opposite_direction(direction: u8) -> u8;
```

`step` must use integer offsets for direction indices 0, 2, 4, and 6. For other
indices, implement the standard eight-direction table already used in
`sim::bridge_state::Direction`.

**Step 5: Add unit tests for the audited table**

Add tests:

- `stamp_dir0_intact_sets_anchor_north_slots_and_south_opposite`
- `stamp_dir6_intact_sets_west_slots_and_two_east_slots`
- `stamp_intact_writes_default_state_bytes_before_overlay_data_overwrite`
- `stamp_intact_sets_0x80_only_on_anchor`
- `stamp_destroy_emits_destroy_flags_only_on_anchor_forward1_forward2_opposite`
- `high_bridge_stamp_classifier_ignores_low_bridge_ids`

**Step 6: Verify**

Run:

```powershell
cargo test bridge_facts -- --nocapture
```

Expected: all bridge fact unit tests pass.

Do not commit unless the user explicitly asks for commits.

### Task 3: Split High-Bridge Overlay Helpers From Broad Bridge Overlay Helpers

**Why:** Existing `is_bridge_overlay_index` includes low bridge overlays. High
bridge fact stamping must use only the four verified high-bridge anchor IDs.

**Files:**
- Modify: `src/map/overlay_types.rs`

**Pattern:** Keep existing broad helper for render/legacy consumers; add a
narrow helper for authoritative facts.

**Step 1: Add narrow helpers**

```rust
pub fn is_high_bridge_anchor_overlay_index(id: u8) -> bool {
    crate::map::bridge_facts::high_bridge_stamp_for_overlay(id).is_some()
}

pub fn high_bridge_stamp_direction(id: u8) -> Option<u8> {
    crate::map::bridge_facts::high_bridge_stamp_for_overlay(id).map(|(_, dir)| dir)
}
```

**Step 2: Leave broad helper in place**

Do not delete `is_bridge_overlay_index` in this task. Existing render and
overlay-atlas consumers may still depend on broad bridge artwork detection.

**Step 3: Update tests**

Add assertions:

- `0x18`, `0x19`, `0xED`, `0xEE` are high bridge anchors.
- Low bridge IDs such as `0x4A`, `0x7A`, `0xCD`, and `0xE9` are not high bridge anchors.
- `is_bridge_overlay_index` still returns true for existing broad bridge IDs.

**Step 4: Verify**

Run:

```powershell
cargo test overlay_types -- --nocapture
```

Expected: overlay type tests pass.

Do not commit unless the user explicitly asks for commits.

### Task 4: Add BridgeCellFacts To ResolvedTerrainCell

**Why:** Resolved terrain is the map data product consumed by sim and render.
Raw bridge facts need to travel with the cell before compatibility fields are
rewired.

**Files:**
- Modify: `src/map/resolved_terrain.rs`
- Modify: every `ResolvedTerrainCell` test fixture that fails to compile

**Pattern:** Follow existing optional metadata fields such as
`bridge_layer` and `bridgehead_anchor_class_at_load`.

**Step 1: Import and add the field**

```rust
use crate::map::bridge_facts::BridgeCellFacts;

pub struct ResolvedTerrainCell {
    pub bridge_facts: BridgeCellFacts,
    pub bridgehead_anchor_class_at_load: Option<crate::sim::bridge_state::BridgeheadAnchorClass>,
}
```

Place `bridge_facts` next to the existing bridge fields, immediately before
`bridgehead_anchor_class_at_load`.

**Step 2: Initialize default facts**

In the main `ResolvedTerrainGrid::build` cell push, set:

```rust
bridge_facts: BridgeCellFacts::default(),
```

In all test fixtures and helper constructors, add the same default value.

**Step 3: Add accessors**

Add to `ResolvedTerrainCell`:

```rust
pub fn bridge_flags(&self) -> u32 {
    self.bridge_facts.raw_flags
}
```

Do not change existing `has_bridge_deck`, `bridge_walkable`, or
`bridge_transition` derivation in this task.

**Step 4: Verify**

Run:

```powershell
cargo test resolved_terrain -- --nocapture
```

Expected: compile succeeds and existing resolved terrain tests pass after fixture updates.

Do not commit unless the user explicitly asks for commits.

### Task 5: Stamp High-Bridge Facts During Resolved Terrain Build

**Why:** This creates the authoritative high-bridge cell facts from the map's
overlay entries in the same order as `[OverlayPack]`.

**Files:**
- Modify: `src/map/resolved_terrain.rs`

**Pattern:** Build a side table during `ResolvedTerrainGrid::build`, then copy
facts into cells after the base cell vector exists. This avoids borrow
conflicts and keeps the stamp pure.

**Step 1: Build a fact side table**

After the base `cells` vector is created and before compatibility bridge passes
run, create:

```rust
let mut bridge_facts = vec![BridgeCellFacts::default(); cells.len()];
```

For each `map.overlays` entry in existing order:

```rust
if let Some((family, direction)) =
    crate::map::bridge_facts::high_bridge_stamp_for_overlay(overlay.overlay_id)
{
    crate::map::bridge_facts::stamp_set_bridge_direction(
        &mut bridge_facts,
        width,
        height,
        (overlay.rx, overlay.ry),
        family,
        direction,
        true,
    );
}
```

**Step 2: Preserve overlay IDs on overlay-bearing cells**

For every overlay entry whose coordinate is in the resolved terrain bounds:

```rust
bridge_facts[idx].overlay_id = Some(overlay.overlay_id);
```

Only the four high-bridge IDs drive stamping. Populate `overlay_id` for
overlay-bearing cells so diagnostics can report the source overlay, but do not
stamp high-bridge flags for any other overlay IDs.

**Step 3: Apply final state bytes from OverlayDataPack when present**

The pure stamping helper must already have written SetBridgeDirection's default
state bytes to anchor, forward 1, forward 2, and opposite slots. Then, only when
the map contained `[OverlayDataPack]`, overwrite every resolved terrain cell:

```rust
if map.has_overlay_data_pack() {
    bridge_facts[idx].state_byte = map.overlay_data_at(cells[idx].rx, cells[idx].ry);
}
```

This must run after stamping, matching the binary's conditional
`[OverlayDataPack]` overwrite. If `[OverlayDataPack]` is absent, keep the
state bytes produced by `stamp_set_bridge_direction`.

**Step 4: Copy side-table facts into cells**

```rust
for (cell, facts) in cells.iter_mut().zip(bridge_facts) {
    cell.bridge_facts = facts;
}
```

**Step 5: Add resolved terrain tests**

Add tests in `resolved_terrain.rs`:

- A single `0x18` overlay at `(5,5)` stamps anchor, north 1, north 2,
  north 3, and south 1 only.
- A single `0x19` overlay at `(5,5)` stamps anchor, west 1, west 2,
  west 3, east 1, and east 2 only.
- A non-overlay stamped neighbor receives its `state_byte` from the data pack,
  not from the overlay entry frame.
- A map with no `[OverlayDataPack]` keeps SetBridgeDirection default state bytes
  on stamped cells.
- Low bridge overlay IDs do not set high-bridge raw flags.

**Step 6: Verify**

Run:

```powershell
cargo test resolved_terrain -- --nocapture
cargo test bridge_facts -- --nocapture
```

Expected: new stamping tests pass.

Do not commit unless the user explicitly asks for commits.

### Task 6: Derive High-Bridge Compatibility Fields From Bridge Facts

**Why:** Current consumers still read flattened fields. This task changes their
source to stamped facts without rewriting traversal.

**Files:**
- Modify: `src/map/resolved_terrain.rs`

**Pattern:** Keep compatibility fields, but make their high-bridge values a view
over `bridge_facts`.

**Step 1: Add compatibility helper methods**

In `bridge_facts.rs`, add:

```rust
impl BridgeCellFacts {
    pub fn has_flag(self, flag: u32) -> bool {
        self.raw_flags & flag != 0
    }

    pub fn has_structural_bridge(self) -> bool {
        self.has_flag(BRIDGE_FLAG_STRUCTURAL)
    }

    pub fn has_transition_flag(self) -> bool {
        self.has_flag(BRIDGE_FLAG_TRANSITION)
    }

    pub fn is_anchor_self(self) -> bool {
        self.has_flag(BRIDGE_FLAG_ANCHOR_SELF)
    }
}
```

**Step 2: Replace high-bridge deck derivation**

After facts are copied into cells, derive high-bridge compatibility fields:

```rust
let structural = cell.bridge_facts.has_structural_bridge();
if structural {
    cell.has_bridge_deck = true;
    cell.bridge_walkable = !cell.terrain_object_blocks && !cell.overlay_blocks;
    cell.bridge_deck_level = cell.level.saturating_add(4);
}
```

Use `level.saturating_add(4)` only as the current compatibility deck-height
view. Do not normalize across components.

**Step 3: Replace high-bridge transition derivation**

For high-bridge facts, set:

```rust
if cell.bridge_facts.has_transition_flag() {
    cell.bridge_transition = true;
}
```

Do not use broad `BridgeSet/WoodBridgeSet` membership as an authoritative
bridge-transition fact in this task.

**Step 4: Preserve render metadata on overlay-bearing cells**

Keep `bridge_layer` only for actual high-bridge overlay cells that need render
art metadata. Do not copy `bridge_layer` to stamped neighbor cells just to make
old grouping work.

**Step 5: Verify with tests**

Add tests:

- Stamped side/gap cells without `0x100` do not get `has_bridge_deck`.
- Forward 3 gets `0x1000` but not `has_bridge_deck`.
- Forward 2 has `has_bridge_deck` but not `0x200`.
- Opposite has `bridge_transition` from `0x200` and lacks `0x1000`.

**Step 6: Verify**

Run:

```powershell
cargo test resolved_terrain -- --nocapture
```

Expected: high-bridge compatibility tests pass.

Do not commit unless the user explicitly asks for commits.

### Task 7: Remove High-Bridge Side Expansion, Deck Normalization, And Gap Fill

**Why:** These passes invent bridge deck cells and heights that the audited
binary path does not create.

**Files:**
- Modify: `src/map/resolved_terrain.rs`

**Pattern:** Delete or gate only the unverified high-bridge inference passes.
Leave unrelated terrain, LAT, smudge, and low-bridge rendering behavior alone.

**Step 1: Remove side-cell expansion**

Remove the block that builds `side_cells` and logs a message containing both:

```text
ResolvedTerrain: extrapolated
high bridge side cells
```

Do not replace it with a different side expansion.

**Step 2: Remove high-bridge connected-component deck normalization**

Remove the BFS block that logs a message containing:

```text
ResolvedTerrain: normalized deck_level
```

Do not normalize stamped bridge levels.

**Step 3: Remove high-bridge gap fill**

Remove the block that builds `gap_fills` and logs a message containing:

```text
ResolvedTerrain: filled
bridge deck gaps
```

Do not fill cells between stamped bridge facts.

**Step 4: Remove diagnostic labels that assume center vs side inference**

Adjust the high-deck diagnostic block so it reports stamped fact flags instead
of `"center"` and `"side"`.

**Step 5: Verify**

Run:

```powershell
cargo test resolved_terrain -- --nocapture
```

Expected: tests assert no side expansion, no deck normalization, and no gap fill.

Do not commit unless the user explicitly asks for commits.

### Task 8: Rewire BridgeRuntimeState Anchor Spans To Use Bridge Facts

**Why:** Once resolved terrain is fact-stamped, runtime spans should use
fact-stamped anchor relation and direction instead of broad `bridge_layer`
heuristics.

**Files:**
- Modify: `src/sim/bridge_state/mod.rs`

**Pattern:** Keep `BridgeRuntimeState::from_resolved_terrain` as the integration
point. Do not move map parsing into sim.

**Step 1: Group bridge cells from structural facts**

In pass 1, replace `resolved.has_bridge_deck` as the source of high-bridge
runtime cells with:

```rust
let structural = resolved.bridge_facts.has_structural_bridge();
```

Keep current `has_bridge_deck` compatibility if tests still need it, but
prefer the raw fact when present.

**Step 2: Build anchor spans from anchor facts**

In pass 2, identify anchors with:

```rust
resolved.bridge_facts.is_anchor_self()
```

Use `resolved.bridge_facts.direction` for the span direction. Use
`resolved.bridge_facts.family` only for metadata and future render/debug
selection; the audited helper bodies are equivalent.

**Step 3: Tag roles from stamped structural slots**

When visiting a runtime cell that already exists in `cells`, use
`resolved.bridge_facts.anchor.map(|r| r.slot)` to set:

- `Anchor` for `BridgeStampSlot::Anchor`
- `Body` for `Forward1` and `Forward2`
- `Tail` for `Opposite`

Do not create or mark runtime bridge cells solely for `Forward3` or
`ExtraDir6`. Those slots are flag-only in the audited helper and must remain in
`AnchorSpan.cells` for slot alignment and `set_bridge_direction` actions, but
they are not structural deck cells by themselves.

If a stamped structural cell lacks anchor relation because of out-of-bounds or
legacy fixture data, keep the existing defensive `Body` fallback.

**Step 4: Preserve overlay byte from raw facts**

Initialize `BridgeRuntimeCell.overlay_byte` from:

```rust
resolved.bridge_facts.overlay_id.unwrap_or(0)
```

For compatibility with existing render tests, keep the old `bridge_layer`
fallback only when `bridge_facts.overlay_id` is `None`.

**Step 5: Update bridge runtime tests**

Add or update tests:

- `from_resolved_terrain_builds_dir0_anchor_span_from_bridge_facts`
- `from_resolved_terrain_builds_dir6_extra_slot_from_bridge_facts`
- `from_resolved_terrain_ignores_low_bridge_overlay_for_high_span`
- Existing blow-up slot tests still pass with slots `0,1,2,4`.

**Step 6: Verify**

Run:

```powershell
cargo test bridge_state -- --nocapture
```

Expected: bridge runtime tests pass.

Do not commit unless the user explicitly asks for commits.

### Task 9: Keep Runtime SetBridgeDirection Slot Table Aligned

**Why:** `src/sim/bridge_specs.rs` already has a runtime
`set_bridge_direction` helper. It should not drift from the new map-load stamp
slot model.

**Files:**
- Modify: `src/sim/bridge_specs.rs`
- Modify: `src/map/bridge_facts.rs` only if sharing slot constants is clean

**Pattern:** Avoid a dependency from `map/` to `sim/`. Sharing from `map/` to
`sim/` is acceptable because `map/` is lower-level data and `sim/` already
depends on `map/`.

**Step 1: Compare slot ordering**

Verify that `AnchorSpan.cells` still uses:

```text
0 anchor
1 forward 1
2 forward 2
3 forward 3
4 opposite
5 extra dir 6
```

**Step 2: Add a test comparing map stamp slots and runtime slots**

Build a small fact-stamped terrain, construct `BridgeRuntimeState`, and call
runtime `set_bridge_direction` on the span. Assert runtime action cells match
`BridgeCellFacts` destroy-table blow-up slots `0,1,2,4`.

**Step 3: Do not change collapse/repair semantics**

This task is alignment only. If the runtime helper has broader damage/repair
gaps, leave them to the existing bridge damage/repair priorities.

**Step 4: Verify**

Run:

```powershell
cargo test bridge_specs -- --nocapture
cargo test bridge_state -- --nocapture
```

Expected: runtime and map-load slot tests pass.

Do not commit unless the user explicitly asks for commits.

### Task 10: Adjust Pathfinding And Movement Tests For Corrected Facts

**Why:** Priority 1 is not the high-bridge traversal rewrite, but corrected
facts may break tests that assumed side expansion or broad bridgehead marking.

**Files:**
- Modify only tests in `src/sim/pathfinding/core_tests.rs`,
  `src/sim/movement/movement_bridge.rs`, and affected bridge tests.

**Pattern:** Keep production traversal logic unchanged unless compilation
requires an accessor rename. Priority 3 owns traversal rule replacement.

**Step 1: Run targeted tests**

Run:

```powershell
cargo test bridge -- --nocapture
cargo test pathfinding -- --nocapture
cargo test movement_bridge -- --nocapture
```

**Step 2: Reclassify failing expectations**

For each failure:

- If the expected cell exists only because of side expansion, update the test to
  expect no high-bridge deck fact.
- If the expected deck level exists only because of component normalization,
  update the test to the stamped cell's own `level + 4` compatibility value.
- If the failure is about exact bridgehead/ramp traversal, mark the test as a
  Priority 2 or Priority 3 test and do not paper over it with broad inference.

**Step 3: Keep production traversal unchanged**

Do not rewrite `compute_neighbor_height`, `CheckBridgeTraversal` analogs, or
`movement_bridge` predicates in this task.

**Step 4: Verify**

Run:

```powershell
cargo test bridge -- --nocapture
cargo test pathfinding -- --nocapture
cargo test movement_bridge -- --nocapture
```

Expected: tests either pass or remaining failures are explicitly attributable to
Priority 2/3 and documented for review before implementation continues.

Do not commit unless the user explicitly asks for commits.

### Task 11: Run Full Verification

**Why:** This change touches map initialization data that many systems consume.

**Files:** No planned file edits unless a compile failure identifies a missed
fixture or import.

**Pattern:** Use project test suites; do not start a dev server.

**Step 1: Format**

Run:

```powershell
cargo fmt
```

**Step 2: Run focused tests**

Run:

```powershell
cargo test bridge_facts -- --nocapture
cargo test resolved_terrain -- --nocapture
cargo test bridge_state -- --nocapture
cargo test bridge_specs -- --nocapture
```

Expected: all focused tests pass.

**Step 3: Run broader tests**

Run:

```powershell
cargo test
```

Expected: pass, except for unrelated pre-existing failures. If unrelated
failures exist, record their names and confirm they do not depend on the edited
bridge/map files.

**Step 4: Inspect logs**

Load a map with high bridges and confirm logs no longer report:

- a log containing both `ResolvedTerrain: extrapolated` and `high bridge side cells`
- a log containing `ResolvedTerrain: normalized deck_level`
- a log containing both `ResolvedTerrain: filled` and `bridge deck gaps`

Expected: high bridge facts are produced by stamping tests, not by those logs.

Do not commit unless the user explicitly asks for commits.

### Task 12: Optional Data Validation Dump

**Why:** The binary evidence is sufficient for implementation, but a stock-map
dump helps catch mismatches between map overlay cells and expected stamped cells.

**Files:**
- Create a temporary debug-only test or local diagnostic under `docs/traces/` if
  the user asks to keep the result.

**Pattern:** This is validation output, not production behavior.

**Step 1: Add a temporary diagnostic**

Print for each high-bridge overlay entry:

```text
anchor=(rx,ry) overlay_id=0xNN direction=D stamped=[(rx,ry,flags,state)]
```

Include each stamped cell's raw flags and final state byte.

**Step 2: Run on one stock high-bridge map**

Use a map from the configured retail RA2/YR install.

**Step 3: Compare with expectations**

Expected:

- only the audited slot cells have high-bridge stamp facts;
- stamped cells receive final state bytes from `[OverlayDataPack]`;
- no side-cell or gap-fill facts appear unless the map itself has overlay
  anchors that stamp those cells.

**Step 4: Remove the temporary diagnostic unless the user asks to keep it**

Do not leave ad hoc debug output in production.

Do not commit unless the user explicitly asks for commits.

---

## Sources & References

- **Priority list:** `docs/plans/2026-05-15-bridge-parity-fix-priority-list.md`
- **Primary audited report:** `docs/research/BRIDGE_SETBRIDGEDIRECTION_STAMPING_GHIDRA_REPORT.md`
- **Parent report:** `docs/research/BRIDGE_MAP_LOAD_AND_BRIDGEHEAD_TRANSITIONS_GHIDRA_REPORT.md`
- **Low-bridge supplement:** `docs/research/BRIDGE_LOW_AND_ZONE_RECORDS_GHIDRA_SUPPLEMENT.md`
- **Related rendering report:** `docs/research/BRIDGE_RENDERING_GHIDRA_REPORT.md`
- **Related damage report:** `docs/research/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`
- **gamemd.exe addresses:** `CellClass__SetBridgeDirection_NESW @ 0x0047E040`, `CellClass__SetBridgeDirection_NWSE @ 0x0047E470`, `OverlayClass__Mark @ 0x005FC570`, `ReadMapOverlayPacks @ 0x005FD2E0`, `CellClass__RecalcAttributes @ 0x0047D2B0`, `MapClass__Resize @ 0x00565C10`, `MapClass__UpdateBridgeEdgeTiles_High @ 0x00576200`
- **INI keys:** `rulesmd.ini [OverlayTypes] 25=BRIDGE1`, `26=BRIDGE2`, `241=BRIDGEB1`, `242=BRIDGEB2`; same base entries in `rules.ini`
- **Current Rust:** `src/map/overlay.rs`, `src/map/map_file.rs`, `src/map/overlay_types.rs`, `src/map/resolved_terrain.rs`, `src/sim/bridge_state/mod.rs`, `src/sim/bridge_specs.rs`, `src/sim/pathfinding/core.rs`, `src/sim/movement/movement_bridge.rs`

---

## Review Notes

Before implementation, run `/review-plan` against this plan. The main review
questions are:

- Does adding a full `OverlayDataPack` byte grid to `MapFile` fit current map
  ownership conventions?
- Should `BridgeCellFacts` live in `src/map/bridge_facts.rs` or inside
  `resolved_terrain.rs` until more consumers use it?
- Should Priority 1 derive `bridge_transition` from `0x200` immediately, or
  leave transition compatibility untouched until Priority 2 exact ramp detection?
- Which existing tests encode side expansion/gap fill as desired behavior and
  need explicit parity correction?
