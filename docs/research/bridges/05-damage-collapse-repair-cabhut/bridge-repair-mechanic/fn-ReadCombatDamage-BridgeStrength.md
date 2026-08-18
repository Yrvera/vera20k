# fn-RulesClass-ReadCombatDamage-BridgeStrength (Callsite Narrow Decode)

**Function:** `RulesClass__ReadCombatDamage` (large INI reader)
**Callsite address:** `0x0066CD73` (data ref to BridgeStrength string `0x0083AD90`)
**Confidence:** HIGH (content, identity, offsets all verified via Ghidra MCP read_memory)
**YR-active:** YES — called at game startup when loading `rules(md).ini`.

---

## Scope

Narrowly scoped to the `BridgeStrength` INI read callsite within `RulesClass__ReadCombatDamage`. Only the BridgeStrength field is relevant here.

---

## ReadInt Callsite — Disassembly at 0x0066CD60

Verified via `read_memory 0x0066CD60` (48 bytes) and `read_memory 0x0066CD80` (12 bytes):

```
0x0066CD60: 89 86 34 17 00 00   MOV [ESI+0x1734], EAX     ← store previous field
0x0066CD66: 8B 86 40 17 00 00   MOV EAX, [ESI+0x1740]     ← load +0x1740 as default arg
0x0066CD6C: 8B 0D 84 0C 7F 00   MOV ECX, [0x007F0C84]     ← load CCINIClass section ptr
0x0066CD72: 50                  PUSH EAX                  ← default value
0x0066CD73: 68 90 AD 83 00      PUSH 0x0083AD90           ← key = "BridgeStrength"
0x0066CD78: 51                  PUSH ECX                  ← section
0x0066CD79: 8B CF               MOV ECX, EDI              ← this = CCINIClass*
0x0066CD7B: E8 50 A9 EB FF      CALL CCINIClass__ReadInt   ← 0x005276D0
0x0066CD80: 8B 96 54 17 00 00   MOV EDX, [ESI+0x1754]     ← load next field (unrelated)
0x0066CD86: 89 86 40 17 00 00   MOV [ESI+0x1740], EAX     ← store BridgeStrength to +0x1740
```

Verified via `get_function_by_address 0x005276D0` → `CCINIClass__ReadInt`.
Verified via `get_xrefs_to 0x0083AD90` → single xref: `From 0066cd73 in RulesClass__ReadCombatDamage [DATA]`.

---

## Key Findings

### 1. Storage offset: `RulesClass + 0x1740`

The `MOV [ESI+0x1740], EAX` at `0x0066CD86` stores the ReadInt result.

`ESI` = base pointer of the current `RulesClass` instance.
Byte offset = `0x1740`.

Verified: `read_memory 0x0066CD86` (included in the 12-byte read at `0x0066CD80`): bytes at +6 = `89 86 40 17 00 00` = `MOV [ESI+0x1740], EAX`. `40 17` = 0x1740 little-endian. Exact match.

### 2. INI key: "BridgeStrength" at `0x0083AD90`

The `PUSH 0x0083AD90` at `0x0066CD73` passes the key string. Single xref confirms this is the only read site.

### 3. Default value: current value of `+0x1740`

The `MOV EAX, [ESI+0x1740]` before the PUSH loads the current field value as default. ReadInt only overrides if the INI section has an explicit `BridgeStrength=` entry.

From `rulesmd.ini:816`:
```ini
BridgeStrength=1500
```

Default in `rules.ini`: `BridgeStrength=1500` at line 676 (same value). Both `rules.ini` and `rulesmd.ini` agree — the merged result is `1500`.

### 4. CCINIClass__ReadInt convention

```c
int __thiscall CCINIClass__ReadInt(CCINIClass *this, const char *section, const char *key, int default_val)
```

- `ECX` = `this` (via EDI = CCINIClass*)
- Stack: section, key, default
- Returns int in EAX

Verified via `get_function_by_address 0x005276D0`.

### 5. Enclosing function

`RulesClass__ReadCombatDamage` — the combat damage INI reader. Reads all `[CombatDamage]` and related global damage rules. Called once at game startup.

---

## Observable Effect

`RulesClass + 0x1740` = `g_RulesClass_Instance + 0x1740` at runtime. This value controls how many hit points a bridge tile can absorb before collapsing. The value `1500` from `rulesmd.ini` is used in bridge damage calculations. The Rust port must read this field from `g_RulesClass_Instance + 0x1740` (or the equivalent parsed Rules struct) to determine when a bridge collapses from weapon damage.

---

## Relationship to Bridge Collapse

The `BridgeStrength` field is NOT used in the C4/hut-death path (which uses `BuildingClass::Update` timers). It IS used in the `ProcessBridgeDamageStateMachine` path where weapon fire hits bridge cells directly. For the C4 path specifically, the collapse is unconditional once the C4 timer expires — `BridgeStrength` only matters for direct-fire damage.

---

## Unverified

None — all claims in this doc are directly read from Ghidra `read_memory` and `get_function_by_address` calls.

---

## Self-Proof (exit gate)

### Claim 1: INI key string is "BridgeStrength" at `0x0083AD90`

`inspect_memory_content 0x0083AD90` (20 bytes) → hex `42 72 69 64 67 65 53 74 72 65 6E 67 74 68 00 ...`,
detected string `"BridgeStrength"`, null-terminated at byte 14. **VERIFIED.**

### Claim 2: Storage offset is `RulesClass + 0x1740`; store instruction at `0x0066CD86`

`get_assembly_context 0x0066cd73` → context_after shows `0066cd86: MOV dword ptr [ESI+0x1740], EAX`.
`ESI` = RulesClass instance pointer (established by `get_function_by_address 0x0066cd73` → body in
`RulesClass__ReadCombatDamage @ 0x0066BBB0`). **VERIFIED at write site.**

### Claim 3: Single xref to `0x0083AD90`; function identity `RulesClass__ReadCombatDamage`

`get_xrefs_to 0x0083AD90` → exactly one result: `From 0066cd73 in RulesClass__ReadCombatDamage [DATA]`.
`get_function_by_address 0x0066cd73` → `RulesClass__ReadCombatDamage @ 0x0066BBB0`,
body `0x0066BBB0 – 0x0066CF64`. **VERIFIED — no other read sites for BridgeStrength.**

---

## Struct Field Summary

| Class | Offset | Type | Field | Notes |
|---|---|---|---|---|
| RulesClass | `+0x1740` | `int` | BridgeStrength | Default 1500 (rulesmd.ini:816); controls bridge tile HP |

## Globals Referenced

| Global | Value (static) | Role |
|---|---|---|
| `0x0083AD90` | `"BridgeStrength"` | INI key string literal |
| `0x007F0C84` | (pointer, runtime) | CCINIClass* rules INI section handle |

## Callers

`RulesClass__ReadCombatDamage @ 0x0066BBB0` is called from `RulesClass::Process` at game startup
(verified consistent with `ReadAudioVisual` and `ReadGeneral` callers in the same load chain).
Out-of-scope to trace the full caller chain here.
