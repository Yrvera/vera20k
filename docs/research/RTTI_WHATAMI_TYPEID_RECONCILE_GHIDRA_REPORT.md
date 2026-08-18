# RTTI / What_Am_I() Type-ID Reconciliation — Ghidra Report

**Date:** 2026-07-19
**Purpose:** Resolve the cross-doc contradiction between `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md` (claims `0xF = CellClass`) and `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` (claims `0xB = CellClass`, `0xF = InfantryClass`).
**Confidence:** HIGH — every `What_Am_I()` return value is a literal `MOV EAX,imm; RET` read directly from bytes (not decompiler paraphrase), and each is bound to its owning class via a verified vtable slot read (base-vtable identity, not label trust).
**Active in YR:** YES — `What_Am_I()` (vtable +0x2C) is the core RTTI dispatch used throughout combat, movement, and radio-link code for every object class; UnitClass, InfantryClass, AircraftClass, BuildingClass, and CellClass are all live in every standard YR skirmish.

---

## 1. Verdict

**`TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md` is WRONG. `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` is CORRECT** on both the `0xB = CellClass` and `0xF = InfantryClass` points.

The disassembly-level fact in the SET_DESTINATION doc (there is a literal `CMP EAX, 0xF` at the cited site) is real — but the parenthetical gloss `(CellClass)` attached to it is wrong. `0xF` is `InfantryClass`, not `CellClass`. `CellClass` is `0xB`.

---

## 2. Authoritative `What_Am_I()` table (vtable +0x2C)

Every row: literal return value read via `disassemble_function`/`read_memory` on the function body (not decompile paraphrase), and the function's ownership confirmed by reading the class's own vtable at slot `+0x2C` and matching addresses exactly (not by trusting the Ghidra label).

| RTTI (dec) | RTTI (hex) | Class | `What_Am_I` addr | Verification |
|---|---|---|---|---|
| 1 | 0x1 | **UnitClass** | `0x00746e20` | `disassemble_function 0x746e20` → `MOV EAX,0x1; RET`. Ownership: `vtable__UnitClass @ 0x007F5C70` (confirmed base via `list_globals name_substring=UnitClass`), `read_memory 0x007F5C9C 4` (base+0x2C) → `20 6e 74 00` = `0x00746e20`, exact match. |
| 2 | 0x2 | **AircraftClass** | `0x0041c180` | Function not Ghidra-defined (no name, `get_function_by_address` fails) — read raw bytes: `read_memory 0x0041c180 16` → `B8 02 00 00 00 C3 90...` = `MOV EAX,0x2; RET` padded with NOPs. Ownership: `vtable__AircraftClass @ 0x007E22A4`, `read_memory 0x007E22D0 4` (base+0x2C) → `80 c1 41 00` = `0x0041c180`, exact match. |
| 6 | 0x6 | **BuildingClass** | `0x00459ec0` | `disassemble_function 0x459ec0` → `MOV EAX,0x6; RET`; also named `BuildingClass__WhatAmI` directly (`search_functions`). Ownership: `vtable_BuildingClass @ 0x007E3EBC`, `read_memory 0x007E3EE8 4` (base+0x2C) → `c0 9e 45 00` = `0x00459ec0`, exact match. |
| 11 | **0xB** | **CellClass** | `0x00487e60` | Function not Ghidra-defined — raw bytes: `read_memory 0x00487e60 16` → `B8 0B 00 00 00 C3 90...` = `MOV EAX,0xB; RET`. Ownership: `vtable__CellClass @ 0x007E4EEC`, `read_memory 0x007E4F18 4` (base+0x2C) → `60 7e 48 00` = `0x00487e60`, exact match. |
| 15 | **0xF** | **InfantryClass** | `0x00523340` | `disassemble_function 0x523340` → `MOV EAX,0xF; RET`; named `InfantryClass__What_Am_I` directly. Ownership: `vtable__InfantryClass @ 0x007EB058`, `read_memory 0x007EB084 4` (base+0x2C) → `40 33 52 00` = `0x00523340`, exact match. |
| 20 | 0x14 | OverlayClass | `0x005fdf50` | `disassemble_function 0x5fdf50` → `MOV EAX,0x14; RET`. Named directly; not independently vtable-cross-checked this session (out of core scope). |
| 21 | 0x15 | OverlayTypeClass | `0x005fef00` | `disassemble_function 0x5fef00` → `MOV EAX,0x15; RET`. Named directly; not independently vtable-cross-checked this session. |
| 36 | 0x24 | TerrainClass | `0x0071d300` | `disassemble_function 0x71d300` → `MOV EAX,0x24; RET`; named directly. Ownership: `vtable__TerrainClass @ 0x007F522C`, `read_memory 0x007F5258 4` (base+0x2C) → `00 d3 71 00` = `0x0071d300`, exact match. |

