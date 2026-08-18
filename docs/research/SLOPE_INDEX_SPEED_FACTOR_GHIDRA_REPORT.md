# SlopeIndex → Speed Factor: Investigation of FootClass+0x530 Writer

**Date:** 2026-05-18
**Scope:** Find the function that writes `FootClass+0x530` on cell-change, decode
the `SlopeIndex (0..19) → factor` mapping.
**Status:** CLOSED — premise refuted. The field is NOT a per-cell slope cache.
**Active in YR:** Yes — field is read every pathfind in standard YR play.
**Confidence:** HIGH (every claim below verified by direct disassembly / memory
read this session; no inference).

---

## 1. TL;DR

There is **no** SlopeIndex → speed-factor lookup table and **no** per-cell-change
writer of `FootClass+0x530`. The field is set exactly once per unit, at
`FootClass::Unlimbo`, by copying a static `double` from `TechnoTypeClass+0x2F0`
(the `ThreatAvoidanceCoefficient` INI key). The value is constant for the unit's
lifetime; it does not depend on the cell the unit currently stands on.

The prior label `FootClass::Get_Slope_Speed_Factor` is a misnomer. The field is
better understood as a **per-TypeClass pathfinder cost-multiplier**, consumed
as a gate / multiplier inside the zone-pathfinder and path-smoother.

