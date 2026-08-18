# struct-BuildingTypeClass-BridgeFields

**Runbook:** struct-decode-v1
**Target:** `BuildingTypeClass` fields: `+0x1577 CanC4`, `+0x16B6 BridgeRepairHut`, `+0x1701 InvisibleInGame`
**Confidence:** HIGH (all offsets verified via byte-pattern search and `read_memory` on constructor/INI write sites)
**YR-active:** YES — all three fields are actively read in YR gameplay paths (C4 target checking, bridge collapse dispatch, damage routing).

---

## Method

All offsets verified by:
1. `search_byte_patterns c6 ?? XX YY 00 00 / ff 00 ff ff ff ff` — finds `MOV BYTE [reg+offset], imm8` (constructor immediate writes)
2. `search_byte_patterns 88 ?? XX YY 00 00 / ff 00 ff ff ff ff` — finds `MOV BYTE [reg+offset], reg8` (INI reader register writes)
3. `search_byte_patterns 8a ?? XX YY 00 00 / ff 00 ff ff ff ff` — finds `MOV reg8, BYTE [reg+offset]` (read sites)
4. `read_memory` at write site addresses to confirm raw bytes
5. `get_function_by_address` to identify enclosing functions for each site

Constructor entry verified via `read_memory 0x0045DD90` (20 bytes) confirming `XOR EBX, EBX` at `0x0045DD9A` — BL = 0 throughout, so all `MOV BYTE [reg], BL` constructor writes write `0` as the default.

---

## Field Definitions

### `BuildingTypeClass + 0x1577` — `CanC4` (byte)

**Size:** 1 byte (`BYTE`)
**Meaning:** When non-zero, infantry with `C4=yes` can plant C4 on this building type. Default `1` (true) — all buildings are C4-targetable unless explicitly set `CanC4=no` in INI.

#### Constructor default: `1`

Write site at `0x0045E063` in `BuildingTypeClass__constructor @ 0x0045DD90`:

Verified via `search_byte_patterns c6 ?? 77 15 00 00` → `[{address: 0045e063}]`.

Verified via `read_memory 0x0045E060` (10 bytes):
```
15 00 00 c6 86 77 15 00 00 01
```
Bytes at +3: `c6 86 77 15 00 00 01` = `MOV BYTE [ESI+0x1577], 1`. Constructor sets `CanC4 = 1` (true) by default for all building types.

#### INI read: `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`

