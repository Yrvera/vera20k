# RepairBridgeWalker_*_* Bodies — Per-Cell State Writes

**Status:** Historical body decode, corrected by current active-retail body and
caller reads on 2026-09-12. The scalar overlay transitions do not establish
complete repair delivery; the occupant callback below is required gameplay work.

## 2026-09-12 correction: synchronous occupant checks are required

Original `0x00487A10` is a gameplay admission/damage callback, not a drawing or
flag-only helper. For each changed strip the four ordinary repair walkers retain
center and perpendicular neighbor Cell identities, write all three overlay fields,
call Recalc on those identities, then invoke `0x00487A10(0)` three times. The call
sites are Low NS `0x0057FB28/31/3A`, Low EW `0x0058003C/45/4E`, High NS
`0x00580564/6D/76`, and High EW `0x00580A88/91/9A`.

The callback first walks the Cell's ground-content list (`+0xE4`). It captures
each member's successor before calling its `+0x1AC` admission slot with
`(cell,-1,-1,null,1)`. Admission result 7 or abstract kind 2 (Aircraft, original
leaf `0x0041C180`) triggers synchronous `+0x16C` damage with a local copy of current
health, distance 0, Rules C4Warhead (`+0xFA8`), null attacker, both force flags set,
and null source house. The nonzero-argument extra branch is bypassed by these
repair callers.

It then builds a probe at the current Cell center and original ground height
`0x0047B3A0(128,128)`, including signed level and slope. The neighboring 5x5 scan
is X-major, re-reads the retained Cell coordinate for each lookup, and excludes
the selected Cell identity. For ground-list Foot members (`AbstractFlags & 4`),
it calls locomotor `Is_At_Coord` (`+0xA0`) with that probe, then the same admission
and damage sequence if the probe matches and admission returns 7. This pass
reads the successor after callbacks, unlike the first pass. Missing-cell dummy
aliasing and callback reentrancy therefore affect traversal.

Current stock locomotor slot owners are Drive `0x004B4920`, Ship `0x006A3F50`,
Hover `0x00517210`, Walk `0x0075CA80`; Fly/Teleport/Jumpjet/Rocket use false stub
`0x004B6630`. Live receiver plumbing in `src/sim/world/bridge_ground.rs` can be
reused, but unconditional collapse damage and snapshot path markers do not
implement this selection/lifecycle behavior. Its Rust delivery remains open.

Active reachability is ordinary engineer arrival in `InfantryClass::PerCellProcess`
`0x00519630`, then `0x00573540/0x00570050`, then `0x0057F440/0x0057F200` and the
four walkers. The engineer's outer family scan always visits all 25 Y-major cells,
with two fresh coordinate getters/lookups per cell and no early exit after finding
Low. It checks signed `wood_base <= tile < wood_base + 16` or overlay 74..101.
The chosen family entry independently scans X-major and returns on its first
matching overlay. The existing runtime-only/deferred Rust repair path does not
deliver these callbacks, and missing runtime entries can conceal live map cells.
No TS-only behavior is required for these findings.

The reproducible `tools/spatial_oracle/bridge_occupants.{py,json,meta.json}`
comparison executes the complete zero-argument controller, native cell lookup,
coordinate construction and signed/slope ground-height evaluation. Its 19 cases
cover both successor orders, an unlinked captured successor revisited with zero
health, neighbor unlinking, health changed by admission/kind callbacks, exact-seven
selection, Aircraft fallback, fresh selected-cell coordinates, retained probes,
and shared-dummy coordinate wrapping. Admission, abstract kind, IsAtCoord and
direct damage remain declared virtual seams. The Rust controller in
`src/sim/bridge_state/repair_occupants.rs` preserves this ordering; the concrete
world host and ordinary-walker connection remain required delivery work.

Current native receiver review establishes these host prerequisites:

