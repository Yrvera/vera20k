# TechnoClass vtable +0x448 Allied Hook Override Scan - Ghidra Research Report

**Address(es):** `0x006F60C0` base hook; call site inside `TechnoClass::DrawExtras @ 0x006F5190`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Determine whether stock YR Techno-family primary vtables override slot `+0x448`, and whether that slot can affect selected-building overlays.  
**Non-Scope:** Full `DrawExtras` topology, health-bar drawing at `+0x44C`, pip-scale drawing at `+0x450/+0x454`, and non-Techno object vtables.  
**Confidence:** High for the scanned vtables and base hook behavior.  
**Active in YR:** Yes for the call site and vtable slots; visible effect is No because all scanned entries target the empty base stub.

## 1. Overview

The `+0x448` slot is a guarded overlay hook called from the selected-object overlay path. In stock YR, every known Techno-family primary vtable checked for this slot points to the same empty function at `0x006F60C0`.

For selected buildings specifically, the hook sits between the four `DrawBracketCorner` front/right bracket helper edges and the later direct single-stub edges. Since `BuildingClass` inherits the empty stub at this slot, the hook does not draw anything and does not alter the visible selected-building bracket overlay in stock YR.

## 2. Vtable Slot Scan

Slot index: `0x448 / 4 = 274`.

| Class / vtable | Slot address | Slot value | Result | Active in YR |
|---|---:|---:|---|---|
| `AircraftClass` primary `0x007E22A4` | `0x007E26EC` | `0x006F60C0` | inherits empty hook | Yes; standard aircraft class vtable, evidence: `ADDRESS_MAP.md`, Ghidra `read_memory` |
| `BuildingClass` primary `0x007E3EBC` | `0x007E4304` | `0x006F60C0` | inherits empty hook | Yes; selected-building path uses this class, evidence: Ghidra `read_memory`, `BUILDINGCLASS_VTABLE_COMPLETE.md` |
| `FootClass` primary `0x007E8C94` | `0x007E90DC` | `0x006F60C0` | inherits empty hook | Yes as inherited base for mobile technos, evidence: Ghidra `read_memory`, `FOOTCLASS_VTABLE_COMPLETE.md` |
| `InfantryClass` primary `0x007EB058` | `0x007EB4A0` | `0x006F60C0` | inherits empty hook | Yes; standard infantry class vtable, evidence: `ADDRESS_MAP.md`, Ghidra `read_memory` |
| `TechnoClass` primary `0x007F4960` | `0x007F4DA8` | `0x006F60C0` | base empty hook | Conditional; base class slot, evidence: Ghidra `read_memory`, `TECHNOCLASS_VTABLE_COMPLETE.md` |
| `UnitClass` primary `0x007F5C70` | `0x007F60B8` | `0x006F60C0` | inherits empty hook | Yes; standard unit class vtable, evidence: `ADDRESS_MAP.md`, Ghidra `read_memory` |

The adjacent slot block is identical across all six scanned vtables:

| Offset | Value | Meaning |
|---:|---:|---|
| `+0x440` | `0x0070EE30` | inherited `TechnoClass::ProcessCloakDraw` |
| `+0x444` | `0x00706640` | inherited `TechnoClass::Draw` |
| `+0x448` | `0x006F60C0` | empty hook scanned here |
| `+0x44C` | `0x006F64A0` | inherited `TechnoClass::DrawHealthBar` |

`search_byte_patterns` for the little-endian pointer `c0 60 6f 00` returned exactly these six data locations: `0x007E26EC`, `0x007E4304`, `0x007E90DC`, `0x007EB4A0`, `0x007F4DA8`, `0x007F60B8`. Active in YR: Yes for the listed Techno-family class vtables; no additional stock Techno-family `+0x448` override was found by this slot scan.

## 3. Base Hook Behavior

`0x006F60C0` decompiles to a bare return. It reads no object fields, takes no visible branch, and calls no renderer. Active in YR: Yes as the target of every scanned `+0x448` vtable entry; visible effect: No.

The hook is not the health-bar function. `+0x44C` points to `0x006F64A0`; the `+0x448` hook is the preceding empty slot. Active in YR: Yes; evidence is the identical adjacent slot block read from all six scanned primary vtables.

## 4. Call Site And Gates

Inside `TechnoClass::DrawExtras @ 0x006F5190`, the hook call is reached only after the selected-building branch has already begun drawing bracket edges. The local gate is:

