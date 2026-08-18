# Gate #3 Resolution — SpeedType/Passability Matrix (Track over Clear)

**Verdict:** CLOSED-PASS for the gate question; one structural DRIFT flagged for the Rust handoff.
**Question:** Does `SpeedType=Track` over `LandType=Clear` PASS — i.e. should
`is_passable_for_speed_type(Clear, Track) == true`?
**Answer:** Yes. `[Clear] Track=100%` → table value `1.0` (nonzero) → `UnitClass::Can_Enter_Cell`
does **not** return the impassable code. Confirmed from the binary table population + lookup + INI.

---

## 1. Confirmed function & data identities

| Symbol | Address | Confirmed via | Role |
|---|---|---|---|
| `UnitClass__Can_Enter_Cell` | `0x0073F0A0` | `get_function_by_address 0x0073F0A0`; `decompile_function 0x0073F0A0` | The terrain/SpeedType passability reader. Body literally reads the land-speed table and compares to 0.0. |
| land-speed table (base) | `0x0089EA40` | `disassemble_function 0x0073F0A0` @ `0x0073FAB5`; writer `0x00674000` uses `EBX=0x89EA44` | `float[12][9]` — 12 LandType rows × 9 columns; **0.0 = impassable**, nonzero = passable speed multiplier. |
| `RulesClass__ReadSpeedTypeLandTypeTable` (writer) | entry `0x00674000` (mid-body `0x006740B4`) | `get_xrefs_to 0x0089EA40` → `[WRITE] 0067f882`/`006740b4`; `disassemble_function 0x00674000` | Populates the table from `[LandType]` INI sections at rules load. |
| `g_LandTypeStringTable` | `0x00839D68` | `disassemble_function 0x00674000` @ `0x00674007` (`MOV ESI,0x839d68`); `read_memory 0x00839D68` | Array of 12 section-name pointers; entry[0] = `0x0081dc1c` = `"Clear"` (`read_memory 0x0081dc1c`). |
| `0x007E1748` (impassable sentinel) | `read_memory 0x007E1748` → `00000000` | float `0.0`; `Can_Enter_Cell` `FCOMP float ptr [0x007e1748]` | The value the table entry is compared against. |
| `SpeedType__ToName` | (decompiled) | `decompile_function SpeedType__ToName` → `g_SpeedTypeNameTable[param_1]` for `param_1 < 8` | 8 named SpeedTypes (0–7); table has a 9th packed column (Buildable). |

The label `UnitClass__Can_Enter_Cell` is **correct** for `0x0073F0A0`: the body computes
`cell+0xEC` (LandType) and `this->TechnoType+0x67C` (SpeedType), indexes a `*9`-stride float
table, and returns the impassable code on `== 0.0`. This matches a Can_Enter_Cell contract, not a label artifact.

---

## 2. Resolved behavior + assembly evidence

### 2.1 The lookup (read side), `0x0073F0A0`
From `disassemble_function 0x0073F0A0`, block at `0x0073FA9E`:

```asm
0073fa9e: MOV  EAX,[EDI + 0xec]            ; EAX = CellClass->LandType   (cell+0xEC)
0073faa4: MOV  EDX,[EBX + 0x6c4]           ; EDX = this->TechnoTypeClass (FootClass+0x6C4)
0073faaa: LEA  ECX,[EAX + EAX*0x8]         ; ECX = LandType * 9
0073faad: MOV  EAX,[EDX + 0x67c]           ; EAX = TechnoTypeClass->SpeedType (+0x67C)
0073fab3: ADD  ECX,EAX                     ; ECX = LandType*9 + SpeedType
0073fab5: FLD  float ptr [ECX*0x4 + 0x89ea40]  ; table[LandType*9 + SpeedType]  (base 0x0089EA40)
0073fabc: FCOMP float ptr [0x007e1748]     ; compare to 0.0
0073fabc..fac7: FNSTSW / TEST AH,0x40 / JZ ...
0073facd: MOV  EAX,0x7 ; ... RET           ; entry == 0.0  -> return 7 (Impassable)
```

So: **index = `LandType*9 + SpeedType`, element size 4 (float), base `0x0089EA40`. Entry `== 0.0` ⇒ impassable.**
(decompile_function 0x0073F0A0 shows the same as
`(&g_SpeedType_LandType_Table)[cell+0xec * 9 + TypeClass+0x67c] == FLOAT_007e1748`.)