- The sentinel direction `-1` takes the special arm in `0x004D9C60`
  (`0x004D9CBC` through `0x004D9E3E`), selecting structural deck level and
  bypassing the normal neighboring traversal comparison. An ordinary path-step
  admission adapter cannot substitute unchanged. The subsequent stock Unit
  locomotor `+0x1C` call is simpler: all eleven interface-table references point
  to unconditional-zero `0x0055ABF0` (`xor eax,eax; ret 8`).
- Terrain admission `0x0071C4D0` walks its type occupation offsets through cell
  lookup and `0x0047C620`, passing null type/owner. Building admission `0x00449440`
  wraps its foundation-placement checks. Both need their own concrete receivers.
- Aircraft's primary table is `0x007E22A4`; its `+0x1AC` slot at `0x007E2450`
  points to `0x004196B0`. Historical identification of `0x00415B10` as Aircraft
  admission used a wrong table base; that function is passenger ejection at
  `+0x100`. The first pass still invokes admission before its Aircraft fallback.
- IsAtCoord uses the active locomotor interface, including a Drive piggyback,
  and full raw-lepton coordinates. Its Head_To owners are Drive `0x004AFCC0`,
  Ship `0x0069F3D0`, Walk `0x0075AC00`, and Hover `0x00514D10`. Stored Head_To Z
  must survive changes to the destination terrain. Current Rust
  `Position.exact_z_leptons` can supply current raw Z, but ordinary
  `movement_step::resolved_track_endpoint` still stores coarse level Z, and the
  path-marker snapshot drops slope/stored endpoint Z. Those inputs must be
  corrected before claiming live repair-occupant delivery.

### Original locomotor query comparison and retained-head producers

`tools/spatial_oracle/locomotor_at_coord.{py,json,meta.json}` now preserves
336 supplied locomotor states and 4,164 recorded coordinate probes, plus eight
queries of the active Fly/Jumpjet/Rocket/Teleport false leaf. Both Head_To and
Is_At_Coord execute original instructions through verified original interface
slots. Drive/Ship also execute original transforms `0x004B4780`/`0x006A3DB0`
and retain the original TurnTrack/RawTrack/point tables. The cases cover all
72 Drive and 64 ordinary Ship descriptors, handoff/reverse gates, separate head
and current heights, complete/partial NullCoord, signed XY truncation, low-word
cell aliases, inclusive height tolerance and wrapping signed-Z boundaries.

All four Head_To owners return retained XYZ unless the complete triple equals
NullCoord; only then do they return current Foot XYZ. Drive/Ship additionally
reject a returned NullCoord. Walk/Hover do not. XY cell comparisons use signed
division by 256 truncated toward zero followed by 16-bit comparison. Height
uses wrapping subtraction and x86 signed absolute value, with inclusive
`<= height_step`; the INT_MIN absolute-value overflow remains negative.

Drive/Ship first consider a track candidate only when the reversed byte is zero,
the TurnTrack index is not -1, its **normal** raw-track byte is nonzero, its
handoff index is nonnegative, and the signed cursor is before that index. The
normal-track handoff point is transformed around the **stored** head XY, even
when Head_To itself fell back from NullCoord to current Foot XYZ. This candidate
uses current Foot Z; the ordinary fallback uses the returned Head_To Z. Using
the selected short raw track or one shared height for both candidates is wrong.
Drive tables are `0x007E7B28`/`0x007E7A28`; Ship tables are
`0x007F2A40`/`0x007F2960`. The TurnTrack stride is 12 and RawTrack stride is 16.
Hover interface `0x007EACFC` binds Head_To at +0x18 and Is_At_Coord at +0xA0.
Drive's Ghidra decompile has a truncated/overlapping boundary; the original
query ends at `0x004B4AF4`, as executed by this comparison.

Independent current-body reads also correct the required head producers:

- Ordinary Drive copies current Foot raw Z at `0x004B32BB..0x004B32EC` and
  stores it in the accepted head at `0x004B46A7/0x004B46D7`. Ship equivalents
  are `0x006A290B..0x006A293B` and `0x006A3CD6/0x006A3D06`. These successful
  producers retain current raw Z, rather than resampling the endpoint terrain.