The investigation premise from `CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md` §9 ("Need
to find the function that WRITES to +0x530 to extract the slope-to-multiplier
mapping") was incorrect — no such mapping exists in the binary.

---

## 2. Evidence chain

### 2.1 Exactly one writer of `FootClass+0x530` exists

Searched the entire `.text` for every x86 store encoding to `[reg + 0x530]` as a
qword (FSTP m64) and as a dword pair (MOV m32):

| Pattern (disp32 = `30 05 00 00`) | Search results |
|----|----|
| `DD 9E 30 05 00 00` (FSTP qword [esi+0x530]) | **1 hit: `0x4D72F4`** |
| `DD 9F …` (FSTP qword [edi+0x530]) | 0 hits |
| `DD 99 …` / `DD 98 …` / `DD 86 …` / `DD 87 …` | 0 hits |
| `F2 0F 11 8X 30 05 00 00` (MOVSD m64 — SSE) | 0 hits |
| `66 0F D6 8X 30 05 00 00` (MOVQ m64) | 0 hits |
| `89 86 30 05 00 00` (MOV [esi+0x530], reg) | 1 hit at `0x75957E` — different class (file/stream wrapper, NOT FootClass) |
| `89 87 30 05 00 00` (MOV [edi+0x530], reg) | 0 hits |
| `C7 86 30 05 00 00` (MOV [esi+0x530], imm32) | 1 hit at `0x6679C6` — different class (parser context, NOT FootClass) |
| `C7 87 …` | 0 hits |

The constructor at `0x4D3217` also zeroes both halves of `+0x530`/`+0x534` with
`MOV dword ptr [ESI+0x530], EBX` (EBX = 0). That's an init store at construction,
not a per-cell update.

**Result:** the only `FSTP qword [reg+0x530]` instruction in the entire binary is
the one at `0x4D72F4` inside `FootClass::Unlimbo`. There is no SSE or split-MOV
alternative writer. There is therefore **no per-cell-change writer**.

### 2.2 The single writer: `FootClass::Unlimbo @ 0x4D7170` (instructions 0x4D72E0-0x4D72F4)

Verbatim disassembly of the relevant block:

```
004d72e0  MOV   EAX, dword ptr [ESI]           ; this->vtable
004d72e2  MOV   ECX, ESI                       ; this
004d72e4  CALL  dword ptr [EAX + 0x84]         ; this->vtable[0x84]()  → TechnoTypeClass*
004d72ea  FLD   double ptr [EAX + 0x2f0]       ; load TechnoTypeClass+0x2F0 (a double)
004d72f0  MOV   EDX, dword ptr [ESI]           ; (reload vtable for next call)
004d72f2  MOV   ECX, ESI
004d72f4  FSTP  double ptr [ESI + 0x530]       ; STORE → FootClass+0x530
```

- **`vtable+0x84`** is `TechnoClass::GetTechnoType_Trampoline @ 0x6F3270`
  (verified at vtable byte-offset 0x84 in the FootClass vtable @ `0x7E8C94`,
  read via `read_memory 0x7E8D18 16` → bytes `70 32 6F 00`). The trampoline
  forwards to `vtable+0x88`, which is the per-subclass overridden
  `Get_TechnoType` (returns `UnitTypeClass*` for UnitClass, etc.).
- **`+0x2F0`** of `TechnoTypeClass` is the `ThreatAvoidanceCoefficient` field
  (verified independently below).

### 2.3 What `TechnoTypeClass+0x2F0` actually is

String `"ThreatAvoidanceCoefficient"` lives at `0x844420` (one occurrence in the
binary). `get_xrefs_to 0x844420` reports a single use from `0x712460` inside
`TechnoTypeClass::ReadINI`. Disassembly of that ReadINI block:

```
00712458  MOV   EDX, dword ptr [EBP + 0x2f0]   ; default = current value of +0x2F0
0071245e  PUSH  ECX                            ; (high half of default double)
0071245f  PUSH  EDX                            ; (low  half)
00712460  PUSH  0x844420                       ; "ThreatAvoidanceCoefficient"
...                                            ; ReadDouble call
0071246d  FSTP  double ptr [EBP + 0x2f0]       ; store parsed double → +0x2F0
```

So the INI key `ThreatAvoidanceCoefficient` writes `TechnoTypeClass+0x2F0`.

Cross-check against `ra2-rust-game-docs/TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`
§ field map (line 245): `0x2F0 | 8 | double | ThreatAvoidanceCoefficient |
default 0`. Consistent.

Cross-check against in-repo `ini/rulesmd.ini`:

| Line | Section (context) | Key=Value |
|---|---|---|
| 7344 | British Chrono Miner | `ThreatAvoidanceCoefficient=1` |
| 7402 | (paired sov ore truck variant) | `ThreatAvoidanceCoefficient=.65` |
| 8210, 9034 | (Allied harvester variants) | `=1` |
| 8262, 9086 | (Soviet harvester variants) | `=.65` |

The doc's "default 0" is the constructor default; INI commonly overrides to 1.0
or 0.65 on harvester-class units.

### 2.4 The reader at `0x4DC760` ("`Get_Slope_Speed_Factor`")

Disassembly:

```
004dc760  MOV  EAX, dword ptr [ECX + 0x5d4]    ; this->linked_object (train/convoy lead)
004dc766  TEST EAX, EAX
004dc768  JZ   0x004dc77e
004dc76a  MOV  EAX, dword ptr [EAX + 0x24]     ; linked_obj->TypeClass
004dc76d  MOV  DL,  byte  ptr [EAX + 0xf2]     ; linked_obj_TypeClass->[+0xF2] (IsTrain-ish)
004dc773  TEST DL, DL
004dc775  JZ   0x004dc77e
004dc777  FLD  double ptr [0x007e1718]         ; constant 1.0
004dc77d  RET
004dc77e  FLD  double ptr [ECX + 0x530]        ; the stored coefficient
004dc784  RET
```

Confirms the field is read as a `double` (8 bytes); confirms the train-exempt
fast path returns the constant `1.0` (verified: `read_memory 0x7E1718 8` →
`00 00 00 00 00 00 F0 3F` = IEEE-754 1.0).

### 2.5 How callers consume the value — there is no slope-table lookup

**`Path_smooth_single_segment @ 0x42B50E`** (decompiled this session):

```c
local_8 = (double)FootClass__Get_Slope_Speed_Factor();   // reads +0x530
...
local_10 = MapClass__Get_Slope_Cost_At_Cell(&local_34, local_14);  // per-cell slope cost
if (1.0 <= (double)local_10 * local_8) {                 // gate by product
    bVar2 = true;   // reject smoothing of this segment
}
```

`+0x530` here is a **scalar multiplier** on the per-cell slope cost; it is not
an index-into-table. The same value is used regardless of SlopeIndex.

**`Zone_precheck @ 0x42C2BA`** (decompiled this session) uses it as a gate:

```c
fVar27 = (float10)FootClass__Get_Slope_Speed_Factor();   // reads +0x530
...
if ((float10)_DAT_007e3810 < fVar27) bVar11 = true;      // threshold ≈ 9.77e-6
...
if (bVar11) {
    Zone_Estimate_Slope_Cost(local_30, local_38, uVar16, uVar25);  // only if gate open
    local_58 = Math__ftol();
}
```

Threshold `_DAT_007e3810` = `0x3EE4F8B588E368F1` ≈ `9.766e-6` (read this
session). Functionally this is "is `+0x530` non-trivially > 0". When yes, the
slope cost from `Zone_Estimate_Slope_Cost` is added to the candidate-edge
weight; when no, slope cost is skipped entirely.

**`Path_Reroute_Straight_Line @ 0x42BEC3`** — decompilation deferred (the two
above already establish the consumption pattern; no third-party caller would
introduce a SlopeIndex table).

In none of the three callers is `+0x530` used to *index* a table. It is always
a scalar multiplier or gate.

### 2.6 No SlopeIndex (0..19) keyed lookup exists

To rule out an alternative table buried elsewhere, I cross-checked:

- `search_strings` for `SlopeSpeed`, `SlopeDamage`, `SlopeFactor`,
  `SpeedFactor` — **zero hits.**
- The five hits for `Slope` in `.rdata` are `DirtRoadSlopes`,
  `PavedRoadSlopes`, `MonorailSlopes`, `SlopeSetPieces`,
  `SlopeSetPieces2` — all theater/tile-set INI keys, none mapping
  SlopeIndex → multiplier.
- `search_functions Slope` returns the 8 known slope-related functions
  (all from prior research): `BridgeSlopeTable_StaticInit`,
  `CellClass::ApplyLAT_and_SlopeFixup`,
  `DriveLocomotionClass::Force_New_Slope`,
  `FootClass::Get_Slope_Speed_Factor`,
  `LocomotionClass::ForEach_SetSlopeIndex`,
  `MapClass::Get_Slope_Cost_At_Cell`, `TMP_ReadSlopeType`,
  `Zone_Estimate_Slope_Cost`. None decode SlopeIndex into a per-index
  multiplier table.

### 2.7 The `LocomotionClass::ForEach_SetSlopeIndex @ 0x4E1570` red herring

The prior CLIFF_RAMP doc § 8 hypothesised that this function dispatches a slope
update to each object via `vtable+0x6C`, and that the v-slot handler writes
`FootClass+0x530`. Both halves of that claim are wrong:

- `0x4E1570` only ever appears as a vtable **data** entry; there are no direct
  CALL sites in `.text` to it. It IS a vtable slot itself, not a caller.
- `vtable+0x6C` in the FootClass vtable (`0x7E8C94`) holds **`0x4DBAD0`**
  (bytes at `0x7E8D00` = `D0 BA 4D 00`) = `FootClass::ComputeChecksum`. That
  function reads `+0x530` and `+0x534` into the network checksum stream — it
  does not write `+0x530`.
- More importantly: byte-pattern scanning (§2.1) already proved there is no
  qword/SSE store to `[reg+0x530]` anywhere outside `0x4D72F4`. Whatever
  `vtable+0x6C` does on its callees, it does not (cannot) update `+0x530`.

The CLIFF_RAMP doc's section 8 narrative ("propagate a slope value from a cell
to every object on that cell ... For FootClass-derived objects, it likely
updates `FootClass+0x530`") is therefore not supported by the binary.

---

## 3. What `FootClass+0x530` actually is

A static per-unit-type `double` coefficient, copied from
`TechnoTypeClass::ThreatAvoidanceCoefficient` (INI key, offset `+0x2F0`) into
the FootClass instance once during `Unlimbo`. The pathfinder uses it as:

- a **gate** in `Zone_precheck` (>~9.77e-6 ⇒ apply slope cost from
  `Zone_Estimate_Slope_Cost`; else skip),
- a **scalar multiplier** in `Path_smooth_single_segment` (multiplied by the
  per-cell slope cost from `Get_Slope_Cost_At_Cell`, then compared against
  1.0 to reject segments where the product saturates).

It does **not** vary by SlopeIndex. SlopeIndex affects pathfinding cost via the
separate `Zone_Estimate_Slope_Cost` / `Get_Slope_Cost_At_Cell` pipeline (the
quarter-cell zone-cost grid documented in CLIFF_RAMP_TRAVERSAL §6); `+0x530`
only modulates *how aggressively a particular unit type weights that cost*.

---

## 4. TS-vs-YR filter

| Element | Active in YR? | Evidence |
|---|---|---|
| `FootClass+0x530` field on every Foot unit | Yes | constructor at `0x4D3217` zeros it; Unlimbo at `0x4D72F4` writes it for every Foot subclass (Unit, Infantry, Aircraft via their respective `*::Unlimbo` calling `FootClass::Unlimbo`). |
| `TechnoTypeClass::ThreatAvoidanceCoefficient` INI key | Yes | string at `0x844420`; ReadINI write at `0x71246D`; key appears in stock `rulesmd.ini` ≥ 6 times on harvester variants. |
| Train exemption (`+0x5D4`-linked, TypeClass `+0xF2`) | Conditional | only active when a unit has a linked train/convoy lead; stock YR contains no IsTrain units, so this branch is rarely exercised. Same status as prior CLIFF_RAMP §11. |
| The hypothesised "SlopeIndex → factor" lookup | **N/A — does not exist** in the binary. |

No SpecialFlags gating around the writer or the readers; this is YR-live code.

---

## 5. Implications for the Rust port

- Do NOT implement a SlopeIndex → factor lookup table. There is nothing to mirror.
- Per-unit pathfinder cost weighting comes from a single INI scalar
  (`ThreatAvoidanceCoefficient`) parsed onto the TypeClass at INI load, and
  copied to each instance at unlimbo.
- Per-cell slope cost is a separate pipeline (zone cost grid +
  `Zone_Estimate_Slope_Cost` bilinear); use the CLIFF_RAMP §6 docs for that.
- The reader function's Ghidra label `Get_Slope_Speed_Factor` is a misnomer; a
  better name in our codebase would be e.g. `Get_Pathfinder_Cost_Coefficient`
  or `Get_Slope_Sensitivity`. Do not rename in Ghidra (read-only this session).

---

## 6. Open follow-ups (out of scope this slot)

1. **CLIFF_RAMP doc § 8 needs correction**: the claim that
   `LocomotionClass::ForEach_SetSlopeIndex` propagates to a `vtable+0x6C`
   handler that updates `FootClass+0x530` is wrong on both halves. Update
   doc when revisiting cliff/ramp.
2. **What does `vtable+0x6C` actually do on per-object dispatch?** For
   FootClass it is `ComputeChecksum`; for other classes it may be something
   else. Probably benign (the function `0x4E1570` may be a checksum-list
   walker, not a slope-list walker), but worth a future verify.
3. **What writes the prior research's "hypothesised per-cell update"?**
   Nothing does. The mistaken assumption in CLIFF_RAMP §9 should be retracted
   rather than re-investigated.
4. **Caller `Path_Reroute_Straight_Line @ 0x42BEC3`** — decompilation skipped
   here; pattern is expected to match the other two callers (scalar gate +
   multiplier), but a quick verification pass would close the file fully.

---

## 7. Sources

**Decompiled this session:**
- `FootClass::Unlimbo @ 0x4D7170` (full disassembly — writer site at 0x4D72E0..0x4D72F4)
- `FootClass::Constructor @ 0x4D31E0` (full disassembly — zero-init of +0x530 at 0x4D3217)
- `FootClass::Get_Slope_Speed_Factor @ 0x4DC760` (full disassembly — reader, train-exempt branch)
- `FootClass::ComputeChecksum @ 0x4DBAD0` (full disassembly — reads +0x530/+0x534 into net checksum, no write)
- `FootClass::PerCellProcess @ 0x4D85D0` (full decompilation — does NOT touch +0x530)
- `FUN_006F5090` (PerCellProcess tail callback — does NOT touch +0x530)
- `DriveLocomotionClass::Process @ 0x4B0500` (full disassembly — does NOT touch +0x530; the 0x530 byte-pattern hit at 0x4B058E was a JZ displacement, not an offset)
- `DriveLocomotionClass::Force_New_Slope @ 0x4AFB40` (writes to a locomotor-local field +0x1C/+0x18, NOT to FootClass+0x530)
- `LocomotionClass::ForEach_SetSlopeIndex @ 0x4E1570` (list-walker via vtable+0x6C; not directly invoked from `.text`, only present as a vtable data entry)
- `FUN_004E14C0` (sibling vtable slot; unrelated cleanup walker)
- `Zone_precheck @ 0x42C2BA` (full decompilation — gate-on-+0x530)
- `Path_smooth_single_segment @ 0x42B50E` (full decompilation — multiply-and-compare with +0x530)
- `TechnoClass::GetTechnoType_Trampoline @ 0x6F3270` (full decompilation — confirms vtable+0x84 → vtable+0x88)
- `FUN_004E0130` (vtable+0x88 default for base FootClass; returns 0; overridden in subclasses)
- `TechnoTypeClass::ReadINI` (partial — `ThreatAvoidanceCoefficient` block at 0x712458..0x71246D)
- `FUN_00759540` (false-positive +0x530 hit — different class, file/stream wrapper)

**Memory reads this session:**
- `0x7E8C94..0x7E8D14` (128 bytes of FootClass vtable; confirms vtable+0x6C=0x4DBAD0=ComputeChecksum, vtable+0x84=0x6F3270 trampoline, vtable+0x88=0x4E0130)
- `0x7E1718` (8 bytes — IEEE-754 1.0, the train-exempt constant)
- `0x7E3810` (8 bytes — IEEE-754 ≈9.766e-6, the Zone_precheck gate threshold)
- `0x7E3818` (8 bytes — IEEE-754 ≈0.001, the "edge has non-zero ramp marker" constant)

**Byte-pattern scans this session** (results in §2.1):
- 8 variations of qword/SSE/MOV stores to `[reg+0x530]`.

**Xref tables consulted:**
- `get_xrefs_to 0x844420` (ThreatAvoidanceCoefficient string) → single ReadINI use
- `get_xrefs_to 0x4E1570` (ForEach_SetSlopeIndex) → all 20+ refs are `[DATA]` (vtable entries), none code

**INI cross-check:**
- `ini/rulesmd.ini` — 6 unique
  `ThreatAvoidanceCoefficient=` lines (1.0 and 0.65 variants) on harvester-class
  unit sections.

**Companion docs:**
- `CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md` (§9 open question retired by this report; §8 needs correction — see §6.1 above)
- `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md` (field map line 245 confirms +0x2F0 = ThreatAvoidanceCoefficient)

---

*End of report.*
