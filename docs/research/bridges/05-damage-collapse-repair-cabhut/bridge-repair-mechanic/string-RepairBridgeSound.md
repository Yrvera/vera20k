# INI Key "RepairBridgeSound" — Decode Doc

**String address:** `0x0083A7FC`
**String content:** `"RepairBridgeSound"` (17 bytes + null terminator)
**Host function:** `RulesClass::ReadAudioVisual @ 0x006691E0`
**Struct field written:** `RulesClass + 0x248` (int, VocIndex)
**INI section:** `[AudioVisual]`
**Scope:** Narrow — string address, INI read callsite, storage offset, emit callsite.

---

## Summary

`"RepairBridgeSound"` is an `[AudioVisual]` INI key. At game load, `RulesClass::ReadAudioVisual`
reads its value, converts the sound name to a `VocClass` index via `VocClass::FindByName`, and
stores the result at `RulesClass + 0x248`. The stored VocIndex is consumed by
`InfantryClass::PerCellProcess` at the bridge-repair completion branch: when an engineer finishes
repairing a bridge, the sound is played at the engineer's world position if the VocIndex is not -1.

Stock `rulesmd.ini` line 721: `RepairBridgeSound=BridgeRepaired`.

---

## Active in YR

**YES.** `RulesClass::ReadAudioVisual` is called from `RulesClass::Process @ 0x00668BF0`
(confirmed via `get_function_callers 0x006691E0`), which fires on every game load from
`ScenarioClass::Full_Init`. The emit is in `InfantryClass::PerCellProcess` — a live YR path
triggered every time an engineer completes bridge repair (once per repair event, fires on the
step where the repair-completion condition is met).

---

## String Verification

From `inspect_memory_content 0x0083A7FC` (20 bytes):

```
hex: 52 65 70 61 69 72 42 72 69 64 67 65 53 6F 75 6E 64 00 ...
     R  e  p  a  i  r  B  r  i  d  g  e  S  o  u  n  d  \0
```

Null terminator at byte offset 17. String: `"RepairBridgeSound"`, 17 characters.

**VERIFIED** via `inspect_memory_content 0x0083A7FC`.

---

## INI Read Callsite

Single xref from `get_xrefs_to 0x0083A7FC`:

> `From 0x00669F0A in RulesClass__ReadAudioVisual [DATA]`

From `get_assembly_context 0x00669F0A` (10 instructions of context):

```asm
00669EF8: MOV EDX, dword ptr [0x007F0C7C]   ; CCINIClass* (rules INI handle)
00669EFE: MOV EBX, dword ptr [ESI + 0x248]  ; save prior VocIndex (default = current value)
00669F04: PUSH ECX
00669F05: PUSH 0x889F64                      ; section name (runtime: "AudioVisual")
00669F0A: PUSH 0x83A7FC                      ; key = "RepairBridgeSound"
00669F0F: PUSH EDX
00669F10: MOV ECX, EDI
00669F12: CALL 0x00528A10                    ; CCINIClass::ReadString
00669F17: TEST EAX, EAX
00669F19: JZ 0x00669F29                      ; not found → restore prior
00669F1B: LEA ECX, [ESP + 0x14]
00669F1F: CALL 0x007514D0                    ; VocClass::FindByName → VocIndex in EAX
00669F24: CMP EAX, -0x1
00669F27: JNZ 0x00669F2B
00669F29: MOV EAX, EBX                       ; restore prior if not found
00669F2B: MOV dword ptr [ESI + 0x248], EAX  ; STORE VocIndex → RulesClass+0x248
```

**VERIFIED** write site at `0x00669F2B`, storage offset `RulesClass + 0x248`.

---

## Storage Field

`RulesClass + 0x248` — `int` (VocIndex), `-1` = no sound configured.

- **Write:** `MOV dword ptr [ESI + 0x248], EAX` at `0x00669F2B` in `RulesClass::ReadAudioVisual`.
- **Read:** `MOV ECX, dword ptr [EAX + 0x248]` at `0x00519BF8` in `InfantryClass::PerCellProcess`.
- `g_RulesClass_Instance` singleton pointer at `0x008871E0` — confirmed from `MOV EAX, [0x008871E0]`
  at `0x00519BCE`.

### Adjacent fields

| Offset | Field identity |
|--------|----------------|
| `+0x244` | Preceding AudioVisual sound index (identity not decoded) |
| **`+0x248`** | **RepairBridgeSound** VocIndex |
| `+0x24C` | `PsychicDominatorActivate` VocIndex (next INI key in `ReadAudioVisual`) |

The `+0x24C` adjacency was confirmed via `inspect_memory_content 0x0083A7DC` → `"PsychicDominatorActivate"`.

