# g_SpeedType_LandType_Table — Ghidra Research Report

**Date:** 2026-05-18
**Active in YR:** Yes — table is populated from `rulesmd.ini` at scenario init and consulted every tick by every moving unit's locomotor speed calculation, plus by `CellClass::RecalcZoneType` for impassability classification.
**Confidence:** HIGH overall (base address, layout, indexing, all 12 row × 9 column values verified directly from binary + INI this pass)

This doc closes the open question in `MOVEMENT_CLASSIFIERS_REFERENCE.md` §8 ("Open question (deferred): Address of `g_SpeedType_LandType_Table` itself") and `WATER_SHORE_EDGE_TRANSITIONS_GHIDRA_REPORT.md` §9.1 ("g_SpeedType_LandType_Table values for Hover-on-Water, Amphibious-on-Water, Amphibious-on-Beach. Not extracted this pass.").

---

## 1. Address and layout

| Item | Value | Evidence |
|---|---|---|
| **Table base** (canonical indexing base) | **`0x0089EA40`** | `FLD float ptr [EDX*0x4 + 0x89ea40]` at `0x006a32f2` (ShipLocomotion), `0x004b3ca3` (DriveLocomotion), `0x0073fab5` (UnitClass::Can_Enter_Cell), `0x004835de` (CellClass::CheckCellPassability), `0x0047ca58` (Cell_passability_building_placement), `0x007400a1` (UnitClass::What_Action_OnObject), `0x00740c9c` (UnitClass::Mission_Hunt) |
| **Loader EBX init** | `0x0089EA44` | `MOV EBX,0x89ea44` at `0x0067400c` in `RulesClass__ReadSpeedTypeLandTypeTable @ 0x00674000` |
| **RecalcZoneType reference base** | `0x0089EA48` | `FLD float ptr [EAX*0x4 + 0x89ea48]` at `0x00483ce8` and `0x00483d57`. This is `base + 8 = base + Wheel_column_offset` — see §3.2. |
| **Loop terminator** | `0x0089EBF4` | `CMP EBX,0x89ebf4` at `0x00674224` |
| **Row stride** | 9 floats = **36 bytes (0x24)** | `ADD EBX,0x24` at `0x0067421e` |
| **Row count populated** | **12** | 12-entry `g_LandTypeStringTable` at `0x00839D68` (each entry = pointer to LandType section name). Loop iterates ~13 times by raw count but the 13th call to `FUN_00526810(*ppuVar5)` fails to find a section by the (invalid) string and the row body is skipped. |
| **Reserved region** | `0x0089EA40` – `0x0089EBF0` | 12 rows × 36 bytes = 432 bytes; backed by BSS (all zeros at module-load; populated by loader on scenario init). |

**Note on the two base addresses:**
The data symbol `DAT_0089ea48` in Ghidra's auto-generated symbol table corresponds to the address used by `RecalcZoneType`'s impassability check — which is *not* the SpeedType-zero column. The actual table base (column 0 = Foot speed) is at `0x0089EA40`. The 8-byte offset to `0x0089EA48` lands on column 2 = Wheel for row 0. See §3.2 for why.

---

## 2. Column layout — verified by loader instruction-by-instruction

The loader `RulesClass__ReadSpeedTypeLandTypeTable @ 0x00674000` writes each row at fixed offsets relative to `EBX` (which advances by +0x24 per row). Decoding the disassembly:

| Col idx | Mem offset from row base (`EBX - 4`) | Loader write addr (row 0) | SpeedType | INI key | Source string |
|---|---|---|---|---|---|
| 0 | +0 | `0x89ea40` | **Foot** | `Foot=` | `0x81dbd4` |
| 1 | +4 | `0x89ea44` | **Track** | `Track=` | `0x81dbcc` |
| 2 | +8 | `0x89ea48` | **Wheel** | `Wheel=` | `0x81dbc4` |
| 3 | +12 | `0x89ea4c` | **Hover** | `Hover=` | `0x81dbbc` |
| 4 | +16 | `0x89ea50` | **Winged** | (hard-coded 1.0; no INI read) | — |
| 5 | +20 | `0x89ea54` | **Float** | `Float=` | `0x81dbac` |
| 6 | +24 | `0x89ea58` | **Amphibious** | `Amphibious=` | `0x81bb18` |
| 7 | +28 | `0x89ea5c` | **FloatBeach** | `FloatBeach=` | `0x81dba0` |
| 8 | +32 | `0x89ea60` | (not a speed — `Buildable=` byte) | `Buildable=` | `0x83d4b4` |

