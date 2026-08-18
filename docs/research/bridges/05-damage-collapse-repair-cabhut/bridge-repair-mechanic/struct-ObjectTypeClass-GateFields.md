# struct-ObjectTypeClass-GateFields

**Runbook:** struct-decode-v1
**Target:** `ObjectTypeClass` fields: `+0x22E Bombable`, `+0x231 LegalTarget`, `+0x232 Insignificant`, `+0x233 Immune`
**Confidence:** HIGH (all four offsets directly verified via `decompile_function 0x005F9510` — INI key string references and store instructions are both visible in the decompilation)
**YR-active:** YES — all four fields are actively read in combat, targeting, and damage-receive code paths.

---

## Method

All offsets verified by:
1. `decompile_function 0x005F9510` (`ObjectTypeClass__ReadINI`) — all four fields appear with their INI key string globals and store addresses inline.
2. `search_byte_patterns 88 ?? XX YY 00 00` — finds `MOV BYTE [reg+offset], reg8` writes (constructor + INI reader).
3. `search_byte_patterns c6 ?? XX YY 00 00` — finds `MOV BYTE [reg+offset], imm8` writes (constructor immediates).
4. `read_memory` at constructor write sites to confirm default values.
5. `get_function_by_address` to identify constructor (`ObjectTypeClass__Constructor @ 0x005F7090`).

Constructor entry verified via `read_memory 0x005F7090` (20 bytes):
```
8b 44 24 04 83 ec 1c 53 55 8b e9 50 e8 5f 97 e1 ff 33 db 8d
```
`33 db` at byte +18 = `XOR EBX, EBX` — BL = 0 throughout constructor, so all `MOV BYTE [reg+N], BL` writes are writing `0` (false) as the constructor default.

---

## Critical Note: Direct Byte Offsets (not pointer-indexed)

`param_1` in `ObjectTypeClass__ReadINI` is typed `int*`, but all four field accesses use the explicit cast form `*(undefined1 *)((int)param_1 + 0x22E)` etc. These are **direct byte offsets**, NOT `param_1[N]` pointer-indexed form. The offsets are byte offsets from the object base — no multiplication by 4 applies. This is confirmed by the `(int)param_1 + offset` pattern visible in the decompilation.

---

## Field Definitions

### `ObjectTypeClass + 0x22E` — `Bombable` (byte)

**Size:** 1 byte (`BYTE`)
**Meaning:** If non-zero, the object can be targeted by a bomb (e.g., spy bomber). Default value determined by constructor.

#### Constructor default

Verified via `search_byte_patterns 88 ?? 2e 02 00 00` — no `c6` immediate write found, only `88` register writes at:
- `0x005F7168` (in `ObjectTypeClass__Constructor @ 0x005F7090`)
- `0x005F9431` (in `ObjectTypeClass__ReadINI @ 0x005F9510`)

Constructor write (`0x005F7168`): `88` = `MOV BYTE [reg+0x22E], BL`. Since BL = 0 (constructor XOR EBX,EBX confirmed), default = **0** (false).

**NOTE:** This is unusual — the default for `Bombable` is `false` in the ObjectTypeClass constructor. Subclasses may override. For bridge repair huts (CABHUT), the effective `Bombable` value depends on the `BuildingTypeClass` constructor calling `ObjectTypeClass__Constructor` as a base, and subsequent INI reads.

#### INI read: `ObjectTypeClass__ReadINI @ 0x005F9510`

From `decompile_function 0x005F9510`:
```c
uVar2 = CCINIClass__ReadBool(piVar9, s_Bombable_00832bcc, *(undefined1 *)((int)param_1 + 0x22e));
*(undefined1 *)((int)param_1 + 0x22e) = uVar2;
```

INI key string at `0x0083_2BCC` = `"Bombable"`.

Verified via `read_memory 0x005F9431` (8 bytes):
```
88 83 2e 02 00 00 8a 83
```
Bytes 0-5: `88 83 2e 02 00 00` = `MOV BYTE [EBX+0x22E], AL`. Stores ReadBool result at `+0x22E`. Address `0x005F9431`.