- object strength field `TechnoClass+0x6C` as `param_1[0x1B]` must be greater than zero;
- owner house at `TechnoClass+0x21C` as `param_1[0x87]` must be allied with `g_PlayerPtr`, or `RulesClass+0x17E6` must be nonzero;
- `RulesClass+0x17E6` corresponds to `[AudioVisual] EnemyHealth`, with standard YR `rulesmd.ini:755` value `yes`.

Active in YR: Conditional. The call site is live in standard YR overlay rendering, and the default `EnemyHealth=yes` makes the non-allied half of the gate normally pass for alive selected objects. The target function still returns immediately.

## 5. Selected-Building Overlay Effect

For selected buildings, `BuildingClass` slot `+0x448` is `0x006F60C0`, so stock YR draws nothing through this hook. It cannot insert an extra overlay between the four helper bracket edges and the three direct bracket stubs.

Active in YR: Yes for the selected-building path; no visible effect because the active vtable target is empty. Evidence: `BuildingClass` vtable slot address `0x007E4304 -> 0x006F60C0`, `TechnoClass::DrawExtras @ 0x006F5190`, and decompile of `0x006F60C0`.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass` slot `+0x448` | verified | `read_memory 0x007F4DA8 -> 0x006F60C0` | none |
| `FootClass` slot `+0x448` | verified | `read_memory 0x007E90DC -> 0x006F60C0` | none |
| `BuildingClass` slot `+0x448` | verified | `read_memory 0x007E4304 -> 0x006F60C0` | none |
| `InfantryClass` slot `+0x448` | verified | `read_memory 0x007EB4A0 -> 0x006F60C0` | none |
| `UnitClass` slot `+0x448` | verified | `read_memory 0x007F60B8 -> 0x006F60C0` | none |
| `AircraftClass` slot `+0x448` | verified | `read_memory 0x007E26EC -> 0x006F60C0` | none |
| Base hook implementation `0x006F60C0` | verified | Ghidra decompile: empty return | none |
| `DrawExtras` local call gate | verified | Ghidra decompile `0x006F5190`; `rulesmd.ini:755` | none for this hook |
| Full health-bar and pip overlay behavior | deferred | outside requested slot | investigate `+0x44C/+0x450/+0x454` separately |

## 7. Open Questions - Final State

[RESOLVED] OQ-TV448-001 - Does any scanned stock Techno-family primary vtable override `+0x448`? No. All scanned entries point to `0x006F60C0`. Evidence: Ghidra `read_memory` at six slot addresses and `search_byte_patterns c0 60 6f 00`.

[RESOLVED] OQ-TV448-002 - What does `0x006F60C0` draw? Nothing; it immediately returns. Evidence: Ghidra decompile `0x006F60C0`.

[RESOLVED] OQ-TV448-003 - Can this hook affect selected building overlays in stock YR? No visible effect; `BuildingClass +0x448` inherits the empty target. Evidence: `read_memory 0x007E4304 -> 0x006F60C0`, `TechnoClass::DrawExtras @ 0x006F5190`.

[RESOLVED] OQ-TV448-004 - Is `EnemyHealth` part of this hook gate? Yes; `DrawExtras` checks `RulesClass+0x17E6`, and standard YR has `[AudioVisual] EnemyHealth=yes` at `rulesmd.ini:755`. Active in YR: Conditional; it controls whether the empty hook is called for non-allied alive objects.

[DEFERRED] OQ-TV448-005 - Are `+0x450` and `+0x454` pip-scale calls fully modeled? Out-of-scope; they are adjacent overlay virtuals, not the `+0x448` hook. Category: out-of-scope.

## Sources

- Ghidra decompiled: `TechnoClass::DrawExtras @ 0x006F5190`
- Ghidra decompiled: `FUN_006F60C0`
- Ghidra read_memory:
  - `0x007E26EC`, `0x007E4304`, `0x007E90DC`, `0x007EB4A0`, `0x007F4DA8`, `0x007F60B8`
  - adjacent blocks at `+0x440..+0x44C` for the same six vtables
- Ghidra search_byte_patterns: `c0 60 6f 00`
- `C:/Users/enok/Documents/ra2-rust-game-docs/ADDRESS_MAP.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/TECHNOCLASS_VTABLE_COMPLETE.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/FOOTCLASS_VTABLE_COMPLETE.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BUILDINGCLASS_VTABLE_COMPLETE.md`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:755`