---

## Sound-Emit Callsite

In `InfantryClass::PerCellProcess @ 0x00519630`, the bridge-repair completion branch:

```asm
00519BCE: MOV EAX, [0x008871E0]              ; load g_RulesClass_Instance
00519BD3: CMP dword ptr [EAX + 0x248], -0x1  ; RepairBridgeSound == -1?
00519BDA: JZ 0x00519C07                      ; no sound → skip
00519BDC: LEA EDX, [EDI + 0x9C]             ; engineer position (TechnoClass+0x9C leptons)
00519BE2: PUSH 0x0                           ; volume param
00519BE4: MOV ECX, dword ptr [EDX]           ; position X
00519BE6: MOV dword ptr [ESP + 0x3C], ECX
00519BEA: MOV ECX, dword ptr [EDX + 0x4]    ; position Y
00519BED: MOV dword ptr [ESP + 0x40], ECX
00519BF1: MOV EDX, dword ptr [EDX + 0x8]    ; position Z
00519BF4: MOV dword ptr [ESP + 0x44], EDX
00519BF8: MOV ECX, dword ptr [EAX + 0x248]  ; VocIndex = RepairBridgeSound
00519BFE: LEA EDX, [ESP + 0x3C]             ; coord buffer
00519C02: CALL 0x007509E0                    ; VocClass::PlayAt(VocIndex, coord, volume)
```

**Emit conditions:**
1. Cell action is bridge-repair completion (gated by `BuildingTypeClass + 0x16B6` BridgeRepairHut check at `0x00519B82`).
2. `RulesClass + 0x248 != -1` (sound is configured; stock YR: `BridgeRepaired`).

Sound plays at the engineer's world position (`TechnoClass + 0x9C` leptons), not at the bridge tile.

**VERIFIED** emit callsite at `0x00519BD3`–`0x00519C02` via `get_assembly_context 0x00519BF8`
(from task #16 decode, cross-referenced with `fn-ReadAudioVisual-RepairBridgeSound.md`).

---

## INI Values in Stock rulesmd.ini

| INI file | Section | Key | Value | Line |
|---|---|---|---|---|
| `rulesmd.ini` | `[AudioVisual]` | `RepairBridgeSound` | `BridgeRepaired` | 721 |

`VocClass::FindByName("BridgeRepaired")` returns the VocIndex for the `BridgeRepaired` sound cue
at load time. Stock YR: sound always configured (not -1).

---

## Out-of-scope Refs

- `CCINIClass::ReadString @ 0x00528A10`
- `VocClass::FindByName @ 0x007514D0`
- `VocClass::PlayAt @ 0x007509E0`
- `RulesClass::ReadAudioVisual @ 0x006691E0` — full function decode out of scope
- `InfantryClass::PerCellProcess @ 0x00519630` — full decode in `fn-InfantryClass-PerCellProcess-C4Plant.md`

---

## Self-Proof

### Claim 1: String `"RepairBridgeSound"` at `0x0083A7FC`, null-terminated at byte 17

`inspect_memory_content 0x0083A7FC` → hex `52 65 70 61 69 72 42 72 69 64 67 65 53 6F 75 6E 64 00`
= `R e p a i r B r i d g e S o u n d \0`. Null at byte 17, string length 17. **VERIFIED.**

### Claim 2: Single xref — only `RulesClass::ReadAudioVisual` reads this string

`get_xrefs_to 0x0083A7FC` → exactly one result: `From 0x00669F0A in RulesClass__ReadAudioVisual [DATA]`.
**VERIFIED — one INI read site, one writer.**

### Claim 3: Storage offset `RulesClass + 0x248`; write at `0x00669F2B`; read at `0x00519BF8`

`get_assembly_context 0x00669F0A` → `00669F2B: MOV dword ptr [ESI + 0x248], EAX` (write site, ESI = RulesClass this).
`get_assembly_context 0x00519BC4` → `00519BF8: MOV ECX, dword ptr [EAX + 0x248]` (read site, EAX = g_RulesClass_Instance).
Both agree on offset `0x248`. **VERIFIED at both write and read sites.**

---

## Globals Referenced

| Address | Content | Role |
|---|---|---|
| `0x0083A7FC` | `"RepairBridgeSound"` | INI key string literal |
| `0x008871E0` | (runtime pointer) | `g_RulesClass_Instance` — RulesClass singleton |
| `0x007F0C7C` | (runtime pointer) | `CCINIClass*` rules INI handle |
| `0x007509E0` | `VocClass::PlayAt` | Sound-emit function |
| `0x007514D0` | `VocClass::FindByName` | Name-to-VocIndex conversion |