### 2.2 The population (write side) — column ordering, `0x00674000`
From `disassemble_function 0x00674000`: `EBX = 0x89EA44` (= base+4 = column 1 of row 0),
stride `ADD EBX,0x24` (36 bytes = 9 floats/row), loop end `CMP EBX,0x89ebf4` →
`(0x89EBF4-0x89EA40)/0x24 = 12` rows. Per-row writes (relative to `EBX`):

| Col | Store insn (offset from EBX) | INI key (string addr → bytes) | Value |
|---:|---|---|---|
| 0 | `FSTP [EBX-4]` | `Foot=` (`0x0081dbd4` → "Foot\0", `read_memory`) | clamped ≤1.0 |
| **1** | `FSTP [EBX]` | **`Track=` (`0x0081dbcc` → "Track\0", `read_memory`)** | clamped ≤1.0 |
| 2 | `FSTP [EBX+4]` | `Wheel=` (`0x0081dbc4`) | clamped ≤1.0 |
| 3 | `FSTP [EBX+8]` | `Hover=` (`0x0081dbbc`) | clamped ≤1.0 |
| 4 | `MOV [EBX+0xC],0x3f800000` | hardcoded **1.0** (the index‑4 slot; SpeedType 4 = Fly/always‑pass in CheckCellPassability) | 1.0 const |
| 5 | `FSTP [EBX+0x10]` | `Float=` (`0x0081dbac`) | clamped ≤1.0 |
| 6 | `FSTP [EBX+0x14]` | `Amphibious=` (`0x0081bb18`) | clamped ≤1.0 |
| 7 | `FSTP [EBX+0x18]` | `FloatBeach=`? (`0x0081dba0`) | clamped ≤1.0 |
| 8 | `MOV byte [EBX+0x1C],AL` | `Buildable=` bool (`0x0083d4b4`) | packed bool (NOT a speed) |

This proves **SpeedType column index 1 = Track**, and explains stride 9 vs 8 SpeedType names:
column 8 is the packed `Buildable` bool, not a SpeedType speed value. Values are read as INI
percentages and clamped to a max of 1.0 (`_g_Const_1_0` / `0x007e1718`).

### 2.3 Row 0 = Clear, value for (Clear, Track)
`g_LandTypeStringTable[0] = 0x0081dc1c = "Clear"` (`read_memory 0x0081dc1c`), and `[Clear]` is the
first `[LandType]` section. From `ini/rulesmd.ini` `[Clear]`: `Foot=100% Track=100% Wheel=100%
Float=0% Hover=50% Amphibious=80% FloatBeach=0% Buildable=yes`.

⇒ `table[Clear(0)*9 + Track(1)] = 1.0`. In `Can_Enter_Cell`, `1.0 != 0.0` ⇒ this SpeedType/LandType
gate does **not** return Impassable. **(Track, Clear) PASSES.** (Cross-check Water row: `[Water]
Track=0%` ⇒ `0.0` ⇒ impassable, as expected.)

### Confirmed column/row map (the table this gate is about)

- **SpeedType columns (0..8):** `0 Foot · 1 Track · 2 Wheel · 3 Hover · 4 (const 1.0 / Fly slot) ·
  5 Float · 6 Amphibious · 7 FloatBeach · 8 Buildable-bool`.
- **LandType rows (0..11), INI order in `[LandType]`/string table:** `0 Clear · then Rough · Road ·
  Water · Rock · Wall · Tiberium · Weeds · Beach · Ice · Railroad · Tunnel` (12 rows; exact order is
  the pointer order in `g_LandTypeStringTable @ 0x00839D68`; only row 0 = Clear was byte-confirmed,
  the rest are the INI section sequence and should be byte-walked if a non-Clear row is load-bearing).

---

## 3. YR-active vs TS-legacy

**Active in YR.** `UnitClass::Can_Enter_Cell @ 0x0073F0A0` is the live per-cell passability path for
ground units; the land-speed table at `0x0089EA40` is populated every rules load by
`0x00674000` from stock `[LandType]` sections. `Track=` over `Clear` is exercised by every tracked
ground unit on the most common terrain. No SpecialFlags gate. (The `Tunnel` row exists in the table
but subterranean movement itself is TS-legacy per project policy; the table row is just parsed, not a
live mechanic — irrelevant to this gate.)

