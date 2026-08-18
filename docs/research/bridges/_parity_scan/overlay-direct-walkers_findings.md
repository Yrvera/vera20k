# Overlay-Direct Destroy Walkers — Parity Scan Findings

**Facet:** overlay-direct-walkers — Overlay-direct destroy walkers (NS/EW × High/Low) + sibling cascade.
**Rust:** `src/sim/bridge_state/walker.rs` (`destroy_bridge_high/low`, `destroy_bridge_walker_*`,
`find_walker_start_*`, `apply_bridge_destruction_*`, `check_bridge_neighbors_*`), plus the
destruction-overlay tables in `src/sim/bridge_specs.rs` and the dispatch gate in
`src/sim/bridge_state/mod.rs::path_matches_cell` / `src/sim/world/bridge_orchestrator.rs`.

**gamemd anchors verified live this session (all addresses re-confirmed via decompile):**
- `ApplyDamageToCell @ 0x00587180` (dispatch gate)
- `DestroyBridge_High @ 0x0057ccf0`, `DestroyBridge_Low @ 0x0057baa0`
- `DestroyBridgeWalker_NS_High @ 0x0057cf60`, `EW_High @ 0x0057d530`, `NS_Low @ 0x0057bcf0`, `EW_Low @ 0x0057c2b0`
- `ApplyBridgeDestruction_NS_High @ 0x0057e7a0`, `NS_Low @ 0x0057dd50`, `EW_Low @ 0x0057e2a0`
- `CheckBridgeNeighbors_EW_High @ 0x0057cab0`, `NS_High @ 0x0057cbe0`, `EW_Low @ 0x0057b870`

---

### D1: Walker suppresses the overlay write on Bridgehead-role cells; binary writes unconditionally
- **Rust now:** Every walker body (HIGH/LOW × NS/EW) and every `apply_bridge_destruction_*` leaf
  skips a cell when `c.role == BridgeCellRole::Bridgehead` via
  `if matches!(c.role, ...Bridgehead) { continue; }` before writing `overlay_byte` /
  `damage_state`. See `walker.rs:879-881` (NS_High), `:972-974` (EW_High), `:1236-1238`
  (NS_Low), `:1325-1327` (EW_Low), and the cascade leaves `:753-755`, `:806-808`, `:1118-1120`,
  `:1170-1172`.
- **gamemd:** `DestroyBridgeWalker_NS_High @ 0x0057cf60` writes the 3-cell length-axis triple
  unconditionally — `local_a0->OverlayTypeIndex = X; local_a4->OverlayTypeIndex = X;
  this->OverlayTypeIndex = X;` — with **no role / bridgehead / anchor-flag guard** of any kind.
  Same in all four walkers and in `ApplyBridgeDestruction_NS_High @ 0x0057e7a0`
  (`local_c4/b8/cc->OverlayTypeIndex = iVar2` with no role check). The only gate is the overlay
  band test (`0xCD..=0xE8` outer, then the per-case overlay value).
- **Fixture:** HIGH NS bridge, impact cell `(10,5)` overlay `0xD4` (final-stage). Triple is
  `(10,5),(10,4),(10,6)`. Suppose `(10,4)` is the bridgehead deck cell (role=Bridgehead). gamemd
  writes `0xE7` to all three and marks all three `RadarClass::MarkTerrainDirty`. Rust writes `0xE7`
  to `(10,5)` and `(10,6)` only, leaves `(10,4)` at `0xD4`, and does NOT push `(10,4)` into
  `destroyed`. Observable: one bridge deck cell (the bridgehead) stays standing/half-damaged after
  a collapse that gamemd renders fully destroyed, and no `BlowUpBridge` fires on it.
- **Player sees:** A stray un-collapsed bridge tile next to the bridgehead after destroying a high
  bridge whose final-stage triple overlaps a bridgehead cell. Triggers any time a walker's 3-cell
  triple includes a cell the Rust port tagged Bridgehead — common at the ends of every bridge span.
- **Severity:** MED
- **Confidence:** PROVEN-DRIFT
- **Verify-call:** `decompile_function 0x0057cf60` (no role guard; unconditional 3-cell write) and
  `decompile_function 0x0057e7a0`.

