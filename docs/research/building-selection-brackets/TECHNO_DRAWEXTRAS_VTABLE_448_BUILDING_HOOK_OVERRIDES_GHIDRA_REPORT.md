# Techno DrawExtras vtable +0x448 Building Hook Overrides - Ghidra Report

**Address(es):** `TechnoClass::DrawExtras @ 0x006F5190`; hook target `0x006F60C0`; `BuildingClass` vtable slot `0x007E4304`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** stock building runtime class/type coverage for the hook called by selected-building `DrawExtras` at vtable `+0x448`.  
**Non-Scope:** full health bar behavior at `+0x44C`, pip-scale hooks at `+0x450/+0x454`, Phobos/Ares extension hooks, or runtime screenshot capture.  
**Confidence:** High for stock `BuildingClass` slot binding and hook behavior.  
**Status:** COMPLETE  
**Active in YR:** Conditional call site; no visible stock behavior because the building target is empty.

## 1. Overview

`TechnoClass::DrawExtras @ 0x006F5190` calls vtable `+0x448` inside the selected-building overlay path after the first bracket-corner edges and before the later direct bracket stubs. For stock YR buildings, `BuildingClass` does not override this hook: slot `+0x448` resolves to `0x006F60C0`, and that function immediately returns.

Stock building "types" are data selected by the `BuildingTypeClass*` stored on each `BuildingClass`; they do not install per-type draw vtables. The live constructor decompile shows `BuildingClass::Constructor @ 0x0043B740` always writes `*this = &vtable_BuildingClass` after receiving the type pointer.

## 2. Verified Binary Evidence

| Finding | Evidence | Active in YR |
|---|---|---|
| The selected-building branch in `DrawExtras` calls `this->vtable + 0x448` only after building bracket setup and before the later `+0x44C` health-bar call. | Ghidra decompile `0x006F5190`: branch checks `WhatAmI()==6`, selected byte `+0x83`, then calls `(**(code **)(*param_1 + 0x448))(...)`; later selected path calls `+0x44C`. | Conditional: selected visible building, alive strength, and owner/alliance gate. |
| `BuildingClass` vtable `+0x448` points to `0x006F60C0`. | Ghidra `read_memory 0x007E4304 16` returned `c0 60 6f 00 a0 64 6f 00 90 9a 70 00 90 a9 70 00`; first dword is `0x006F60C0`. | Yes as stock building vtable binding. |
| Adjacent `BuildingClass` slots distinguish the hook from health-bar drawing. | Ghidra `read_memory 0x007E42F4 32`: `+0x448 = 0x006F60C0`, `+0x44C = 0x006F64A0`, `+0x450 = 0x00709A90`, `+0x454 = 0x0070A990`. | Yes; `+0x44C` is the health-bar path, not this hook. |
| Hook implementation has no stock behavior. | Ghidra decompile `0x006F60C0`: `void FUN_006f60c0(void) { return; }`. It reads no fields and calls no renderer. | Yes as target, visible effect No. |
| All building instances use the same primary `BuildingClass` vtable regardless of stock building type. | Ghidra decompile `BuildingClass::Constructor @ 0x0043B740`: stores the type pointer at `this+0x520`, then writes `*this = &vtable_BuildingClass`. | Yes for standard constructed buildings. |

## 3. Building Subclass / Type Scan Result

The live `BuildingClass` vtable slot is the relevant stock-building override point. The slot value at `0x007E4304` is inherited `0x006F60C0`, not a `BuildingClass`-specific renderer.

No stock building type can override this slot independently: the constructor accepts the building type pointer as data but installs the same primary `vtable_BuildingClass` for the object. This covers standard YR structures such as construction yards, refineries, defenses, tech buildings, and deployed-building results insofar as they are normal `BuildingClass` objects.

`search_byte_patterns c0 60 6f 00` found the empty-hook pointer at six known Techno-family vtable slots: `0x007E26EC`, `0x007E4304`, `0x007E90DC`, `0x007EB4A0`, `0x007F4DA8`, `0x007F60B8`. The building hit is exactly `0x007E4304`.

## 4. Call Gate

The call is conditional, even though the target is empty:

- `TechnoClass+0x6C` / `param_1[0x1B]` must be greater than zero.
- Owner at `TechnoClass+0x21C` must be allied with `g_PlayerPtr`, or `RulesClass+0x17E6` must be true.
- Standard YR has `[AudioVisual] EnemyHealth=yes` in `ini/rulesmd.ini:755`, so the non-allied half of the gate is normally enabled for alive selected objects.

Active in YR: Conditional. The call site is live, but stock building output is unchanged because the callee returns.

## 5. Inference

The slot looks like a reserved/alliance overlay hook from its placement and gate, but that semantic name is inference. The verified fact is narrower: selected-building `DrawExtras` can dispatch `+0x448`, and stock `BuildingClass` dispatches an empty function there.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass::DrawExtras` selected-building call site | verified | Ghidra decompile `0x006F5190` | none for `+0x448` |
| `BuildingClass` primary vtable `+0x448` | verified | `read_memory 0x007E4304` -> `0x006F60C0` | none |
| `0x006F60C0` hook body | verified | Ghidra decompile `0x006F60C0` bare return | none |
| Per-building-type override possibility | verified-negative | `BuildingClass::Constructor @ 0x0043B740` installs `vtable_BuildingClass` while storing type pointer separately | none for stock `BuildingClass` types |
| Adjacent health/pip overlay virtuals | deferred | `read_memory 0x007E42F4` shows `+0x44C/+0x450/+0x454` | out of scope |

## 7. Open Questions - Final State

[RESOLVED] OQ-448-BLD-001 - Does stock `BuildingClass` override vtable `+0x448` with a building-specific function? No. The slot points to `0x006F60C0`. Evidence: `read_memory 0x007E4304`.

[RESOLVED] OQ-448-BLD-002 - Does the `+0x448` target draw anything for buildings? No. `0x006F60C0` immediately returns. Evidence: Ghidra decompile `0x006F60C0`.

[RESOLVED] OQ-448-BLD-003 - Can stock building types override this hook independently? No evidence of per-type vtables; constructor installs the same primary `vtable_BuildingClass` and stores the type pointer as data at `this+0x520`. Evidence: Ghidra decompile `0x0043B740`.

[RESOLVED] OQ-448-BLD-004 - Is the hook active in standard YR? Conditional call site Yes; visible behavior No. Evidence: `0x006F5190` gate, `rulesmd.ini:755`, and empty callee `0x006F60C0`.

[DEFERRED] OQ-448-BLD-005 - What exactly do adjacent `+0x450/+0x454` pip-scale/veterancy paths draw for every building case? Out-of-scope; they are not the `+0x448` hook. Category: out-of-scope.

## Sources

- Ghidra decompile: `TechnoClass::DrawExtras @ 0x006F5190`
- Ghidra decompile: `FUN_006F60C0`
- Ghidra decompile: `TechnoClass::DrawHealthBar @ 0x006F64A0`
- Ghidra decompile: `BuildingClass::Constructor @ 0x0043B740`
- Ghidra read memory: `0x007E4304`, `0x007E42F4`, `0x007E3EBC`
- Ghidra search byte pattern: `c0 60 6f 00`
- INI: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:755`