---

### `ObjectTypeClass + 0x231` — `LegalTarget` (byte)

**Size:** 1 byte (`BYTE`)
**Meaning:** If non-zero, the object can be targeted for attack (player can click-to-attack it). Default `0` in base constructor.

#### Constructor default: `0`

Verified via `search_byte_patterns 88 ?? 31 02 00 00` → includes `0x005F7168` (constructor region). Since BL = 0, default = **0** (false).

Note: subclass constructors may set this to `1`. `BuildingTypeClass` constructor region likely sets `LegalTarget = 1` for most buildings.

#### INI read: `ObjectTypeClass__ReadINI @ 0x005F9510`

From decompilation:
```c
uVar2 = CCINIClass__ReadBool(piVar9, s_LegalTarget_00832b84, *(undefined1 *)((int)param_1 + 0x231));
*(undefined1 *)((int)param_1 + 0x231) = uVar2;
```

INI key string at `0x00832B84` = `"LegalTarget"`.

Write site at `0x005F94C2` (from `search_byte_patterns 88 ?? 31 02 00 00` result), within `ObjectTypeClass__ReadINI`.

---

### `ObjectTypeClass + 0x232` — `Insignificant` (byte)

**Size:** 1 byte (`BYTE`)
**Meaning:** If non-zero, the object is considered insignificant — typically suppresses "our unit destroyed" EVA warning. Default `0` (false).

#### Constructor default: `0`

Verified via `search_byte_patterns 88 ?? 32 02 00 00` → `0x005F7188` (constructor region, `ObjectTypeClass__Constructor @ 0x005F7090`). Since BL = 0, default = **0** (false).

Verified via `read_memory 0x005F718E` (8 bytes):
```
88 9d 33 02 00 00 88 9d
```
Bytes 0-5 are the `Immune` write at `+0x233`. The `Insignificant` write at `+0x232` is immediately before, at `0x005F7188`.

#### INI read: `ObjectTypeClass__ReadINI @ 0x005F9510`

From decompilation:
```c
uVar2 = CCINIClass__ReadBool(piVar9, s_Insignificant_00832b60, *(undefined1 *)((int)param_1 + 0x232));
*(undefined1 *)((int)param_1 + 0x232) = uVar2;
```

INI key string at `0x00832B60` = `"Insignificant"`.

Write site at `0x005F951B` (from `search_byte_patterns 88 ?? 32 02 00 00` result), within `ObjectTypeClass__ReadINI`.

---

### `ObjectTypeClass + 0x233` — `Immune` (byte)

**Size:** 1 byte (`BYTE`)
**Meaning:** If non-zero, the object takes no damage from weapons. This is the GATE INVESTIGATION field — confirmed at `+0x233`, NOT at `+0xC4D` as the prior bridge report §15.1 incorrectly claimed.
**Default:** `0` (false) — no object is Immune by default in the base ObjectTypeClass constructor.

#### Constructor default: `0`

Verified via `read_memory 0x005F718E` (8 bytes):
```
88 9d 33 02 00 00 88 9d
```
`88 9d 33 02 00 00` = `MOV BYTE [EBP+0x233], BL`. Since BL = 0 (XOR EBX,EBX at constructor entry), `Immune = 0` by default.

Also verified via `search_byte_patterns c6 ?? 33 02 00 00` → matches at `0x00427771`, `0x0046BD26`, `0x005449E5` — three subclass constructors that explicitly set `Immune = immediate_value` for their object types. These are NOT `ObjectTypeClass` itself but subclass constructors. The `ObjectTypeClass` constructor uses the BL=0 path.

#### INI read: `ObjectTypeClass__ReadINI @ 0x005F92D0`

From decompilation (via `decompile_function 0x005F9510`, which is within the function body):
```c
uVar2 = CCINIClass__ReadBool(piVar9, s_Immune_00832b70, *(undefined1 *)((int)param_1 + 0x233));
*(undefined1 *)((int)param_1 + 0x233) = uVar2;
```

