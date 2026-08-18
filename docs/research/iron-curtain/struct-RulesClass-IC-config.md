# struct-RulesClass-IC-config

## Scope

RulesClass fields that configure the Iron Curtain system: duration, invoke animation, color overlay, and the warhead used to kill infantry.

## Source functions

All offsets derived from direct decompilation of the relevant INI read functions:
- `decompile_function 0x0066bbb0` (RulesClass__ReadCombatDamage) — verified `+0xfa8`, `+0xfe8`
- `get_assembly_context 0x0066e244` + `get_xrefs_to 0x0083cda0` (ReadGeneral context) — verified `+0x348`
- `get_assembly_context 0x0066b844` + `read_memory 0x0083a194/0x0083a1a4/0x0083a1b8` (ReadAudioVisual context) — verified `+0x18a8`

## Field table

| Offset | Type | Size | INI Section | INI Key | Default (YR) | Confidence |
|---|---|---|---|---|---|---|
| `+0x348` | `AnimTypeClass*` | 4 | `[General]` | `IronCurtainInvokeAnim=` | `IRONBLST` | VERIFIED |
| `+0x18a8` | int (color) | 4 | `[AudioVisual]` | `IronCurtainColor=` | unknown | VERIFIED offset |
| `+0xfa8` | `WarheadTypeClass*` | 4 | `[CombatDamage]` | `C4Warhead=` | `C4` | VERIFIED |
| `+0xfe8` | int (frames) | 4 | `[CombatDamage]` | `IronCurtainDuration=` | 750 | VERIFIED |

## Detailed field analysis

### `RulesClass + 0x348` — IronCurtainInvokeAnim

**Source**: `get_assembly_context 0x0066e244`, `get_xrefs_to 0x0083cda0`.

Assembly at site `0x0066e244` in `RulesClass__ReadGeneral`:
```asm
0066e22f: MOV EBX, dword ptr [ESI + 0x348]   ; load existing default
...
0066e244: PUSH 0x83cda0                        ; "IronCurtainInvokeAnim" string
0066e249: PUSH ECX
0066e24a: MOV ECX, EDI                         ; ECX = CCINIClass* (section object)
0066e24c: CALL 0x00528a10                       ; CCINIClass__ReadString
0066e251: TEST EAX, EAX
0066e253: JZ 0x0066e260                         ; if not found, keep EBX (prior value)
0066e255: LEA ECX, [ESP + 0x50]                ; string buffer
0066e259: CALL 0x00428b80                       ; AnimTypeClass__FindOrAllocate
0066e25e: JMP 0x0066e262
0066e260: MOV EAX, EBX                          ; use prior value if key absent
0066e262: ...
0066e26b: MOV dword ptr [ESI + 0x348], EAX     ; store AnimTypeClass* at +0x348
```

**Type**: `AnimTypeClass*` — pointer to the animation type.
**INI key**: `IronCurtainInvokeAnim=` in `[General]`.
**Default**: `IRONBLST` (confirmed from task manifest; the prior value loaded from `+0x348` before the read). This is the "IRONBLST" animation played when Iron Curtain is applied to a unit.

### `RulesClass + 0x18a8` — IronCurtainColor

**Source**: `get_assembly_context 0x0066b844`, `read_memory 0x0083a1a4`.

Assembly at site `0x0066b844` in `RulesClass__ReadAudioVisual`:
```asm
0066b844: PUSH 0x83a1a4                         ; "IronCurtainColor" string
...
0066b84c: CALL 0x005276d0                        ; CCINIClass__ReadColor (or similar)
0066b851: MOV dword ptr [ESI + 0x18a8], EAX    ; store result at +0x18a8
```

Adjacent fields in the same ReadAudioVisual function:
- `+0x18a4` = `LaserTargetColor` (stored at `0x0066b812` → `[ESI + 0x18a4]`)
- `+0x18a8` = `IronCurtainColor` (stored at `0x0066b851` → `[ESI + 0x18a8]`)
- `+0x18ac` = `BerserkColor` (stored at `0x0066b871` → `[ESI + 0x18ac]`)

**Type**: `int` (4 bytes) — a packed color value. The function at `0x005276d0` is a CCINIClass color-read variant that packs R/G/B into a single int, or it may store a palette index. Exact packing format is **YELLOW — unverified** without decompiling `0x005276d0`.
**INI key**: `IronCurtainColor=` in `[AudioVisual]`.
**Default**: Not determined from this analysis. Stock YR uses a golden/yellow color for the IC tint.

