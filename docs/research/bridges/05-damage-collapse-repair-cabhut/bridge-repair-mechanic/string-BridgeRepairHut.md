# INI Key "BridgeRepairHut" — Decode Doc

**String address:** `0x0081A898`
**String content:** `"BridgeRepairHut"` (15 bytes + null terminator)
**Host function:** `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`
**Struct field written:** `BuildingTypeClass + 0x16B6` (bool byte)
**INI section:** `[BuildingTypes]` (written per building-type entry)
**Scope:** Narrow — string address, single INI read callsite, struct field write, consume sites.

---

## Summary

`"BridgeRepairHut"` is a `BuildingType` INI key that marks a building as the bridge-repair hut.
When set to `yes`, the building type's `+0x16B6` flag is set to `1`. This flag is the primary
gate in two bridge-mechanic functions:

1. `BuildingClass::Update` (`0x0043FB20`) — C4-timer expiry dispatches to `DestroyBridge_High/Low_OnHutDeath`
   only when the target building type has `BridgeRepairHut = 1`. Without this flag, C4 instead
   inflicts generic damage.

2. `InfantryClass::PerCellProcess` (`0x00519630`) — engineer bridge-repair completion triggers
   `RepairBridgeSound` emit only when the cell's building type has `BridgeRepairHut = 1`.

In stock `rulesmd.ini`, line 16348: `BridgeRepairHut=yes` appears on `CABHUT` (the Allied bridge
repair hut) and `NABR` (the Soviet/Yuri counterpart).

---

## Active in YR

**YES.** The flag is read in `BuildingClass::Update` on every C4-timer expiry (fires whenever an
engineer has planted C4 on a building and the countdown completes — at most once per building per
engineer interaction) and in `InfantryClass::PerCellProcess` on every engineer cell step while
adjacent to a bridge hut (up to once per game tick per engineer). Both paths are live in standard
YR skirmish whenever bridges and engineers are present.

---

## String Verification

From `inspect_memory_content 0x0081A898` (20 bytes):

```
hex: 42 72 69 64 67 65 52 65 70 61 69 72 48 75 74 00 50 6F 77 65
     B  r  i  d  g  e  R  e  p  a  i  r  H  u  t  \0 P  o  w  e
```

Null terminator at byte offset 15. String: `"BridgeRepairHut"`, 15 characters.
Followed by `"Powe..."` — adjacent INI key string (PowersUpBuilding or similar, not decoded here).

**VERIFIED** via `inspect_memory_content 0x0081A898`.

---

## INI Read Callsite

Single xref from `get_xrefs_to 0x0081A898`:

> `From 00460e8d in BuildingTypeClass_ReadINI_Water [DATA]`

From `get_assembly_context 0x00460E9A` (the write site, 8 instructions of context):

```asm
00460e86: MOV DL, byte ptr [EBP + 0x16b6]    ; load prior value (default = 0)
00460e8c: PUSH EDX
00460e8d: PUSH 0x81a898                       ; key string = "BridgeRepairHut"
00460e92: PUSH EBX
00460e93: MOV ECX, ESI
00460e95: CALL 0x005295f0                     ; CCINIClass::ReadBool(section, key, default)
00460e9a: MOV byte ptr [EBP + 0x16b6], AL    ; STORE → BuildingTypeClass+0x16B6
```

`EBP` = BuildingTypeClass `this`. `CCINIClass::ReadBool @ 0x005295F0` reads the bool from the
current building type's INI section, returning the current value at `+0x16B6` as the default if
the key is absent.

**VERIFIED** write site at `0x00460E9A`.

---

## Struct Field

`BuildingTypeClass + 0x16B6` — bool (byte), default `0`.

Constructor default: `0` (false) — verified via `read_memory 0x0045E100` in the
`BuildingTypeClass__constructor @ 0x0045DD90` body:
```
Bytes at +15 (of 24-byte read starting at 0x0045E100):
88 9e b6 16 00 00  →  MOV BYTE [ESI+0x16B6], BL   ; BL = 0 (constructor zeroed EBX)
```
(from struct-BuildingTypeClass-BridgeFields.md, verified via `read_memory 0x0045E100`)