---

### D2: `find_walker_start_*` uses `saturating_add(1)` for the south/east start; binary uses raw `+1` (16-bit short)
- **Rust now:** `find_walker_start_high_ns` returns `(rx, ry.saturating_add(1))` on the
  "north neighbor off" branch (`walker.rs:518`); same `saturating_add` in `_high_ew:540`,
  `_low_ns:560`, `_low_ew:582`.
- **gamemd:** `DestroyBridge_High @ 0x0057ccf0` computes the forward start as
  `CONCAT22(psVar1[1] + 1, *psVar1)` — a raw 16-bit short add with wrap, no saturation.
- **Fixture:** Boundary only: `ry == 0xFFFF`. Rust saturates to `0xFFFF` (start = `(rx,0xFFFF)`);
  binary wraps `0xFFFF + 1 = 0x0000` (start = `(rx,0)`), then Get_CellClass routes the off-map
  sentinel. For all `ry < 0xFFFF` the two agree exactly.
- **Player sees:** Nothing in normal play — a bridge body cell at map-row 65535 cannot exist on any
  retail/WAE map (map cells max out well below 0xFFFF). Boundary divergence only.
- **Severity:** LOW
- **Confidence:** PROVEN-DRIFT (unreachable on real maps; surfaced per "no disparity too small").
- **Verify-call:** `decompile_function 0x0057ccf0` (`CONCAT22(psVar1[1] + 1, *psVar1)`).

---

### D3: Cascade-leaf de-dup of final cells (`destroyed.contains`) has no binary analog; binary re-writes
- **Rust now:** After a walker's triple write, the sibling cascade pushes leaf finals into
  `destroyed`/`actions` only `if !destroyed.contains(&pos)` (`walker.rs:901-905`, `:992-996`,
  `:1256-1260`, `:1344-1348`).
- **gamemd:** `DestroyBridgeWalker_NS_High` calls `ApplyBridgeDestruction_NS_High` for each sibling
  column independently; that leaf writes `OverlayTypeIndex` and (for `iVar2 == 0xe7`) calls
  `RadarClass::MarkTerrainDirty` 3× and `CellClass::RecalcAttributes` 3× with **no cross-call
  de-dup**. There is no shared "already destroyed" set across calls.
- **Fixture:** A cell that is in both the impact triple and a sibling-cascade triple. gamemd would
  re-issue MarkTerrainDirty/RecalcAttributes for it twice; Rust collapses it to a single
  `BlowUpBridge` action. The final overlay byte ends identical (`0xE7` both times). Because the
  Rust `actions`/`destroyed` set feeds `BlowUpBridge` (a one-shot kill/limbo of a cell), the
  de-dup is benign for the **observable end-state** (a cell can only be blown up once) — but the
  ordering/count of per-cell dirty events differs from the binary.
- **Player sees:** No observable difference in the final bridge state. Potential one-frame
  redraw-ordering difference only; not gameplay. Surfaced for completeness.
- **Severity:** LOW
- **Confidence:** LIKELY-DRIFT (end-state identical; per-cell dirty event multiplicity differs,
  not proven observable).
- **Verify-call:** `decompile_function 0x0057e7a0` (per-call MarkTerrainDirty/RecalcAttributes,
  no de-dup).

---

### D4: Walker collapses RadarClass::MarkTerrainDirty + DirtyScreenRect + RecalcAttributes into outcome flags; per-cell radar-dirty fidelity unverified for the direct path
- **Rust now:** The HIGH/LOW walker final-collapse path sets `zones_dirty: is_final` and packages
  destroyed cells, but the direct walker (`destroy_bridge_walker_ns_high` etc.) does **not** emit a
  per-cell radar-dirty list — only the `apply_repair_to_strip_cell` repair path populates
  `outcome.radar_cells`. The destruction walker returns `StateOutcome::Collapsed { ... }` with
  `destroyed_cells` but the orchestrator's radar-dirty handling for direct collapse is out of this
  file's scope (`bridge_orchestrator.rs`).
