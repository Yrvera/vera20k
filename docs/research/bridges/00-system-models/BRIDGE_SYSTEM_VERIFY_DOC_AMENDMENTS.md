# BRIDGE_SYSTEM.md — Verify-Doc Amendment List

Audit date: 2026-05-18. Source: live Ghidra MCP decompilation against gamemd.exe.
Doc is NOT modified — corrections enumerated here for manual application.

## Tally

- VERIFIED: 17 load-bearing claims
- WRONG (must amend): 4
- UNVERIFIABLE / flagged: 1

---

## WRONG — must amend

### W1. Line 18 — `+0x11A` is NOT `bridge_sub_type`

**Doc claim:** `+0x11A | byte | bridge_sub_type | Bridge body orientation / sub-type`

**Binary evidence:**
- Ghidra `CellClass` struct (`get_struct_layout`) defines offset 282 (0x11A) as
  field name `Height` (1 byte). Not bridge-specific.
- `CellOverlay_TileDraw @ 0x00480350` reads `*(undefined1 *)(param_1 + 0x11a)`
  unconditionally and passes it as the sub-tile (icon) index to
  `TMP_TileBlitter`. Path is taken for ALL terrain (clear cells skip the load
  but still pass `uVar1=0`). Confirms universal use, not bridge-only.
- `CellClass::RecalcAttributes` writes `this->Height = 0` (raw bytes
  `88 86 1A 01 00 00` = `MOV [ESI+0x11A], AL` at `0x47D5E9`) when clearing a
  cell — applies to any cleared cell, not bridges.

**Corrected wording:**
`+0x11A | byte | sub_tile_index | IsoTile sub-tile (icon) index — universal terrain field, used by ALL tile types (sand, grass, slope, water, bridges). Bridge-rim matchers compare it to specific slot literals (e.g., 2, 4, 5, 7, 8, 12). Damage state lives at +0x11E.`

---

### W2. Line 36 — Bit `0x0400` is NOT "Bridge rail/guard post"

**Doc claim:** `10 | 0x0400 | Bridge rail/guard post | SetBridgeDirection`

**Binary evidence:**
- `SetBridgeDirection_NESW @ 0x47E040` decompile shows
  `cVar14 = (char)param_3; param_3 = (uint)(cVar14 == '\0') << 10;`
  — bit `0x400` is SET when collapse-state byte param_3==0 (destroyed),
  CLEARED otherwise (alive). Mutually exclusive with bit `0x100`.
- No rendering reader uses bit `0x400` (no rail/guard-post draw call gated on
  this bit was found).
- Per slot-4 `CELL_FLAGS_0x400_SEMANTIC_GHIDRA_REPORT.md`: tested in
  `DestroyBridge_*_OnHutDeath @ 0x5742E4 / 0x574F00` and
  `UpdateAdjacentBridges_High @ 0x576770` as the destroyed-state body marker.

**Corrected wording:**
`10 | 0x0400 | Bridge body cell, destroyed state (mutually exclusive with bit 0x100 "alive"). Used by hut-death and damage walkers to identify collapsed bridge body cells. | SetBridgeDirection (set when collapse param=0, cleared when alive)`

---

### W3. Line 58 — "Process_Drive_Track ramp detection (0x004b1812): `SUB EAX, 4`"

**Doc claim:** address `0x004b1812` contains `SUB EAX, 4`.

**Binary evidence:**
- `read_memory 0x4b180c` = `00 00 0F BE 8B 1B 01 00 00 83 E8 04 89 5C 24 40`
- `0F BE 8B 1B 01 00 00` = `MOVSX ECX, byte ptr [EBX+0x11B]` starting at `0x4b180f`.
- `83 E8 04` = `SUB EAX, 4` starts at **`0x4b1819`**, not `0x4b1812`.

**Corrected wording:**
Replace `0x004b1812` with `0x004b1819`. Operation `SUB EAX, 4` is correct.

---

### W4. Line 59 — "A* pathfinding start/goal height (0x00429b77): `ADD ECX, 4`"

**Doc claim:** address `0x00429b77` contains `ADD ECX, 4`.

**Binary evidence:**
- `read_memory 0x00429b72` = `8B 88 40 01 00 00 F6 C5 01 74 36 8D 8B 9C 00 00`
- Bytes at `0x00429b77` decode to `F6 C5 01 74 36` = `TEST CH, 1; JZ +0x36`.
  No `ADD ECX, 4` at this address.
- The `+4` bridge-height arithmetic IS present in `AStar_main_loop @ 0x00429b00`
  as decompiled C: `iVar13 = *(char *)(iVar13 + 0x11b) + 4;` (twice) and
  `*(int *)(param_1 + 0x30) = *(int *)(param_1 + 0x30) + 4;` — but the named
  assembly instruction is at a different offset. The pattern
  `MOVSX ECX, [EAX+0x11B]; ADD ECX, 4` exists at `0x4ACA2E` and `0x62CD08`,
  not at `0x429b77`.

