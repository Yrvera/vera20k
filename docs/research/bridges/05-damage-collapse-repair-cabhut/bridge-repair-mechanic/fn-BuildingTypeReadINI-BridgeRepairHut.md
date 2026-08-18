# fn-BuildingTypeReadINI-BridgeRepairHut (Callsite Narrow Decode)

**Function:** `BuildingTypeClass_ReadINI_Water` (large INI reader)
**Address:** `0x0045FE50` – `0x00464A62`
**Callsite address:** `0x00460E8D` (data ref to BridgeRepairHut string `0x0081A898`)
**Confidence:** HIGH (content, identity, offsets all verified via Ghidra MCP read_memory)
**YR-active:** YES — called at game startup when loading `rules(md).ini`, unconditionally for every BuildingType object.

---

## Scope

This decode is narrowly scoped to the `BridgeRepairHut` INI read callsite within `BuildingTypeClass_ReadINI_Water`. The full function (`~17KB`) reads all `BuildingTypeClass` INI fields; only the BridgeRepairHut field is relevant here.

---

## ReadBool Callsite — Disassembly at 0x00460E80

Verified via `read_memory 0x00460E80` (40 bytes) and `read_memory 0x00460E9A` (8 bytes):

```
0x00460E80: 89 85 FC 16 00 00   MOV [EBP+0x16FC], EAX     ← store previous field result
0x00460E86: 8A 95 B6 16 00 00   MOV DL, BYTE [EBP+0x16B6] ← load current +0x16B6 (default arg)
0x00460E8C: 52                  PUSH EDX                   ← default value arg
0x00460E8D: 68 98 A8 81 00      PUSH 0x0081A898            ← key = "BridgeRepairHut"
0x00460E92: 53                  PUSH EBX                   ← INI section (calling convention)
0x00460E93: 8B CE               MOV ECX, ESI               ← this = CCINIClass*
0x00460E95: E8 56 87 0C 00      CALL CCINIClass__ReadBool   ← at 0x005295F0
0x00460E9A: 88 85 B6 16 00 00   MOV BYTE [EBP+0x16B6], AL  ← store result to +0x16B6
```

Verified via `get_function_by_address 0x005295F0` → `CCINIClass__ReadBool`.

---

## Key Findings

### 1. Storage offset confirmed: `BuildingTypeClass + 0x16B6`

The `MOV BYTE [EBP+0x16B6], AL` at `0x00460E9A` stores the ReadBool result.

`EBP` = base pointer of the current `BuildingTypeClass` instance on the stack frame.
Byte offset = `0x16B6` = `BuildingTypeClass + 0x16B6`.

Verified: `read_memory 0x00460E9A` returns `88 85 b6 16 00 00` = x86 `MOV BYTE [EBP+0x16B6], AL`. Exact match.

### 2. INI key: "BridgeRepairHut" at `0x0081A898`

The `PUSH 0x0081A898` at `0x00460E8D` passes the key string address. This matches `get_xrefs_to 0x0081A898` which returns a single xref: `From 00460e8d in BuildingTypeClass_ReadINI_Water [DATA]`.

### 3. Default value: current value of `+0x16B6`

The `MOV DL, BYTE [EBP+0x16B6]` before the PUSH EDX loads the field's current value as the default. This means ReadBool only changes the field if the INI section has an explicit entry. For CABHUT in `rulesmd.ini` (line 16348), `BridgeRepairHut=yes` is set explicitly, so the read result is `1` (true).

### 4. Enclosing function

The callsite lives in `BuildingTypeClass_ReadINI_Water` at `0x0045FE50`. This is the primary INI reader for all building type data. It is called from the game's type-loading initialization sequence (not from gameplay).

---

## CCINIClass__ReadBool Signature

```c
bool __thiscall CCINIClass__ReadBool(CCINIClass *this, const char *section, const char *key, bool default_val)
```

From the calling convention at the callsite:
- `ECX = this` (CCINIClass pointer via ESI)
- `PUSH EBX` = section name
- `PUSH 0x0081A898` = key name ("BridgeRepairHut")
- `PUSH EDX` = default value (current value of +0x16B6)
- Returns result in AL

Verified via `get_function_by_address 0x005295F0` → `CCINIClass__ReadBool at 005295f0`.

---

## INI Default

From `rulesmd.ini:16348` (in-repo INI files, per CLAUDE.md):
```ini
[CABHUT]
BridgeRepairHut=yes
```

The ReadBool will return `1` (true) for CABHUT. For all other building types that don't define this key, the field retains whatever default was set in the constructor (YELLOW — constructor at `0x0045E000` region, not decoded in this session; likely `0` per default bool convention).

---

## Observable Effect

`BuildingTypeClass + 0x16B6` is the flag checked by `InfantryClass::PerCellProcess` (at offset in the C4 branch, verified in task #2) to confirm the target building is a bridge repair hut before dispatching to `ProcessBridgeDestruction_Low/High`. Without this field being `1`, the C4 plant on CABHUT would fall through to the generic building C4 path instead of triggering the bridge collapse sequence.

---

## Unverified

**YELLOW:** `BuildingTypeClass` constructor default for `+0x16B6` — the field may default to `0` (false) for non-bridge-hut buildings. Needs constructor decode to confirm. For the parity surface, only CABHUT matters and its value is `yes`.

**YELLOW:** Whether any other building type in `rulesmd.ini` defines `BridgeRepairHut=yes`. A grep of the INI would confirm.