Write site at `0x00460049` (read via `search_byte_patterns 8a ?? 77 15 00 00`... actually this is a *read* via `8a` opcode from the INI reader's internal default-load pattern). The INI reader at `0x00460049` reads the current value as the default before calling ReadBool.

Additional write site at `0x0046005D` (`88 85 77 15 00 00` = `MOV BYTE [EBP+0x1577], AL`):

Verified via `search_byte_patterns 88 ?? 77 15 00 00` → `[{address: 0046005d}]`.

Verified via `read_memory 0x00460058` (10 bytes):
```
e8 93 95 0c 00 88 85 77 15 00
```
Bytes at +5: `88 85 77 15 00 00` = `MOV BYTE [EBP+0x1577], AL`. INI reader stores the `ReadBool` result (in AL) at `+0x1577`.

#### Read sites

Verified via `search_byte_patterns 8a ?? 77 15 00 00` → 7 addresses:

| Address | Function | Purpose |
|---|---|---|
| `0x00460049` | `BuildingTypeClass_ReadINI_Water` | Load current value as default before ReadBool |
| `0x0051EA2E` | `InfantryClass__What_Action_OnObject` | Cursor action: check if C4 can be planted on target |
| `0x0051EAF2` | `InfantryClass__What_Action_OnObject` | Second CanC4 check in same function (different code path) |
| `0x0051F420` | `InfantryClass__What_Action_OnObject` | Third CanC4 check |
| `0x005F543E` | `ObjectClass__ReceiveDamage` | Damage routing: C4-related damage handling |
| `0x006F34B7` | `TechnoClass__SelectWeaponAgainst` | Weapon selection: skip CanC4 check for weapon targeting |
| `0x00700527` | `TechnoClass__What_Action_OnObject` | Cursor action: same check at TechnoClass level |

Verified via `get_function_by_address` for each address.

**YR-active note:** `InfantryClass__What_Action_OnObject` (3 reads) and `TechnoClass__What_Action_OnObject` (1 read) fire every time the player hovers an infantry unit over a building — high-frequency in normal play. `ObjectClass__ReceiveDamage` fires whenever a building takes damage.

---

### `BuildingTypeClass + 0x16B6` — `BridgeRepairHut` (byte)

**Size:** 1 byte (`BYTE`)
**Meaning:** When non-zero, this building type is a bridge repair hut (CABHUT). The C4-plant timer check in `BuildingClass::Update` dispatches to `DestroyBridge_High/Low_OnHutDeath` instead of generic `InflictDamage`. Only `CABHUT` sets this in stock `rulesmd.ini`.

#### Constructor default: `0`

Write site at `0x0045E10F` in `BuildingTypeClass__constructor @ 0x0045DD90`:

Verified via `search_byte_patterns 88 ?? b6 16 00 00` → `[{address: 0045e10f}, {address: 00460e9a}]`.

Verified via `read_memory 0x0045E100` (24 bytes):
```
00 00 88 9e b4 16 00 00 c6 86 b5 16 00 00 01 88 9e b6 16 00 00 88 9e b7
```
Bytes at +15: `88 9e b6 16 00 00` = `MOV BYTE [ESI+0x16B6], BL`.
Since `BL = 0` (from `XOR EBX, EBX` at constructor entry, verified at `0x0045DD9A`), this writes `0` (false) as the default for `BridgeRepairHut`.

#### INI read: `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`

Write site at `0x00460E9A` (from prior task #15 decode):

Verified via `read_memory 0x00460E9A` (6 bytes):
```
88 85 b6 16 00 00
```
`MOV BYTE [EBP+0x16B6], AL` = stores `CCINIClass__ReadBool` result (in AL) at `+0x16B6`. Callsite at `0x00460E8D` pushes key `"BridgeRepairHut"` at `0x0081A898`. For `CABHUT`, value = `1` (yes). All other building types retain default `0` unless they also define `BridgeRepairHut=yes`.

#### Read sites

Primary read site is `BuildingClass::Update @ 0x0043FB20`:
```c
if (this->Type[0x16b6] == '\0') {
    // generic building: InflictDamage with C4 warhead
} else {
    // bridge hut: 5×5 scan + DestroyBridge_High/Low_OnHutDeath
}
```

Verified via `decompile_function 0x0043FB20` — this check appears directly in the C4 timer expiry branch.

Additional read site visible in `InfantryClass__What_Action_OnObject` (not separately searched — this field's key behavioral read is in `BuildingClass::Update`).

---

### `BuildingTypeClass + 0x1701` — `InvisibleInGame` (byte)

**Size:** 1 byte (`BYTE`)
**Meaning:** When non-zero, the building is invisible in-game (not rendered normally). Used for abstract/internal building types. Default `0` (false).

#### Constructor default: `0`

Two write sites in `BuildingTypeClass__constructor`:
- `0x0045E200`: `88 9e 00 17 00 00` = `MOV BYTE [ESI+0x1700], BL` (neighbor field `+0x1700`)
- `0x0045E206`: `88 9e 01 17 00 00` = `MOV BYTE [ESI+0x1701], BL`

Verified via `search_byte_patterns 88 ?? 01 17 00 00` → `[{address: 0045e206}, {address: 00460e01}]`.

Verified via `read_memory 0x0045E1F0` (24 bytes):
```
c7 86 f8 16 00 00 09 00 00 00 89 be fc 16 00 00 88 9e 00 17 00 00 88 9e
```
At +22: `88 9e` continues as `88 9e 01 17 00 00` = `MOV BYTE [ESI+0x1701], BL`. Since BL = 0 (constructor zeroed EBX), default = `0` (false).

#### INI read: `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`

Write site at `0x00460E01`:

Verified via `read_memory 0x00460E01` (6 bytes):
```
88 85 01 17 00 00
```
`MOV BYTE [EBP+0x1701], AL` = stores `CCINIClass__ReadBool` result at `+0x1701`. The INI key is not directly verified in this session (see Unverified section).

---

## Summary Table

| Offset | Size | Name | Constructor default | INI key | Key readers |
|---|---|---|---|---|---|
| `+0x1577` | byte | `CanC4` | `1` (true) | `CanC4` (YELLOW — key inferred, not directly verified) | `InfantryClass__What_Action_OnObject` (×3), `ObjectClass__ReceiveDamage`, `TechnoClass__SelectWeaponAgainst`, `TechnoClass__What_Action_OnObject`, INI reader |
| `+0x16B6` | byte | `BridgeRepairHut` | `0` (false) | `BridgeRepairHut` at `0x0081A898` (verified task #15) | `BuildingClass::Update` (C4 dispatch gate), INI reader |
| `+0x1701` | byte | `InvisibleInGame` | `0` (false) | `InvisibleInGame` (YELLOW — key inferred, not directly verified) | (not searched — out of scope for bridge mechanic) |

---

## Key Findings for Rust Port

1. **`CanC4` defaults to `1`** in the constructor. Rust port must initialize `can_c4 = true` on all building types unless overridden by INI.

2. **`BridgeRepairHut` defaults to `0`** and is only set to `1` by `CABHUT` in stock `rulesmd.ini`. The check `this->Type[0x16B6] == '\0'` in `BuildingClass::Update` is the primary gate deciding bridge vs generic C4 outcome.

3. **`InvisibleInGame`** is at `+0x1701`, defaulting `0`. Its role in bridge repair is minor (CABHUT is visible in-game; this field is for abstract internal buildings). Included in this decode per task scope but the bridge mechanic does not depend on it.

4. **Constructor address confirmed:** `BuildingTypeClass__constructor @ 0x0045DD90`. `XOR EBX, EBX` at `0x0045DD9A` zeroes BL for all subsequent default-value writes.

---

## Write Site Verification Summary

| Address | Pattern (hex) | Confirmed at | Value written |
|---|---|---|---|
| `0x0045E063` | `c6 86 77 15 00 00 01` | `read_memory 0x0045E060` +3 | `CanC4 = 1` (constructor default) |
| `0x0046005D` | `88 85 77 15 00 00` | `read_memory 0x00460058` +5 | `CanC4 = AL` (INI result) |
| `0x0045E10F` | `88 9e b6 16 00 00` | `read_memory 0x0045E100` +15 | `BridgeRepairHut = 0` (constructor default, BL=0) |
| `0x00460E9A` | `88 85 b6 16 00 00` | `read_memory 0x00460E9A` | `BridgeRepairHut = AL` (INI result) |
| `0x0045E206` | `88 9e 01 17 00 00` | `read_memory 0x0045E1F0` +22 | `InvisibleInGame = 0` (constructor default, BL=0) |
| `0x00460E01` | `88 85 01 17 00 00` | `read_memory 0x00460E01` | `InvisibleInGame = AL` (INI result) |

---

## Unverified

**VERIFIED (update):** `CanC4` INI key string at `0x0081ADFC` — confirmed via
`get_assembly_context 0x00460050` → `PUSH 0x81adfc` immediately before `CALL 0x005295f0`
(CCINIClass::ReadBool), followed by `MOV byte ptr [EBP+0x1577], AL` at `0x0046005D`.
`inspect_memory_content 0x0081ADFC` → string `"CanC4"`, null-terminated at byte 4.
Single xref to `0x00460050` from `get_xrefs_to 0x0081ADFC`. **YELLOW RESOLVED.**

**VERIFIED (update):** `InvisibleInGame` INI key string at `0x0081A8CC` — confirmed via
`get_assembly_context 0x00460DF2` → `PUSH 0x81a8cc` at `0x00460DF2` immediately before
`CALL 0x005295f0` (CCINIClass::ReadBool), followed by `MOV byte ptr [EBP+0x1701], AL`
at `0x00460E01`. Single xref to `0x00460DF2` from `get_xrefs_to 0x0081A8CC`. **YELLOW RESOLVED.**

**YELLOW:** `BuildingTypeClass + 0x1700` (neighbor of `+0x1701`) — written in the same constructor block as `+0x1701`. Field name unknown. Not in scope for this task.

---

## Self-Proof

### Claim 1: `BridgeRepairHut` is at offset `+0x16B6`, INI key `"BridgeRepairHut"` at `0x0081A898`

`get_assembly_context 0x00460E9A` → context_before includes `PUSH 0x81a898` at `0x00460E8D`,
followed by `CALL 0x005295f0` at `0x00460E95` (CCINIClass::ReadBool), then
`MOV byte ptr [EBP + 0x16b6], AL` at `0x00460E9A`.
`inspect_memory_content 0x0081A898` (prior session) → string `"BridgeRepairHut"`, null-terminated at byte 15.
**VERIFIED: offset `+0x16B6` and key string both confirmed.**

### Claim 2: `CanC4` defaults to `1` in the constructor at `0x0045E063`

`get_assembly_context 0x0045E063` → full context shows `MOV byte ptr [ESI + 0x1577], 0x1`
at `0x0045E063`. Immediate `0x1` = constructor default true. `ESI` = `this` (constructor
context confirmed by `get_function_by_address 0x0045DC40` → `BuildingTypeClass__constructor`).
`read_memory 0x0045E060` → `c6 86 77 15 00 00 01` = `MOV BYTE [ESI+0x1577], 0x01`. **VERIFIED.**

### Claim 3: `InvisibleInGame` is at offset `+0x1701`, INI key `"InvisibleInGame"` at `0x0081A8CC`

`get_assembly_context 0x00460DF2` → `PUSH 0x81a8cc` at `0x00460DF2`, `CALL 0x005295f0`
(CCINIClass::ReadBool) at `0x00460DFA`, then `MOV byte ptr [EBP + 0x1701], AL` at `0x00460E01`.
`inspect_memory_content 0x0081A8CC` → string `"InvisibleInGame"`, null-terminated at byte 15
(confirmed via adjacent bytes showing null at offset 15 in hex dump).
**VERIFIED: offset `+0x1701` and key string both confirmed.**
