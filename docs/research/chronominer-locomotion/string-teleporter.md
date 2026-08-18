# STR_INIKey_Teleporter — 0x00843e60

**Proposed Ghidra label:** `STR_INIKey_Teleporter`

## Summary

Null-terminated ASCII string `"Teleporter"` at `0x00843e60`. This is the INI key read by `TechnoTypeClass__ReadINI` to determine whether a unit type selects `TeleportLocomotionClass` as its locomotor. When `Teleporter=yes` is present in a unit's INI section, the parsed boolean is stored at `TechnoTypeClass+0xCD4`.

Verified via `read_memory 0x00843e60` (16 bytes): `54 65 6c 65 70 6f 72 74 65 72 00 00` = `"Teleporter\0"`.

## Active in YR

**Yes.** Read by `TechnoTypeClass__ReadINI` at `0x00713fe9` during game startup INI parsing (confirmed via `get_xrefs_to 0x00843e60`: 1 DATA xref from that function). `TechnoTypeClass__ReadINI` is called for every unit/building/infantry type at startup. Four unit types in `rulesmd.ini` carry `Teleporter=yes`.

## Type, Address, Value

| Symbol | Address | Type | Value |
|---|---|---|---|
| `STR_INIKey_Teleporter` | `0x00843e60` | `const char *` (null-terminated ASCII) | `"Teleporter"` |

Memory layout: 10 chars + null terminator at `0x00843e60..0x00843e6a`. The next string in the data segment starts at `0x00843e62` (offset +2 from the null) — `"Super..."` visible in the memory dump (confirmed via `read_memory 0x00843e60` 16 bytes: `..Teleporter\0\0Supe`).

## Readers

| Address | Function | Purpose |
|---|---|---|
| `0x00713fe9` | `TechnoTypeClass__ReadINI` | Reads `Teleporter=` boolean; stores result at TechnoTypeClass+0xCD4 |

## Struct Field Binding

**TechnoTypeClass+0xCD4 = Teleporter flag (bool/byte)**

Source: `get_assembly_context 0x00713fe9` (context_instructions=15). The write after the `ReadBool("Teleporter",...)` call at `0x00713ff1`:

```asm
; 0x00713fe9: PUSH 0x843e60   ; push "Teleporter" string
; 0x00713ff1: CALL 0x005295f0  ; ReadBool
; 0x00713ff6: MOV byte ptr [EBP + 0xcd4], AL  ; store result at TechnoTypeClass+0xcd4
```

EBP = TechnoTypeClass* in this context (used as object base pointer throughout ReadINI). The `MOV byte ptr [EBP + 0xcd4]` write directly after the `ReadBool` call confirms **TechnoTypeClass+0xCD4** as the Teleporter flag storage. Verified via assembly context at `0x00713ff6`.

| TechnoTypeClass Byte Offset | Type | Purpose |
|---|---|---|
| +0xCD4 | bool (1 byte) | Teleporter flag: true = unit uses TeleportLocomotionClass |

## INI Key Usage

Key: `Teleporter=yes` in any `[UnitType]`, `[InfantryType]`, or `[BuildingType]` section.

Units carrying `Teleporter=yes` in `rulesmd.ini` (confirmed via grep):

| Unit Section | rulesmd.ini Line | Notes |
|---|---|---|
| `[CLEG]` | 4141 | Chrono Legionnaire (Allied infantry) |
| `[CCOMAND]` | 4210 | Chrono Commando (Allied infantry) |
| `[CIVAN]` | 4707 | Chrono Ivan (Soviet infantry) |
| `[CMIN]` | 7396 | Chrono Miner (Allied vehicle) |

The chrono miner (`[CMIN]`, `[HARV]`-type vehicle) is the primary locomotor target for the chronominer-locomotion system. `[CLEG]`, `[CCOMAND]`, and `[CIVAN]` are infantry units whose teleport is driven by a separate mission state machine.

Units in `rules.ini` (base RA2) also carry the flag (lines 3749, 4843, 4954, 6070) — YR (`rulesmd.ini`) takes priority.

## Relationship to Locomotor CLSID

The `Teleporter=yes` flag is **separate** from the `Locomotor=` key. The locomotor CLSID `{4A582747-9839-11d1-B709-00A024DDAFD1}` (visible in rulesmd.ini line 4208 for CCOMAND) directly selects `TeleportLocomotionClass` via COM. The `Teleporter` flag is an additional type-level attribute that gates behavior (e.g., `InitiateWarp` checks `TechnoTypeClass+0xCD4` to confirm the unit is a teleporter before arming the locomotor). Both must be present for teleport behavior.

## Out-of-Scope Refs

| Symbol | Address | Reason |
|---|---|---|
| `TechnoTypeClass__ReadINI` | `0x00713fe9` | Massive general INI parser; only the "Teleporter" fragment is in scope here |

## Unverified (YELLOW)

- **`InitiateWarp` cross-check**: the claim that `TechnoTypeClass+0xCD4` is checked by `InitiateWarp` needs confirmation from `fn-initiate-warp.md` offset tables. The struct offset itself is VERIFIED (see Struct Field Binding above).
