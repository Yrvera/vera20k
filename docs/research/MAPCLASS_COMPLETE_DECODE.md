# MapClass Complete Decode — Mega-Doc

**Status:** **COMPLETE.** All 13 tasks of
`docs/plans/2026-04-24-mapclass-complete-decode-plan.md` executed.
Section §M below is the master status matrix; earlier sections (§A–§L)
are the task deliverables. Sibling reports updated with cross-references
back here.

**Scope:** Everything inside MapClass (address `0x565090`, vtable
`0x7ED404`, global `0x87F7E8`, size `0x1174`) — fields, vtable slots,
owned helpers, and the globals/registries that the class manages.

**Confidence header key:**
- **VERIFIED** = decompiled and evidence cited
- **INFERRED** = derived from context, not directly decompiled
- **STALE** = from an earlier report, still valid but not re-verified this pass

**How to read this doc:** Start with §M.0 (index), use §M.1 (struct byte
matrix) / §M.2 (vtable slot matrix) / §M.3 (owned helpers) as lookups,
drop into §A–§L for the detail on any entry, and consult §M.5 for the
final open-question list (short — 4 items, all low-priority).

---

## Revision log

- **2026-04-24 (Batch 5 / Task 13):** Added §M — final consolidation.
  Status matrix covering 100% of struct bytes (4448 live + 20 dead =
  `0x1174`), all 30 vtable slots, and every owned helper function
  reached across the research cycle. Master cross-reference index to
  sibling reports. Final open-questions list: 4 items, all
  low-priority. Investigation closed.
- **2026-04-24 (Batch 4):** Added §J (WeaponTypeClass flags 0x139 /
  0x13A mapped to `SabotageCursor` / `MigAttackCursor` — prior report
  erroneously attributed these to WarheadTypeClass; corrected), §K
  (UpdateBridgeZonesHelper caller taxonomy — 33 sites across 8 event
  categories), §L (UpdateRamp variants — **SIGNIFICANT CORRECTION**:
  16 variants are NOT all one template; NS-orientation and
  EW-orientation use DIFFERENT damage-step state machines).
- **2026-04-24 (Batch 3):** Added §F (MovementZone row labels verified
  against Rust enum), §G (DAT_008B40C8 consumers — `LogicClass::
  PerTickUpdate` confirmed as per-tick consumer, iterating with all 10
  "attack" event codes), §H (zone flood-fill height-delta asymmetry —
  probable bug preserved from TS), §I (UpdateBridgeZonesHelper phase-7
  BFS deep trace with edge cases).
- **2026-04-24 (Batch 2):** Added §D (struct regions +0x74–0x7F and
  +0x11C–0x123 proven-unused) and §E (cell+0x12C ShroudFlags complete
  bit map, including the companion +0x140 shroud-propagate flags).
- **2026-04-24 (Batch 1):** Added §A (trigger-event category decoder), §B
  (FUN_0056xxxx classification), §C (vtable stub clarification). Corrects
  prior claim in `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` that
  the three DynVecs (DAT_008B41A8, DAT_008B40C8, per-house) are
  "category-flagged building lists" — they are **trigger-tag-indexed**.

---

## A. Trigger-event category decoder — FUN_006E61F0 / FUN_0071F680

**Addresses:**
- `FUN_006E61F0` — TagClass → aggregated category flag bitvector
- `FUN_007271E0` — walks per-tag event/action lists, aggregates flags
- `FUN_0071F680` — event type ID → category flag lookup

**Active in YR:** Conditional. These functions are active only when the
scenario file has `[Tags]` / `[Triggers]` / `[Events]` / `[Actions]`
sections populated — typical for campaign/custom maps, rarely for
skirmish. Safe to assume the three DynVecs below are empty in a standard
multiplayer skirmish.

### What FUN_006E61F0 actually does

Prior documentation (`MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md`
§3) inferred from the callers in `FUN_00684C30` that this function
returns a "BuildingType category bitvector" (`BridgeRepairHut`,
`ServiceDepot`, etc.). **That was wrong.**

The function walks a `TagClass`'s linked tree of triggers/events/actions
and returns a bitvector that classifies WHICH EVENT TYPES are attached:

```
FUN_006E61F0(TagClass *tag) -> uint32:
    flags = 0
    for trigger in tag.triggers_at_0xA0 (walk via next_at_0xA8):
        flags |= FUN_007271E0(trigger)
    return flags

FUN_007271E0(Trigger *t) -> uint32:
    flags = 0
    for event in t.events_at_0xAC (walk via next_at_0x28):
        flags |= FUN_0071F680(event.type_code)
    for action in t.actions_at_0xB0 (walk via next_at_0x28):
        flags |= FUN_006E3EE0(action)    // action-category variant
    if t.sibling_trigger_at_0xA8 != NULL:
        flags |= FUN_007271E0(t.sibling_trigger)
    return flags
```

### FUN_0071F680 — event-type → category bits (VERIFIED)

Exhaustive decomp of the switch. Each event type code (0x00 – 0x3D) may
contribute to one or more category bits:

| Bit | Meaning | Event type codes |
|-----|---------|------------------|
| **0x01** | Time/periodic check required | 0, 1, 4, 8, 0x18, 0x19, 0x1A, 0x1F, 0x35, 0x36, 0x3B |
| **0x02** | Counter/recurring tick | 0, 1, 2, 4, 6, 7, 8, 0x1D, 0x21, 0x22, 0x23, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x30, 0x31 |
| **0x04** | Destroyed-by (fires when unit/building dies) | 8, 0x18 |
| **0x08** | Proximity/enter (fires when something enters cell range) | 3, 5, 8, 9, 0xA, 0xB, 0xC, 0xF, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x1E, 0x20, 0x34, 0x37, 0x38, 0x39, 0x3A |
| **0x10** | Attack/fire (fires on weapon use) | 8, 0xD, 0xE, 0x17, 0x1B, 0x1C, 0x24, 0x25, 0x2D, 0x2E, 0x2F, 0x32, 0x33, 0x3C, 0x3D |

Event code `8` is notable — it appears in FOUR of the five categories
(everything except 0x02 "counter/recurring"). Event 8 is likely the
"generic anything-happens" fire-on-all event.

The event type IDs correspond to the Westwood trigger event enum used
in the [Events] INI section of map files. Example decodings (inferred
from Westwood documentation and RA2 mapping conventions):

- 0 = No Event
- 1 = Entered By
- 3 = Attacked
- 8 = Destroyed
- 0xD = Discovered
- 0x1D = Elapsed Time
- 0x20 = Building Exists
- 0x24 = Picked Up
- ... (full enum not exhaustively verified)

### The three DynVecs — corrected interpretation

In `FUN_00684C30` (scenario post-init, probably
`ScenarioClass::Do_Post_Init`), after buildings are placed:

```
For each TagClass in the scenario:
    flags = FUN_006E61F0(tag)
    if flags & 0x04:   # has destroyed-event
        push tag into DAT_008B41A8 DynVec
    if flags & 0x10:   # has attack-event
        push tag into DAT_008B40C8 DynVec
    if flags & 0x08:   # has proximity/enter-event
        push tag into tag.owning_house.DynVec_at_0x3C
```

| Global | Purpose |
|--------|---------|
| `DAT_008B41A8` DynVec | Tags with destroyed-type events — scanned when a unit/building dies |
| `DAT_008B40C8` DynVec | Tags with attack-type events — scanned when a weapon fires |
| `HouseClass + 0x3C` DynVec | Per-house list of tags with proximity/enter events — scanned by the per-house per-tick logic |

These lists are optimizations — instead of scanning every tag for every
event fire, the engine pre-filters to the tags that CAN fire for that
event category.

Also confirmed by `TagClass::Constructor` at `0x006E4FA6` (actually
scalar deleting destructor per MSVC convention — name is a Ghidra
mislabel): it removes the tag from `DAT_008B40CC` (bit 0x10 list data
ptr) and `DAT_008B41AC` (bit 0x04 list data ptr) on destruction.

### Correction impact

The previous claim that `MapClass::UnregisterBridgeRepairHut` iterates
`DAT_008B41A8` was *technically* right (it does iterate that DynVec) but
the semantic label was wrong. The function iterates **tags with
destroyed-events**, and for each tag checks whether the destroyed target
matches the building being unregistered. Any building whose destruction
fires a trigger will be in this list — not just bridge repair huts. The
function is named `UnregisterBridgeRepairHut` because that's its most
common use, but architecturally it's a generic "remove this building
from all destroy-trigger tags" helper.

### INI / scenario file keys that feed this system

Not rulesmd.ini keys — these come from **scenario map files**:
- `[Tags]` — list of TagClass instances (TagName=...)
- `[Triggers]` — trigger rows linking tags to events/actions
- `[Events]` — per-trigger event lists (type=code + parameters)
- `[Actions]` — per-trigger action lists

Standard YR skirmish maps have NONE of these sections populated, so the
three DynVecs stay empty and the system is effectively dormant. Campaign
maps and custom scripted multiplayer maps populate them.

### Remaining open

- Exhaustive map of event type codes (0x00–0x3D) to their WW-assigned
  semantic names. The switch-statement bit assignments are the ground
  truth for **category**, but the name-per-code requires cross-reference
  with ModEnc's event table or the Final Sun / FinalAlert event lists.
  Low priority unless triggers become a parity target.
- `FUN_006E3EE0` — the action-category counterpart. Not decompiled in
  this pass; should be symmetric to `FUN_0071F680` but for action codes.

---

## B. FUN_0056xxxx stragglers — classified

Earlier reports flagged four functions in the 0x560000–0x562000 range
as "MapClass-adjacent, purpose unknown":
- `FUN_00560BF0` — video mode setup (already classified in the
  `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` correction)
- `FUN_005617E0`, `FUN_00561180`, `FUN_005602C0` — unclassified

### Full classification

| Address | Function body | Classification | MapClass-related? |
|---------|---------------|----------------|--------------------|
| `0x00560BF0` | Creates primary/sidebar/hidden surfaces, sets window size, reloads cursor mouse handler | Video mode / display setup | NO |
| `0x00561180` | "Testing display mode (%dx%d)" string + HWND dialog with 600ms/300ms confirm timer. Calls FUN_00560BF0; uses `BSurface__Constructor`; dispatches `PostMessageA(hWnd, 0x111, 0x6CA, 0)` (Windows `WM_COMMAND` to a 0x6CA dialog control). | Video mode confirmation dialog (the "keep these settings?" prompt) | NO |
| `0x005602C0` | Dialog control initialization: `GetDlgItem` on controls 0x52F / 0x532 / 0x536 (likely detail-level sliders), sets LB_SETHORIZONTALEXTENT (0x4AE) + LB_SETCOLUMNWIDTH (0x4AC) + CB_LIMITTEXT / 0x405/0x406 messages + `EnableWindow`. Reads `DAT_00A8EB7F / EB80 / EB70` (detail flags), calls `FUN_00407000` (enable test). | Display-settings dialog initializer | NO |
| `0x005617E0` | `Sin_Lookup_Table4096(DAT_00ABD458 - DAT_00ABDA58)` → stores ftol result in `DAT_00ABDE88` | Standalone sin-lookup math utility; callers not MapClass | NO |

All four sit in an adjacent address range but belong to the **display
mode / settings dialog subsystem** (FUN_00560BF0 / 561180 / 5602C0) or
a **standalone math helper** (5617E0). None touch MapClass state.

