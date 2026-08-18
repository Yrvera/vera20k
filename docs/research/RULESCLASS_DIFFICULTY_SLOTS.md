# RulesClass Embedded DifficultyClass Slots

**Parent:** `RulesClass` (singleton at `0x008871E0`, size `0x18C0`)
**Sibling report:** `AI_DIFFICULTY_SYSTEM.md` (DifficultyClass struct layout)
**Confidence:** HIGH (confirmed from raw x86 `LEA EDX` / `PUSH` context at the three call sites).

## Slot base offsets

| Slot | Offset in RulesClass | Section | INI section name addr |
|------|---:|---|---|
| Easy | **`0x1538`** | `[Easy]` | `0x00818134` |
| Normal | **`0x1588`** | `[Normal]` | `0x0081BB60` |
| Difficult | **`0x15D8`** | `[Difficult]` | `0x0083A0C4` |

Each slot is `0x50` bytes = 80 decimal (9 × double + 3 × bool + padding to 8-byte alignment).

Total difficulty block: `0x1538–0x1627` = 240 bytes contiguous.

## Evidence

At call site `0x00668F02` (Easy), the raw assembly preceding the `CALL 0x0066d270`:

```asm
00668eed  PUSH  ESI                       ; push `section_ini_handler_ptr`
00668eee  MOV   ECX, EDI                  ; ECX = this pointer for previous call
00668ef0  CALL  0x00679a10                ; Type_Read_INI_All (step 21 of dispatcher)
00668ef5  PUSH  0x818134                  ; PUSH "Easy" string address
00668efa  LEA   EDX, [EDI + 0x1538]       ; EDX = &Rules->Easy  ← SLOT OFFSET
00668f00  MOV   ECX, ESI                  ; ECX = section_ini_handler
00668f02  CALL  0x0066d270                ; DifficultyClass::Read_INI
```

The `LEA EDX, [EDI + 0xNNNN]` at each call gives the slot base — EDI holds the
RulesClass pointer throughout the dispatcher.

| Call site | LEA instruction | Push (section) | Resolved slot |
|---|---|---|---|
| `0x00668F02` | `LEA EDX, [EDI + 0x1538]` | `PUSH 0x818134` ("Easy") | Easy @ 0x1538 |
| `0x00668F14` | `LEA EDX, [EDI + 0x1588]` | `PUSH 0x81BB60` ("Normal") | Normal @ 0x1588 |
| `0x00668F26` | `LEA EDX, [EDI + 0x15D8]` | `PUSH 0x83A0C4` ("Difficult") | Difficult @ 0x15D8 |

## Why Ghidra's decomp was ambiguous

`FUN_0066D270` uses `__fastcall`, so the three args map to:
- `ECX` = arg1 (unused as a real input, but present in the signature)
- `EDX` = arg2 (the `DifficultyClass*` output — our slot pointer)
- stack = arg3 (the INI section name, `"Easy"`/`"Normal"`/`"Difficult"`)

The decompiled call-site view only shows ONE explicit argument (`FUN_0066d270(&DAT_00818134)`)
because Ghidra couldn't reconcile the ECX/EDX register setup with the call signature.
Reading the raw x86 resolves it unambiguously.

## DifficultyClass layout (recap)

`sizeof(DifficultyClass) = 0x50` bytes, verified:
`0x1588 - 0x1538 = 0x50`, `0x15D8 - 0x1588 = 0x50`.