- **gamemd:** The final-collapse branch of `DestroyBridgeWalker_NS_High` calls
  `RadarClass::MarkTerrainDirty` on each of the 3 triple cells (`&local_a0->MapCoord_X`,
  `&pCVar4->MapCoord_X`, `&this->MapCoord_X`), then `TacticalClass::DirtyScreenRect`,
  `CellClass::RecalcAttributes` ×3, and `UpdateBridgeZonesHelper()` at the tail. The cascade leaf
  additionally marks the sibling cell + its two perpendicular neighbors dirty when it hits
  `0xe7`/`0x65`.
- **Fixture:** Final HIGH NS collapse at `(10,5)`: gamemd dirties radar for `(10,4),(10,5),(10,6)`
  plus, in each sibling leaf reaching `0xE7`, the sibling cell and its N/S neighbors. The Rust
  destruction walker hands back only `destroyed_cells`; whether the orchestrator re-derives the
  exact same radar-dirty set (including the cascade-leaf neighbors) is not established in this file.
- **Player sees:** Possible minimap (radar) tile not refreshing on the exact same cells/frame as
  gamemd after a high/low bridge collapse. Frequency: every direct collapse. Needs an orchestrator-
  side check to confirm whether this is a true drift or already covered.
- **Severity:** MED
- **Confidence:** UNCHECKED (orchestrator radar handling is outside the assigned file; the walker
  itself does not reproduce the per-cell MarkTerrainDirty list).
- **Verify-call:** `decompile_function 0x0057cf60` (3× MarkTerrainDirty + DirtyScreenRect in the
  final branch).

---

## PARITY-CONFIRMED

These sub-aspects were checked live against the binary and match exactly:

1. **Dispatch entry gate (`path_matches_cell`).** Binary `ApplyDamageToCell @ 0x587180` routes
   `(0x49 < ov) && (ov < 100)` → `DestroyBridge_Low` = `[0x4A..=0x63]`, and
   `(0xcc < ov) && (ov < 0xe7)` → `DestroyBridge_High` = `[0xCD..=0xE6]`. Rust
   `path_matches_cell` uses exactly `(0x4A..=0x63)` (LowDirect) and `(0xCD..=0xE6)` (HighDirect)
   — `mod.rs:848-849`. The final-anchor overlays `0x64,0x65,0xE7,0xE8` correctly do NOT route to
   the direct walker from the damage dispatcher. **Match.** (Verify: `decompile_function 0x587180`.)

2. **Axis subrange classification in `DestroyBridge_High/Low`.** Binary NS-class HIGH =
   `[0xCD..=0xD5] ∪ [0xDF..=0xE2] ∪ {0xE7}`; EW-class HIGH = `[0xD6..=0xDE] ∪ [0xE3..=0xE6] ∪
   {0xE8}`. Rust `is_ns_walker_overlay_high`/`is_ew_walker_overlay_high` match byte-for-byte
   (`walker.rs:597-607`). LOW NS = `[0x4A..=0x52] ∪ [0x5C..=0x5F] ∪ {0x64}`; LOW EW =
   `[0x53..=0x5B] ∪ [0x60..=0x63] ∪ {0x65}` — `walker.rs:609-619` match the binary
   (`0x49 < ov < 0x53`, `0x5b < ov < 0x60`, `==100` etc.). **Match.**