---

## Consume Sites

### 1. `BuildingClass::Update @ 0x0043FB20` — C4 dispatch gate

From `decompile_function 0x0043FB20` (in the C4-timer-expiry branch):

```c
if (this->Type[0x16b6] == '\0') {
    // BridgeRepairHut = 0: generic C4 damage
    InflictDamage(warhead, ...);
} else {
    // BridgeRepairHut = 1: bridge-hut C4 → scan + destroy bridge
    // 5×5 scan + DestroyBridge_High/Low_OnHutDeath
}
```

This fires on every C4 timer expiry for the building. For CABHUT/NABR (`BridgeRepairHut = 1`),
it routes to `DestroyBridge_Low_OnHutDeath` or `DestroyBridge_High_OnHutDeath` rather than
inflicting direct building damage.

### 2. `InfantryClass::PerCellProcess @ 0x00519630` — bridge-repair completion gate

From the emit callsite documented in `fn-ReadAudioVisual-RepairBridgeSound.md`:

```asm
00519B82: MOV AL, byte ptr [ECX + 0x16B6]   ; read BridgeRepairHut flag
00519B8A: JZ 0x00519D47                      ; if 0 (not a repair hut) → skip repair branch
; [bridge-repair completion checks]
00519BF8: MOV ECX, dword ptr [EAX + 0x248]  ; VocIndex = RepairBridgeSound
00519C02: CALL 0x007509E0                    ; VocClass::PlayAt(VocIndex, coord, volume)
```

The entire bridge-repair completion flow (tile restoration, sound emit) is gated on this flag.
Fires on each engineer step while standing on or adjacent to the bridge hut during repair.

---

## INI Values in Stock rulesmd.ini

From task description / rulesmd.ini line 16348:

| Building | BridgeRepairHut value | Notes |
|---|---|---|
| `CABHUT` | `yes` | Allied bridge repair hut |
| `NABR` | `yes` | Soviet/Yuri bridge repair hut |

All other building types retain default `no` (field = 0).

---

## Out-of-scope Refs

- `BuildingTypeClass_ReadINI_Water @ 0x0045FE50` — full function decode out of scope
- `CCINIClass::ReadBool @ 0x005295F0` — bool-read helper
- `BuildingClass::Update @ 0x0043FB20` — full decode in `fn-BuildingClass_Update_BridgeBranch.md`
- `InfantryClass::PerCellProcess @ 0x00519630` — full decode in `fn-InfantryClass-PerCellProcess-C4Plant.md`
- `DestroyBridge_Low_OnHutDeath @ 0x00574C20` / `DestroyBridge_High_OnHutDeath @ 0x00574000`

---

## Self-Proof

### Claim 1: String `"BridgeRepairHut"` at `0x0081A898`, null-terminated at byte 15

`inspect_memory_content 0x0081A898` → hex `42 72 69 64 67 65 52 65 70 61 69 72 48 75 74 00` =
`B r i d g e R e p a i r H u t \0`. Null at byte 15, string length 15. **VERIFIED.**

### Claim 2: Single xref — only `BuildingTypeClass_ReadINI_Water` reads this string

`get_xrefs_to 0x0081A898` → exactly one result: `From 00460e8d in BuildingTypeClass_ReadINI_Water [DATA]`.
No other function references this string. **VERIFIED — one INI read site, one writer.**

### Claim 3: Write site at `0x00460E9A` stores result to `BuildingTypeClass + 0x16B6`

`get_assembly_context 0x00460E9A` → `MOV byte ptr [EBP + 0x16b6], AL` at `0x00460E9A`.
`EBP` = BuildingTypeClass `this` (confirmed by enclosing function `BuildingTypeClass_ReadINI_Water`).
Offset `0x16B6` unambiguous from the instruction encoding. **VERIFIED.**

---

## Globals Referenced

| Address | Content | Role |
|---|---|---|
| `0x0081A898` | `"BridgeRepairHut"` | INI key string literal |