| Offset | Size | Type | Field | INI key |
|------:|-----:|------|-------|---------|
| +0x00 | 8 | double | FirePower | `FirePower` |
| +0x08 | 8 | double | Groundspeed | `Groundspeed` |
| +0x10 | 8 | double | Airspeed | `Airspeed` |
| +0x18 | 8 | double | Armor | `Armor` |
| +0x20 | 8 | double | ROF | (parsed via `PTR_LAB_00825478` at src `0x00825478`) |
| +0x28 | 8 | double | Cost | (parsed via `DAT_00825470`) |
| +0x30 | 8 | double | BuildTime | `BuildTime` |
| +0x38 | 8 | double | RepairDelay | `RepairDelay` (default `0.02`) |
| +0x40 | 8 | double | BuildDelay | `BuildDelay` (default `0.03`) |
| +0x48 | 1 | bool | BuildSlowdown | `BuildSlowdown` |
| +0x49 | 1 | bool | DestroyWalls | `DestroyWalls` (default `true`) |
| +0x4A | 1 | bool | ContentScan | `ContentScan` |
| +0x4B–0x4F | 5 | pad | (alignment) | — |

Source: `FUN_0066D270` decomp (saved at `scripts/research/_decomp/FUN_0066d270.c`).

## Absolute offsets (all three slots × all 12 fields)

For quick reference:

| Field | Easy (0x1538 +) | Normal (0x1588 +) | Difficult (0x15D8 +) |
|-------|---:|---:|---:|
| FirePower | `0x1538` | `0x1588` | `0x15D8` |
| Groundspeed | `0x1540` | `0x1590` | `0x15E0` |
| Airspeed | `0x1548` | `0x1598` | `0x15E8` |
| Armor | `0x1550` | `0x15A0` | `0x15F0` |
| ROF | `0x1558` | `0x15A8` | `0x15F8` |
| Cost | `0x1560` | `0x15B0` | `0x1600` |
| BuildTime | `0x1568` | `0x15B8` | `0x1608` |
| RepairDelay | `0x1570` | `0x15C0` | `0x1610` |
| BuildDelay | `0x1578` | `0x15C8` | `0x1618` |
| BuildSlowdown | `0x1580` | `0x15D0` | `0x1620` |
| DestroyWalls | `0x1581` | `0x15D1` | `0x1621` |
| ContentScan | `0x1582` | `0x15D2` | `0x1622` |

## Consumer pattern

Code that applies difficulty reaches for the right slot via
`g_RulesClass_Instance + <base> + <field_offset>`. The base is selected
at runtime by `HouseClass::IQ_Difficulty` or `ScenarioClass::Difficulty`
(0 = Easy, 1 = Normal, 2 = Difficult), so the effective address is:

```
slot_base = 0x1538 + difficulty * 0x50
```

Any system that consumes a difficulty multiplier (production speed, armor,
ROF, cost, etc.) indexes into this block.

## Cross-reference with ctor defaults

The constructor (FUN_00665650) zeros these 240 bytes implicitly via
`operator_new(0x18C0)`. `FUN_0066D270` then fills each slot with INI-parsed
values using `CCINIClass__ReadDouble(..., default=1.0)` for all 9 doubles
and `CCINIClass__ReadBool(..., default=0/1)` for the three bools. So:

- Easy/Normal/Difficult all **default to `1.0`** for every multiplier (FirePower/Groundspeed/Airspeed/Armor/ROF/Cost/BuildTime all neutral).
- RepairDelay default `0.02` (sec → ~0.3 frame @ 15Hz).
- BuildDelay default `0.03`.
- BuildSlowdown default `false`.
- DestroyWalls default `true`.
- ContentScan default `false`.

INI override:

```
[Easy]  Armor=1.2  ROF=.8   BuildTime=.8 ...
[Normal] (all 1.0 unless overridden)
[Difficult] Armor=.8  ROF=1.2  BuildTime=1.0  RepairDelay=.05  BuildDelay=.1  BuildSlowdown=yes ...
```

## Sources

- Dispatcher: `FUN_00668BF0` (decomp saved at `scripts/research/_decomp/FUN_0066d270.c`-adjacent batch)
- Raw assembly: `get_assembly_context` at call sites `0x00668F02` / `0x00668F14` / `0x00668F26`
- DifficultyClass callee: `FUN_0066D270` (12-field reader, signature `(undefined4 param_1, double *param_2, undefined4 param_3)`)
- String refs verified: `0x818134` = "Easy", `0x81BB60` = "Normal", `0x83A0C4` = "Difficult"
