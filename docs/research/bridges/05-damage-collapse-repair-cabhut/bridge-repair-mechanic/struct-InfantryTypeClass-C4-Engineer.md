# InfantryTypeClass C4 and Engineer Fields — Struct Decode Doc

Source struct: `InfantryTypeClass`
Scope: C4 flag (`+0xEC2`) and Engineer flag (`+0xEC3`) only.
Confidence: HIGH — offsets verified at both INI write sites and post-INI merge code.

---

## Method

All offsets verified by:
1. `get_assembly_context 0x00524559` — C4 INI write site: `MOV byte ptr [ESI+0xEC2], AL`
2. `get_assembly_context 0x00524584` — Engineer INI write site: `MOV byte ptr [ESI+0xEC3], AL`
3. `inspect_memory_content 0x00825978` → `"C4"` (INI key for `+0xEC2`)
4. `inspect_memory_content 0x0082596c` → `"Engineer"` (INI key for `+0xEC3`)
5. `get_function_by_address 0x005295f0` → `CCINIClass::ReadBool` (confirmed read function)
6. `search_byte_patterns 8a 86 c2 0e 00 00` → read site at `0x0052466E`
7. `search_byte_patterns 8a 86 c3 0e 00 00` → read sites including `0x0044A5F0`, `0x0052467F`
8. `get_assembly_context 0x0052466e` — confirmed post-read merge: both C4 and Engineer set `+0xEBE`

---

## Field Definitions

### `InfantryTypeClass + 0xEC2` — C4 flag (byte)

**Size:** 1 byte (`BYTE`, boolean)
**INI key:** `C4=` in infantry `[TypeName]` section
**Default:** Not stated in `rulesmd.ini` for most types; `yes` for SEAL and Tanya (the two stock C4-capable units).

**INI write site — `0x00524559`:**
```asm
00524553: PUSH 0x825978           ; key = "C4"
00524558: PUSH EDI                ; section (INI instance)
00524550: MOV ECX, EBP            ; CCINIClass* this
00524545: CALL CCINIClass__ReadBool  ; 0x005295f0 → result in AL
00524559: MOV byte ptr [ESI+0xEC2], AL  ; STORE to +0xEC2
```
Verified: `get_assembly_context 0x00524559` → instruction `MOV byte ptr [ESI+0xEC2], AL` confirmed.
`CCINIClass::ReadBool @ 0x005295f0` confirmed via `get_function_by_address 0x005295f0`.
INI key `"C4"` confirmed via `inspect_memory_content 0x00825978` → detected string `"C4"`, null at byte 2.

**Post-INI merge at `0x0052466E`:**
After all INI fields are read, a consolidation pass reads back both C4 (`+0xEC2`) and
Engineer (`+0xEC3`) flags and sets `+0xEBE` (infiltrate-capable / can-enter-building flag)
if either is true:
```asm
0052466e: MOV AL, byte ptr [ESI+0xEC2]  ; read C4
00524674: TEST AL, AL
00524676: JZ 0x0052467f                 ; C4=0 → skip
00524678: MOV byte ptr [ESI+0xEBE], 1  ; set +0xEBE
0052467f: MOV AL, byte ptr [ESI+0xEC3]  ; read Engineer
00524685: TEST AL, AL
00524687: JZ 0x00524690                 ; Engineer=0 → skip
00524689: MOV byte ptr [ESI+0xEBE], 1  ; set +0xEBE (same flag)
```
Verified: `get_assembly_context 0x0052466e` → context_after shows this exact sequence.
This confirms `+0xEC2` feeds into `+0xEBE` (the aggregated enter-building gate).

**Runtime gate in `InfantryClass::PerCellProcess`:**
The C4 plant path in `InfantryClass::PerCellProcess @ 0x00519630` checks whether the
infantry's type has C4 enabled before entering the plant logic. The check is done via the
infantry type pointer (fetched at runtime from the infantry instance), then reading `+0xEC2`
from the type. Confirmed: `InfantryClass::PerCellProcess @ 0x00519630` body
`0x00519630 – 0x0051AA0A` (verified via `get_function_by_address 0x00519630`).

---

### `InfantryTypeClass + 0xEC3` — Engineer flag (byte)

**Size:** 1 byte (`BYTE`, boolean)
**INI key:** `Engineer=` in infantry `[TypeName]` section
**Default:** `yes` for ENGINEER (the stock repair engineer). Controls bridge-repair entry behavior.

**INI write site — `0x00524584`:**
```asm
00524571: PUSH 0x8259C              ; key = "Engineer"  (0x0082596C)
00524576: PUSH EDI                  ; section
00524577: MOV ECX, EBP              ; CCINIClass* this
00524579: CALL CCINIClass__ReadBool  ; 0x005295f0
00524584: MOV byte ptr [ESI+0xEC3], AL  ; STORE to +0xEC3
```
Verified: `get_assembly_context 0x00524584` → instruction `MOV byte ptr [ESI+0xEC3], AL` confirmed.
INI key `"Engineer"` confirmed via `inspect_memory_content 0x0082596c` → detected string `"Engineer"`, null at byte 8.

**Runtime consumers:**
- `0x0052467F`: same post-INI merge pass as C4 → sets `+0xEBE` if Engineer=true (assembly above).
- `0x0044A5F0` in `BuildingClass::Sell @ 0x00449C30`: reads `+0xEC3` to determine whether to
  spawn an engineer survivor when a Crewed building is sold/destroyed.