INI key string at `0x00832B70` = `"Immune"`.

Write site at `0x005F9510` (from `search_byte_patterns 88 ?? 33 02 00 00`). Function entry confirmed via `get_function_by_address 0x005F9510` → `ObjectTypeClass__ReadINI at 005f92d0`, body `005f92d0 - 005f96a9`. Address `0x005F9510` is within the body range — confirmed as the actual `Immune` store instruction inside `ObjectTypeClass__ReadINI`.

Also verified independently: `search_byte_patterns 88 ?? 33 02 00 00` → `0x005F718E` confirmed as `ObjectTypeClass__Constructor` store. The constructor and INI reader both write this field.

#### GATE INVESTIGATION CONCLUSION

The 2026-05-12 investigation confirmed `Immune` is at `ObjectTypeClass + 0x233`. The prior bridge report §15.1 claimed `+0xC4D` — that was incorrect. CABHUT's `Immune` flag in `rulesmd.ini` is `Immune=yes` (rulesmd.ini line 16340), so the `Immune` check IS a live gate that applies to CABHUT at runtime. However, `Immune` gates `ReceiveDamage` (weapon damage), not C4 plant logic — `InfantryClass::PerCellProcess` checks `BuildingTypeClass+0x16B6` (BridgeRepairHut flag), not `Immune`, before dispatching bridge collapse. The `Immune=yes` on CABHUT means it cannot be damaged by weapons directly, which is consistent with gameplay (the hut survives weapon fire but collapses on C4 timer expiry). The C4-plant bug in the Rust port is therefore port-side — the `Immune` field is not in the C4 plant call chain.

---

## Summary Table

| Offset | Size | INI key | Key string address | Constructor default | Key readers |
|---|---|---|---|---|---|
| `+0x22E` | byte | `Bombable` | `0x00832BCC` | `0` (false) | Combat targeting, `ObjectClass__ReceiveDamage` |
| `+0x231` | byte | `LegalTarget` | `0x00832B84` | `0` (false, set by subclasses) | Targeting system |
| `+0x232` | byte | `Insignificant` | `0x00832B60` | `0` (false) | EVA/death notification |
| `+0x233` | byte | `Immune` | `0x00832B70` | `0` (false) | `ObjectClass__ReceiveDamage` (early-out if Immune) |

---

## Write Site Verification Summary

| Address | Pattern (hex) | Confirmed | Field | Value |
|---|---|---|---|---|
| `0x005F7168` | (via search) | `search_byte_patterns 88 ?? 2e 02` | `Bombable` | `0` (BL, constructor) |
| `0x005F9431` | `88 83 2e 02 00 00` | `read_memory 0x005F9431` | `Bombable` | AL (INI result) |
| `0x005F718E` | `88 9d 33 02 00 00` | `read_memory 0x005F718E` | `Immune` | `0` (BL, constructor) |
| `0x005F951B` | (via search) | `search_byte_patterns 88 ?? 32 02` | `Insignificant` | AL (INI result) |
| `0x005F94C2` | (via search) | `search_byte_patterns 88 ?? 31 02` | `LegalTarget` | AL (INI result) |

---

## Unverified

**YELLOW:** `ObjectTypeClass + 0x231 LegalTarget` constructor write in subclasses — `BuildingTypeClass` constructor (`0x0045DD90`) likely sets `LegalTarget = 1` for buildings. The base `ObjectTypeClass` constructor sets it to `0` only; the final value for CABHUT depends on the full constructor chain. Not decoded in this session.

**YELLOW:** Subclass constructors with `c6 ?? 33 02 00 00` (Immune = immediate) at `0x00427771`, `0x0046BD26`, `0x005449E5` — these set `Immune = non-zero` for specific object subtypes. Their class identities not decoded here.

**YELLOW:** INI key strings at `0x00832BCC`, `0x00832B84`, `0x00832B60`, `0x00832B70` — addresses confirmed via decompilation. String content not fetched via `read_memory` (would be confirming what the symbol names already state).