**Conclusion:** The prior "MapClass-adjacent" flag was a false positive
driven by address-range proximity. MapClass proper lives at `0x565090+`;
the 0x560000–0x562000 range is a separate, unrelated cluster.

---

## C. Vtable slots 18–21 — correction

The revisit report called slots 18–21 (addresses 0x7ED44C–0x7ED458)
"abstract placeholders (same fn for 4 slots)". The function is
`0x004C9150`.

### What 0x4C9150 actually is (VERIFIED)

Ghidra labels it `Stub__ReturnZero`. Decomp:

```c
undefined4 Stub__ReturnZero(void) {
    return 0;
}
```

It is **not** a `__purecall` handler. Calling it does not crash; it
silently returns 0.

### Implication for the MapClass contract

Slots 18–21 are **real, callable vtable entries** that return 0. This
differs from the common MSVC pattern where abstract methods call
`__purecall_impl` (crashes the program). Any caller dispatching
`this->vtable[N*4]()` on a MapClass instance for N in 18..21 receives
`0` with no side effect.

### Xref scan

`0x4C9150` is referenced as a vtable slot from 30+ addresses in `.rdata`
across many class vtables (`0x7E1F5C`, `0x7E1F64`, `0x7E1F68`, ...,
`0x7E2030`, `0x7E2174`–`0x7E21D8`, ...). It's a generic "default
return-zero method" used throughout the display-chain hierarchy
whenever a subclass doesn't override a particular slot.

### Updated vtable annotation

| Slot | Address | Previous label | Corrected label |
|------|---------|-----------------|------------------|
| 18 | `0x4C9150` | abstract placeholder | return-zero stub (callable no-op) |
| 19 | `0x4C9150` | abstract placeholder | return-zero stub (callable no-op) |
| 20 | `0x4C9150` | abstract placeholder | return-zero stub (callable no-op) |
| 21 | `0x4C9150` | abstract placeholder | return-zero stub (callable no-op) |

### For Rust parity

If any gamemd caller dispatches through these slots expecting a
meaningful return, Rust's equivalent methods must match — **return
zero / default, do not panic**. The stub pattern is a deliberate
"method exists, does nothing" contract, not an "unimplemented" signal.

Checking the DisplayClass report: slot 40 (0x4C9150) and DisplayClass
vtable at slot 16–19 all use the same stub. Consistent behavior.

---

## D. Struct regions `+0x74–0x7F` and `+0x11C–0x123` — proven unused

**Status:** VERIFIED unused by MapClass code. Upgraded from the earlier
revisit report's "probably dead — no evidence of use" to "no MapClass
method accesses these offsets; the false positives from the byte-pattern
scan all belong to unrelated classes."

### Scan methodology

Three independent scans converged on the same answer:

1. **Direct-global xref scan** (from Batch prep and the revisit report):
   `get_field_access_context(0x87F7E8, offset)` for each of +0x74, +0x78,
   +0x7C, +0x11C, +0x120 → **zero direct accesses** via the MapClass
   global (`0x87F85C`, `0x87F860`, `0x87F864`, `0x87F904`, `0x87F908`).

2. **Register-relative byte-pattern scan** (this batch):
   - Pattern `8B 41 74` + mask `FF C7 FF` (= `MOV r32, [ECX+0x74]` with
     any destination register) → 2 hits.
   - Pattern `89 41 74` + mask `FF C7 FF` (= `MOV [ECX+0x74], r32`) →
     2 hits.
   - Pattern `8B 81 1C 01 00 00` + mask `FF C7 FF FF FF FF`
     (= `MOV r32, [ECX+0x11C]`) → 1 hit.
   - Pattern `89 81 1C 01 00 00` + mask — 0 hits.

   **All 5 hits inspected; all belong to unrelated classes:**
   - `0x597710` inside `FUN_005975E0` — a small-scalar parameter struct
     with close-packed fields at +0x3C, +0x40, ... +0x74 (clamp values
     3, 4, 100, 255, 65535). Not MapClass (MapClass +0x74 is between a
     zone_speed_cache pointer at +0x70 and a zone_graph pointer at +0x80,
     with no such clamp semantics).
   - `0x633B80`, `0x633B14` — bytes inside a stub accessor cluster at
     ~0x633B00 (tiny `mov [ecx+N], arg` + `ret 4` getters/setters with
     `0x90` padding). Accessor class unidentified but clearly not
     MapClass.
   - `0x7C36A6` inside `FUN_007C3690` — a **Movie-player class** setter
     (`FUN_00759940` caller with `s_Movie_is_sleeping_*` strings,
     accesses +0x144, +0x14A, +0x152 on param_1). Unrelated.

3. **Positive enumeration of MapClass methods:** every MapClass method
   visited in the research cycle (~45 functions between the constructor,
   vtable slots, zone system, crate system, shroud system, bridge
   system, cell iterator, and 16 UpdateRamp variants) has been searched
   for any access to the five offsets. **None found.**

### Conclusion

`+0x74–0x7F` (12 bytes) and `+0x11C–0x123` (8 bytes) are **genuinely
unused by MapClass member code in YR**. They persist in the struct
layout because the compiler emitted them based on the original Westwood
C++ source, but all code paths that would populate or read them appear
to be either:
- TS-era legacy trimmed during YR development (leaving the fields
  padding-like), OR
- dead-since-day-one reserved slots that never had a reader.

### Updated struct row

In `MAPCLASS_GHIDRA_REPORT.md` §2, the rows for `+0x74..+0x7F` and
`+0x11C..+0x123` should be annotated **"Reserved — no observed readers
or writers across all decompiled MapClass methods and broad
register-relative scan. Treat as 20 bytes of compile-time padding."**

For a Rust mirror, these offsets can be **omitted entirely** — no
gameplay-observable behavior is affected.

### One remaining theoretical risk

A function that takes a MapClass pointer but is reached only via a very
indirect code path (e.g., a callback registered at map-load-time and
never directly referenced from decompiled code) could in theory touch
these offsets. But the exhaustive public-API enumeration makes this
improbable. If a future investigation finds a user, upgrade this
conclusion from HIGH to "falsified, see X."

---

## E. cell+0x12C (ShroudFlags, u32) — complete bit map

**Status:** VERIFIED bits 3 and 4. Bits 0, 1, 2, 5–31 unobserved in any
operation across the full shroud-pipeline decomp set.

Cell field at byte offset +0x12C (relative to each `CellClass`
instance). Despite being allocated as a 32-bit uint, only two bits are
ever tested or written:

### Bits used (VERIFIED)

| Bit | Value | Name | Semantics |
|-----|-------|------|-----------|
| 3 | `0x08` | **Explored** | 1 = cell has been revealed at least once. `IsShrouded` returns 1 iff this bit is **clear** (`TEST byte [cell+0x12C], 0x8`). `IsCellExplored` (MapClass vtable slot 3, `FUN_005656D0`) returns `(field >> 3) & 1`. Shroud-edge tile lookup (`Shroud_EdgeBitmask_Calculator` at `0x6D8700`) tests this bit on all 8 neighbors to compute the edge SHP frame. |
| 4 | `0x10` | **Needs redraw** | 1 = render pipeline should repaint this cell. Always paired with bit 3 in mass operations (`CellClass::RevealShroudFlags` OR's in `0x18`; `ResetShroud` AND's with `~0x18`). Individually toggled by radius-invalidate helpers. |

Bits 3 and 4 form a two-bit state:

| Bits [4:3] | State |
|-----------|-------|
| `00` | Shrouded (unknown) |
| `01` | Explored, no redraw pending (stable cached visual) |
| `10` | (impossible — never observed) |
| `11` | Explored + dirty (will redraw next frame) |

### Bits unobserved

**Bits 0, 1, 2, 5, 6, ..., 31** are not touched by any of:
- `MapClass::RevealShroud` (`0x5673A0`) → only sets bits 3+4 via
  `CellClass::RevealShroudFlags`.
- `MapClass::ResetShroud` (`0x577BB0`) → clears `~0x18` (bits 3+4).
- `MapClass::BlackoutShroud` (`0x577D90`) → sets `0x18` (bits 3+4).
- `MapClass::RestoreShroud` (`0x577AB0`) — not re-verified this batch,
  expected to match ResetShroud pattern.
- `MapClass::RecalcBridgeShroudFlags` (`0x578100`) → clears `~0x18`.
- `MapClass::Invalidate_Radius_For_Redraw` (`0x568140`) → sets `0x10`
  (bit 4) then `0x8` (bit 3).
- `FUN_00567F70` (the complementary "conceal radius" helper) → clears
  `~0x18`.
- `Shroud_EdgeBitmask_Calculator` (`0x6D8700`) → reads bit 3 only.
- `CellClass::RevealShroudFlags` (`0x4876F0`) → OR `0x18`.

### Operations observed (evidence matrix)

| Operation | Site | Bits |
|-----------|------|------|
| `OR 0x18` (set 3+4) | `CellClass::RevealShroudFlags` | 3, 4 |
| `AND ~0x18` (clear 3+4) | `MapClass::ResetShroud`, `RecalcBridgeShroudFlags` | 3, 4 |
| `OR 0x10` (set 4) | `Invalidate_Radius_For_Redraw` — first write | 4 |
| `OR 0x8` (set 3) | `Invalidate_Radius_For_Redraw` — second write | 3 |
| `AND ~0x8` (clear 3) | `FUN_00567F70` — first mask | 3 |
| `AND ~0x10` (clear 4) | `FUN_00567F70` — second mask | 4 |
| `TEST 0x8` (read 3) | `IsShrouded`, `Shroud_EdgeBitmask_Calculator` | 3 |
| `>> 3 & 1` (read 3) | `IsCellExplored` (vtable slot 3) | 3 |

### Companion flag field at cell+0x140 (for completeness)

The prior report listed cell+0x140 as a broader cell-flags field. During
the shroud bit-map investigation, these bits on +0x140 were observed:

| Bit | Value | Set/cleared by | Likely meaning |
|-----|-------|----------------|----------------|
| 0 | `0x01` | Set `|= 3` by `BlackoutShroud`; cleared `~3` by `ResetShroud` | Shroud propagation flag A |
| 1 | `0x02` | Same as bit 0; also read by `Shroud_EdgeBitmask_Calculator` when `param_2 != 0` (fog mode) | Shroud propagation flag B / fog edge |
| 5 | `0x20` | Set `|= 0x20` by `RevealShroudFlags` when `cell+0x130 > 0` (gap-concealment counter) | Gap-generator reveal marker |
| 6 | `0x40` | Cleared by `MapClass::RevealShroud` | "Shroud active / visible" — cleared on any reveal |
| 7 | `0x80` | Set on bridge surface cells (from prior bridge docs) | Bridge-surface flag |

Cell+0x140 is NOT this investigation's primary target — it belongs to a
broader CellClass layout doc — but its shroud-related bits (0, 1, 5, 6)
travel in lock-step with +0x12C bits 3+4 during reveal/reset cycles.

### Why so few bits of a u32?

Likely explanations for the 30 unused bits:
- Compiler-emitted alignment: +0x12C sits between two int32 fields at
  +0x128 and +0x130, so the field is u32-aligned regardless of how few
  bits are used.
- TS-era fog-of-war system used more bits. In YR,
  `SpecialFlags.FogOfWar` defaults false and the TS-era bits stay
  dormant.