**Corrected wording:**
Either (a) remove the specific address citation and reference
`AStar_main_loop @ 0x00429b00` with the C-level expression, or (b) cite
`0x004aca2e` (verified `MOVSX ECX,[EAX+0x11B]; ADD ECX, 4` pattern). The
`0x429b77` address is wrong.

---

## UNVERIFIABLE — flag for user

### U1. Line 622 — BridgeExplosions list: `TWLT026, TWLT036, TWLT050, TWLT070`

The decompile of `BlowUpBridge @ 0x47DD70` confirms the array pointer at
`g_RulesClass_Instance + 0x15c` is used and indexed via `RandomRanged(0, count-1)`
with count at `+0x168` (BridgeVoxelMax — note: also used as count divisor, which
is itself worth a footnote in the doc). However, the actual *string contents*
of the array (TWLT026 vs other anim names) were not re-confirmed in this audit —
the doc may have been verified earlier against the INI defaults. Recommend
re-confirming the four names by inspecting the array at `*(int *)(rules+0x15c)`
plus dumping the AnimType names at each pointer if precise names matter.

---

## VERIFIED (load-bearing claims confirmed)

CellClass field table (lines 14–27):
- `+0x24` packed_cell (MapCoord_X/Y) ✓ (Ghidra struct offset 36)
- `+0x38` IsoTileTypeIndex ✓ (struct offset 56)
- `+0x11B` height_level (`Level`) ✓ (struct offset 283)
- `+0x11C` SlopeIndex ✓ (struct offset 284)
- `+0xE4` FirstObject / ground occupant ✓
- `+0xE8` AltObject / bridge occupant ✓
- `+0xEC` LandType ✓
- `+0x124` OccupationFlags ✓
- `+0x128` AltOccupationFlags ✓
- `+0x140` Flags ✓

Cell Flag table (lines 33–43):
- Bit `0x80` set by SetBridgeDirection (not RecalcAttributes) ✓ — confirmed by
  `param_3 & 1) << 7` write in 0x47E040 and absence from 0x47D5E9.
- Bit `0x100` written by SetBridgeDirection ✓ (`uVar9 << 8`).
- Bit `0x200` written by SetBridgeDirection ✓ (`uVar9 << 9`).
- Bit `0x800` (NS axis) written by SetBridgeDirection ✓ (`(param_2 == 0) << 0xb`).
- Bit `0x10000` set by RecalcAttributes ✓ (`Flags | 0x10000` near end).
- Bit `0x20000` set by RecalcAttributes ✓ (`Flags | 0x20000`, tile anim placed).

Height arithmetic & thresholds:
- `CheckBridgeTraversal (0x004d9c60)`: function exists at this address ✓.
- Threshold `CMP EAX, 1; JLE 0x0073f0e8` at `0x0073f0dc` ✓ — bytes
  `83 F8 01 7E 07`.
- Threshold `CMP EAX, 1; JG 0x00429e7f` at `0x00429e75` ✓ — bytes
  `83 F8 01 7F 05`.
- Threshold `CMP EAX, 2; JG 0x004b1f28` at `0x004b1f1e` ✓ — bytes
  `83 F8 02 7F 05`.

Rules table & BlowUpBridge:
- `Rules+0x1740` BridgeStrength write ✓ — `MOV [ESI+0x1740], EAX` at
  `0x66cd86` (doc cites `0x66cd88`, 2-byte drift; instruction itself is
  correct).
- `Rules+0xFA8` C4Warhead used as bridge-damage warhead ✓ — passed as
  `*(undefined4 *)(g_RulesClass_Instance + 0xfa8)` to ReceiveDamage in
  BlowUpBridge.
- Phase 2 debris Z offset `0x600` ✓ — both AnimClass constructor calls in
  `BlowUpBridge @ 0x47DD70`.
- BridgeExplosions 1-5 frame random start delay ✓ — `RandomRanged(1,5)`.
- Lists `+0x140` MetallicDebris, `+0x14C` count, `+0x15C` BridgeExplosions,
  `+0x168` BridgeVoxelMax — pointer offsets all confirmed in BlowUpBridge.

---

## Notes

- The two known contradictions (W1 and W2) are confirmed and have the highest
  user-visible impact: another doc relying on either claim would build the
  wrong sub-tile index or the wrong flag-bit semantic.
- The two address-drift errors (W3, W4) are smaller — operations exist but
  citations are stale. Worth fixing because doc presents itself as "verified
  against gamemd.exe via Ghidra MCP" and any future grep-for-address audit
  would fail at those exact lines.
- No "stubbed-for-Phase-2" / unverified-status labels were used in this
  amendment list (per CLAUDE.md verification discipline).