- Walk `0x0075C240` stores full XYZ returned by subcell selection `0x00481180`
  at `0x0075C546..0x0075C562`. Its deck request needs the target structural
  flag and current Foot Z **strictly greater than** requested ground Z plus
  three steps (`0x0075C4CD..0x0075C51A`). The returned coordinate includes
  ground slope at the final selected subcell and its requested deck plane.
- Hover `0x0051533C..0x005153D8` obtains the new cell center and samples ground
  height through `0x00578080`. It adds the bridge offset only when current Foot
  Z is **greater than or equal to** that ground Z plus three steps. OnBridge
  and the accepted path layer do not select this branch.
- The existing Rust endpoint comment citing `0x004B2196` as a bridge-offset
  write is incorrect: it loads the height step for an absolute destination-Z
  comparison. Drive destination setter `0x004AFD40` writes destination XYZ;
  it does not simultaneously replace the separate retained head.

This corpus supplies initialized NullCoord=(0,0,0) and per-family height step104;
it does not execute startup, movement producers, object admission or repair
lifecycle. Scalar boundary states are branch witnesses, not stock-map reachability
claims. Mech's separate query `0x005B1AA0` is dormant TS behavior (its retail
RULESMD locomotor GUID appears only in comments) and is excluded, as are dormant
Tunnel/DropPod behaviors. This is native query evidence only: the shared Rust
query, exact producer state, save/hash/piggyback handling and live repair/marker
consumers remain required integration work.

## 0. TL;DR

The four `RepairBridgeWalker_{NS,EW}_{Low,High}` functions are the per-cell
write engine for engineer-driven bridge repair. They walk a column or row of
3-cell-wide bridge span and **write only one cell field: `OverlayTypeIndex`
(`+0x44`)**. No direct writes occur to `+0x11E` (damage-state ladder),
`+0x11A` (Height), `+0x11B` (Level), or `+0x140` (Flags) — the only state byte
the walker touches per cell is `+0x44`. The damage→intact transition on
`+0x11E` happens **indirectly** through `CellClass__RecalcAttributes`, which
is called on each of the 3 modified cells after the overlay rewrite.