All five vtable base-reads land at exactly offset `+0x2C` for a real class-owned `What_Am_I` override — this independently confirms `+0x2C` is the `What_Am_I` slot (already asserted in `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` §4, now cross-verified against six separate classes' vtables, not one).

---

## 3. The two disputed callsites

### 3.1 `0x00741970` (`TechnoClass::Set_Destination` — live UnitClass override, per `NAVCOM_LIFECYCLE` §1 correction) — contains exactly ONE `CMP EAX,0xB` and exactly ONE `CMP EAX,0xF`

**The `0xB` compare — `0x007419cc`** (inside the BalloonHover early-cancel block, §3.1 of NAVCOM doc):
```
007419c7: MOV EDX,dword ptr [ECX]        ; ECX = this->NavCom (+0x5A4)
007419c9: CALL dword ptr [EDX + 0x2c]    ; NavCom->What_Am_I()
007419cc: CMP EAX,0xb                    ; RTTI == 0xB
007419cf: JNZ 0x00741a01
```
This gates the "clicking stop on an attack-move while NavCom already equals ArchiveTarget" branch: `NavCom == ArchiveTarget || (NavCom->RTTI == 0xB && NavCom->OwnerHouse == ArchiveTarget)`. This matches `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` §3.1 verbatim (`NavCom->RTTI == 0xB /* CellClass */`) — **CORRECT**, confirmed CellClass per §2 table.

**The `0xF` compare — `0x00741afc`** (inside the LEAVE_DOCK cancel-dock block, the block `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md` §Stage 3 labels item 3):
```
00741aee: MOV ECX,EBP
00741af0: CALL 0x0065ad30                ; FootClass::GetDestination(0) = Contacts[0]
00741af5: MOV EDX,dword ptr [EAX]
00741af7: MOV ECX,EAX
00741af9: CALL dword ptr [EDX + 0x2c]    ; Contacts[0]->What_Am_I()
00741afc: CMP EAX,0xf                    ; RTTI == 0xF
00741aff: JNZ 0x00741b28
00741b01: TEST EBX,EBX
00741b03: JZ 0x00741c9d
00741b09: TEST byte ptr [EBX + 0x14],0x1  ; new-destination "occupied" flag
00741b0d: JZ 0x00741b28
```
The literal immediate IS `0xF`, exactly as `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md` reported. But per §2, `0xF = InfantryClass`, not CellClass. **`TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md`'s `(CellClass)` gloss on this line is WRONG.** The gate actually reads: "the unit's current radio contact (`Contacts[0]`) is an `InfantryClass` object AND the new destination cell has its occupied flag set" — semantically coherent (e.g., an escorted/captured infantry radio-link case), unlike the doc's original reading (a radio *contact* being a bare `CellClass` doesn't fit the radio-link data model, since contacts are Technos).

**Bonus resolution:** this also **resolves `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md` §10 Open Question #6** ("The RTTI==2 contact check at 0x741b64 — is RTTI==2 Aircraft?"): confirmed YES — `0x741b64: CMP EAX,0x2` on `Contacts[0]->What_Am_I()` is a genuine AircraftClass check, per §2 table (RTTI 2 = AircraftClass, byte-verified this session).

### 3.2 `0x004D94B0` (`FootClass::Set_Destination_Internal`) — task anchor was WRONG: this function has NO RTTI compare at all

The dispatch prompt for this investigation stated `FootClass::Set_Destination_Internal 0x004D94B0 (has RTTI==0xB compare)`. Full `disassemble_function 0x4D94B0` read this session contains no `CMP ..., 0xB` and no `CMP ..., 0xF` anywhere — the only What_Am_I-adjacent check in this function is `CALL dword ptr [EDX+0x2c]; CMP EAX,0x2` at `0x004D967a` (`this->What_Am_I() == 2`, i.e. **AircraftClass**, used for the "attacking vehicle, don't stop locomotor" carve-out described in `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` §4.2/§4 STEP 5). That doc's own inline comment glosses this as `what == 2 /* UnitClass */` — per §2 this session, RTTI 2 is AircraftClass, not UnitClass (UnitClass is 1). **This is a secondary, narrower error in `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`** (§4 STEP 5 comment and the "attacking vehicle" framing in §4.2), flagged below — out of the requested reconciliation scope (0xB/0xF) but directly adjacent evidence from the same read, so it is recorded rather than discarded.

The actual `0xB` (CellClass, arrival check) and `0xF` (InfantryClass, escort/capture check) compares that the task anchor was pointing at live in **`DriveLocomotionClass::Process` (`0x004B0500`)**, not in `Set_Destination_Internal`:
```
004b066f: MOV ECX,dword ptr [EDX + 0x5a4]   ; owner->NavCom
004b0679: MOV EAX,dword ptr [ECX]
004b067b: CALL dword ptr [EAX + 0x2c]        ; NavCom->What_Am_I()
004b067e: CMP EAX,0xb                        ; RTTI == 0xB  (CellClass — arrival cell-match check)
```
```
004b05d9: CALL dword ptr [EDX + 0x2c]        ; owner->What_Am_I()
004b05dc: CMP EAX,0x1                        ; == 1 (UnitClass, confirmed §2)
004b05e4: MOV ECX,dword ptr [EAX + 0x5a4]    ; owner->NavCom
004b05f0: CALL dword ptr [EDX + 0x2c]        ; NavCom->What_Am_I()
004b05f3: CMP EAX,0xf                        ; RTTI == 0xF  (InfantryClass)
```
This second site is an independent, non-definitional confirmation that `0xF = InfantryClass` is used consistently elsewhere in the movement code (a UnitClass owner checking whether its NavCom target is an InfantryClass object), not an isolated artifact of one callsite.

---

## 4. Implementation Handoff

- If/when Rust code encodes gamemd RTTI-equivalent type tags for parity-critical dispatch (radio contact type checks, NavCom target type checks, Set_Destination gating), use: UnitClass=1, AircraftClass=2, BuildingClass=6, CellClass=0xB(11), InfantryClass=0xF(15). Do NOT use TerrainClass/OverlayClass/OverlayTypeClass values (0x24/0x14/0x15) for anything except those literal classes.
- Any future doc or code comment citing `0xF` in a radio-contact or NavCom-target context in `TechnoClass::Set_Destination` (`0x00741970`) or `DriveLocomotionClass::Process` (`0x004B0500`) must read **InfantryClass**, not CellClass.
- Any future doc or code comment citing `0xB` in a NavCom-target context in the same two functions must read **CellClass** — this one was already correct in `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`.
- `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md` needs an inline patch (not performed here — read-only investigation): §Stage 3 item 3 changes `"FootClass__GetDestination(0) RTTI == 0xF (CellClass)"` → `"FootClass__GetDestination(0) RTTI == 0xF (InfantryClass)"`; §Table row `0x00741AFC` description likewise; §10 Open Question #6 should be marked RESOLVED (RTTI==2 confirmed AircraftClass).

---

## 5. Negative Facts / Do Not Do

- Do NOT use `0xF = CellClass` anywhere — this was `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md`'s error; disproven by direct vtable-slot read of `InfantryClass::What_Am_I` landing exactly on the function containing the `0xF` immediate.
- Do NOT assume `RTTI 0 = UnitClass`. `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md`'s own §Stage 3 item 6 text ("What_Am_I() == 0 — this is Unit") does not correspond to any verified `What_Am_I()` compare in the disassembly at that point (the actual instruction there, `0x00741b3e: CALL [EDX+0x184]`, is a *different* vtable slot, not the `+0x2C` What_Am_I slot) — left as UNVERIFIABLE/wrong-slot rather than corrected here (outside the 0xB/0xF scope of this task); flag for a future targeted check before anyone relies on "RTTI 0 = UnitClass."
- Do NOT trust `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` §4 STEP 5's inline comment `what == 2 /* UnitClass */` literally — RTTI 2 is AircraftClass per this session's byte-verified table. The surrounding behavioral description (attacking vehicle with TarCom keeps its locomotor arc) may still be accurate for whichever class actually reaches that `Set_Destination_Internal(NULL)` call with What_Am_I()==2 in practice, but the class name in the comment is wrong and unverified as to *which* real-world callers hit that branch. Not re-investigated here — out of scope.
- Do NOT extend this table by inference — RTTI values for `CivilianClass`, `TeamClass`, `BulletClass`, `TriggerClass`, `SuperClass`, `WaypointClass`, etc. were NOT looked up this session and must not be assumed from adjacency or "seems like it should be N."

---

## 6. Remaining Uncertainty

- `RTTI 0` semantic meaning is UNVERIFIED this session (see §5). Do not assume it's UnitClass without a separate check.
- `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` §4 STEP 5 / §4.2's "attacking vehicle" framing names the wrong class (says UnitClass, byte evidence says RTTI 2 = AircraftClass) — the *mechanism* (What_Am_I()==2 && Mission==Attack && TarCom!=NULL keeps the locomotor arc during `Set_Destination(NULL)`) is verified to exist at `0x004D967a`, but which class instances legitimately reach `Set_Destination_Internal` with `What_Am_I()==2` in that state was not re-traced. Flagged, not fixed — outside the assigned 0xB/0xF scope.
- OverlayClass (0x14) and OverlayTypeClass (0x15) were read directly (`disassemble_function`) but their owning-vtable base was not independently cross-checked this session (only TerrainClass, UnitClass, InfantryClass, AircraftClass, BuildingClass, CellClass got the double vtable-slot-read verification). Treat Overlay(Type)Class rows as MEDIUM confidence (direct-read HIGH, ownership-binding UNCHECKED) versus HIGH for the other six.

---

## Sources

All calls this session, live against `gamemd.exe` (project `testProsjekt`):
- `disassemble_function 0x746e20` → `MOV EAX,0x1; RET` (UnitClass)
- `disassemble_function 0x459ec0` → `MOV EAX,0x6; RET` (BuildingClass, also named `BuildingClass__WhatAmI`)
- `search_functions name_pattern=What_Am_I` → `InfantryClass__What_Am_I @ 523340`, `OverlayClass__What_Am_I @ 5fdf50`, `OverlayTypeClass__What_Am_I @ 5fef00`, `TerrainClass__What_Am_I @ 71d300`, `UnitClass__What_Am_I @ 746e20`
- `disassemble_function 0x523340` → `MOV EAX,0xf; RET` (InfantryClass)
- `disassemble_function 0x71d300` → `MOV EAX,0x24; RET` (TerrainClass)
- `disassemble_function 0x5fdf50` → `MOV EAX,0x14; RET` (OverlayClass)
- `disassemble_function 0x5fef00` → `MOV EAX,0x15; RET` (OverlayTypeClass)
- `list_globals name_substring=UnitClass/AircraftClass/CellClass/InfantryClass/BuildingClass/TerrainClass` → vtable base addresses `0x007F5C70`, `0x007E22A4`, `0x007E4EEC`, `0x007EB058`, `0x007E3EBC`, `0x007F522C`
- `read_memory 0x007F5C9C 4` → `20 6e 74 00` = `0x00746e20` (UnitClass vtable+0x2C)
- `read_memory 0x007E22D0 4` → `80 c1 41 00` = `0x0041c180` (AircraftClass vtable+0x2C)
- `read_memory 0x007E4F18 4` → `60 7e 48 00` = `0x00487e60` (CellClass vtable+0x2C)
- `read_memory 0x007EB084 4` → `40 33 52 00` = `0x00523340` (InfantryClass vtable+0x2C)
- `read_memory 0x007E3EE8 4` → `c0 9e 45 00` = `0x00459ec0` (BuildingClass vtable+0x2C)
- `read_memory 0x007F5258 4` → `00 d3 71 00` = `0x0071d300` (TerrainClass vtable+0x2C)
- `read_memory 0x0041c180 16` → `B8 02 00 00 00 C3 90...` = `MOV EAX,0x2; RET` (AircraftClass::What_Am_I, raw bytes — not a Ghidra-defined function)
- `read_memory 0x00487e60 16` → `B8 0B 00 00 00 C3 90...` = `MOV EAX,0xB; RET` (CellClass::What_Am_I, raw bytes — not a Ghidra-defined function)
- `get_function_by_address 0x0041c180` / `0x00487e60` → both `"No function found"` (confirms these are undefined-function raw code, hence not reachable by name search — read via `read_memory` instead)
- `disassemble_function 0x00741970` (full, 1894-line body saved to tool-results file) → located sole `CMP EAX,0xb` at `0x007419cc` and sole `CMP EAX,0xf` at `0x00741afc` via text search of the saved disassembly
- `disassemble_function 0x004D94B0` (full body, ~140 instructions) → confirmed zero `CMP ..., 0xB` / `CMP ..., 0xF`; found `CMP EAX,0x2` at `0x004D967a` (What_Am_I()==2 check)
- `disassemble_function 0x004B0500` (full body) → found `CMP EAX,0xf` at `0x004b05f3` and `CMP EAX,0xb` at `0x004b067e`

Docs reconciled (read, not modified — read-only investigation):
- `TECHNOCLASS_SET_DESTINATION_GHIDRA_REPORT.md` — §Stage 3 item 3 and §Table row `0x00741AFC` contain the error (`0xF` mislabeled `CellClass`)
- `NAVCOM_LIFECYCLE_GHIDRA_REPORT.md` — §3.1 and §6.3 `0xB`/`0xF` claims CONFIRMED CORRECT; §4 STEP 5 comment `what == 2 /* UnitClass */` flagged as a separate, narrower error (should read AircraftClass)