**Critical detail — the 9th column is NOT a speed entry.** It's a single byte (`MOV byte ptr [EBX + 0x1c], AL` at `0x0067421b`) that stores the result of `CCINIClass::ReadBool(section, "Buildable", default=0)`. The remaining 3 bytes of that 4-byte slot are padding. Any code reading `g_SpeedType_LandType_Table[8 + LT*9]` is interpreting a packed bool+padding as a float — which yields denormalized garbage and is NOT a valid 9th SpeedType. The 9-stride is a layout artifact (Buildable piggy-backed on the row), not a real 9th speed column.

The previous `MOVEMENT_CLASSIFIERS_REFERENCE.md` §8 comment ("the 9th slot is likely an unused padding entry (or a 'FloatExtended' reserved slot from TS that YR doesn't populate)") was a guess that turned out to be wrong: **the 9th slot is the `Buildable=` flag, repurposed as part of the per-LandType row.**

**Verified column→string mapping** (each "source string" address was read via `read_memory` this pass and matched against the loader's `PUSH <addr>` arguments):
- `0x81dba0` = `"FloatBeach"`
- `0x81dbac` = `"Float"`
- `0x81dbb4` = `"Winged"`
- `0x81dbbc` = `"Hover"`
- `0x81dbc4` = `"Wheel"`
- `0x81dbcc` = `"Track"`
- `0x81dbd4` = `"Foot"`
- `0x81bb18` = `"Amphibious"`
- `0x83d4b4` = `"Buildable"`

**Note on Winged:** The Winged column is hard-coded to **1.0** (`MOV dword ptr [EBX + 0xc],0x3f800000` at `0x00674148`). The loader does NOT consult any `Winged=` INI key. Aircraft always traverse every LandType at full base speed in the table; their slowing/altitude effects come from elsewhere (`JumpjetControls`, `Flight` locomotor). **Mod authors writing `Winged=50%` in `[Rough]` etc. will have zero effect.**

---

## 3. Indexing — `[SpeedType + LandType*9]` from base `0x89EA40`

### 3.1 Per-tick speed lookup (Process_Movement, Can_Enter_Cell)

Verified at `0x006a32e7..0x006a32f2` in `ShipLocomotionClass::Process_Movement`:

```asm
MOV EDX, dword ptr [EAX + 0x67c]   ; EDX = techno.TypeClass.SpeedType
LEA ECX, [ESI + ESI*0x8]            ; ECX = LandType * 9  (ESI = LandType)
ADD EDX, ECX                          ; EDX = SpeedType + LandType*9
FLD float ptr [EDX*0x4 + 0x89ea40]   ; load table[base + (ST + LT*9) * 4]
```

Identical pattern in `DriveLocomotionClass::Process_Movement @ 0x004b3ca3` and `UnitClass::Can_Enter_Cell @ 0x0073fab5` and four others. All use `TechnoTypeClass+0x67C` (= SpeedType) and base `0x0089EA40`.

C=HIGH (full instruction-level decode of read and write sites), I=HIGH (function names verified against existing labels — `ShipLocomotionClass__Process_Movement`, `DriveLocomotionClass__Process_Movement`, etc.), B=HIGH (consistent base address across 7 independent caller sites).

### 3.2 Impassability check (RecalcZoneType) — uses Wheel column, NOT Foot

`CellClass::RecalcZoneType @ 0x00483C80` has TWO checks that read the speed table:

1. **Overlay impassability (line 0x00483ce8):**
   ```asm
   LEA EDX,[EAX + EAX*0x8]             ; EDX = overlay.LandType * 9
   FLD float ptr [EDX*0x4 + 0x89ea48]   ; addr = 0x89ea48 + LT*36
   FCOMP float ptr [0x007e1748]         ; compare with 0.0 (FLOAT_007e1748)
   ```
   Exact `== 0.0` check. Sets `ZoneType = 6 (Impassable)` if true.

2. **Cell LandType impassability (line 0x00483d57):**
   ```asm
   LEA EAX,[EAX + EAX*0x8]              ; EAX = LandType * 9
   FLD float ptr [EAX*0x4 + 0x89ea48]   ; addr = 0x89ea48 + LT*36
   FCOMP double ptr [0x007e3808]        ; compare with 0.01 (verified by read_memory:
                                         ; bytes 7b 14 ae 47 e1 7a 84 3f = 0x3F847AE147AE147B = 0.01)
   ```
   `<= 0.01` check. Sets `ZoneType = 6 (Impassable)` if true.

**The base in both is `0x89EA48`** = `0x89EA40 + 8 bytes` = **column 2 = Wheel**.

**This is a correction to `MOVEMENT_CLASSIFIERS_REFERENCE.md` §4 and §8** which described the threshold as `g_SpeedType_LandType_Table[LandType * 9]` and assumed the SpeedType-0 (Foot) column. **The actual binary reads the Wheel (col 2) value for each LandType as the impassability indicator.** Per the matrix in §4 below, this changes which LandTypes get classified Impassable:

- `[Rough] Wheel=100%` → 1.0 > 0.01 → NOT impassable (falls through to ZoneType 0 = Ground). Foot=100% would have given the same answer here.
- `[Tiberium] Wheel=50%` → 0.5 > 0.01 → NOT impassable (passable for all by zone check, though Tiberium-specific damage applies). Foot=90% would also have passed.
- `[Beach] Wheel=0%` → 0.0 ≤ 0.01 → **WOULD be impassable**, but the LandType==6 special case earlier in the function sets ZoneType=3 (Beach) first, so the Wheel check is never reached for Beach cells.
- `[Rock] Wheel=0%` → 0.0 ≤ 0.01 → **IS impassable**. (Foot=0% too — same answer.)
- `[Wall] Wheel=0%` → 0.0 ≤ 0.01 → **IS impassable**. (Foot=0% too — same answer.)

For stock YR rulesmd.ini values, the choice of Wheel-vs-Foot column **does not change the impassability classification of any of the 12 LandTypes** — but a mod that set `Wheel=0%` on a row where Foot is nonzero (e.g., a custom rough that infantry can cross but no vehicles can) would classify the cell **Impassable for everyone** because of the Wheel-based check, *even though* infantry could theoretically traverse it. This is a real player-visible mod behavior, not a bookkeeping detail.

C=HIGH (instruction-level read of both check sites + threshold read_memory), I=HIGH, B=HIGH.

---

## 4. Verified table contents — 12 rows × 8 SpeedType columns + Buildable

Values sourced from `rulesmd.ini` sections `[Clear]`, `[Road]`, `[Water]`, `[Rock]`, `[Wall]`, `[Tiberium]`, `[Beach]`, `[Rough]`, `[Ice]`, `[Railroad]`, `[Tunnel]`, `[Weeds]` (verified by grep this pass — lines 30191–30330 in the repo's `ini/rulesmd.ini`). The loader's `ReadDouble` parses `N%` as `N/100`, then `min(value, 1.0)`.

LandType enum ordering matches `g_LandTypeStringTable @ 0x00839D68` (read this pass — 12 pointers, verified each by dereferencing): `Clear, Road, Water, Rock, Wall, Tiberium, Beach, Rough, Ice, Railroad, Tunnel, Weeds`.

| LT idx | Section | Foot | Track | Wheel | Hover | Winged | Float | Amphibious | FloatBeach | Buildable |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | `[Clear]` | 1.0 | 1.0 | 1.0 | 0.5 | **1.0** | 0.0 | 0.8 | 0.0 | yes |
| 1 | `[Road]` | 1.0 | 1.0 | 1.0 | 0.75 | **1.0** | 0.0 | 1.0 | 0.0 | yes |
| 2 | `[Water]` | 0.0 | 0.0 | 0.0 | 1.0 | **1.0** | 1.0 | 1.0 | 1.0 | no |
| 3 | `[Rock]` | 0.0 | 0.0 | 0.0 | 0.0 | **1.0** | 0.0 | 0.0 | 0.0 | no |
| 4 | `[Wall]` | 0.0 | 0.0 | 0.0 | 0.0 | **1.0** | 0.0 | 0.0 | 0.0 | no |
| 5 | `[Tiberium]` | 0.9 | 0.7 | 0.5 | 0.5 | **1.0** | 0.0 | 0.5 | 0.0 | no |
| 6 | `[Beach]` | 0.0 | 0.0 | 0.0 | 0.75 | **1.0** | 0.0 | 0.6 | 1.0 | no |
| 7 | `[Rough]` | 1.0 | 1.0 | 1.0 | 0.5 | **1.0** | 0.0 | 0.8 | 0.0 | yes |
| 8 | `[Ice]` | 0.5 | 0.8 | 0.5 | 1.0 | **1.0** | 0.0 | 0.5 | 0.0 | no |
| 9 | `[Railroad]` | 0.9 | 1.0 | 0.5 | 1.0 | **1.0** | 0.0 | 0.5 | 0.0 | no |
| 10 | `[Tunnel]` | 1.0 | 1.0 | 1.0 | 1.0 | **1.0** | 0.0 | 1.0 | 0.0 | no |
| 11 | `[Weeds]` | 0.5 | 0.7 | 0.5 | 1.0 | **1.0** | 0.0 | 0.5 | 0.0 | no |

(Winged column shown bold to flag that the loader **does not read it from INI** — it is unconditionally `1.0`. The corresponding INI keys, if present, are ignored.)

**Open-question resolutions from previous docs:**

- `WATER_SHORE_EDGE_TRANSITIONS_GHIDRA_REPORT.md` §9.1:
  - **Hover-on-Water** = `[Water] Hover=100%` = **1.0** (full speed)
  - **Amphibious-on-Water** = `[Water] Amphibious=100%` = **1.0**
  - **Amphibious-on-Beach** = `[Beach] Amphibious=60%` = **0.6**
  - **Hover-on-Beach** = `[Beach] Hover=75%` = **0.75**

- `MOVEMENT_CLASSIFIERS_REFERENCE.md` §8: address resolved (`0x0089EA40`), stride-9 rationale resolved (col 8 = `Buildable=` flag, not a speed).

---

## 5. Non-trivial values worth flagging

Filtering the table for "interesting" entries (non-0, non-1, or unexpected combinations):

| Combination | Value | Player-visible effect |
|---|---|---|
| Hover × Clear | 0.5 | Hovercraft moves at HALF speed on land. Visible: Hover units crawl over land but zip across water at full speed. |
| Hover × Road | 0.75 | Hovercraft slightly faster on roads than off-road land (50% → 75%). |
| Hover × Beach | 0.75 | Hovercraft slows entering Beach from Water (1.0 → 0.75 → 0.5 onto Clear). The visible transition cadence as a Hover unit leaves the water. |
| Amphibious × Clear | 0.8 | Amphibious vehicles (e.g. SAPC if treated as Amphibious-speed not Hover-speed) move at 80% speed on land. |
| Amphibious × Beach | 0.6 | Amphibious slows MORE on beach than on clear land (80% → 60%). |
| Tiberium × Foot | 0.9 | Infantry slowed slightly in Tiberium. Track 70%, Wheel 50% — wheeled hardest hit. |
| Ice × Track | 0.8 | Tanks slower on ice than on Clear. Wheel 50%, Foot 50%, but Hover 100% (Hover ignores ice friction). |
| Weeds × Track | 0.7 | Tanks slowed in Weeds (TS-legacy LandType; YR maps rarely use). Hover 100%. |
| Railroad × Wheel | 0.5 | Wheeled units take penalty on train tracks. Track 100%, Foot 90%. |
| Railroad × Hover | 1.0 | Hover crosses railroad without penalty. |
| Float × any land | 0.0 | Ships cannot cross land at all (speed 0). Combined with the passability matrix at col 4 = Water, this is the "ships locked to water" mechanism on TWO axes (passability AND speed). |
| Winged × everything | 1.0 | Aircraft ignore terrain. The Winged column is loader-hard-coded — no INI override possible. |

**Zero rows for non-traversal:**
- Rock (LT=3) and Wall (LT=4) rows are all-zero except Winged=1.0. All ground/water locomotors have 0.0 speed, meaning even if passability matrix said "passable" the unit couldn't move. Aircraft (Winged=1.0) pass freely. This is the speed-side reinforcement of the cliff/wall barrier.

**Tunnel row (LT=10) is all-1.0 except Float=0.0** — this is TS-legacy subterranean. Per the user's `[[feedback_no_tunnel_subterranean]]` memory note, tunnels are not in stock YR play, but the values are still loaded into the table at scenario init.

**Beach Hover=0.75 is the key Hover-shore transition speed.** When a Hover unit crosses from Water (Hover=1.0) onto Beach (Hover=0.75) and then onto Clear (Hover=0.5), it visibly decelerates in two steps. This produces the characteristic "Hovercraft slowing as it makes landfall" feel.

---

## 6. Loader call path and active-in-YR confirmation

| Subsystem | Active in YR? | Evidence |
|---|---|---|
| `RulesClass__ReadSpeedTypeLandTypeTable @ 0x00674000` | Yes | Called unconditionally from `RulesClass::Process @ 0x00668bf0` (master rules INI processor), which is called from `ScenarioClass__Full_Init @ 0x00686b20` at every scenario load. |
| 12 INI sections present | Yes | All 12 sections (`[Clear]` etc.) present in repo `ini/rulesmd.ini` lines 30191–30330 (verified by grep this pass). |
| Per-tick reads from Drive/Ship/Walk locomotors | Yes | Every moving unit triggers Process_Movement every tick. All read base `0x89EA40`. |
| Impassability classification | Yes | `CellClass::RecalcZoneType @ 0x00483C80` called on every cell when terrain/overlay/object changes. Reads base `0x89EA48` (Wheel column). |
| Speed table itself loaded at runtime, BSS-zero at module load | Yes | `read_memory 0x0089EA40 length 468` returned all zeros this pass — confirming the binary is in unloaded state (no scenario running). |

**No TS-legacy gating** — the table is read unconditionally during normal YR initialization. The Winged-hardcoded-1.0 special case is not gated; it's always 1.0.

---

## 7. Open questions

1. **Why Wheel column for impassability?** The `RecalcZoneType` impassability check uses the Wheel speed (col 2 = `0x89EA48`) rather than Foot (col 0 = `0x89EA40`). This appears intentional but unexplained. Hypothesis: in TS, Wheel was the "lowest-common-denominator ground unit" — if a wheeled vehicle can't get >1% speed on this terrain, treat the cell as impassable for zone-classification purposes. Confirming via TS source or commentary would help, but the binary fact is verified. *Status: low priority — the resulting classification matches Foot for all stock YR LandTypes.*

2. **The hard-coded Winged = 1.0.** Was there an earlier version that read `Winged=` from INI? The current loader unconditionally writes 1.0 (`MOV dword ptr [EBX + 0xc],0x3f800000`) without any ReadDouble. *Status: noted for parity — mods writing `Winged=` are silently ignored.*

3. **Buildable column (col 8) is a byte, not a float.** Documented above but worth re-flagging: any code accessing `g_SpeedType_LandType_Table[8 + LT*9]` as a float is reading bytes `[bool, 0, 0, 0]` interpreted as IEEE-754 — almost certainly garbage. No such code was found this pass among the 13 xrefs to base `0x89EA40`; needs verification that no consumer ever index ST=8.

4. **Related but out-of-scope gaps** (not investigated per the narrow scope):
   - The `Buildable=` flag's consumer — what reads `g_SpeedType_LandType_Table[8 + LT*9]` (the byte)? Likely a building-placement check; address-trace would identify it.
   - The 12 LandType-to-TMP-tile mapping (which TMP tile IDs produce which LandType). Documented in `TODO_ZONE_FIDELITY_FIXES.md` as `TMP→LandType` table at `0x008288E4` — separate table, separate doc.
   - The 13th iteration of the loader loop reads an out-of-table string pointer at `g_LandTypeStringTable[12]`. Verified that the section-lookup `FUN_00526810` will reject the invalid pointer and skip the body, but worth confirming the 13th-slot bytes are stable (no spurious side-effect).

---

## 8. Sources

**Ghidra functions read this pass:**
- `RulesClass__ReadSpeedTypeLandTypeTable @ 0x00674000` — full disassembly (loader)
- `CellClass__RecalcZoneType @ 0x00483C80` — full disassembly (reader, impassability)
- `ShipLocomotionClass__Process_Movement @ 0x006A1C80` — assembly extracted around the table read at `0x006A32F2`
- Read-site assembly context for 4 additional callers via `get_assembly_context`

**Xrefs to base addresses (this pass):**
- `0x0089EA40` (canonical base, SpeedType-zero column): 13 xrefs (8 readers + 5 misc); all readers use `[ST*4 + 0x89EA40]` or `[LT*9*4 + 0x89EA40]` pattern.
- `0x0089EA44` (Track column): only loader xrefs.
- `0x0089EA48` (Wheel column / RecalcZoneType base): 3 xrefs (2 from RecalcZoneType, 1 from loader).

**Memory reads:**
- `0x0089EA40` length 468 → all zeros (BSS, binary not running a scenario)
- `0x0081DBA0` to `0x0081DBD8` → SpeedType column-name strings
- `0x0081DBC0` to `0x0081DC20` → LandType section-name strings
- `0x00839D68` length 48 → `g_LandTypeStringTable` 12 entries (each dereferenced and verified)
- `0x0081BAE8` → "Water" (verifies `g_LandTypeStringTable[2]`)
- `0x0081AC58` → "Wall" (verifies `g_LandTypeStringTable[4]`)
- `0x00839D98` → "Powerups" (verifies table ends at entry 12, no entry 13)
- `0x007E1748` → 0.0 (overlay impassable threshold)
- `0x007E1718` → 1.0 (loader clamp ceiling)
- `0x007E3808` → bytes `7b 14 ae 47 e1 7a 84 3f` = double `0.01` (LandType impassable threshold)

**INI cross-referenced:**
- `ini/rulesmd.ini` lines 30191–30330 — all 12 LandType sections with `Foot=/Track=/Wheel=/Hover=/Float=/Amphibious=/FloatBeach=/Buildable=` keys (verified by Grep + Read this pass).

**Companion docs:**
- `MOVEMENT_CLASSIFIERS_REFERENCE.md` §8 — primary cross-reference; this doc resolves its open question.
- `WATER_SHORE_EDGE_TRANSITIONS_GHIDRA_REPORT.md` §3 + §9.1 — Hover/Amphibious-on-Water value extraction.
- `CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md` §5 — references the table from Process_Movement context.
- `TODO_ZONE_FIDELITY_FIXES.md` — Rust-side implementation gaps; the speed-table values were a missing data input.
- `SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md` §8.6 — the per-tick speed computation formula that consumes this table.

**Confidence — 3-axis labelling per `[[feedback_research_confidence_axes]]`:**
- **Content** (table layout, indexing, values) = **HIGH**. Every claim is backed by either an instruction-level read from disassembly or a verified `read_memory` of strings, or a direct INI line citation.
- **Identity** (function names, struct fields) = **HIGH**. `RulesClass__ReadSpeedTypeLandTypeTable` name pre-existed in Ghidra's annotations; consistency with caller chains and INI-source semantics confirms it.
- **Binding** (which-callers-actually-call-this) = **HIGH**. 7 reader-call-sites identified by xref + assembly context, each verified by instruction pattern; loader call from `RulesClass::Process` confirmed via `get_function_callers`.

---

*End of report. The g_SpeedType_LandType_Table is fully characterized: base `0x0089EA40`, 12 rows × 9 slots (8 SpeedType floats + 1 Buildable byte), 36-byte stride, loaded from `rulesmd.ini` `[LandType-name]` sections at scenario init via `RulesClass__ReadSpeedTypeLandTypeTable @ 0x00674000`, consumed by all locomotors via `[ST + LT*9]` indexing and by `CellClass::RecalcZoneType` via the Wheel-column offset for impassability classification.*