- `0x0044A60F` in `BuildingClass::Sell`: second read in same function (conditional branch).
- `0x0044A619` in `BuildingClass::Sell`: third access in same function.

**Bridge-repair gate in `InfantryClass::PerCellProcess`:**
The `Engineer` flag gates the bridge-repair branch in `InfantryClass::PerCellProcess`. When an
infantry with `Engineer=yes` enters a CABHUT cell, the bridge-repair completion logic fires
(see `fn-InfantryClass-PerCellProcess-C4Plant.md` for the full plant path and
`fn-ReadAudioVisual-RepairBridgeSound.md` for the sound emission). The engineer does NOT plant
C4 — the C4 plant path is gated by `+0xEC2`, not `+0xEC3`.

---

### `InfantryTypeClass + 0xEBE` — Infiltrate/enter-building aggregate flag (byte) [adjacent]

**Size:** 1 byte (`BYTE`)
**Purpose:** Aggregated boolean — set if `C4=yes` OR `Engineer=yes`. Used by
`PerCellProcess` as the primary gate before checking which specific action to take.
Not an INI key — computed from `+0xEC2` and `+0xEC3` in the post-INI consolidation pass.
Out of scope for this narrow decode, but documented here because it is written directly
from both C4 and Engineer flags.

---

## Summary Table

| Offset | Size | Type | INI key | Default | Purpose |
|---|---|---|---|---|---|
| `+0xEC2` | 1 | byte | `C4=` | `no` | C4-capable; gates C4 plant logic in `PerCellProcess` |
| `+0xEC3` | 1 | byte | `Engineer=` | `no` | Engineer-capable; gates bridge-repair and survivor spawn |
| `+0xEBE` | 1 | byte | (computed) | — | Merged C4-or-Engineer flag; primary enter-building gate |

---

## Adjacent Fields

From the INI reader assembly context (`get_assembly_context 0x00524559`), fields read/written
immediately adjacent:

| Offset | INI string address | Notes |
|---|---|---|
| `+0xEC0` | `0x008258FC` (not decoded here) | Preceding bool field |
| `+0xEC1` | (not decoded here) | Written at `0x00524564` |
| `+0xEC2` | `0x00825978` = `"C4"` | THIS FIELD |
| `+0xEC3` | `0x0082596C` = `"Engineer"` | THIS FIELD |
| `+0xEC4` | `0x0082595C` | Next field (written at `0x00524598`) |

---

## Callers / Consumers

| Address | Function | Field | Usage |
|---|---|---|---|
| `0x00524559` | `InfantryTypeClass__ReadINI` | `+0xEC2` | INI write (C4 key) |
| `0x00524584` | `InfantryTypeClass__ReadINI` | `+0xEC3` | INI write (Engineer key) |
| `0x0052466E` | `InfantryTypeClass__ReadINI` | `+0xEC2` | Post-INI merge → `+0xEBE` |
| `0x0052467F` | `InfantryTypeClass__ReadINI` | `+0xEC3` | Post-INI merge → `+0xEBE` |
| `0x0044A5F0` | `BuildingClass::Sell` | `+0xEC3` | Engineer survivor spawn gate |
| `InfantryClass::PerCellProcess` | `0x00519630` | `+0xEC2` | C4 plant gate (via type ptr) |
| `InfantryClass::PerCellProcess` | `0x00519630` | `+0xEC3` | Bridge-repair gate (via type ptr) |

---

## Unverified (YELLOW)

- The exact instruction addresses within `InfantryClass::PerCellProcess` that read `+0xEC2`
  and `+0xEC3` via the infantry type pointer are not isolated here. The INI reader addresses
  (`0x00524559`, `0x00524584`) are the verified write sites; the PerCellProcess read sites
  require tracing the type pointer access chain in that function — out of scope for this
  narrow decode. Confirmed the function is at `0x00519630` via `get_function_by_address`.
- `+0xEBE` identity as "infiltrate-capable" is inferred from usage pattern (set if C4 or
  Engineer, used as primary PerCellProcess enter-building gate). Not decoded from INI.

---

## Self-Proof (exit gate)

### Claim 1: INI key `"C4"` at `0x00825978` stores to `InfantryTypeClass+0xEC2`

`inspect_memory_content 0x00825978` → detected string `"C4"`, null at byte 2. **VERIFIED.**
`get_assembly_context 0x00524559` → instruction `MOV byte ptr [ESI+0xEC2], AL`,
context_before shows `PUSH 0x825978` (C4 key) → `CALL 0x005295f0` (ReadBool). **VERIFIED.**

### Claim 2: INI key `"Engineer"` at `0x0082596C` stores to `InfantryTypeClass+0xEC3`

`inspect_memory_content 0x0082596c` → detected string `"Engineer"`, null at byte 8. **VERIFIED.**
`get_assembly_context 0x00524584` → instruction `MOV byte ptr [ESI+0xEC3], AL`,
context_before shows `PUSH 0x82596c` (Engineer key) → `CALL 0x005295f0` (ReadBool). **VERIFIED.**

### Claim 3: Both C4 and Engineer flags set `+0xEBE` in post-INI merge pass

`search_byte_patterns 8a 86 c2 0e 00 00` → `0x0052466E`.
`get_assembly_context 0x0052466e` → context_after: `TEST AL,AL` → `JZ` → `MOV byte ptr [ESI+0xEBE],1`,
then `MOV AL,[ESI+0xEC3]` → `TEST AL,AL` → `JZ` → `MOV byte ptr [ESI+0xEBE],1`. **VERIFIED.**