3. **`find_walker_start_*` 3-case pre-shift.** Binary: probe neighbor-1 behind (north/west); if
   off-band → start = +1 forward; else probe neighbor-2 behind; if in-band → start = -1 back; else
   unshifted. Rust `find_walker_start_high_ns/ew/low_ns/low_ew` reproduce the exact 3-case logic
   with the `0xCD..=0xE8` / `0x4A..=0x65` in-band test, including the `ry==0`/`rx==0` and
   `ry>=2`/`rx>=2` boundary handling (`walker.rs:509-593`). **Match** (modulo D2's saturating-add).

4. **Walker case values + transition overlays (all 4 walkers).**
   - HIGH NS: `0xDF→0xE0`, `0xE1→0xE2`, `<0xD3→0xD3`, `0xD3..0xD5→0xE7` (final), `>0xD5→noop`.
     Rust `destroy_bridge_walker_ns_high:852-870` matches `0x0057cf60` exactly.
   - HIGH EW: `0xE3→0xE4`, `0xE5→0xE6`, `<0xDC→0xDC`, `0xDC..0xDE→0xE8` (final). Rust `:946-964`
     matches `0x0057d530`.
   - LOW NS: `0x5C→0x5D`, `0x5E→0x5F`, `<0x50→0x50`, `0x50..0x52→0x64` (final). Rust `:1210-1228`
     matches `0x0057bcf0`.
   - LOW EW: `0x60→0x61`, `0x62→0x63`, `<0x59→0x59`, `0x59..0x5B→0x65` (final). Rust `:1299-1317`
     matches `0x0057c2b0`.
   All four checked. **Match.**

5. **Sibling-cascade dispatch coordinates (perpendicular direction).** Binary intermediate cases
   dispatch ApplyBridgeDestruction at ONE perpendicular sibling; healthy/final cases at BOTH.
   - HIGH NS `0xDF`→ west `(x-1,y)`, `0xE1`→ east `(x+1,y)`, healthy/final → both. Rust `:852-867`. Match.
   - HIGH EW `0xE3`→ south `(x,y+1)`, `0xE5`→ north `(x,y-1)`, healthy/final → both. Rust `:946-961`. Match.
   - LOW NS `0x5C`→ west, `0x5E`→ east. Rust `:1210-1224`. Match.
   - LOW EW `0x60`→ south `(x,y+1)`, `0x62`→ north `(x,y-1)`. Rust `:1299-1313`. Match. (Verified
     against `CONCAT22(param_1[1]±1, *param_1)` in `0x0057c2b0`.)

6. **`pick_destruction_overlay` tables (all 4) vs binary `local_70[16]`.** Read live from the four
   `ApplyBridgeDestruction_*` functions:
   - HIGH NS `0x0057e7a0`: `[-1,0xD2,0xD5,-1,0xD1,0xD3,0xD5,-1,0xD4,0xD4,0xE7,-1×5]`. Rust
     `DESTRUCTION_OVERLAY_HIGH_NS` = `[FF,D2,D5,FF,D1,D3,D5,FF,D4,D4,E7,FF×5]`. Match.
   - LOW NS `0x0057dd50`: `[-1,0x4F,0x52,-1,0x4E,0x50,0x52,-1,0x51,0x51,0x64,-1×5]`. Rust
     `DESTRUCTION_OVERLAY_LOW_NS` matches. Match.
   - LOW EW `0x0057e2a0`: `[-1,0x58,0x5B,-1,0x57,0x59,0x5B,-1,0x5A,0x5A,0x65,-1×5]`. Rust
     `DESTRUCTION_OVERLAY_LOW_EW` matches. Match.
   - HIGH EW (`0x0057ed00`, not re-pulled this session but the Rust `DESTRUCTION_OVERLAY_HIGH_EW`
     = `[FF,DB,DE,FF,DA,DC,DE,FF,DD,DD,E8,FF×5]`) is the documented twin; see UNCHECKED.

7. **`CheckBridgeNeighbors_*` classifiers (3 of 4 verified live).** Bit assignment east/north-first
   then west/south:
   - EW_High `0x0057cab0`: east{D1,D3,D5,E0}=1, east{D4,E7}=2, west{D2,D3,D4,E2}=4, west{D5,E7}=8.
     Rust `check_bridge_neighbors_ew_high:636-658` matches.
   - NS_High `0x0057cbe0`: north{DA,DC,DE,E4}=1, north{DD,E8}=2, south{DB,DC,DD,E6}=4, south{DE,E8}=8.
     Rust `check_bridge_neighbors_ns_high:667-689` matches.
   - EW_Low `0x0057b870`: east{4E,50,52,5D}=1, east{51,64}=2, west{4F,50,51,5F}=4, west{52,64}=8.
     Rust `check_bridge_neighbors_ew_low:1023-1045` matches.
   The binary's `return uVar4 | 4` early-return for the west/south set is algebraically identical
   to Rust's `idx |= 4` + continue, because the two switch sets per neighbor are disjoint (a single
   byte cannot be in both `{D2,D3,D4,E2}` and `{D5,E7}`). PROVEN equivalent by disjointness.