This table is **distinct** from the 13×8 `ZonePassabilityMatrix @ 0x0082A594`
(rows = MovementZone `+0x5B4`, cols = reduced CellClass ZoneType `+0x4C`; only value 1 passes).
That matrix governs zone connectivity/A* precheck; the `0x0089EA40` table governs the literal
per-cell SpeedType×LandType legality + speed multiplier. Two different tables, two different index
spaces — do not conflate. (See `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`.)

---

## 4. Rust delta (what `passability.rs` gets wrong)

`src/sim/pathfinding/passability.rs::is_passable_for_speed_type(land, speed)` does **not** use the
`0x0089EA40` SpeedType×LandType table at all. It routes `SpeedType::Track` →
`zone_layer_for_speed_type` → MovementZone **row 2** of the 13×8 `MOVEMENT_ZONE_PASSABILITY`, then
indexes `[2][land]`. For `(Clear=0, Track)` it returns `MOVEMENT_ZONE_PASSABILITY[2][0] == 1` ⇒
`true`. So the gate's boolean answer **coincidentally agrees** (PASS), but via the wrong table.

- **Result-equal for the gate:** `(Clear, Track)` ⇒ true in both. The FNPC flat-terrain test
  assumption is SOUND.
- **DRIFT (structural):** `is_passable_for_speed_type` collapses (a) a `SpeedType→MovementZone` guess
  and (b) the 13×8 zone matrix into one helper, losing (1) the per-terrain speed *multiplier*
  (`Tiberium Track=70%`, `Weeds Track=70%`, `Ice Track=80%` — non-1.0, affects movement cost/speed,
  not just legality), and (2) the exact 12-row LandType space (Rust `LandType` enum has only 8
  buckets in a different order: `Clear,Road,Rough,Beach,Water,Tiberium,Railroad,Rock`). For pure
  legality of common ground terrain the boolean matches; for speed/cost and for less-common terrain
  rows it can drift. This is out of scope for the gate's PASS verdict but must not be sold as
  "the binary uses this matrix."

---

## 5. Rust handoff

For the dependent cell-validation / FNPC task: **treat `is_passable_for_speed_type(Clear, Track)` ==
true as a verified, safe assumption** — `speed_type_allows_cell` for a tracked unit on Clear/Ground
should return passable, and the `find_nearby_*` flat-terrain tests can rely on it. Concretely:

1. Keep the FNPC flat-terrain tests; their (Track over Clear ⇒ passable) premise is binary-confirmed.
2. The *legality* contract the binary uses is `landSpeedTable[LandType*9 + SpeedType] != 0.0`, where
   `SpeedType` column 1 = Track and `LandType` row 0 = Clear; populated from `[LandType]` `Track=` etc.
   If/when speed-cost parity matters, model this `float[12][9]` table directly (per-terrain
   multiplier), NOT the 13×8 zone matrix, and do not approximate it with `zone_layer_for_speed_type`.
3. Do not "fix" the boolean now — it is correct for this gate. File the speed-multiplier drift as a
   separate follow-up (it changes movement *speed/cost*, a different parity surface).

---

## Sources
- Ghidra: `get_function_by_address 0x0073F0A0`; `decompile_function 0x0073F0A0`; `disassemble_function 0x0073F0A0`;
  `decompile_function 0x006740B4`; `disassemble_function 0x00674000`; `decompile_function 0x00476FC0`;
  `decompile_function SpeedType__ToName`.
- Ghidra reads: `read_memory 0x007E1748` (0.0), `read_memory 0x0089EA40` (zeroed in static image — runtime-filled),
  `read_memory 0x0081dbcc` ("Track"), `read_memory 0x0081dbd4` ("Foot"), `read_memory 0x00839D68` (LandType ptr table),
  `read_memory 0x0081dc1c` ("Clear").
- Ghidra xrefs: `get_xrefs_to 0x0089EA40` (writer `0x006740B4 [WRITE]`, reader `0x0073FAB5`, others).
- INI: `ini/rulesmd.ini` `[Clear]`/`[Water]`/… `[LandType]` sections (line ~30191+).
- Rust: `src/sim/pathfinding/passability.rs` (`is_passable_for_speed_type`, `zone_layer_for_speed_type`).
- Prior docs: `pathfinding/PATHFINDING_ASTAR_GHIDRA_REPORT.md §6`, `pathfinding/ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`.