### `RulesClass + 0xfa8` — C4Warhead (used by infantry instakill path)

**Source**: `decompile_function 0x0066bbb0` (RulesClass__ReadCombatDamage).

From the decompile (verbatim excerpt):
```c
uVar3 = *(undefined4 *)(param_1 + 0xfa8);    // load existing value (default)
// ... ReadString "C4Warhead"
if (iVar2 != 0) {
    uVar3 = WarheadTypeClass__FindOrAllocate();
}
*(undefined4 *)(param_1 + 0xfa8) = uVar3;    // store WarheadTypeClass*
```

**Type**: `WarheadTypeClass*`
**INI key**: `C4Warhead=` in `[CombatDamage]`. String `s_C4Warhead_0083b1d4` confirmed at data ref `0x0066c32c`.
**Default**: `C4` (INI default in stock `rulesmd.ini`).

**Usage by Iron Curtain**: `InfantryClass__IronCurtain` (0x00522600) reads this field and passes the warhead pointer to the instakill damage call:
```c
*(undefined4 *)(g_RulesClass_Instance + 0xfa8)  // = C4Warhead ptr
```
Verified via `decompile_function 0x00522600`.

**IMPORTANT CORRECTION to task preflight**: The preflight note said "likely a huge damage constant or IronCurtainDamage INI key." This is incorrect. `+0xfa8` is `C4Warhead` (a WarheadTypeClass pointer), NOT a damage integer. The warhead is used alongside the unit's own `Strength` as the damage value. The observable result is an instakill with the C4 warhead type.

### `RulesClass + 0xfe8` — IronCurtainDuration

**Source**: `decompile_function 0x0066bbb0` (RulesClass__ReadCombatDamage).

From the decompile (verbatim excerpt):
```c
local_d8 = (double)CONCAT44(*(undefined4 *)(param_1 + 0xfe8), s_IronCurtainDuration_0083b0b8);
// ...
uVar3 = CCINIClass__ReadInt();
*(undefined4 *)(param_1 + 0xfe8) = uVar3;
```

**Type**: `int` (frames)
**INI key**: `IronCurtainDuration=` in `[CombatDamage]`. String `s_IronCurtainDuration_0083b0b8` confirmed at data ref `0x0066c63e` (also the task anchor).
**Default**: 750 frames (≈50 seconds at 15 fps, as noted in task manifest and confirmed by prior decode task #6).

**Usage**: `TechnoClass__IronCurtain` (0x0070e2b0) stores this duration into `TechnoClass + 0x194` when IC is applied. `TechnoClass__IsIronCurtainActive` compares `+0x18c + +0x194` against `g_CurrentFrameCounter` to determine active state.

## Surrounding RulesClass fields (for layout context)

From `decompile_function 0x0066bbb0`, sequential reads near `+0xfa8`:

| Offset | INI Key | Type | Notes |
|---|---|---|---|
| `+0xf84` | `FlameDamage=` | `WarheadTypeClass*` | not IC-related |
| `+0xf88` | `FlameDamage2=` | `WarheadTypeClass*` | not IC-related |
| `+0xfa8` | `C4Warhead=` | `WarheadTypeClass*` | **IC instakill warhead** |
| `+0xfac` | `CrushWarhead=` | `WarheadTypeClass*` | not IC-related |
| `+0xfb0` | `V3Warhead=` | `WarheadTypeClass*` | not IC-related |
| ... | | | |
| `+0xfe8` | `IronCurtainDuration=` | `int` (frames) | **IC duration** |
| `+0xfec` | `PsychicRevealRadius=` | `int` | not IC-related |

## Summary layout (IC-relevant fields only)

```
RulesClass:
  +0x348    AnimTypeClass* IronCurtainInvokeAnim  [General]
  +0x18a8   int            IronCurtainColor        [AudioVisual]
  +0xfa8    WarheadTypeClass* C4Warhead            [CombatDamage]
  +0xfe8    int            IronCurtainDuration     [CombatDamage], default 750
```

## Active in YR: Yes

All four fields are read from the standard `rulesmd.ini` section during game startup. No TS-legacy gate.