The walker's per-cell logic for each input overlay is dispatched via a
4-target jump table indexed by a per-walker byte-LUT:
- **case 0 (damaged input)** → write RNG-selected intact overlay in a 4-value range
- **case 1** → normalize half-damaged overlay pair to its base (e.g. {0x5c,0x5d}→0x5c)
- **case 2** → normalize half-damaged overlay pair to its base (e.g. {0x5e,0x5f}→0x5e)
- **case 3** → no-op (overlay is outside this walker's repairable set)

After all cells are written, the walker conditionally calls
`MapClass__UpdateBridgeZonesHelper` (only if any case-0 repair happened) and
`FUN_005868a0` (X-major rectangle enumeration into two-pass batch `0x00586990`).

## 1. Walker inventory (address + dispatcher + role)

All four are labelled in the Ghidra project (do NOT rename).

| # | Address | Name | Dispatcher | Discriminator |
|---|---------|------|------------|---------------|
| 1 | `0x0057F6A0` | `MapClass__RepairBridgeWalker_NS_Low` | `MapClass__RepairBridge_Low @ 0x0057F200` | overlay `+0x44` ∈ `[0x4A..0x52] ∪ [0x5C..0x5F] ∪ {0x64}` (NS-low span set) — see §3.3 of `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` |
| 2 | `0x0057FBC0` | `MapClass__RepairBridgeWalker_EW_Low` | `MapClass__RepairBridge_Low @ 0x0057F200` | overlay `+0x44` ∈ `[0x53..0x5B] ∪ [0x60..0x63] ∪ {0x65}` (EW-low span set) |
| 3 | `0x005800D0` | `MapClass__RepairBridgeWalker_NS_High` | `MapClass__RepairBridge_High @ 0x0057F440` | overlay `+0x44` ∈ `[0xCD..0xD5] ∪ [0xDF..0xE2] ∪ {0xE7}` (NS-high span set) |
| 4 | `0x00580600` | `MapClass__RepairBridgeWalker_EW_High` | `MapClass__RepairBridge_High @ 0x0057F440` | overlay `+0x44` ∈ `[0xD6..0xDE] ∪ [0xE3..0xE6] ∪ {0xE8}` (EW-high span set) |

Caller verification (`get_function_callers`):
- `RepairBridgeWalker_NS_Low` and `RepairBridgeWalker_EW_Low` — **single
  caller** `MapClass__RepairBridge_Low @ 0x0057F200`.
- `RepairBridgeWalker_NS_High` and `RepairBridgeWalker_EW_High` — **single
  caller** `MapClass__RepairBridge_High @ 0x0057F440`.

The two dispatchers are themselves called only from
`ProcessBridgeDestruction_Low @ 0x00570050` and
`ProcessBridgeDestruction_High @ 0x00573540` (verified) — and those in turn
are called by `InfantryClass__PerCellProcess @ 0x00519630` (the engineer
walks-into-hut path) plus their own self-recursion.

Walkers do **not** call each other or any other walker; this is **not** the
shared destruction-walker family.

## 2. Per-walker per-cell write map (state-table format)

All four walkers share the same control flow. For each cell in the column/row
of the span (advanced via the inner loop and gated by `FUN_00580B20`/`B70` —
both are simple `0x4A ≤ +0x44 ≤ 0x65` / `0xCD ≤ +0x44 ≤ 0xE8` range checks),
the walker:
1. Reads the **current** cell's `+0x44` (`OverlayTypeIndex`).
2. Computes `EAX = +0x44 − base` where base = `0x4E` (NS_Low), `0x57` (EW_Low),
   `0xD1` (NS_High), `0xDA` (EW_High).
3. Looks up `LUT[EAX]` (per-walker byte table, address below) to select one
   of 4 case targets.
4. Computes the **new** `+0x44` value for the selected case.
5. If new != current, writes new to **three cells**: the walker cell (`this_01`),
   the cell at `y−1` (NS) / `x−1` (EW) (`this_00`), and the cell at `y+1` /
   `x+1` (`this`). This is the **3-wide perpendicular strip** behavior.

### Jump-table + LUT addresses (read directly from binary)

| Walker | Jump table | LUT | LUT length | LUT base offset |
|--------|-----------|-----|------------|------------------|
| NS_Low  | `0x0057FB94` | `0x0057FBA4` | 23 bytes | `+0x44 − 0x4E` |
| EW_Low  | `0x005800A8` | `0x005800B8` | 15 bytes | `+0x44 − 0x57` |
| NS_High | `0x005805D0` | `0x005805E0` | 23 bytes | `+0x44 − 0xD1` |
| EW_High | `0x00580AF4` | `0x00580B04` | 15 bytes | `+0x44 − 0xDA` |

Each LUT entry is one of `{0,1,2,3}` selecting the case target.

### Case action per walker

| Walker | case 0 (damaged) | case 1 (half-dmg pair) | case 2 (half-dmg pair) | case 3 |
|--------|-----------------|----------------------|----------------------|--------|
| NS_Low  | `+0x44 = 0x4A + RNG(0..3)` (→ `0x4A..0x4D` intact) | `+0x44 = 0x5C` | `+0x44 = 0x5E` | no-op |
| EW_Low  | `+0x44 = 0x53 + RNG(0..3)` (→ `0x53..0x56` intact) | `+0x44 = 0x60` | `+0x44 = 0x62` | no-op |
| NS_High | `+0x44 = 0xCD + RNG(0..3)` (→ `0xCD..0xD0` intact) | `+0x44 = 0xDF` | `+0x44 = 0xE1` | no-op |
| EW_High | `+0x44 = 0xD6 + RNG(0..3)` (→ `0xD6..0xD9` intact) | `+0x44 = 0xE3` | `+0x44 = 0xE5` | no-op |

The RNG is `FUN_00598030(0, 3)` — verified to be a rejection-loop around
`Random__Next()` + `Math__ftol`, returning a value in `[0..3]`.

### Per-walker LUT decode

NS_Low LUT bytes at `0x0057FBA4`:
`00 00 00 00 00  03 03 03 03 03 03 03 03 03  01 01 02 02  03 03 03 03  00`

- `0x4E..0x52` → 0 (damage → RNG repair to `0x4A..0x4D`)
- `0x53..0x5B` → 3 (EW span — no-op)
- `0x5C..0x5D` → 1 (normalize to `0x5C`)
- `0x5E..0x5F` → 2 (normalize to `0x5E`)
- `0x60..0x63` → 3 (EW half-damaged — no-op)
- `0x64`      → 0 (NS full-damaged single → RNG repair)

EW_Low LUT bytes at `0x005800B8`:
`00 00 00 00 00  03 03 03 03  01 01 02 02  03  00`

- `0x57..0x5B` → 0 (damage → RNG repair to `0x53..0x56`)
- `0x5C..0x5F` → 3 (NS half-damaged — no-op)
- `0x60..0x61` → 1 (normalize to `0x60`)
- `0x62..0x63` → 2 (normalize to `0x62`)
- `0x64`      → 3 (NS full-damaged — no-op)
- `0x65`      → 0 (EW full-damaged → RNG repair)

NS_High LUT bytes at `0x005805E0`:
`00 00 00 00 00  03 03 03 03 03 03 03 03 03  01 01 02 02  03 03 03 03  00`

- `0xD1..0xD5` → 0 (damage → RNG repair to `0xCD..0xD0`)
- `0xD6..0xDE` → 3 (EW span — no-op)
- `0xDF..0xE0` → 1 (normalize to `0xDF`)
- `0xE1..0xE2` → 2 (normalize to `0xE1`)
- `0xE3..0xE6` → 3 (EW half-damaged — no-op)
- `0xE7`      → 0 (NS full-damaged single → RNG repair)

EW_High LUT bytes at `0x00580B04`:
`00 00 00 00 00  03 03 03 03  01 01 02 02  03  00`

- `0xDA..0xDE` → 0 (damage → RNG repair to `0xD6..0xD9`)
- `0xDF..0xE2` → 3 (NS half-damaged — no-op)
- `0xE3..0xE4` → 1 (normalize to `0xE3`)
- `0xE5..0xE6` → 2 (normalize to `0xE5`)
- `0xE7`      → 3 (NS full-damaged — no-op)
- `0xE8`      → 0 (EW full-damaged → RNG repair)

### Disassembly anchors (the +0x44 writes themselves)

All four walkers write `MOV dword ptr [<cellreg> + 0x44], EAX` at three sites
per iteration. Exact addresses:

| Walker | Walker-cell write | Side-cell write A | Side-cell write B |
|--------|------------------|-------------------|-------------------|
| NS_Low  | `0x0057F995` | `0x0057F998` | `0x0057F99B` |
| EW_Low  | `0x0057FEB2` | `0x0057FEBC` | `0x0057FEBF` |
| NS_High | `0x005803CE` | `0x005803D1` | `0x005803D4` |
| EW_High | `0x005808FB` | `0x00580905` | `0x00580908` |

These are the **only** `MOV […+offset], …` writes in the walker bodies. **No
`MOV byte ptr [<reg>+0x11E]`, `+0x11A`, `+0x11B`, or any `[<reg>+0x140]
AND/OR/XOR` instructions are present in any of the four functions.** Verified
by reading the full disassembly of each (`disassemble_function` for all four
addresses; grep for `0x11E`, `0x11A`, `0x11B`, `0x140` returns zero hits).

After the writes the walker calls, for each of the 3 modified cells:
- `CellClass__RecalcAttributes(cell)` at `0x0047D2B0` — which **does** read
  the new `+0x44`, derive `LandType`/`SlopeIndex`, and **may set
  `field_0x11e = 0`** under one specific condition: when `SlopeIndex != 0`
  AND the new overlay type has flag at `OverlayTypeClass+0x2a9 != 0` (the
  bridge-overlay-clears-overlay-on-slope branch). This is the indirect path
  by which damage-state `+0x11E` is reset to 0 on repair.
- `FUN_00487a10(0)` — synchronous occupant admission and damage; see the dated
  correction above. It is required simulation behavior.

After all iterations:
- If `bVar1` (any case-0 repair fired) → call `MapClass__UpdateBridgeZonesHelper @ 0x0056C510`.
- If the damaged-cell rect accumulator (`local_ec`, `local_e8`) is non-empty
  → call `FUN_005868a0` with the rect. It enumerates X outer/Y inner, then calls
  `0x00586990` even for an empty vector; the batch preserves reverse query/Recalc
  and local hierarchy-patch order. It does not itself replace base connectivity.

## 3. Neighbor-step pattern

The walkers do **not** use the compass-direction table `g_DirectionOffsets @
0x0089F688`. Instead they hardcode axis-aligned neighbors via direct
`MapCoord` shorts manipulation:

- **NS walkers**: Y is incremented (`local_fc + 1` after each iteration; pre-pass
  steps Y back via `local_fc + −1` loop until exiting the overlay band).
  Side cells are at `(x, y−1)` and `(x, y+1)`.
  → Wait — re-reading: the **iteration axis** is the high short (`sStack_fa`) for
  NS; let me restate.
  - NS_Low/High: pre-pass `do { local_fc -= 1; if outside-range break } while
    in-range`. Outer iteration: `local_fc += 1` per turn (so increases X
    along the south-walker direction). Side cells at `(x, y−1)` and `(x, y+1)`.
  - EW_Low/High: pre-pass decrements `local_fc.hi` (Y). Outer iteration:
    `local_fc.hi += 1`. Side cells at `(x−1, y)` and `(x+1, y)`.

(MapCoord is encoded as `short[2] = {X, Y}` in the walker locals; the
`CONCAT22` patterns match the decompile. The naming "NS walker iterates Y"
vs "EW walker iterates X" matches the §3.3/3.4 description in
`BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`.)

The 3-wide perpendicular strip is written on every iteration:
- NS walker: rewrites `(x, y)`, `(x−1, y)`, `(x+1, y)` → wait, again the
  walker decompile shows `local_d8 = CONCAT22(sStack_fa + −1, local_fc)` —
  the **first** word is X (`local_fc`), the **second** is Y (`sStack_fa`).
  So `(x, y−1)` is the side cell for NS. Per-iteration write set: walker cell
  + cell at `y−1` + cell at `y+1`.

Actually re-reading carefully: the NS walker `local_d8 = CONCAT22(sStack_fa
+ -1, local_fc)` packs `(low=local_fc, high=sStack_fa−1)`. If MapCoord layout
is `(X, Y)` with X in low and Y in high, that's `(x, y−1)`. The walker is then
iterating across columns by `local_fc += 1` (incrementing X), which is an
**EW**-direction iteration with **NS** side spread. So the function name
`NS_Low` refers to the **span axis** (the bridge span runs N-S, so the strip
of intact pieces lies along that axis and the walker steps **across** the
span). This matches the parent doc's note that walker labels refer to the
bridge axis, not the iteration axis.

(This naming ambiguity is documented but not resolved further here — it is
not load-bearing for the per-cell write map.)

## 4. Recursion / cross-walker calls

- No walker self-recurses (no call to its own address within its body).
- No walker calls a sibling walker (no `CALL 0x57F6A0/0x57FBC0/0x5800D0/0x580600`
  appears in any of the four bodies — verified by full disassembly grep).
- No walker calls a `RepairBridge_*` dispatcher.

The only callees (verified by inspecting the disassembly) are:
- `MapClass__Get_CellClass @ 0x005657A0` (cell lookup)
- `FUN_00598030` (RNG-bounded; 0x00598030 in NS_Low / 0x00598030 in NS_High
  etc.) at the case-0 site
- `CellClass__RecalcAttributes @ 0x0047D2B0` ×3 (post-write)
- `FUN_00487a10` ×3 (live occupant admission/damage; see dated correction above)
- `MapClass__UpdateBridgeZonesHelper @ 0x0056C510` (conditional; post-loop)
- `FUN_005868a0` (conditional; post-loop)
- `FUN_00580B20` (NS_Low/EW_Low) / `FUN_00580B70` (NS_High/EW_High) — loop
  guard, returns 1 iff `0x4A ≤ +0x44 ≤ 0x65` (low) / `0xCD ≤ +0x44 ≤ 0xE8`
  (high).
- Various geometry helpers (`FUN_0047fde0`, `FUN_0047fb90`, `FUN_00487f40`,
  `FUN_0045a130`, `TacticalClass__DirtyScreenRect`, `RadarClass__MarkTerrainDirty`)
  — out of scope, no bridge-state writes.

## 5. Flag-bit writes at +0x140

**None.** No `+0x140` read, write, OR, AND, or XOR appears in any of the
four walker bodies. Verified by full disassembly inspection (`disassemble_function`
on all four addresses; no operand `[..+0x140]` found).

The only `+0x140` interaction in the area is inside `RecalcAttributes` itself,
which can OR `0x10000` into `+0x140` of neighboring cells when an attached
animation table is iterated (`*(uint *)(iVar8 + 0x140) | 0x10000` at line
`0x0047D...` in the helper) — this is a side-effect of `RecalcAttributes`
**on neighbor cells**, not on the repaired cells, and only fires when the
overlay type has an attached animation-coord-list. Not directly bridge-repair
behavior.

**Bits 0x80, 0x100, 0x200, 0x400, 0x800 on `+0x140`: not touched by walkers.**

## 6. Active in YR — verdict + caller-chain evidence

**Verdict: Active in YR (all 4 walkers).**

Evidence — call chain (verified bottom-up):

```
InfantryClass__PerCellProcess @ 0x00519630   ← engineer steps onto cell
   └─→ ProcessBridgeDestruction_Low  @ 0x00570050    [or _High @ 0x00573540]
         └─→ MapClass__RepairBridge_Low  @ 0x0057F200   [or _High @ 0x0057F440]
               ├─→ RepairBridgeWalker_NS_Low  @ 0x0057F6A0
               └─→ RepairBridgeWalker_EW_Low  @ 0x0057FBC0
                   (NS_High @ 0x005800D0 / EW_High @ 0x00580600 for high-bridge variant)
```

`InfantryClass::PerCellProcess` is the standard per-tick infantry occupancy
hook, invoked every cell-enter event for every infantry. Engineer-on-hut
detection (gated by `BridgeRepairHut=yes` rule per the parent doc §3.6 and
the `[General]` parse path) is YR-active by default. No `SpecialFlags` gate
applies to these four functions or their dispatchers.

No TS-only branches inside the walker bodies — every case in the LUT is
reachable from YR-live overlay states during a normal damaged-bridge state.

## 7. Open Questions (deferred)

1. **Damage-state `+0x11E` indirect path.** Walker does not write `+0x11E`
   directly. The damage→intact transition on `+0x11E` must come from
   `RecalcAttributes`. There is exactly one site in `RecalcAttributes` that
   sets `field_0x11e = 0`: when `SlopeIndex != 0` AND
   `OverlayTypeClass[+0x2a9] != 0`. Whether the YR bridge overlays
   (`[0x4A..0x65]`, `[0xCD..0xE8]`) carry `+0x2a9 != 0` on their
   `OverlayTypeClass` is not verified here — this is the most-load-bearing
   open question for the Rust port's regression behavior. Verify by reading
   `g_OverlayTypeClass_Array[0x4A].+0x2a9` (and a few neighbours) in a
   separate investigation.

2. **`FUN_005868a0` delivery.** The original body collects the rectangle's
   cells in X-major order and calls `0x00586990` even for an empty collection.
   Its zone-batch semantics are now captured in
   `tools/spatial_oracle/bridge_hierarchy.{py,json}`. Wiring the rectangle
   callers into the prepared live Rust batch owner remains open.

3. **`FUN_00487a10(0)` delivery.** The dated correction above establishes
   live occupant admission and direct damage. This remains a required Rust
   ordinary-repair integration prerequisite.

4. **NS/EW span-axis naming.** The walker iterates **across** the bridge,
   not along it (NS walker iterates X; EW walker iterates Y), with the
   3-wide perpendicular strip lying along the **iteration** axis. The
   walker name refers to the **bridge span** axis. This matches the parent
   doc but is worth a single sentence somewhere if it isn't already.

## 8. Sources (addresses decompiled, memory reads, strings searched)

Functions decompiled (Ghidra MCP `decompile_function`):
- `0x0057F200` — `MapClass__RepairBridge_Low` (dispatcher; for callee verification)
- `0x0057F440` — `MapClass__RepairBridge_High`
- `0x0057F6A0` — `MapClass__RepairBridgeWalker_NS_Low`
- `0x0057FBC0` — `MapClass__RepairBridgeWalker_EW_Low`
- `0x005800D0` — `MapClass__RepairBridgeWalker_NS_High`
- `0x00580600` — `MapClass__RepairBridgeWalker_EW_High`
- `0x00580B20` — `FUN_00580B20` (NS_Low/EW_Low loop guard; verified
  range-check on `+0x44`)
- `0x00580B70` — `FUN_00580B70` (NS_High/EW_High loop guard)
- `0x00598030` — `FUN_00598030` (rejection-loop RNG; bound 3 → returns 0..3)
- `0x005868A0` — `FUN_005868A0` (region rect iterator; out-of-scope)
- `0x0047D2B0` — `CellClass__RecalcAttributes` (for indirect `+0x11E` path)

Functions disassembled (`disassemble_function`) for byte-level write audit:
- `0x0057F6A0` — full NS_Low body; only `+0x44` writes at `0x0057F995/8/B`
- `0x0057FBC0` — full EW_Low body; only `+0x44` writes at `0x0057FEB2/BC/BF`
- `0x005800D0` — full NS_High body; only `+0x44` writes at `0x005803CE/D1/D4`
- `0x00580600` — full EW_High body; only `+0x44` writes at `0x005808FB/905/908`

Memory reads (`read_memory`) for jump tables + LUTs:
- `0x0057FB94` (16 bytes) — NS_Low 4-entry jump table
- `0x0057FBA4` (24 bytes) — NS_Low 23-byte LUT
- `0x005800A8` (32 bytes) — EW_Low jump table + LUT
- `0x005800B8` (16 bytes) — EW_Low 15-byte LUT (re-read for cleanliness)
- `0x005805D0` (48 bytes) — NS_High jump table + LUT
- `0x00580AF4` (32 bytes) — EW_High jump table + LUT

Caller-chain verifications (`get_function_callers`):
- `0x0057F6A0` → only `0x0057F200`
- `0x0057FBC0` → only `0x0057F200`
- `0x005800D0` → only `0x0057F440`
- `0x00580600` → only `0x0057F440`
- `0x0057F200` → only `0x00570050`
- `0x0057F440` → only `0x00573540`
- `0x00570050` → `InfantryClass__PerCellProcess @ 0x00519630` + self
- `0x00573540` → `InfantryClass__PerCellProcess @ 0x00519630` + self

Cross-references read:
- `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §3.3, §3.4, §3.5, §7
- `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §11.1 (the
  `UpdateRamp_*_High` family — confirmed disjoint from the walker family;
  the walkers act on `+0x44`, the ramp helpers act on `+0x11E`, and they
  serve opposite paths)

No Ghidra mutations performed. Read-only session.