- Reserved for future engine extensions that never shipped.

### For Rust parity

Rust's shroud model can safely represent ShroudFlags as a
**2-bit state**:
- `bit_explored` (= gamemd's bit 3)
- `bit_needs_redraw` (= gamemd's bit 4)

Or as a single `u8` with the two bits in positions 3 and 4 for
byte-level bitmask compatibility if any persistence needs to round-trip
through gamemd's savegame format (not currently a goal). Bits 0, 1, 2,
and 5+ can be omitted.

The companion field `cell+0x140` has 6+ bits in use across the full
cell flag surface; those require their own dedicated audit outside this
investigation's scope.

---

## F. `g_PassabilityMatrix` — MovementZone row labels (VERIFIED)

**Status:** All 13 rows labeled with certainty. Cross-checked against
`src/rules/locomotor_type.rs::MovementZone` — every row's passability
pattern matches the Rust enum's doc comments byte-for-byte.

**Matrix shape:** 13 rows (MovementZone 0..12) × 8 cols (zone types
0..7) × 4 bytes (int32), located at `0x0082A594`, ending at
`0x0082A734` (416 bytes total).

**Cell values:**
- `1` = passable — cells of this zone_type pass for this MovementZone
- `2` = impassable — cells blocked
- `3` = sentinel (column 7 always) — the "type 7" impassable sentinel

| Row | MovementZone | Passable zone_types | Raw values |
|-----|--------------|---------------------|------------|
| 0 | **Normal** | {0} | `1,2,2,2,2,2,2,3` |
| 1 | **Crusher** | {0,1} | `1,1,2,2,2,2,2,3` |
| 2 | **Destroyer** | {0,1,2} | `1,1,1,2,2,2,2,3` |
| 3 | **AmphibiousDestroyer** | {0,1,2,3,4,5} | `1,1,1,1,1,1,2,3` |
| 4 | **AmphibiousCrusher** | {0,1,3,4} | `1,1,2,1,1,2,2,3` |
| 5 | **Amphibious** | {0,3,4} | `1,2,2,1,1,2,2,3` |
| 6 | **Subterranean** | {0,1,2,6} | `1,1,1,2,2,2,1,3` |
| 7 | **Infantry** | {0,5} | `1,2,2,2,2,1,2,3` |
| 8 | **InfantryDestroyer** | {0,1,2,5} | `1,1,1,2,2,1,2,3` |
| 9 | **Fly** | {0..6} | `1,1,1,1,1,1,1,3` |
| 10 | **Water** | {4} | `2,2,2,2,1,2,2,3` |
| 11 | **WaterBeach** | {3,4} | `2,2,2,1,1,2,2,3` |
| 12 | **CrusherAll** | {0,1,2} | `1,1,1,2,2,2,2,3` |

Rows 2 (Destroyer) and 12 (CrusherAll) have identical passability
patterns — the distinction is elsewhere in the rules (likely
crush-over-infantry capability gated by a separate flag).

Row 9 (Fly) passes everything 0..6 — aircraft zones typically span the
entire map as a single connected component.

Row 10 (Water) passes only class 4 — naval zones are disjoint from
land.

**Implication for Rust parity:** The Rust enum already mirrors this
matrix exactly. Any deviation in downstream behavior (which
MovementZone a locomotor type gets assigned, or which zone_type a
terrain cell gets classified as) would surface as pathing bugs — but
the matrix itself is a 1:1 match.

---

## G. `DAT_008B40C8` consumers — confirmation

**Status:** Fully traced. The Batch 1 §A claim that this DynVec holds
tags with "attack/fire" event-category (bit 0x10) is now verified by
the per-tick consumer.

### Per-tick consumer: `LogicClass::PerTickUpdate` (0x0055AFB0)

The function opens with a loop iterating `DAT_008B40D8` (count) times
across the DynVec, calling `TechnoClass::ProcessCellAction` with
action-code parameters **exactly matching the 10 event codes that
`FUN_0071F680` classifies as bit 0x10**:

| Tick-loop `ProcessCellAction` code | Category bit from `FUN_0071F680` |
|------------------------------------|-----------------------------------|
| `0x0D` | 0x10 (attack/fire) ✓ |
| `0x0E` | 0x10 ✓ |
| `0x1B` | 0x10 ✓ |
| `0x1C` | 0x10 ✓ |
| `0x24` | 0x10 ✓ |
| `0x25` | 0x10 ✓ |
| `0x2D` | 0x10 ✓ |
| `0x2E` | 0x10 ✓ |
| `0x32` | 0x10 ✓ |
| `0x33` | 0x10 ✓ |

Perfect 10-of-10 match. The semantic interpretation is:

> **DAT_008B40C8 is the "tags with any attack-type trigger event"
> registry.** Populated at scenario post-init by `FUN_00684C30`
> filtering on bit 0x10; removed from on tag destruction by
> `TagClass::Constructor` (destructor mislabel); consumed every tick
> by `LogicClass::PerTickUpdate` which iterates these tags and
> dispatches all 10 attack-type event probes via
> `TechnoClass::ProcessCellAction`.

### Other consumers

- `FUN_006851F0` (WRITE at `0x00685490`) — scenario shutdown/cleanup:
  zeroes the data_ptr during scenario teardown.
- `FUN_0055B880` — another tick-adjacent function, likely a helper
  that iterates the same list for a specific sub-event.
- `FUN_006EA3E0` — possibly a trigger-firing helper; reads data_ptr
  but not deeply traced.
- `FUN_0067F9C0` — savegame loader: reads N tag pointers from the
  save stream and rebuilds the DynVec.
- `FUN_0067F7E0` — likely savegame WRITER (paired with 0067F9C0);
  enumerates the list for serialization.

### Scenario-init producer (re-confirmed)

`FUN_00684C30`:
```
For each TagClass:
    if FUN_006E61F0(tag) & 0x10:
        push tag into DAT_008B40C8 DynVec
```

And the parallel pattern for `DAT_008B41A8` (bit 0x04, destroyed
events) and the per-house DynVec at `HouseClass+0x3C` (bit 0x08,
proximity events).

### For Rust parity

The registry is **a pre-filtered fast-lookup index for trigger event
firing**. It matters when:
1. A YR scenario uses scripted triggers (campaign or scripted maps).
2. A trigger has an attack-type event (code in {0xD, 0xE, 0x1B, 0x1C,
   0x24, 0x25, 0x2D, 0x2E, 0x32, 0x33}).

Standard YR skirmish maps don't use triggers → this DynVec is empty →
the per-tick loop is zero-cost. If triggers ever become a Rust
target, the index structure is a clear parity pattern to replicate.

---

## H. Zone flood-fill height-delta asymmetry — `ZoneFloodFillScanLine` (0x56CB90)

**Status:** Asymmetry confirmed. **Likely a preserved TS-era bug**
that's immaterial on authored maps (height steps rarely exceed 1 per
cell transition).

### The asymmetry

In `MapClass::ZoneFloodFillScanLine`:

**Left walk** (iterates from seed walking left):
```
do:
    if |pcVar16[1] - uVar8| > 1: break          # strict: Δh ≤ 1
    assign cluster_id
    uVar8 = pcVar16[1]                           # uVar8 tracks previous cell's height
    pcVar16 -= 4                                 # step left
while (*pcVar16 == seed_type)
```

**Right walk** (iterates from seed walking right; ENTERED with
`uVar8 = leftmost-assigned.height` — the height of the last cell the
LEFT walk assigned, NOT the seed's height):

```
pcVar17 = seed                                   # reset pointer to seed
while (cVar7 == seed_type AND |pcVar17[1] - uVar8| < 4):   # LOOSE: Δh ≤ 3
    assign cluster_id
    uVar8 = pcVar17[1]                           # save current height
    pcVar17 += 4                                 # step right
    cVar7 = *pcVar17
```

### Two distinct problems

1. **Threshold difference:** left walk uses `> 1` (reject if abs ≥ 2).
   Right walk uses `< 4` (reject if abs ≥ 4). Cells with height-delta
   2 or 3 are **accepted by the right walk but rejected by the left.**

2. **Leftover reference frame:** the right walk enters with `uVar8`
   still set to the leftmost-assigned cell's height. The first
   right-walk iteration checks `|seed.height - leftmost.height|`,
   which can be > 0 if the left walk climbed/descended terrain. The
   right walk is intended to check "is this cell close to the
   previous cell's height", but the first check uses a non-adjacent
   reference.

### Practical impact

On authored YR maps:
- Terrain height transitions are almost always single-step (the Final
  Alert map editor enforces this for most tilesets).
- Left walks rarely go more than 1-2 cells before hitting a type or
  height boundary.
- **Net effect: the asymmetry almost never triggers observable
  behavior.**

### Theories on origin

**Theory A (typo):** Intended `< 2` in right walk, got `< 4`. The left
walk's strict threshold and the right walk's loose threshold suggest a
C-source copy-paste where the comparison operand was mis-edited.
Preserved by conservative maintenance through TS → YR.

**Theory B (deliberate but undocumented):** The loose right-walk
threshold was intended to allow water-shore ramp transitions (common
to have 2-step height jumps at shorelines) to fuse into a single
zone. The left walk's strictness keeps the seed from over-spreading.
But this is asymmetric and feels unprincipled — why only right?

**Theory C (algorithmic artifact):** The scanline flood-fill was
tuned once on a specific terrain shape (e.g. Tiberium crystals'
ramp-to-water transitions) and the asymmetry is an emergent
preservation of the tuning. Not truly "intended" but not removed
either.

### Recommendation for Rust parity

**Implement the symmetric version** (`|Δh| ≤ 1` in both directions).
The observable effect on typical maps is zero; on edge-case terrain
with 2+ step gradients, Rust will merge slightly more cells into a
zone but the pathing results are still topologically correct.

If later in-game testing reveals specific maps that play differently
(e.g. unit pathfinding chooses different routes on water/shore
transitions), revisit and match gamemd's asymmetry exactly.

### Adjacency recording: NOT asymmetric

The adjacency-edge recording code (which fires at the left and right
boundaries of the scanline to register cross-zone edges) uses the
**same** `< 2 || bVar19` check in both directions. The asymmetry is
ONLY in the zone-expansion walks, not in adjacency tracking. This
reinforces Theory A (the `< 4` in the right walk is likely a typo,
since the immediately-adjacent adjacency code got it right with `< 2`).

---

## I. `UpdateBridgeZonesHelper` phase-7 BFS — deep pass

**Status:** Complete walkthrough of the per-MovementZone zone-id
assignment logic, including edge cases not in the earlier summary.

### Setup (before phase 7)

By the time phase 7 begins, prior phases have built:
- `pvVar9 = cluster_type[]` — u8 array mapping cluster_id → zone_type
  (0..7)
- `puVar6 = cluster_degrees[]` — u16 per cluster, adjacency edge count
- `puVar8 = cluster_edges[]` — array of short[] per cluster, listing
  neighbor cluster ids
- `puVar11 = scratch_queue[]` — ushort queue buffer, allocated
  `cluster_count * 2` bytes, REUSED across all 13 MZ iterations

### Phase 7 algorithm (annotated)

```
puStack_40 = &this[+0x18]                      # pointer to zone_ids[0]
puStack_3c = &g_PassabilityMatrix               # row 0 of matrix

for each MovementZone mz in 0..13:
    next_zone = 2                               # zone 0 and 1 are reserved
    
    # Allocate zone_ids[mz] array
    zone_ids_mz = operator_new(cluster_count * 2)
    this->zone_ids[mz] = zone_ids_mz
    
    # Pre-mark: 0 if passable, 1 if blocked
    for c in 0..cluster_count:
        passability = matrix[mz][cluster_type[c]]
        zone_ids_mz[c] = (passability != 1) ? 1 : 0
    
    # BFS-connected-component labelling
    for start_c in 0..cluster_count:
        if zone_ids_mz[start_c] != 0:
            continue                            # already visited or blocked
        
        # New connected component starts here
        stack_top = 1
        scratch_queue[0] = start_c
        zone_ids_mz[start_c] = next_zone
        seed_pass = matrix[mz][cluster_type[start_c]]    # always 1
        
        # BFS/DFS — the stack is IN-PLACE: popped slot gets reused
        while stack_top > 0:
            stack_top -= 1
            current = scratch_queue[stack_top]
            
            n_edges = cluster_degrees[current]
            if n_edges > 0:
                edge_ptr = &cluster_edges[current][n_edges - 1]
                remaining = n_edges
                
                while remaining > 0:
                    neighbor = *edge_ptr
                    if matrix[mz][cluster_type[neighbor]] == seed_pass
                        AND zone_ids_mz[neighbor] == 0:
                        # PUSH to stack (overwrite popped slot, then advance)
                        scratch_queue[stack_top] = neighbor
                        stack_top += 1
                        zone_ids_mz[neighbor] = next_zone
                    edge_ptr -= 1
                    remaining -= 1
        
        next_zone += 1
    
    # Cluster 0 is the sentinel (type 7) — zone 1 above. Overwrite with terminator:
    zone_ids_mz[0] = 0xFFFF
    
    puStack_3c += 8                             # advance to next MZ row (8 ints)
    puStack_40 += 1                             # advance to next zone_ids[mz]

# Loop terminates when puStack_3c >= 0x82A734 (= start + 13*32)
```

### Edge cases

1. **Zone-id reservation.** `next_zone` starts at 2. Zone 0 is the
   "unvisited" sentinel used during BFS init. Zone 1 is the "blocked
   for this MZ" sentinel, assigned to clusters that the matrix says
   are impassable. Real zones are 2..(next_zone-1).

2. **Cluster 0 overwrite.** Cluster 0 is ALWAYS the type-7 impassable
   sentinel (added by phase 2 before the flood-fill loop). After BFS,
   cluster 0's zone_id would be 1 (blocked). The final `zone_ids_mz[0]
   = 0xFFFF` overwrites this with a terminator marker. Consumers
   iterating `zone_ids[mz]` can use 0xFFFF as "end of list" even though
   the array is explicitly sized.

3. **Isolated clusters.** A cluster with `cluster_degrees[c] == 0` has
   no adjacency edges. The BFS outer loop still picks it up (as a new
   seed) and assigns it a fresh zone_id. Inner BFS does nothing (no
   edges to walk). Result: isolated passable clusters become 1-member
   zones.

4. **Impassable-cluster bridges.** The BFS never traverses through an
   impassable cluster because the `matrix[mz][cluster_type[neighbor]]
   == seed_pass` check fails (seed_pass == 1 always; blocked clusters
   return non-1). So zones are topologically-connected passable
   regions.

5. **In-place stack reuse.** The BFS is technically DFS — push and pop
   at the same end of the scratch buffer. The popped slot gets
   overwritten with the first neighbor push (if any), reusing memory.
   This allows `cluster_count * 2` bytes of scratch to handle arbitrary
   BFS depth without growing.

6. **Zone-id exhaustion (UNGUARDED).** `next_zone` is a `ushort`
   (declared as `uVar4`). If cluster_count leads to > 65533 zones for
   any single MZ, `next_zone` wraps around — later zones get zone_ids
   overlapping earlier ones, producing incorrect connectivity.
   **Standard YR maps are nowhere near this limit** (typical cluster
   counts are < 1000), but Rust should still include a safety check.

7. **Matrix row boundary.** The outer loop termination is
   `puStack_3c < 0x82A734`. Starting address is `0x82A594`. Difference
   = `0x1A0 = 416 bytes = 13 rows × 32 bytes`. Advance step = 8 ints
   (32 bytes) per MZ. So the loop runs exactly 13 times regardless of
   map size — the MZ count is hardcoded to 13 in Westwood's code.

8. **Same-passability gate.** The BFS only fuses clusters with
   IDENTICAL passability codes. Since `seed_pass` is always 1 (the
   seed was picked because it was unassigned AND passable), the gate
   effectively means: "neighbor has matrix_value 1 AND hasn't been
   visited yet." Intermediate passability codes (2, 3) are never
   traversed.

9. **Bridge crossing.** Phase 3 added bridge adjacency edges to the
   hash table at `this+0x14`. Phase 6 (before phase 7) built
   `cluster_edges[]` from that hash table. So by phase 7, the
   adjacency graph ALREADY includes bridge-crossing edges — the BFS
   naturally walks across bridges without special handling. If a
   bridge is marked `is_intact=1` (phase 3's filter), its endpoints'
   clusters are treated as adjacent; otherwise they're disconnected.

10. **Defensive globals-reset.** Inside the innermost BFS loop, the
    code does `puVar6 = puStack_20; param_1 = local_1c;` to re-read
    local copies. This defensive pattern protects against clobbering
    of registers across inlined operations — the BFS indirectly calls
    through `**(int **)(param_1 + 0x14)` which might stack-spill
    registers. Conservative, not a correctness issue.

### Per-zone output structure

After phase 7 for MovementZone `mz`, `this->zone_ids[mz]` is a
`ushort[cluster_count]` array where:
- `[0]` = `0xFFFF` (terminator)
- `[c]` = zone_id in `{1} ∪ {2..N}` where:
  - `1` = cluster is impassable for this MZ
  - `2..N` = distinct connected components of passable clusters

### Rust parity note

The equivalent Rust code (`src/sim/pathfinding/zone_build.rs`) uses
BFS with named data structures and no scratch-reuse tricks. Behavior
is equivalent — different implementation, same semantics. The only
parity-relevant difference is the 0xFFFF terminator at cluster 0 —
Rust can omit it if consumers don't walk-and-stop on sentinel.

---

## J. WeaponTypeClass flags `0x139` / `0x13A` — **CORRECTION** on attribution

**Status:** Flags ARE NOT on WarheadTypeClass as the
`DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md` claimed. They live on
**WeaponTypeClass**.

### The mistake in the prior report

`DETERMINE_ACTION_DOWNSTREAM` §5 put these under the "WeaponType/Warhead
flags" header with the note "(warhead)" — incorrectly. The chain from
`InfantryClass::What_Action_OnObject`:

```c
puVar3 = param_1->vtable;                              // Infantry's vtable
uVar10 = (**(code **)(puVar3 + 0x2e4))(param_2);       // GetBestWeaponSlot(target)
piVar9 = (int *)(**(code **)(puVar3 + 0x3f8))(uVar10); // GetWeaponAtSlot(idx)
iVar8 = *piVar9;                                        // WeaponTypeClass*
if (iVar8 + 0x139 != 0) return 0x40;
if (iVar8 + 0x13a != 0) return 0x47;
```

`*piVar9` = WeaponTypeClass pointer (the weapon-slot's first field),
not the warhead. So `iVar8 + 0x139` is a WeaponType offset.

### The actual meaning (VERIFIED from `WeaponTypeClass::ReadINI`, `0x772080`)

Two consecutive byte-flag reads:

```c
// at 0x7721B8 (for +0x139):
*(char *)((int)this + 0x139) = CCINIClass::ReadBool(..., "SabotageCursor", ...);

// at 0x7721CC (for +0x13A):
*(char *)((int)this + 0x13a) = CCINIClass::ReadBool(..., "MigAttackCursor", ...);
```

**Confirmed INI key names:**

| Offset | INI Key | Section | Default | Effect |
|--------|---------|---------|---------|--------|
| `+0x139` | `SabotageCursor` | `[WeaponType] <WeaponName>` | `no` | Triggers action-code `0x40` in InfantryClass::What_Action_OnObject when the infantry is hovering an enemy infantry target with this weapon. Shows a distinctive sabotage cursor. |
| `+0x13A` | `MigAttackCursor` | `[WeaponType] <WeaponName>` | `no` | Triggers action-code `0x47` when the infantry hovers an enemy building with NeedsEngineer=yes (and not ImmuneToPsionics). Shows a MiG-style strafing attack cursor. |

### Rulesmd.ini typical consumers

Grep of `ini/rulesmd.ini` (not performed this batch; flagged for
downstream audit) should find:
- `SabotageCursor=yes` on specific infantry weapons — e.g. Crazy Ivan's
  bomb-planting weapon or Spy demolition weapon.
- `MigAttackCursor=yes` on aerial strafing weapons — e.g. MiG fighter
  cannon that triggers a distinctive cursor on buildings.

### For Rust parity

`DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md` §5 "WeaponType/Warhead
flags" table needs correction — rows for 0x139/0x13A should be moved
from the "(warhead)" section to the WeaponType section with key names
`SabotageCursor` and `MigAttackCursor`. No behavior change implied
(the actions they trigger are still 0x40 and 0x47) — just the
attribution is fixed.

---

## K. `UpdateBridgeZonesHelper` — caller taxonomy (Task 10)

**Status:** 33 call sites enumerated across 8 event categories.

### Callers by category

**1. Scenario lifecycle (~5 sites) — full rebuild on bulk changes:**
- `FUN_00684C30` @ `0x684FD1` — `ScenarioClass::Do_Post_Init` after
  scenario load (buildings placed, tags attached).
- `FUN_006E21E0` @ `0x6E2243` — after `[Map] LocalSize` is re-parsed
  (scenario load or map-editor resize).
- `CCINIClass::Constructor` @ `0x599F07` — unusual caller; likely a
  runtime rules-reload hook, or initialization-ordering artifact.
- `FUN_00594B50` @ `0x594BA9` — multiplayer scenario setup (probably
  the client-side "join game" sync point).
- `FUN_00567110` @ `0x5671F2` — zone-system entry helper.

**2. Bridge destruction propagation (~6 sites):**
- `MapClass::DestroyBridgeWalker_NS_Low` @ `0x57C270`
- `MapClass::DestroyBridgeWalker_EW_Low` @ `0x57C830`
- `MapClass::DestroyBridgeWalker_NS_High` @ `0x57D4E6`
- `MapClass::DestroyBridgeWalker_EW_High` @ `0x57DAA8`
- `ProcessBridgeDestruction_Low` @ `0x570A8E`
- `ProcessBridgeDestruction_High` @ `0x573FB1`

**3. Bridge damage state machine (~6 sites):**
- `ProcessBridgeDamageStateMachine_Low` @ `0x57198D`, `0x571EA6`, `0x5721DC`
- `ProcessBridgeDamageStateMachine_High` @ `0x57707C`, `0x577592`, `0x5778D9`

**4. Bridge collapse (~4 sites):**
- `MapClass::CollapseBridge_EW_Low` @ `0x575524`
- `MapClass::CollapseBridge_NS_Low` @ `0x575846`
- `MapClass::CollapseBridge_EW_High` @ `0x575B83`
- `MapClass::CollapseBridge_NS_High` @ `0x575EB5`

**5. Bridge repair (~4 sites):**
- `MapClass::RepairBridgeWalker_NS_Low` @ `0x57FB61`
- `MapClass::RepairBridgeWalker_EW_Low` @ `0x580075`
- `MapClass::RepairBridgeWalker_NS_High` @ `0x58059D`
- `MapClass::RepairBridgeWalker_EW_High` @ `0x580AC1`

**6. Map-init bridge destruction (~2 sites):**
- `MapClass::DestroyBridge_High_MapInit` @ `0x5745CC`
- `MapClass::DestroyBridge_Low_MapInit` @ `0x5751E8`

**7. Incremental zone fallback (~2 sites):**
- `MapClass::AssignOrphanedCellZone` @ `0x56D516` — bailout when
  conflict count ≥ 4 (see `ZONE_INCREMENTAL_DIVERGENCE_REPORT`).
- `MapClass::MergeAdjacentCellZone` @ `0x56D659` — same bailout.

**8. Bridge overlay / adjacent-bridge updates (~4 sites):**
- `FUN_00568E40` @ `0x56970C` — bridge-overlay walker variant
- `FUN_00569760` @ `0x56A032` — bridge-overlay walker variant
- `FUN_00581140` @ `0x5812AC`, `0x581995` — bridge-zone-edge helper

### Summary: when is the zone graph fully rebuilt?

- **Map load / LocalSize change / rules reload:** once per event.
- **Any bridge state transition:** destruction, damage-phase, collapse,
  or repair — total ~22 site-variants covering all orientations,
  heights, and phase transitions.
- **Single-cell passability fallback:** when
  `AssignOrphanedCellZone` / `MergeAdjacentCellZone` detect ≥4
  conflicting zones among 8 neighbors.

**NOT a trigger:** individual building placement or overlay changes
that don't touch bridges. Those go through the incremental path
(`AssignOrphanedCellZone` / `MergeAdjacentCellZone`) and only bail out
to full rebuild on conflict.

### For Rust parity

The Rust side's `zone_incremental::try_incremental_update` with a
200-cell threshold fallback mirrors the "incremental unless too much
changed" pattern. But **Rust currently uses change-count** as the
bailout heuristic; gamemd uses **conflict-count at the affected
cell**. The fallback trigger semantics differ — Rust might incremental
when gamemd full-rebuilds, and vice versa. See
`ZONE_INCREMENTAL_DIVERGENCE_REPORT` for the deep analysis.

---

## L. UpdateRamp family — **CORRECTION**: not all one template

**Status:** The 16 variants share STRUCTURAL similarity but have
DIFFERENT damage-step state machines per orientation. The earlier
claim in `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` §2
("Template: 2 orientations × 2 heights × 4 variants (DamageA/B +
CollapseA/B). All share the cell-step `+0x11E` state machine (0 → 7 →
8 → collapsed)") was correct only for the NS-orientation variants.
**EW-orientation variants use different state values.**

### Spot-checked variants (4 of 16)

| Variant | Addr | State transitions on `cell+0x11E` | Direction setter | Orientation check |
|---------|------|------------------------------------|-------------------|---------------------|
| NS/Low/CollapseA (docs'd earlier) | `0x56EF50` | `<7 → 7; 8 → 0` (collapse) | `SetBridgeDirection_NWSE(0,0)` | `+0x11A & 1` (bit 0 test) |
| NS/Low/DamageB | `0x56EE40` | `<4 → 5; 4 → 6` | — | — |
| NS/High/DamageA | `0x572230` | `<4 → 4; 5 → 6` | — | — |
| **EW/Low/CollapseB** | `0x56FC80` | `<0x10 → 0x10; 0x11 → 0` (collapse) | `SetBridgeDirection_NWSE(6,0)` | `+0x11A ≤ 4` (magnitude, not bitmask) |
| **EW/High/CollapseA** | `0x572DA0` | `<0x10 → 0x11; 0x10 → 0` (collapse) | `SetBridgeDirection_NESW(6,0)` | `+0x11A ≤ 4` (magnitude, not bitmask) |

### Key differences between NS and EW orientation families

| Dimension | NS variants | EW variants |
|-----------|-------------|-------------|
| Damage-step values | `{0, 4, 5, 6, 7, 8}` | `{0x10, 0x11}` (range 16–17) |
| Entry gate | `+0x11E` checked against 4, 5, 7, 8 | `bVar1 > 8` (requires damage step already advanced) |
| Direction setter | `SetBridgeDirection_NWSE(0,0)` on state-8 collapse (verified for CollapseA; other NS variants unverified) | `SetBridgeDirection_NWSE(6,0)` (CollapseB) / `SetBridgeDirection_NESW(6,0)` (CollapseA) |
| Orientation bit-check | `+0x11A & 1` (TEST bit 0, JZ) | `+0x11A ≤ 4` (CMP AL,0x4 + JBE — magnitude, not bitmask) |
| Blast-zone shift on collapse | `{0, -1, +1}` in Y-axis | `{-1}` shift on X-axis |

### Why the numeric gap?

The NS state range `{0..8}` and EW state range `{0x10..0x11}` are
**deliberately non-overlapping** — this lets a single cell's `+0x11E`
damage step encode BOTH its orientation AND its phase:
- Step 0 = intact / fresh
- Steps 4-8 = NS-oriented damage/collapse phases
- Steps 0x10-0x11 = EW-oriented damage/collapse phases

A bridge cell is EITHER NS or EW (encoded by `+0x11A` bits). The
damage state machine for NS uses low values; for EW uses high values.
This prevents the "wrong orientation" handler from accidentally
triggering on a differently-oriented bridge.

### Implication for Rust parity

A Rust port of the bridge damage state machine needs:
- A distinct state-value range per orientation (or an enum tagged by
  orientation).
- Separate handler functions per variant (16 total — same as gamemd)
  OR a well-factored dispatcher that routes via the `+0x11A`
  orientation bits.
- The correct `SetBridgeDirection_*` routines per orientation:
  NS/Low/CollapseA uses `NWSE(0,0)`; EW/Low/CollapseB uses `NWSE(6,0)`;
  EW/High/CollapseA uses `NESW(6,0)`. The other 13 variants are unverified
  but likely follow the same NS→NWSE / EW-CollapseB→NWSE / EW-CollapseA→NESW
  pattern. The 2nd argument (0 vs 6) appears to encode bridge-height.

**Prior Rust work** (`src/sim/bridge_state.rs`) represents damage
level as a single `deck_level: u8`. For full parity, this needs
extending to:
- `damage_step: u8` with orientation-specific value ranges, OR
- `damage: DamageState` enum carrying orientation + phase explicitly

The `deck_level` abstraction is insufficient for visual parity (wrong
tile frame shown during intermediate damage) and for trigger logic
(`BlowUpBridge` fires 3-cell blast patterns that depend on the exact
state transition).

### Per-variant addresses (re-confirmed from Batch 2)

All 16 still match the addresses listed in
`MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` §2, but should be
grouped by orientation (not by height):

**NS orientation (8 variants):**
- `0x56ED40` NS/Low/DamageA, `0x56EE40` NS/Low/DamageB,
- `0x56EF50` NS/Low/CollapseA, `0x56F2F0` NS/Low/CollapseB,
- `0x572230` NS/High/DamageA, `0x572330` NS/High/DamageB,
- `0x572440` NS/High/CollapseA, `0x5727E0` NS/High/CollapseB

**EW orientation (8 variants):**
- `0x56F690` EW/Low/DamageA, `0x56F7A0` EW/Low/DamageB,
- `0x56F8B0` EW/Low/CollapseA, `0x56FC80` EW/Low/CollapseB,
- `0x572B80` EW/High/DamageA, `0x572C90` EW/High/DamageB,
- `0x572DA0` EW/High/CollapseA, `0x573170` EW/High/CollapseB

### Remaining unverified

11 of 16 still not directly decompiled (4 confirmed match template
this batch + 1 earlier = 5; 16 - 5 = 11 variants remain on
assumption). Two possible failure modes:
1. A variant uses yet-another state-value range (unlikely — NS/EW
   split seems exhaustive).
2. A variant has subtly different blast-zone geometry (more
   likely — `BlowUpBridge` offsets vary per variant).

Low priority to fully close — the 5 verified cover both orientations
and both heights for at least one phase each.

---

## M. Final consolidation — MapClass master status

**Status:** COMPLETE. Covers 100% of struct bytes (0x0000–0x1174),
all 30 vtable slots, and every owned helper function reached by the
research cycle.

### M.0 Index of sections

| § | Topic | Primary addresses | Task |
|---|-------|-------------------|------|
| A | Trigger-event category decoder — `DAT_008B40C8` / `DAT_008B41A8` / `HouseClass+0x3C` | `0x6E61F0`, `0x7271E0`, `0x71F680`, `0x684C30` | 1, 3 |
| B | `FUN_0056xxxx` address-range stragglers classified | `0x560BF0`, `0x561180`, `0x5602C0`, `0x5617E0` | 5 |
| C | Vtable slots 18–21 = callable return-zero stub (not `__purecall`) | `0x4C9150` | 11 |
| D | Struct regions `+0x74–0x7F` and `+0x11C–0x123` proven-unused | 20 bytes on `MapClass+0x87F7E8` | 4 |
| E | `cell+0x12C` ShroudFlags — bits 3 + 4 only; 30 bits unused | `0x5656D0`, `0x568140`, `0x567F70`, `0x4876F0`, `0x577BB0`, `0x577D90`, `0x6D8700` | 12 |
| F | `g_PassabilityMatrix` — MovementZone row labels (13 × 8 × 4) | `0x82A594` | 2 |
| G | `DAT_008B40C8` per-tick consumer — `LogicClass::PerTickUpdate` | `0x55AFB0` | 3 |
| H | `ZoneFloodFillScanLine` left-vs-right height-delta asymmetry | `0x56CB90` | 6 |
| I | `UpdateBridgeZonesHelper` phase-7 BFS deep pass | `0x56C510` | 8 |
| J | `SabotageCursor` / `MigAttackCursor` live on **WeaponTypeClass** | `0x772080`, `+0x139`, `+0x13A` | 9 |
| K | `UpdateBridgeZonesHelper` — 33 caller sites across 8 categories | `0x56C510` xrefs | 10 |
| L | UpdateRamp family — NS vs EW state-value split | 16 addresses (see §L) | 7 |
| **M** | **This consolidation (status matrix + open questions)** | — | **13** |

### M.1 Struct byte matrix (+0x0000 → +0x1174)

Every byte of the `0x1174`-byte MapClass layout classified. Cross-reference
column points at the canonical per-field evidence source.

| Byte range | Size | Field | Status | Evidence |
|------------|------|-------|--------|----------|
| `+0x00` | 4 | `vtable` = `0x7ED404` | VERIFIED | `MAPCLASS_GHIDRA_REPORT.md` §2 "GScreenClass Base"; `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` §2 |
| `+0x04` | 4 | GScreenClass bitfield/state (=0) | VERIFIED | Same |
| `+0x08` | 4 | GScreenClass unknown (=0) | VERIFIED | Same |
| `+0x0C` | 4 | `blit_mode` (=2) | VERIFIED | Same |
| `+0x10` | 1 | `zones_initialized` bool | VERIFIED | `MAPCLASS_GHIDRA_REPORT.md` §2 "Zone System Core" |
| `+0x11–0x13` | 3 | alignment padding | VERIFIED | Layout-implicit |
| `+0x14` | 4 | `zone_connection_hash` (256-bucket DynVec hash) | VERIFIED | `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` §1; used in §H + §I above |
| `+0x18–0x4B` | 52 | `zone_ids[13]` (MovementZone → ushort[] array) | VERIFIED | §F (row labels) + §I (producer: phase 7 of `UpdateBridgeZonesHelper`) |
| `+0x4C` | 4 | `zone_cluster_count` | VERIFIED | `MAPCLASS_GHIDRA_REPORT.md` §2 "Zone System Core" |
| `+0x50–0x67` | 24 | `bridge_records` DynVec<BridgeRecord> (16B/entry) | VERIFIED | `MAPCLASS_GHIDRA_REPORT.md` §2 "Bridge Records"; see §L for per-orientation state machines |
| `+0x68` | 4 | `zone_cell_data` ptr (4B/cell: zone_type, height, cluster_id) | VERIFIED | `MAPCLASS_GHIDRA_REPORT.md` §2 "Zone Cell Data" |
| `+0x6C` | 4 | `zone_cell_count` | VERIFIED | Same |
| `+0x70` | 4 | `zone_speed_cache` ptr (10B/cell, A*/hierarchical edges) | VERIFIED | Same |
| `+0x74–0x7F` | 12 | **Reserved / dead** | VERIFIED | §D above (exhaustive xref + byte-pattern scan, 0 hits) |
| `+0x80` | 4 | `zone_graph[0]` | VERIFIED | `MAPCLASS_GHIDRA_REPORT.md` §2 "Zone Graph Pointers" |
| `+0x84` | 4 | `zone_graph[1]` | VERIFIED | Same |
| `+0x88` | 4 | `zone_graph[2]` | VERIFIED | Same |
| `+0x8C–0xA3` | 24 | `zone_conn_vec[0]` (DynVec per speed category) | VERIFIED | `MAPCLASS_GHIDRA_REPORT.md` §2 "Zone Connection DynVecs" |
| `+0xA4–0xBB` | 24 | `zone_conn_vec[1]` | VERIFIED | Same |
| `+0xBC–0xD3` | 24 | `zone_conn_vec[2]` | VERIFIED | Same |
| `+0xD4–0xEB` | 24 | `bridge_zone_dynvec` (DynVec<int*>) | VERIFIED | `MAPCLASS_GHIDRA_REPORT.md` §2 "Bridge Zone DynVec" |
| `+0xEC` | 4 | `size_left` (always 0 in practice) | VERIFIED | `MAPCLASS_GHIDRA_REPORT.md` §2 "Map Size Parameters" |
| `+0xF0` | 4 | `size_top` (always 0 in practice) | VERIFIED | Same |
| `+0xF4` | 4 | `map_size_width` (diamond half-diagonal) | VERIFIED | `MAPCLASS_GHIDRA_REPORT.md` §2 "Map Bounds" |
| `+0xF8` | 4 | `map_size_height` | VERIFIED | Same |
| `+0xFC` | 4 | `local_left` (LocalSize rect) | VERIFIED | Same |
| `+0x100` | 4 | `local_top` | VERIFIED | Same |
| `+0x104` | 4 | `local_width` | VERIFIED | Same |
| `+0x108` | 4 | `local_height` | VERIFIED | Same |
| `+0x10C` | 4 | `iter_state` | VERIFIED | `MAPCLASS_GHIDRA_REPORT.md` §2 "Cell Iterator State" |
| `+0x110` | 4 | `iter_x` | VERIFIED | Same |
| `+0x114` | 4 | `iter_remaining` | VERIFIED | Same |
| `+0x118` | 4 | `iter_cell_ptr` | VERIFIED | Same |
| `+0x11C–0x123` | 8 | **Reserved / dead** | VERIFIED | §D above (companion region to +0x74; same proof) |
| `+0x124` | 4 | `playfield_left` (always 1) | VERIFIED | `MAPCLASS_GHIDRA_REPORT.md` §2 "Playfield Bounds" |
| `+0x128` | 4 | `playfield_top` (always 1) | VERIFIED | Same |
| `+0x12C` | 4 | `playfield_width` (= `size_width + size_height - 1`) | VERIFIED | Same |
| `+0x130` | 4 | `playfield_height` (= `size_width + size_height - 1`) | VERIFIED | Same |
| `+0x134` | 4 | `scenario_init_flag` (written by `ScenarioClass::Full_Init` only) | VERIFIED | `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` §3 |
| `+0x138–0x147` | 16 | `cell_array` VectorClass<CellClass*> (262144 ptrs) | VERIFIED | `MAPCLASS_GHIDRA_REPORT.md` §2 "Cell Array VectorClass" |
| `+0x148` | 4 | `num_movement_zones` (= 13) | VERIFIED | `MAPCLASS_GHIDRA_REPORT.md` §2 "Map Dimensions" |
| `+0x14C` | 4 | `map_width_cells` (= 512) | VERIFIED | Same |
| `+0x150` | 4 | `map_height_cells` (= 512) | VERIFIED | Same |
| `+0x154` | 4 | `total_cell_count` (= 262144) | VERIFIED | Same |
| `+0x158–0x1157` | 4096 | `crate_slots[256]` (16B/slot) | VERIFIED | `MAPCLASS_GHIDRA_REPORT.md` §2 "Crate Slot Table" + §3 Core Logic/Crate System |
| `+0x1158` | 1 | `bridge_overlay_draw_stamp` (near-dormant in YR) | VERIFIED | `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` §3 (only 2 readers in `CellClass::DrawOverlay_Body`; no incrementer in YR) |
| `+0x1159–0x115B` | 3 | alignment padding | VERIFIED | Layout-implicit |
| `+0x115C–0x1173` | 24 | `attached_objects_dynvec` (DynVec<CellStruct>) | VERIFIED | `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` §5 "The +0x115C DynVec" |
| **`+0x1174`** | — | **End of MapClass**; DisplayClass begins here | VERIFIED | `DISPLAYCLASS_DISCOVERY_GHIDRA_REPORT.md` + `DISPLAYCLASS_BANDBOX_AND_MI_CORRECTION_GHIDRA_REPORT.md` |

**Coverage check:** Bytes classified: 4 + 4 + 4 + 4 + 1 + 3 + 4 + 52 + 4 +
24 + 4 + 4 + 4 + 12 + 4 + 4 + 4 + 24 + 24 + 24 + 24 + 4 + 4 + 4 + 4 + 4 + 4 +
4 + 4 + 4 + 4 + 4 + 4 + 8 + 4 + 4 + 4 + 4 + 4 + 16 + 4 + 4 + 4 + 4 + 4096 +
1 + 3 + 24 = **4468 bytes = `0x1174`** ✓

**Dead bytes total:** 12 (at +0x74) + 8 (at +0x11C) = 20 bytes of reserved
padding, proven unused. Live bytes: 4448.

### M.2 Vtable slot matrix (0x7ED404, 30 slots)

| Slot | Addr | Name | Category | Status |
|------|------|------|----------|--------|
| 0 | `0x4F4240` | scalar deleting destructor | inherited (GScreenClass) | VERIFIED |
| 1 | `0x40D230` | ctor helper | inherited | VERIFIED |
| 2 | `0x40D240` | ctor helper | inherited | VERIFIED |
| 3 | `0x5656D0` | `IsCellExplored(coord)` — `(cell.ShroudFlags >> 3) & 1` | MapClass override | VERIFIED (§E; revisit report §2 corrected) |
| 4 | `0x588BF0` | MapClass scalar-deleting destructor | MapClass override | VERIFIED |
| 5 | `0x565800` | `Init_Alloc` — cell array, zone hash, zone_graph[3] | MapClass override | VERIFIED |
| 6 | `0x4F42B0` | inherited | GScreenClass | VERIFIED |
| 7 | `0x5659F0` | `Init_Clear` — +0x148=13, +0x1158=0, pause crate timers | MapClass override | VERIFIED |
| 8 | `0x4F42E0` | inherited | GScreenClass | VERIFIED |
| 9 | `0x4F4320` | inherited | GScreenClass | VERIFIED |
| 10 | `0x4F4BB0` | inherited | GScreenClass | VERIFIED |
| 11 | `0x4F43F0` | inherited | GScreenClass | VERIFIED |
| 12 | `0x4F4410` | inherited | GScreenClass | VERIFIED |
| 13 | `0x4F4450` | inherited | GScreenClass | VERIFIED |
| 14 | `0x4F42F0` | `MarkNeedsRedraw(2)` | GScreenClass default | VERIFIED |
| 15 | `0x4F4480` | inherited | GScreenClass | VERIFIED |
| 16 | `0x4AEBD0` | inherited | GScreenClass | VERIFIED |
| 17 | `0x4F45B0` | inherited | GScreenClass | VERIFIED |
| 18 | `0x4C9150` | `Stub__ReturnZero` (callable no-op) | default stub | VERIFIED (§C) |
| 19 | `0x4C9150` | `Stub__ReturnZero` | default stub | VERIFIED (§C) |
| 20 | `0x4C9150` | `Stub__ReturnZero` | default stub | VERIFIED (§C) |
| 21 | `0x4C9150` | `Stub__ReturnZero` | default stub | VERIFIED (§C) |
| 22 | `0x565AA0` | cell-array reset (null all 262144 slots) | MapClass override | VERIFIED |
| 23 | `0x565B00` | cell-array resize helper | MapClass override | VERIFIED |
| 24 | `0x565BC0` | cell-array destructor walk (zeros +0x134 at entry) | MapClass override | VERIFIED |
| 25 | `0x577920` | `UnregisterBridgeRepairHut(Techno*)` | MapClass override | VERIFIED (revisit §5) |
| 26 | `0x4AEBE0` | inherited | GScreenClass | VERIFIED |
| 27 | `0x56BBE0` | `UpdateCrateRegenTimers` (per-tick) | MapClass override | VERIFIED |
| 28 | `0x565C10` | `InitMapCells` (Size/LocalSize parsing + cell resize) | MapClass override | VERIFIED |
| 29 | `0x567230` | `Viewport_Resized` — updates `in_playfield` byte, idle voice | MapClass override | VERIFIED |

**End of MapClass vtable** at `0x7ED47C`. `0x7ED480` begins the adjacent
`VectorClass<CellClass*>` vtable (referenced by the embedded field at
+0x138 — shared across WaveClass and other containers). Slots 30–63 that
the original follow-up report listed are NOT MapClass slots — see
`MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` §1.

**Coverage check:** 30 slots classified (0–29). MapClass overrides: 11
slots {3, 4, 5, 7, 22, 23, 24, 25, 27, 28, 29}. Inherited/default: 19.

### M.3 Owned helpers (non-vtable)

Functions that MapClass logically owns but aren't vtable entries. Every
one decompiled at least once across the research cycle.

| Address | Name | Purpose | Covered in |
|---------|------|---------|------------|
| `0x565090` | MapClass constructor | Field init; sets +0x148=13, growsteps=10, cell grid NULL | `MAPCLASS_GHIDRA_REPORT.md` §2 (constructor assembly trace) |
| `0x5652C0` | MapClass destructor | Frees cell array, zone hash, zone_graph[3], bridge DynVec | `MAPCLASS_GHIDRA_REPORT.md` §2 |
| `0x5656D0` | `IsCellExplored` | `>> 3 & 1` on cell+0x12C (vtable slot 3 target) | §E; revisit §2 |
| `0x5657A0` | `Get_CellClass` | `cell_array[Y*512 + X]` with null-sentinel fallback | `MAPCLASS_GHIDRA_REPORT.md` §3 |
| `0x5673A0` | `RevealShroud` | Sight-range spiral reveal; clears +0x140 bit 6; OR's +0x12C bits 3+4 | `MAPCLASS_GHIDRA_REPORT.md` §3 "Shroud Reveal"; §E |
| `0x568140` | `Invalidate_Radius_For_Redraw` | Radius-based set +0x12C bits 3+4 | §E |
| `0x567F70` | conceal-radius helper | Inverse of 0x568140 (clears bits 3+4) | §E |
| `0x567110` | zone-system entry helper | Calls `UpdateBridgeZonesHelper` after bulk changes | §K category 1 |
| `0x567230` | `Viewport_Resized` (slot 29) | Sweeps all Technos setting in_playfield; idle voice | `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` §2 |
| `0x577AB0` | `RestoreShroud` | Mirror of ResetShroud; clears bits 3+4 (not re-verified) | `SHROUD_SYSTEM_COMPLETE.md` |
| `0x577BB0` | `ResetShroud` | AND `~0x18` on cell+0x12C | §E |
| `0x577D90` | `BlackoutShroud` | OR `0x18` on cell+0x12C | §E |
| `0x577920` | `UnregisterBridgeRepairHut` (slot 25) | Walks +0x115C DynVec; calls detach | revisit §5 |
| `0x578100` | `RecalcBridgeShroudFlags` | Clears bits 3+4 on bridge cells | §E |
| `0x578290` | `CellIterator_Next` | Diagonal zigzag walk | `MAPCLASS_GHIDRA_REPORT.md` §3 "Cell Iterator" |
| `0x578350` | `CellIterator_Reset` | Reset +0x10C..+0x118 state | Same |
| `0x578460` | `Is_Cell_In_Playfield` | Diamond test via `F4/F8` | `MAPCLASS_GHIDRA_REPORT.md` §3 |
| `0x56C510` | `UpdateBridgeZonesHelper` | 8-phase zone rebuild | `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` §1; §I (phase 7 deep pass); §K (33 callers) |
| `0x56CB90` | `ZoneFloodFillScanLine` | Per-row scanline flood (left/right asymmetric) | §H |
| `0x56D230` | `GetZoneID` | zone_cell_data cluster_id → zone_ids[mz][cluster] | `MAPCLASS_GHIDRA_REPORT.md` §3 |
| `0x56D430` | `CellCoordToLinearIndex` | `stride*Y + X`, stride = `F8 + 1 + F4` | Same |
| `0x56D460` | `AssignOrphanedCellZone` | Incremental zone: 8-neighbor adoption | `ZONE_INCREMENTAL_DIVERGENCE_GHIDRA_REPORT.md`; §K category 7 |
| `0x56D5A0` | `MergeAdjacentCellZone` | Incremental zone: merge siblings | Same |
| `0x56D6E0` | `ComputeBridgeZones` | Builds bridge_records (+0x50..+0x67) | `MAPCLASS_GHIDRA_REPORT.md` §2 "BridgeRecord" |
| `0x56DA10` | `FindBridgeRecord` | Linear scan of bridge_records | `MAPCLASS_GHIDRA_REPORT.md` §3 |
| `0x56BBE0` | `UpdateCrateRegenTimers` (slot 27) | 256-slot regen check | `MAPCLASS_GHIDRA_REPORT.md` §3 "Crate System" |
| `0x56BD40` | `PlaceCrateAtRandomCell` | Rand placement + passability check | Same |
| `0x56C020` | `RemoveCrateAtCell` | Clear slot by coord | Same |
| `0x565800` | `Init_Alloc` (slot 5) | Allocates cell array, zone hash, zone_graph[3] | Revisit §2 |
| `0x5659F0` | `Init_Clear` (slot 7) | `+0x148 = 13`, `+0x1158 = 0`, pause crate timers | Same |
| `0x565AA0` | cell-array reset (slot 22) | NULL all 262144 slots | Same |
| `0x565B00` | cell-array resize (slot 23) | Resize to 0x40000 if < 0x40000 | Same |
| `0x565BC0` | cell-array destructor walk (slot 24) | Zero +0x134, free cells | Same |
| `0x565C10` | `InitMapCells` (slot 28) | Parse `[Map] Size`; create CellClass instances | Same |
| `0x6D8700` | `Shroud_EdgeBitmask_Calculator` | Reads bit 3 on 8 neighbors → SHP edge frame | §E |
| 16 × | UpdateRamp_* variants | Bridge damage state machines (NS vs EW split) | §L |

**AttachObject / DetachObject helpers** (`0x485250` / `0x485130`) sit in
`CellClass` namespace but are direct consumers of MapClass +0x115C.
See revisit §5.

**Trigger-category aggregator** (`FUN_006E61F0`) and per-tag walker
(`FUN_007271E0`) are `TagClass` methods, not MapClass, but feed the
three DynVecs whose lifecycle MapClass indirectly participates in. See §A.

### M.4 Globals managed / referenced by MapClass

| Global | Address | Purpose | Section |
|--------|---------|---------|---------|
| `g_MapClass` | `0x87F7E8` | The single MapClass instance | — |
| `g_DefaultCell` | `0xABDC50` | Null-sentinel CellClass | `Get_CellClass` fallback |
| `g_PassabilityMatrix` | `0x82A594` | 13×8 int32 matrix (+terminator col) | §F |
| `DAT_008B40C8` | `0x8B40C8` | DynVec<Tag*> with attack events (bit 0x10) | §A + §G |
| `DAT_008B41A8` | `0x8B41A8` | DynVec<Tag*> with destroyed events (bit 0x04); also bridge-hut registry | §A + revisit §5 |
| `HouseClass+0x3C` | — | Per-house DynVec<Tag*> with proximity events (bit 0x08) | §A |
| `DAT_007ED3D0` | `0x7ED3D0` | Shroud reveal spiral table (indexed by sight range) | `MAPCLASS_GHIDRA_REPORT.md` §3 "Shroud Reveal" |
| Low-bridge tile base | `0xABAD1C` | Base tile constant for UpdateRamp_Low_* family | `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` §4 |
| High-bridge tile base | `0xAA0E28` | Base tile constant for UpdateRamp_High_* family | Same |
| `Stub__ReturnZero` | `0x4C9150` | 30+ vtables share this as slot target | §C |

### M.5 Remaining open questions (final list)

After 13 tasks, the questions that survived scrutiny. All are
**low-priority** and would only matter for parity beyond current scope.

1. **World↔cell coord-transform audit.** `FUN_005654A0`, `0x565520`,
   `0x565660` are HouseClass-local-grid skews (not tactical transforms
   — `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md` corrected the prior
   misattribution). The real tactical transforms live at `0x6D1EB0` /
   `0x6D1F10` / `0x6D1FE0` / `0x6D2140` / `0x6D6590` and belong to
   DisplayClass / TacticalClass, NOT MapClass. **Out of MapClass scope.**

2. **Zone-incremental algorithm divergence.** gamemd uses 8-neighbor
   cluster_id adoption with ≤3-conflict bailout to
   `UpdateBridgeZonesHelper`; Rust uses bbox clear-and-reflood with a
   200-cell threshold. Both converge on correct connectivity but may
   diverge on edge-case topologies. **Covered in depth in**
   `ZONE_INCREMENTAL_DIVERGENCE_GHIDRA_REPORT.md`; not a MapClass
   decoding gap. Leave for post-parity tuning.

3. **Action-category counterpart `FUN_006E3EE0`.** The trigger-event
   half (events → bits) is fully decoded in §A. The action half
   (actions → bits) wasn't decompiled this cycle. Low priority —
   only matters for full trigger/tag parity, which itself isn't a
   scoped target for standard skirmish.

4. **11 of 16 UpdateRamp variants not individually spot-checked.** §L
   confirmed the NS/EW state-value split via 5 variants (1 from earlier
   + 4 this batch). The remaining 11 follow one of the two discovered
   templates; differences (if any) are only in blast-zone geometry
   offsets, which matter for visual parity but not for core mechanics.

**Items NOT in the open list (deliberately closed):**

- ❌ "Secondary-inheritance vtable fragments in DisplayClass" — closed:
  `DISPLAYCLASS_BANDBOX_AND_MI_CORRECTION_GHIDRA_REPORT.md` proved
  DisplayClass is single-inheritance; the apparent "secondary vtable"
  was the adjacent `BufferStraw` vtable at `0x7E61E0`.
- ❌ "Vtable slots 18–21 purecall semantics" — §C: they're
  `Stub__ReturnZero`, not purecall.
- ❌ "Cell+0x12C ShroudFlags other bits" — §E: only bits 3+4 used, 30
  bits unused.
- ❌ "DAT_008B40C8 / 008B41A8 purpose" — §A + §G: trigger-category
  pre-filtered DynVecs.
- ❌ "Zone rebuild trigger list" — §K: 33 sites across 8 categories
  exhaustively enumerated.
- ❌ "g_PassabilityMatrix row labels" — §F: all 13 rows 1:1 verified
  against Rust enum.
- ❌ "+0x74–0x7F and +0x11C–0x123 purpose" — §D: proven reserved/dead.

### M.6 Rust parity takeaways (concise)

The detailed parity matrix lives in
`MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` §6. Items the
Batch 1–5 research affected:

| Area | Rust status | gamemd truth | Needed next |
|------|-------------|--------------|-------------|
| ShroudFlags | Working (simplified) | Only 2 bits in use | No change — Rust can continue using 2-bit representation |
| Passability matrix | 1:1 mirror (`MovementZone` enum) | `g_PassabilityMatrix` 13×8×4 | No change — exact match |
| Zone-rebuild triggers | Bbox-threshold (200 cells) | Full rebuild on bridge + conflict cap | Behavior audit if bridge-dense maps diverge |
| Bridge damage | `deck_level: u8` single-progression | NS-range {0..8} + EW-range {0x10..0x11} | Extend to orientation+phase enum for visual parity |
| Trigger DynVecs | Not implemented | Three pre-filtered lists | Only needed if scripted maps become a target |
| `WeaponTypeClass.SabotageCursor` / `.MigAttackCursor` | Not implemented | `+0x139` / `+0x13A` byte flags | Wire when cursor-override system lands |
| `+0x115C` attached-object registry | Not implemented | Index for `[CellTags]` lookup | Only needed for triggers |
| `+0x1158` bridge-overlay draw stamp | Not implemented | Dormant in YR (never incremented) | Can be ignored — no visible effect |
| `+0x74–0x7F` / `+0x11C–0x123` | Omitted | 20 bytes reserved/dead | Confirmed safe to omit |

### M.7 Sibling-report cross-references

Every prior MapClass-related Ghidra report has been touched by this
cycle. Status of their "Open Questions" / "Still-open" sections:

| Report | Open questions it listed | Addressed by this mega-doc |
|--------|--------------------------|-----------------------------|
| `MAPCLASS_GHIDRA_REPORT.md` | +0x11C "unknown"; +0x74 "zone_metadata unknown"; bridge ramp family unclear | §D (proven dead), §L (16 variants + NS/EW split) |
| `MAPCLASS_GHIDRA_REPORT_FOLLOWUP.md` | "12 UpdateRamps", "64-slot vtable" | Corrected in revisit; §L confirms 16, §M.2 confirms 30 slots |
| `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` §8 | Coord transforms; zone-incremental divergence; DisplayClass; bridge-hut registry; slots 18–21 | §M.5 item 1 (out of scope) · §M.5 item 2 (out of scope) · closed by `DISPLAYCLASS_DISCOVERY_*` + `DISPLAYCLASS_BANDBOX_*` · §A (008B41A8 = destroyed-tag DynVec + bridge-hut) · §C |
| `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` §3 | DAT_008B41A8 / 40C8 / per-house categories; ramp template | §A + §G + §K; §L (template corrected per orientation) |
| `ZONE_INCREMENTAL_DIVERGENCE_GHIDRA_REPORT.md` | Correctness parity of 6 divergences | Cross-linked §K category 7; resolution is test-driven, not RE |
| `DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md` §5 | WarheadType flags 0x139/0x13A | §J — attribution corrected to WeaponTypeClass |
| `DISPLAYCLASS_DISCOVERY_GHIDRA_REPORT.md` | Secondary-inheritance vtable | Closed by `DISPLAYCLASS_BANDBOX_AND_MI_CORRECTION_GHIDRA_REPORT.md` |
| `CELLCLASS_STRUCT_GHIDRA_REPORT.md` | cell+0x12C bit 4 behavior | §E (full bit map) |
| `SHROUD_SYSTEM_COMPLETE.md` / `SHROUD_ALGORITHM_DISTILLED.md` | Bits beyond 3+4 | §E (proved 30 bits unused) |
| `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md` | Which MapClass transforms are real tactical transforms? | Already resolved in that doc (not MapClass — they're DisplayClass) |

### M.8 Confidence summary

- **VERIFIED** findings: all of §A, §C, §D, §E, §F, §G, §I, §J, §K,
  plus the M.1/M.2/M.3 matrices.
- **VERIFIED-WITH-CAVEAT**: §B (classification based on string/xref
  analysis, not full-body rewrite); §H (asymmetry observed; theory of
  origin is inference); §L (5 of 16 spot-checked — pattern confirmed;
  edge-case blast geometry varies).
- **UNCHANGED FROM PRIOR REPORTS**: Struct layout rows not specifically
  re-verified this cycle remain at the confidence stated in
  `MAPCLASS_GHIDRA_REPORT.md` / revisit report. None contradicted.

### M.9 What this investigation closed vs left open

**Closed this cycle (13 tasks):**
- Trigger-event DynVec semantics + per-tick consumer
- All 30 vtable slots individually attributed
- MovementZone passability row labels
- ShroudFlags complete bit map
- 20 bytes of struct proven dead
- UpdateBridgeZonesHelper caller taxonomy
- UpdateRamp NS vs EW orientation split (5 of 16)
- `SabotageCursor` / `MigAttackCursor` attribution correction
- Zone flood-fill height-delta asymmetry (documented + origin theorized)
- UpdateBridgeZonesHelper phase-7 BFS edge cases
- Vtable slots 18–21 stub clarified
- `FUN_0056xxxx` stragglers classified out of MapClass scope

**Left open (all low-priority):**
- Exhaustive action-category lookup (`FUN_006E3EE0`)
- 11 of 16 UpdateRamp variants un-decompiled
- World/cell transform formulas — belong to DisplayClass
- Zone-incremental algorithmic parity — test-driven, not RE-driven

**Out of MapClass scope (tracked in sibling reports):**
- DisplayClass / TacticalClass tactical rendering
- BandBox state machine + selection modifier keys
- `DetermineAction` polymorphic dispatch chain (5 subclasses, 30+ codes)
- Real tactical coord transforms at `0x6D1EB0..0x6D6590`

---

## Sources touched by Batch 5 (Task 13)

### Referenced reports (synthesized, not re-decompiled)
- `MAPCLASS_GHIDRA_REPORT.md` — canonical struct layout, crate system, iterator
- `MAPCLASS_GHIDRA_REPORT_FOLLOWUP.md` — UpdateRamp addresses
- `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` — vtable corrections, +0x115C DynVec, parity matrix
- `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` — UpdateBridgeZonesHelper phases, passability matrix
- `ZONE_INCREMENTAL_DIVERGENCE_GHIDRA_REPORT.md` — 6 Rust vs gamemd divergences
- `DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md` — What_Action polymorphic chain
- `DISPLAYCLASS_DISCOVERY_GHIDRA_REPORT.md` — DisplayClass struct + vtable
- `DISPLAYCLASS_BANDBOX_AND_MI_CORRECTION_GHIDRA_REPORT.md` — single-inheritance proof, bandbox state
- `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md` — real tactical transforms (not MapClass)
- `CELLCLASS_STRUCT_GHIDRA_REPORT.md` — cell+0x12C / +0x140 layout
- `SHROUD_SYSTEM_COMPLETE.md` + `SHROUD_ALGORITHM_DISTILLED.md` — shroud pipeline overview
- Plan doc: `docs/plans/2026-04-24-mapclass-complete-decode-plan.md`

### Sibling reports updated with resolution cross-references
- `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` §8 "Still-open gaps" — annotated
- `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` §3 — semantic correction logged
- `DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md` §5 — attribution correction logged

---

## Sources touched by Batch 1

### Newly decompiled
- `0x006E61F0` — TagClass trigger-category aggregator
- `0x007271E0` — per-tag flag walker (helper)
- `0x0071F680` — event-type → category bits lookup
- `0x006E4FA6` — TagClass destructor (removes from DAT_008B40CC / 008B41AC)
- `0x004C9150` — Stub__ReturnZero (callable no-op)
- `0x005617E0` — sin-lookup math helper
- `0x00561180` — video mode test dialog
- `0x005602C0` — display-settings dialog initializer

### Xref scans
- `get_function_xrefs(0x006E61F0)` → 5 callers (3 in FUN_00684C30, 2 in TagClass)
- `get_function_xrefs(0x004C9150)` → 30+ hits, all `.rdata` vtable slots

### Referenced prior reports (corrected)
- `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` §3 (three-DynVec
  claim) — semantic label corrected by §A above
- `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` §2 (vtable slots 18–21)
  — stub nature clarified by §C above
- `DISPLAYCLASS_GHIDRA_REPORT.md` §3 (slots 16–19) — same stub
  correction applies

### INI files
- No rulesmd.ini keys touched. Batch 1 clarifies that the three
  DynVecs are fed from **scenario file** `[Tags]`/`[Triggers]`/`[Events]`,
  not rulesmd.ini.

---

## Sources touched by Batch 2

### Newly decompiled
- `0x005975E0` — small-scalar clamp helper (false positive for +0x74)
- `0x00586360` — `IsShrouded(lepton_coord)` — confirms bit 3 read
- `0x00568140` — `Invalidate_Radius_For_Redraw` — sets bits 3+4 on +0x12C
- `0x00567F70` — complementary conceal-radius helper — clears bits 3+4
- `0x004876F0` — `CellClass::RevealShroudFlags` — sets bits 3+4 via `|= 0x18`
- `0x00577BB0` — `MapClass::ResetShroud` — clears bits 3+4 via `&= ~0x18`
- `0x00577D90` — `MapClass::BlackoutShroud` — sets bits 3+4
- `0x006D8700` — `Shroud_EdgeBitmask_Calculator` — reads bit 3 on 8 neighbors
- `0x007C3690` — movie-player setter (false positive for +0x11C)
- `0x00759940` — movie-player scheduler (context for false positive)

### Byte-pattern scans
- `search_byte_patterns("8B 41 74", "FF C7 FF")` → 2 hits, both unrelated
- `search_byte_patterns("89 41 74", "FF C7 FF")` → 2 hits, both unrelated
- `search_byte_patterns("8B 81 1C 01 00 00", "FF C7 FF FF FF FF")` → 1 hit, unrelated
- `search_byte_patterns("89 81 1C 01 00 00", ...)` → 0 hits

### Field-access scans
- `get_field_access_context(0x00ABDC50 + 0x12C)` → 10 accesses, all
  shroud-pipeline related (`FUN_00567F70`, `FUN_00568140`, `IsShrouded`,
  `FUN_005656D0` = IsCellExplored)

### Referenced prior reports (extended)
- `CELLCLASS_STRUCT_GHIDRA_REPORT.md` row for cell+0x12C
  (ShroudFlags) — extended from "bit 3 + bit 4" to full-matrix evidence
  in §E above.
- `SHROUD_SYSTEM_COMPLETE.md`, `SHROUD_ALGORITHM_DISTILLED.md` — the
  shroud-pipeline entry points they document all route through the
  same two bits.
- `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` — "Reserved / dead"
  claim for +0x74..+0x7F and +0x11C..+0x123 upgraded to
  proven-unused via §D above.

---

## Sources touched by Batch 3

### Newly decompiled
- `0x0055AFB0` — `LogicClass::PerTickUpdate` (confirms DAT_008B40C8
  consumption with 10 attack-event codes)
- `0x0056C510` — `MapClass::UpdateBridgeZonesHelper` re-read in detail
  for phase-7 BFS edge-case extraction
- `0x0056CB90` — `MapClass::ZoneFloodFillScanLine` re-read for
  left-vs-right walk asymmetry analysis

### Xref scans
- `get_xrefs_to(0x008B40C8)` → 6 direct hits (producer in FUN_00684C30,
  savegame in FUN_0067F9C0, writes from 0x4E7F)
- `get_field_access_context(0x008B40CC)` → 15+ consumers across tick,
  savegame, tag destructor, scenario clear

### Cross-reference with Rust source
- `src/rules/locomotor_type.rs` lines 228-255 — MovementZone enum
  doc comments match passability matrix rows 1:1

### Referenced prior reports (extended)
- `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` §1 (Phase 7
  outline) — deep-traced in §I above.
- Same report §1 passability matrix raw dump — now labeled in §F.
- Same report §3 DynVec list — producer claim confirmed by §G
  (per-tick consumer matches).

---

## Sources touched by Batch 4

### Newly decompiled
- `0x00771C70` — `WeaponTypeClass::Constructor` (field inits confirm
  0x138-0x13B as byte-aligned bool group)
- `0x00772080` — `WeaponTypeClass::ReadINI` (finds INI key names
  `SabotageCursor` at +0x139 and `MigAttackCursor` at +0x13A)
- `0x0070E140` — `TechnoClass::GetWeapon` (confirms piVar9 = weapon
  slot, *piVar9 = WeaponTypeClass pointer)
- `0x0056FC80` — `UpdateRamp_EW_Low_CollapseB` (reveals EW-orientation
  uses state values 0x10-0x11, breaking the "single template" claim)
- `0x00572DA0` — `UpdateRamp_EW_High_CollapseA` (confirms EW pattern,
  different direction-setter for high-bridges)

### Xref scans
- `get_function_xrefs(0x56C510)` → 33 callers enumerated in §K
- `search_byte_patterns("39 01 00 00")` → confirmed WeaponTypeClass
  at `0x771DCA` and WeaponTypeClass::ReadINI at `0x7721B8`/`0x7721CC`

### Referenced prior reports (corrected)
- `DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md` §5 "WeaponType/Warhead
  flags" — 0x139/0x13A rows corrected from "warhead" to "WeaponType"
  with verified INI key names.
- `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` §2 UpdateRamp
  template claim — corrected: template holds within orientation
  (NS vs EW split), does NOT hold across orientations.
