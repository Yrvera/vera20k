# RulesClass::ReadAudioVisual — RepairBridgeSound Field — Decode Doc

Host function address: `0x006691E0` (`RulesClass::ReadAudioVisual`)  
INI callsite address: `0x00669F0A` (data reference to string `"RepairBridgeSound"`)  
Store address: `0x00669F2B` (`MOV dword ptr [ESI + 0x248], EAX`)  
Emit callsite: `0x00519BD3`–`0x00519C02` in `InfantryClass::PerCellProcess @ 0x00519630`  
Scope: Narrow — RepairBridgeSound INI read + storage offset + sound-emit callsite trace.

## Summary

`RulesClass::ReadAudioVisual` reads the `RepairBridgeSound` key from `[AudioVisual]`
in `rules(md).ini`, converts the name via `VocClass::FindByName`, and stores the
resulting VocIndex at `RulesClass+0x248`. The stored index is consumed in
`InfantryClass::PerCellProcess` at the bridge-repair-complete branch, which calls
`VocClass::PlayAt` with the VocIndex and the engineer's world position.

Default in `rulesmd.ini` line 721: `RepairBridgeSound=BridgeRepaired`.

## Active in YR

**Yes.** `RulesClass::ReadAudioVisual` is called from `RulesClass::Process @
0x00668BF0` (verified via `get_function_callers 0x006691E0`), which fires on every
game load. The emit is in `InfantryClass::PerCellProcess` — a live YR path on every
bridge repair completion event.

## INI Read Callsite

From `get_assembly_context 0x00669F0A` (10 instructions of context):

```asm
00669EF8: MOV EDX, dword ptr [0x007F0C7C]   ; CCINIClass* (rules INI)
00669EFE: MOV EBX, dword ptr [ESI + 0x248]  ; save prior value
00669F04: PUSH ECX
00669F05: PUSH 0x889F64                      ; section name handle
00669F0A: PUSH 0x83A7FC                      ; key = "RepairBridgeSound"
00669F0F: PUSH EDX
00669F10: MOV ECX, EDI
00669F12: CALL 0x00528A10                    ; CCINIClass::ReadString
00669F17: TEST EAX, EAX
00669F19: JZ 0x00669F29                      ; not found → keep prior
00669F1B: LEA ECX, [ESP + 0x14]
00669F1F: CALL 0x007514D0                    ; VocClass::FindByName → VocIndex in EAX
00669F24: CMP EAX, -0x1
00669F27: JNZ 0x00669F2B
00669F29: MOV EAX, EBX                       ; not found: restore prior
00669F2B: MOV dword ptr [ESI + 0x248], EAX  ; STORE VocIndex to RulesClass+0x248
```

String `"RepairBridgeSound"` confirmed via `inspect_memory_content 0x0083A7FC`
(detected string, null-terminated at byte 17).

`CCINIClass::ReadString @ 0x00528A10` confirmed via `get_function_by_address 0x00528A10`.

`VocClass::FindByName @ 0x007514D0` confirmed via `get_function_by_address 0x007514D0`.

## Storage Offset

**`RulesClass + 0x248`** — `int` VocIndex; `-1` = no sound configured.

Confirmed at write: `MOV dword ptr [ESI+0x248], EAX` at `0x00669F2B`, `ESI` = RulesClass `this`.

Confirmed at read: `MOV ECX, dword ptr [EAX+0x248]` at `0x00519BF8`, `EAX` = singleton pointer.

### Adjacent fields

Assembly shows `[ESI+0x244]` is written just before this read in `ReadAudioVisual`, and
`[ESI+0x24C]` is the next field read after (per `get_assembly_context 0x00669F29` context_after):

| Offset | Known field |
|---|---|
| `+0x244` | Preceding AudioVisual sound index (identity not decoded here) |
| **`+0x248`** | **RepairBridgeSound** |
| `+0x24C` | Next AudioVisual sound index (next INI key at `0x0083A7DC` = `"PsychicDominatorActivate"`) |

## Sound-Emit Callsite

Found via `get_assembly_context 0x00519BC4` within `InfantryClass::PerCellProcess @
0x00519630`:

```asm
00519B82: MOV AL, byte ptr [ECX + 0x16B6]   ; BuildingTypeClass+0x16B6 = BridgeRepairHut flag
00519B8A: JZ 0x00519D47                      ; not a BridgeRepairHut → skip
; [bridge-repair completion checks]
00519BCE: MOV EAX, [0x008871E0]              ; load g_RulesClass_Instance
00519BD3: CMP dword ptr [EAX + 0x248], -0x1  ; RepairBridgeSound == -1?
00519BDA: JZ 0x00519C07                      ; no sound → skip
00519BDC: LEA EDX, [EDI + 0x9C]             ; engineer position (leptons at TechnoClass+0x9C)
00519BE2: PUSH 0x0                           ; volume param
00519BE4: MOV ECX, dword ptr [EDX]           ; position X
00519BE6: MOV dword ptr [ESP + 0x3C], ECX
00519BEA: MOV ECX, dword ptr [EDX + 0x4]    ; position Y
00519BED: MOV dword ptr [ESP + 0x40], ECX
00519BF1: MOV EDX, dword ptr [EDX + 0x8]    ; position Z
00519BF4: MOV dword ptr [ESP + 0x44], EDX
00519BF8: MOV ECX, dword ptr [EAX + 0x248]  ; VocIndex = RepairBridgeSound
00519BFE: LEA EDX, [ESP + 0x3C]             ; coord buf
00519C02: CALL 0x007509E0                    ; VocClass::PlayAt(VocIndex, coord, volume)
00519C07: [continue]
```

`VocClass::PlayAt @ 0x007509E0` confirmed via `get_function_by_address 0x007509E0`.

`g_RulesClass_Instance = 0x008871E0` confirmed from the direct `MOV EAX, [0x008871E0]`
instruction at `0x00519BCE`.

### Emit trigger conditions

1. Cell action is bridge-repair completion (gated by `BuildingTypeClass+0x16B6` BridgeRepairHut check).
2. `RulesClass+0x248 != -1` (sound is configured; stock YR: `BridgeRepaired`).

The sound plays at the engineer's world position (`TechnoClass+0x9C` leptons), not at the bridge tile.

## Struct Field Summary

| Class | Offset | Type | Field | Notes |
|---|---|---|---|---|
| RulesClass | `+0x248` | `int` | RepairBridgeSound | VocIndex; -1 = disabled |

## Globals Referenced

| Global | Value (static) | Role |
|---|---|---|
| `0x008871E0` | (pointer, runtime) | `g_RulesClass_Instance` — RulesClass singleton |
| `0x0083A7FC` | `"RepairBridgeSound"` | INI key string literal |
| `0x007F0C7C` | (pointer, runtime) | CCINIClass* rules INI handle |

## Callers

Verified via `get_function_callers 0x006691E0`:

| Caller | Address | Notes |
|---|---|---|
| `RulesClass::Process` | `0x00668BF0` | Live YR path; called from `ScenarioClass::Full_Init` |
| `CDFileClass::Constructor` | `0x0052CD70` | TS-era stub path |

## Out-of-scope Refs

- `CCINIClass::ReadString` @ `0x00528A10`
- `VocClass::FindByName` @ `0x007514D0`
- `VocClass::PlayAt` @ `0x007509E0`
- `InfantryClass::PerCellProcess` @ `0x00519630` — emit host (decode task #2)
- `RulesClass::ReadAudioVisual` @ `0x006691E0` — full function decode out of scope

## Self-Proof

### Claim 1: INI key is "RepairBridgeSound" at `0x0083A7FC`

`inspect_memory_content 0x0083A7FC` → detected string `"RepairBridgeSound"`, null at byte 17.
`get_xrefs_to 0x0083A7FC` → single xref from `0x00669F0A` in `RulesClass__ReadAudioVisual`.
**VERIFIED.**

### Claim 2: Storage offset is `RulesClass+0x248`

`get_assembly_context 0x00669F0A` → `00669F2B: MOV dword ptr [ESI+0x248], EAX` (write site).
`get_assembly_context 0x00519BC4` → `00519BF8: MOV ECX, dword ptr [EAX+0x248]` (read site).
Both use offset `0x248` on the RulesClass instance. **VERIFIED at both write and read.**

### Claim 3: Emit callsite in `InfantryClass::PerCellProcess`; `g_RulesClass_Instance=0x008871E0`

`get_assembly_context 0x00519BC4` → `00519BCE: MOV EAX, [0x008871E0]`, then
`CMP dword ptr [EAX+0x248], -0x1`, then `CALL 0x007509E0`.
`get_function_by_address 0x007509E0` → `VocClass__PlayAt`. **VERIFIED.**

## Unverified Claims (YELLOW)

- The next INI key after `RepairBridgeSound` at `+0x24C` is `PsychicDominatorActivate`
  (confirmed: `inspect_memory_content 0x0083A7DC` → `"PsychicDominatorActivate"`). This
  is consistent with the assembly sequence but the field identity at `+0x24C` is not a
  primary focus of this decode.
- The exact INI section string at `DAT_00889F64` (passed as section name) reads as all-zeros
  at static time — runtime-populated, likely `"AudioVisual"` based on function name.
  Not directly verified.
- `BuildingTypeClass+0x16B6` as BridgeRepairHut flag at the emit gate: inferred from
  task #1 and task #15 docs. Consistent but struct layout confirmation belongs to task #19.