8. **Cascade-leaf two-stage progression gate.** Binary `ApplyBridgeDestruction_NS_High`: if idx>0
   then `if (cur < 0xDF) v = table[idx]; if (cur==v) return;` `else if (cur==0xDF) v=0xE0; else if
   (cur==0xE1) v=0xE2; else return;`. Rust `apply_bridge_destruction_ns_high:736-748` reproduces
   this (`cur < 0xDF` → table; `0xDF`→0xE0; `0xE1`→0xE2; else return), with the `n != cur` no-op
   guard matching the binary's `cur == v → return`. LOW NS (`<0x5C`/0x5C→5D/0x5E→5F),
   LOW EW (`<0x60`/0x60→61/0x62→63), HIGH EW (`<0xE3`/0xE3→E4/0xE5→E6) all match. **Match.**

9. **Table-`0xFF` (sentinel) write path is unreachable.** Binary writes `local_70[idx]` even when
   it is `0xFFFFFFFF` (no sentinel check before the write), whereas Rust treats `0xFF` as `None` →
   no write. PROVEN benign: the only indices with `0xFF` are 0,3,7,11..15. idx 0 is gated out by
   `if (0 < iVar2)`; idx 3,7,11..15 are unreachable because each neighbor switch contributes only
   `{0, 1, 2}` (east/north) or `{0, 4, 8}` (west/south), so reachable idx ∈ `{0,1,2,4,5,6,8,9,10}`.
   3 and 7 require two bits set on the same neighbor — impossible (disjoint switch). Equivalent by
   reachability.

10. **`is_final`/`zones_dirty` semantics.** Binary sets `local_a5=1` and calls
    `UpdateBridgeZonesHelper()` only on the final-anchor branch; Rust sets `zones_dirty: is_final`
    only on the `0xE7`/`0x64`/`0xE8`/`0x65` write. Intermediate transitions set the success flag
    `local_a5=0` (binary returns 0 → CollapseBridge retry) which maps to Rust `Absorbed`. **Match.**

11. **`DestroyBridge_High/Low` internal subrange re-check vs walker dispatch.** The binary
    re-checks the overlay band inside `DestroyBridge_*` (`0xCD..=0xE8` / `0x4A..=0x65`) and the
    Rust `destroy_bridge_high/low` does the same redundant inner check before walker dispatch
    (`walker.rs:465-503`). Harmless redundancy; both reach the same walker for the same overlay.

---

## UNCHECKED

1. **`ApplyBridgeDestruction_EW_High @ 0x0057ed00` and `CheckBridgeNeighbors_NS_Low @ 0x0057b990`**
   were not re-decompiled live this session (the other 3 of each family were). The Rust
   `DESTRUCTION_OVERLAY_HIGH_EW` table and `check_bridge_neighbors_ns_low` classifier are documented
   as compiled twins and the in-code comments cite prior verification, but per the burden-of-proof
   rule they remain UNCHECKED for *this* scan. Recommend `decompile_function 0x0057ed00` and
   `0x0057b990` to confirm the HIGH EW table (`[FF,DB,DE,FF,DA,DC,DE,FF,DD,DD,E8,...]`) and the
   LOW NS classifier byte sets (`north{57,59,5B,61}=1, north{5A,65}=2, south{58,59,5A,63}=4,
   south{5B,65}=8`).

2. **Orchestrator radar-dirty derivation for the direct collapse path (D4).** Whether the
   orchestrator re-creates the exact per-cell `MarkTerrainDirty` set (triple cells + cascade-leaf
   sibling neighbors) that the binary walker emits inline is out of this file's scope and was not
   traced. Needs a separate check of `bridge_orchestrator::apply_bridge_damage_events` consumption
   of `StateOutcome::Collapsed { destroyed_cells, .. }`.

3. **`FindBridgeEndpoints_*` + `FUN_005868a0` object-on-bridge notification.** The binary's final
   branch additionally calls `FindBridgeEndpoints_NS_High(...)` and, with the `local_78=3,
   local_74=3` rect, `FUN_005868a0` (notify objects standing on the collapsing span). The Rust
   walker returns `destroyed_cells` but the equivalent "kill units on the span" wiring lives in the
   orchestrator and was not verified here. Flag for the cascade/fallout facet.
